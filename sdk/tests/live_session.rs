use kagi_sdk::{session_web::models::SearchRequest, ClientConfig, KagiClient, SessionToken};
use url::Url;

fn required_env_or_skip(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ if std::env::var_os("GITHUB_ACTIONS").is_none() => {
            eprintln!("Skipping live_session test: {name} is not set.");
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
async fn live_session_search_smoke_test() -> Result<(), kagi_sdk::KagiError> {
    let Some(session_token) = required_env_or_skip("KAGI_SESSION_TOKEN") else {
        return Ok(());
    };

    let client = KagiClient::builder()
        .config(live_config())
        .session_token(SessionToken::new(session_token)?)
        .build()?;

    let response = client
        .session_web()?
        .search(SearchRequest::new("rust")?)
        .await?;

    assert!(
        !response.results.is_empty(),
        "session live search must return at least one result"
    );

    for result in &response.results {
        assert!(!result.title.trim().is_empty());
        assert!(!result.url.trim().is_empty());
        let parsed_url = Url::parse(&result.url).expect("session result URL must be absolute");
        assert!(
            matches!(parsed_url.scheme(), "http" | "https"),
            "session result URL must be http or https"
        );
    }

    Ok(())
}
