use kagi_sdk::{official_api::models::SearchRequest, BotToken, ClientConfig, KagiClient};
use serde_json::Value;
use url::Url;

fn required_env_or_skip(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ if std::env::var_os("GITHUB_ACTIONS").is_none() => {
            eprintln!("Skipping live_official test: {name} is not set.");
            None
        }
        _ => panic!("{name} must be set for GitHub Actions live test runs."),
    }
}

fn live_config() -> ClientConfig {
    let Ok(raw_base_url) = std::env::var("KAGI_BASE_URL") else {
        return ClientConfig::default();
    };

    if raw_base_url.trim().is_empty() {
        return ClientConfig::default();
    }

    let parsed_base_url =
        Url::parse(&raw_base_url).expect("KAGI_BASE_URL must be a valid absolute URL.");
    ClientConfig::default().with_base_url(parsed_base_url)
}

fn extract_search_items(data: &Value) -> Option<&[Value]> {
    match data {
        Value::Array(items) => Some(items.as_slice()),
        Value::Object(map) => {
            for key in ["results", "search_results", "organic_results", "items"] {
                if let Some(items) = map.get(key).and_then(Value::as_array) {
                    return Some(items.as_slice());
                }
            }

            map.get("data")
                .and_then(Value::as_object)
                .and_then(|nested| nested.get("results"))
                .and_then(Value::as_array)
                .map(Vec::as_slice)
        }
        _ => None,
    }
}

fn extract_first_nonblank<'a>(
    map: &'a serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        map.get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn is_usable_search_result(item: &Value) -> bool {
    let Some(map) = item.as_object() else {
        return false;
    };

    let Some(url) = extract_first_nonblank(map, &["url", "link"]) else {
        return false;
    };

    if extract_first_nonblank(map, &["title", "name"]).is_none() {
        return false;
    }

    Url::parse(url)
        .ok()
        .is_some_and(|parsed_url| matches!(parsed_url.scheme(), "http" | "https"))
}

#[tokio::test]
#[ignore = "manual live test; run with -- --ignored"]
async fn live_official_search_smoke_test() -> Result<(), kagi_sdk::KagiError> {
    let Some(api_key) = required_env_or_skip("KAGI_API_KEY") else {
        return Ok(());
    };

    let client = KagiClient::builder()
        .config(live_config())
        .bot_token(BotToken::new(api_key)?)
        .build()?;

    let response = client
        .official_api()?
        .search(SearchRequest::new("rust")?)
        .await?;

    let items = extract_search_items(&response.data).expect(
        "official live search response data must be an array or object with a results-like array",
    );
    assert!(
        !items.is_empty(),
        "official live search response data must include at least one result item"
    );
    assert!(
        items.iter().any(is_usable_search_result),
        "official live search response data must include at least one usable result with non-empty title/name and absolute http(s) url/link"
    );

    Ok(())
}
