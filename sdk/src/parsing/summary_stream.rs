use super::summarize::parse_kagi_failure_payload;
use serde_json::Value;

use crate::{
    error::KagiError,
    routing::{EndpointId, ParserShape},
    session_web::models::SummaryStreamResponse,
};

pub fn parse_summary_stream_response(
    endpoint: EndpointId,
    raw_body: &str,
) -> Result<SummaryStreamResponse, KagiError> {
    if let Some((code, message)) = parse_kagi_failure_payload(raw_body) {
        return Err(KagiError::ApiFailure {
            endpoint,
            status: 200,
            code,
            message,
        });
    }

    if body_looks_like_html(raw_body) {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: "expected framed summary stream but received HTML".to_string(),
        });
    }

    if looks_like_kagi_prefixed_stream(raw_body) {
        return parse_kagi_prefixed_stream(endpoint, raw_body);
    }

    parse_sse_stream(endpoint, raw_body)
}

fn parse_kagi_prefixed_stream(
    endpoint: EndpointId,
    raw_body: &str,
) -> Result<SummaryStreamResponse, KagiError> {
    let mut chunks = Vec::new();
    let mut saw_prefixed_frame = false;

    for frame in iter_prefixed_frames(raw_body) {
        if frame.is_empty() {
            continue;
        }

        let (prefix, payload) = frame
            .split_once(':')
            .ok_or_else(|| KagiError::ResponseParse {
                endpoint,
                parser: ParserShape::Stream,
                reason: format!("malformed stream frame without prefix delimiter: `{frame}`"),
            })?;

        saw_prefixed_frame = true;
        if let Some(chunk) = parse_prefixed_frame_payload(endpoint, prefix, payload)? {
            chunks.push(chunk);
        }
    }

    if !saw_prefixed_frame {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: "stream did not contain Kagi prefixed frames".to_string(),
        });
    }

    if chunks.is_empty() {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: "stream did not contain any summary text chunks".to_string(),
        });
    }

    let text = chunks.concat();
    Ok(SummaryStreamResponse { chunks, text })
}

fn parse_prefixed_frame_payload(
    endpoint: EndpointId,
    prefix: &str,
    payload: &str,
) -> Result<Option<String>, KagiError> {
    let normalized_prefix = prefix.trim();
    let payload_text = payload.trim_end_matches(['\r', '\n']);

    if let Some((code, message)) = parse_kagi_failure_payload(payload_text) {
        return Err(KagiError::ApiFailure {
            endpoint,
            status: 200,
            code,
            message,
        });
    }

    match normalized_prefix {
        "hi" => Ok(None),
        "new_message.json" => parse_required_json_frame(endpoint, normalized_prefix, payload_text),
        "update" | "final" => parse_text_or_json_frame(endpoint, normalized_prefix, payload_text),
        _ => Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: format!("unsupported stream frame prefix `{normalized_prefix}`"),
        }),
    }
}

fn parse_required_json_frame(
    endpoint: EndpointId,
    prefix: &str,
    payload: &str,
) -> Result<Option<String>, KagiError> {
    let normalized_payload = strip_optional_json_leading_space(payload);
    if normalized_payload.is_empty() {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: format!("stream frame `{prefix}` did not contain JSON payload"),
        });
    }

    let parsed_json = serde_json::from_str::<Value>(normalized_payload).map_err(|source| {
        KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: format!("stream frame `{prefix}` contained invalid JSON: {source}"),
        }
    })?;

    Ok(extract_stream_text(&parsed_json))
}

fn parse_text_or_json_frame(
    endpoint: EndpointId,
    prefix: &str,
    payload: &str,
) -> Result<Option<String>, KagiError> {
    if payload == "[DONE]" {
        return Ok(None);
    }

    if looks_like_json_payload(payload) {
        return parse_required_json_frame(endpoint, prefix, payload);
    }

    Ok(Some(payload.to_string()))
}

fn parse_sse_stream(
    endpoint: EndpointId,
    raw_body: &str,
) -> Result<SummaryStreamResponse, KagiError> {
    let mut chunks = Vec::new();
    let mut stream_event_name = String::from("message");
    let mut stream_event_data = Vec::new();
    let mut saw_sse_field = false;

    for line in raw_body.lines() {
        let raw_line = line.trim_end_matches('\r');
        let trimmed_line = raw_line.trim();

        if trimmed_line.is_empty() {
            flush_sse_event(
                endpoint,
                &stream_event_name,
                &mut stream_event_data,
                &mut chunks,
            )?;
            stream_event_name.clear();
            stream_event_name.push_str("message");
            continue;
        }

        if trimmed_line.starts_with(':') {
            continue;
        }

        let (field_name, field_value) =
            parse_sse_field(trimmed_line).ok_or_else(|| KagiError::ResponseParse {
                endpoint,
                parser: ParserShape::Stream,
                reason: format!("encountered non-SSE line: `{trimmed_line}`"),
            })?;
        saw_sse_field = true;

        match field_name {
            "event" => {
                stream_event_name.clear();
                stream_event_name.push_str(&field_value);
            }
            "data" => {
                if should_flush_before_appending_data(&stream_event_data, &field_value) {
                    flush_sse_event(
                        endpoint,
                        &stream_event_name,
                        &mut stream_event_data,
                        &mut chunks,
                    )?;
                }

                stream_event_data.push(field_value);
            }
            "id" | "retry" => {}
            unknown_field => {
                return Err(KagiError::ResponseParse {
                    endpoint,
                    parser: ParserShape::Stream,
                    reason: format!("unsupported SSE field `{unknown_field}`"),
                });
            }
        }
    }

    flush_sse_event(
        endpoint,
        &stream_event_name,
        &mut stream_event_data,
        &mut chunks,
    )?;

    if !saw_sse_field {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: "response did not contain supported stream frames".to_string(),
        });
    }

    if chunks.is_empty() {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: "stream did not contain any summary text chunks".to_string(),
        });
    }

    let text = chunks.concat();
    Ok(SummaryStreamResponse { chunks, text })
}

