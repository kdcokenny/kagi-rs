use scraper::{Html, Selector};

use crate::{
    error::KagiError,
    routing::{EndpointId, ParserShape},
    session_web::models::{HtmlSearchResponse, HtmlSearchResult},
};

pub fn parse_html_search_response(
    endpoint: EndpointId,
    html_body: &str,
) -> Result<HtmlSearchResponse, KagiError> {
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

            parsed_results.push(HtmlSearchResult {
                title,
                url: href.to_string(),
                snippet,
            });
        }

        if !parsed_results.is_empty() {
            return Ok(HtmlSearchResponse {
                results: deduplicate_results(parsed_results),
            });
        }
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

fn deduplicate_results(results: Vec<HtmlSearchResult>) -> Vec<HtmlSearchResult> {
    let mut deduped = Vec::with_capacity(results.len());

    for result in results {
        let exists = deduped.iter().any(|existing: &HtmlSearchResult| {
            existing.url == result.url && existing.title == result.title
        });

        if !exists {
            deduped.push(result);
        }
    }

    deduped
}
