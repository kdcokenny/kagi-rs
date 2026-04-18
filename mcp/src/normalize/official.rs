use serde_json::Value;

use crate::{
    error::ToolFailure,
    schema::{SearchResultCard, SearchToolOutput, SummarizeToolOutput},
};

pub fn normalize_search(data: Value, limit: usize) -> Result<SearchToolOutput, ToolFailure> {
    let Some(raw_items) = extract_search_items(&data) else {
        return Err(ToolFailure::parse_drift(
            "official search payload did not include a `results` array",
        ));
    };

    let mut results = Vec::new();
    for raw_item in raw_items {
        if results.len() >= limit {
            break;
        }

        if let Some(card) = normalize_search_item(raw_item) {
            results.push(card);
        }
    }

    if raw_items.is_empty() {
        return Ok(SearchToolOutput {
            results,
            total_returned: 0,
        });
    }

    if results.is_empty() {
        return Err(ToolFailure::parse_drift(
            "official search results were present but no item exposed `title` + `url`",
        ));
    }

    Ok(SearchToolOutput {
        total_returned: results.len(),
        results,
    })
}

pub fn normalize_summarize(
    data: Value,
    source_url_hint: Option<&str>,
) -> Result<SummarizeToolOutput, ToolFailure> {
    let Some(payload) = extract_summary_object(&data) else {
        return Err(ToolFailure::parse_drift(
            "official summarize payload was not a JSON object",
        ));
    };

    let Some(markdown) = extract_first_string(
        payload,
        &["markdown", "summary_markdown", "output_markdown", "summary"],
    ) else {
        return Err(ToolFailure::parse_drift(
            "official summarize payload did not contain markdown text",
        ));
    };

    let text = extract_first_string(payload, &["text", "summary_text", "plain_text"]);

    let source_url = extract_first_string(payload, &["source_url", "url", "source"])
        .or_else(|| source_url_hint.map(ToOwned::to_owned));

    Ok(SummarizeToolOutput {
        markdown,
        text,
        source_url,
    })
}

fn extract_search_items(data: &Value) -> Option<&Vec<Value>> {
    match data {
        Value::Array(items) => Some(items),
        Value::Object(map) => {
            let candidates = ["results", "search_results", "organic_results", "items"];
            for key in candidates {
                if let Some(items) = map.get(key).and_then(Value::as_array) {
                    return Some(items);
                }
            }

            map.get("data")
                .and_then(Value::as_object)
                .and_then(|nested| nested.get("results"))
                .and_then(Value::as_array)
        }
        _ => None,
    }
}

fn normalize_search_item(value: &Value) -> Option<SearchResultCard> {
    let object = value.as_object()?;
    let title = extract_first_string_from_object(object, &["title", "name"])?;
    let url = extract_first_string_from_object(object, &["url", "link"])?;
    let snippet = extract_first_string_from_object(object, &["snippet", "description", "desc"]);

    Some(SearchResultCard {
        title,
        url,
        snippet,
    })
}

fn extract_summary_object(data: &Value) -> Option<&serde_json::Map<String, Value>> {
    match data {
        Value::Object(payload) => Some(payload),
        _ => None,
    }
}

fn extract_first_string(payload: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    extract_first_string_from_object(payload, keys)
}

fn extract_first_string_from_object(
    payload: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<String> {
    for key in keys {
        let Some(raw) = payload.get(*key) else {
            continue;
        };

        let Some(text) = raw.as_str() else {
            continue;
        };

        if text.trim().is_empty() {
            continue;
        }

        return Some(text.to_string());
    }

    None
}
