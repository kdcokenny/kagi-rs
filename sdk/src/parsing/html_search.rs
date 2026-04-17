use scraper::{Html, Selector};

use crate::{
    error::KagiError,
    routing::{EndpointId, ParserShape},
    session_web::models::{SearchResponse, SearchResult},
};

pub fn parse_html_search_response(
    endpoint: EndpointId,
    html_body: &str,
) -> Result<SearchResponse, KagiError> {
    let document = Html::parse_document(html_body);

    let item_selectors = [
        "div.search-result",
        "div.__sri",
        "div.__srgi",
        "section.__srgi",
    ];
    let title_selector = parse_selector("a.__sri_title_link")?;
    let snippet_selector = parse_selector(".__sri-desc")?;

    let mut parsed_results = Vec::new();

    for item_selector in item_selectors {
        let selector = parse_selector(item_selector)?;
        for item in document.select(&selector) {
            let Some(link) = item.select(&title_selector).next() else {
                continue;
            };
            let Some(href) = link.value().attr("href") else {
                continue;
            };

            if !href.starts_with("http://") && !href.starts_with("https://") {
                continue;
            }

            let title = collect_text(&link);
            if title.is_empty() {
                continue;
            }

            let snippet = item
                .select(&snippet_selector)
                .next()
                .map(|node| collect_text(&node))
                .filter(|text| !text.is_empty());

            parsed_results.push(SearchResult {
                title,
                url: href.to_string(),
                snippet,
            });
        }

        if !parsed_results.is_empty() {
            return Ok(SearchResponse {
                results: deduplicate_results(parsed_results),
            });
        }
    }

    if looks_like_empty_search_results(&document, html_body)? {
        return Ok(SearchResponse {
            results: Vec::new(),
        });
    }

    Err(KagiError::ResponseParse {
        endpoint,
        parser: ParserShape::Html,
        reason: "response did not contain Kagi search-result markers (`.__sri_title_link`)"
            .to_string(),
    })
}

fn parse_selector(raw: &str) -> Result<Selector, KagiError> {
    Selector::parse(raw).map_err(|source| KagiError::InvalidClientConfiguration {
        reason: format!("invalid built-in selector `{raw}`: {source}"),
    })
}

fn collect_text(element: &scraper::ElementRef<'_>) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn deduplicate_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut deduped = Vec::with_capacity(results.len());

    for result in results {
        let exists = deduped.iter().any(|existing: &SearchResult| {
            existing.url == result.url && existing.title == result.title
        });

        if !exists {
            deduped.push(result);
        }
    }

    deduped
}

fn looks_like_empty_search_results(document: &Html, raw_html: &str) -> Result<bool, KagiError> {
    let empty_state_selector = parse_selector(
        ".search-no-results, .__search-no-results, .__empty-results, .__no-results",
    )?;
    if document.select(&empty_state_selector).next().is_some() {
        return Ok(true);
    }

    let normalized = raw_html.to_ascii_lowercase();
    let has_no_results_phrase = normalized.contains("no results")
        || normalized.contains("no results found")
        || normalized.contains("did not match any documents");

    let has_search_page_markers = normalized.contains("/html/search")
        || normalized.contains("name=\"q\"")
        || normalized.contains("search-input");

    Ok(has_no_results_phrase && has_search_page_markers)
}