fn flush_sse_event(
    endpoint: EndpointId,
    event_name: &str,
    event_data: &mut Vec<String>,
    chunks: &mut Vec<String>,
) -> Result<(), KagiError> {
    if event_data.is_empty() {
        return Ok(());
    }

    let data_payload = event_data.join("\n");
    event_data.clear();

    if data_payload.is_empty() || data_payload == "[DONE]" {
        return Ok(());
    }

    if event_name == "error" {
        let (code, message) = parse_error_payload(&data_payload);
        return Err(KagiError::ApiFailure {
            endpoint,
            status: 200,
            code,
            message,
        });
    }

    if let Some(text_chunk) = parse_sse_stream_chunk(endpoint, &data_payload)? {
        chunks.push(text_chunk);
    }

    Ok(())
}

fn parse_sse_stream_chunk(
    endpoint: EndpointId,
    payload: &str,
) -> Result<Option<String>, KagiError> {
    if !looks_like_json_payload(payload) {
        return Ok(Some(payload.to_string()));
    }

    let parsed_json =
        serde_json::from_str::<Value>(payload).map_err(|source| KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: format!("invalid stream JSON event: {source}"),
        })?;

    Ok(extract_stream_text(&parsed_json))
}

fn parse_sse_field(line: &str) -> Option<(&str, String)> {
    let (name, value_with_optional_space) = match line.split_once(':') {
        Some((name, value)) => (name, value),
        None => (line, ""),
    };

    if name.is_empty() {
        return None;
    }

    let value = value_with_optional_space
        .strip_prefix(' ')
        .unwrap_or(value_with_optional_space)
        .to_string();

    Some((name, value))
}

fn parse_error_payload(payload: &str) -> (Option<String>, String) {
    let json_error = serde_json::from_str::<Value>(payload);
    let Ok(value) = json_error else {
        return (None, payload.to_string());
    };

    let code = value
        .get("code")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            value
                .get("error")
                .and_then(|error| error.get("code"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        });

    let message = value
        .get("message")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            value
                .get("error")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| payload.to_string());

    (code, message)
}

fn extract_stream_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    if let Some(text) = value.get("delta").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    if let Some(text) = value.get("content").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    if let Some(text) = value.get("chunk").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    if let Some(text) = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|first_choice| first_choice.get("delta"))
        .and_then(|delta| delta.get("content"))
        .and_then(Value::as_str)
    {
        return Some(text.to_string());
    }

    if let Some(text) = value.get("summary").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    if let Some(text) = value
        .get("message")
        .and_then(|message| message.get("text"))
        .and_then(Value::as_str)
    {
        return Some(text.to_string());
    }

    None
}

fn iter_prefixed_frames(raw_body: &str) -> Vec<&str> {
    if raw_body.contains('\0') {
        raw_body
            .split('\0')
            .map(|frame| frame.trim_end_matches(['\r', '\n']))
            .collect()
    } else {
        raw_body.lines().collect()
    }
}

fn looks_like_kagi_prefixed_stream(raw_body: &str) -> bool {
    if raw_body.contains('\0') {
        return true;
    }

    raw_body.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("hi:")
            || trimmed.starts_with("new_message.json:")
            || trimmed.starts_with("update:")
            || trimmed.starts_with("final:")
    })
}

fn body_looks_like_html(raw_body: &str) -> bool {
    let normalized = raw_body.trim_start().to_ascii_lowercase();
    normalized.starts_with("<!doctype html") || normalized.starts_with("<html")
}

fn should_flush_before_appending_data(current_data: &[String], next_data_line: &str) -> bool {
    if current_data.is_empty() {
        return false;
    }

    if current_data
        .iter()
        .all(|line| looks_like_complete_json_line(line) || line == "[DONE]")
    {
        return looks_like_complete_json_line(next_data_line) || next_data_line == "[DONE]";
    }

    false
}

fn looks_like_complete_json_line(line: &str) -> bool {
    let trimmed = line.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

fn looks_like_json_payload(payload: &str) -> bool {
    let normalized = strip_optional_json_leading_space(payload);
    normalized.starts_with('{') || normalized.starts_with('[')
}

fn strip_optional_json_leading_space(payload: &str) -> &str {
    let without_prefix_space = payload.strip_prefix(' ').unwrap_or(payload);
    without_prefix_space.trim_end_matches(['\r', '\n'])
}
