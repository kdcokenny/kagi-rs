use kagi_sdk::session_web::models::{SearchResponse, SummarizeResponse};

use crate::schema::{SearchResultCard, SearchToolOutput, SummarizeToolOutput};

pub fn normalize_search(response: SearchResponse, limit: usize) -> SearchToolOutput {
    let mut results = Vec::new();
    for item in response.results {
        if results.len() >= limit {
            break;
        }

        results.push(SearchResultCard {
            title: item.title,
            url: item.url,
            snippet: item.snippet,
        });
    }

    SearchToolOutput {
        total_returned: results.len(),
        results,
    }
}

pub fn normalize_summarize(
    response: SummarizeResponse,
    source_url_hint: Option<&str>,
) -> SummarizeToolOutput {
    let source_url = response
        .metadata
        .get("source_url")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| source_url_hint.map(ToOwned::to_owned));

    SummarizeToolOutput {
        markdown: response.markdown,
        text: response.text,
        source_url,
    }
}
