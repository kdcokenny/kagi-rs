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
    if body_looks_like_html(raw_body) {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: "expected SSE stream but received HTML".to_string(),
        });
    }

    let mut chunks = Vec::new();
    let mut stream_event_name = String::from("message");
    let mut stream_event_data = Vec::new();
    let mut saw_sse_field = false;

    for line in raw_body.lines() {
        let raw_line = line.trim_end_matches('\r');
        let trimmed_line = raw_line.trim();

        if trimmed_line.is_empty() {
            flush_stream_event(
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
                    flush_stream_event(
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

    flush_stream_event(
        endpoint,
        &stream_event_name,
        &mut stream_event_data,
        &mut chunks,
    )?;

    if !saw_sse_field {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: "response did not contain SSE fields".to_string(),
        });
    }

    if chunks.is_empty() {
        return Err(KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: "stream did not contain any summary text chunks".to_string(),
        });
    }

    let text = chunks.join("");
    Ok(SummaryStreamResponse { chunks, text })
}

fn flush_stream_event(
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

    if data_payload.trim().is_empty() || data_payload.trim() == "[DONE]" {
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

    if let Some(text_chunk) = parse_stream_chunk(endpoint, &data_payload)? {
        if !text_chunk.trim().is_empty() {
            chunks.push(text_chunk);
        }
    }

    Ok(())
}

fn parse_stream_chunk(endpoint: EndpointId, payload: &str) -> Result<Option<String>, KagiError> {
    if !payload.trim_start().starts_with('{') {
        return Ok(Some(payload.to_string()));
    }

    let parsed: Value =
        serde_json::from_str(payload).map_err(|source| KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::Stream,
            reason: format!("invalid stream JSON event: {source}"),
        })?;

    Ok(extract_stream_text(&parsed))
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
    let json_error: Result<Value, _> = serde_json::from_str(payload);
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
        .all(|line| looks_like_complete_json_line(line) || line.trim() == "[DONE]")
    {
        return looks_like_complete_json_line(next_data_line) || next_data_line.trim() == "[DONE]";
    }

    false
}

fn looks_like_complete_json_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('{') && trimmed.ends_with('}')
}

fn extract_stream_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    if let Some(text) = value.get("delta").and_then(Value::as_str) {
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

    None
}
