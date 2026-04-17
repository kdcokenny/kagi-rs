use serde_json::{Map, Value};

use crate::{
    error::KagiError,
    routing::{EndpointId, ParserShape},
    session_web::models::SummarizeResponse,
};

pub fn parse_summarize_response(
    endpoint: EndpointId,
    raw_body: &str,
) -> Result<SummarizeResponse, KagiError> {
    let parsed_json: Value =
        serde_json::from_str(raw_body).map_err(|source| KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::JsonEnvelope,
            reason: format!("malformed default summarize JSON: {source}"),
        })?;

    let root_object = parsed_json
        .as_object()
        .ok_or_else(|| KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::JsonEnvelope,
            reason: "malformed default summarize JSON: expected top-level object".to_string(),
        })?;

    if let Some((code, message)) = extract_kagi_failure_from_object(root_object) {
        return Err(KagiError::ApiFailure {
            endpoint,
            status: 200,
            code,
            message,
        });
    }

    let payload = root_object
        .get("data")
        .and_then(Value::as_object)
        .unwrap_or(root_object);

    if let Some((code, message)) = extract_kagi_failure_from_object(payload) {
        return Err(KagiError::ApiFailure {
            endpoint,
            status: 200,
            code,
            message,
        });
    }

    let markdown = extract_required_markdown(payload).ok_or_else(|| KagiError::ResponseParse {
        endpoint,
        parser: ParserShape::JsonEnvelope,
        reason: "malformed default summarize JSON: missing markdown text".to_string(),
    })?;

    let text =
        extract_optional_string(payload, "text").map_err(|reason| KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::JsonEnvelope,
            reason,
        })?;
    let status =
        extract_optional_string(payload, "status").map_err(|reason| KagiError::ResponseParse {
            endpoint,
            parser: ParserShape::JsonEnvelope,
            reason,
        })?;
    let metadata = extract_metadata(payload).map_err(|reason| KagiError::ResponseParse {
        endpoint,
        parser: ParserShape::JsonEnvelope,
        reason,
    })?;

    Ok(SummarizeResponse {
        markdown,
        text,
        status,
        metadata,
    })
}

pub(crate) fn parse_kagi_failure_payload(raw_body: &str) -> Option<(Option<String>, String)> {
    let parsed: Value = serde_json::from_str(raw_body).ok()?;
    let object = parsed.as_object()?;
    extract_kagi_failure_from_object(object)
}

fn extract_required_markdown(object: &Map<String, Value>) -> Option<String> {
    for field in [
        "markdown",
        "summary_markdown",
        "output_markdown",
        "summary",
        "output",
    ] {
        if let Some(value) = object.get(field).and_then(Value::as_str) {
            return Some(value.to_string());
        }
    }

    None
}

fn extract_optional_string(
    object: &Map<String, Value>,
    field_name: &'static str,
) -> Result<Option<String>, String> {
    let Some(value) = object.get(field_name) else {
        return Ok(None);
    };

    match value {
        Value::String(text) => Ok(Some(text.clone())),
        Value::Null => Ok(None),
        _ => Err(format!(
            "malformed default summarize JSON: `{field_name}` must be a string when present"
        )),
    }
}

fn extract_metadata(object: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    let Some(value) = object.get("metadata") else {
        return Ok(Map::new());
    };

    match value {
        Value::Object(metadata) => Ok(metadata.clone()),
        Value::Null => Ok(Map::new()),
        _ => Err("malformed default summarize JSON: `metadata` must be an object".to_string()),
    }
}

fn extract_kagi_failure_from_object(
    object: &Map<String, Value>,
) -> Option<(Option<String>, String)> {
    if !looks_like_failure(object) {
        return None;
    }

    Some(extract_failure_details(object))
}

fn looks_like_failure(object: &Map<String, Value>) -> bool {
    if object.get("error").is_some_and(is_failure_marker) {
        return true;
    }

    if object
        .get("success")
        .is_some_and(|success| success.as_bool() == Some(false))
    {
        return true;
    }

    object
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| {
            let normalized_status = status.trim().to_ascii_lowercase();
            matches!(normalized_status.as_str(), "error" | "failed" | "failure")
        })
}

fn extract_failure_details(object: &Map<String, Value>) -> (Option<String>, String) {
    let code = object
        .get("code")
        .and_then(value_to_code)
        .or_else(|| object.get("error").and_then(value_to_code));

    let message = object
        .get("message")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            object
                .get("error")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            object
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "unknown Kagi failure".to_string());

    (code, message)
}

fn value_to_code(value: &Value) -> Option<String> {
    if let Some(code) = value.as_str() {
        return Some(code.to_string());
    }

    if let Some(code) = value.as_i64() {
        return Some(code.to_string());
    }

    value
        .as_object()
        .and_then(|object| object.get("code"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn is_failure_marker(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(is_error) => *is_error,
        Value::String(message) => !message.trim().is_empty(),
        Value::Number(number) => match number.as_i64() {
            Some(numeric) => numeric != 0,
            None => true,
        },
        Value::Array(items) => !items.is_empty(),
        Value::Object(_) => true,
    }
}
