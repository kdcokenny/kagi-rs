use kagi_sdk::{official_api::models::SearchRequest, BotToken, ClientConfig, KagiClient};
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

    let data_object = response
        .data
        .as_object()
        .expect("official live search response data must be a JSON object");
    assert!(
        !data_object.is_empty(),
        "official live search response data object must not be empty"
    );

    Ok(())
}
