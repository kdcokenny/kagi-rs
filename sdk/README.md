# kagi-sdk

Typed Rust SDK for Kagi with explicit official-api and session-web surfaces.

## Features at a glance

| Feature | What you get |
|---|---|
| Explicit protocol boundaries | Separate `official_api()` and `session_web()` clients |
| Typed request models | Constructor-level validation for common invalid input |
| Fail-fast auth/surface checks | Invalid credential/surface combinations fail before request execution |
| Endpoint scope clarity | v1 support tracked in [`../docs/endpoint-auth-version-matrix.md`](../docs/endpoint-auth-version-matrix.md) |

## Install

Choose one installation mode:

### crates.io (recommended)

```toml
[dependencies]
kagi-sdk = "0.1.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### Path dependency (same workspace or local checkout)

```toml
[dependencies]
kagi-sdk = { path = "../sdk" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### Git dependency (unreleased revisions)

```toml
[dependencies]
kagi-sdk = { git = "https://github.com/kdcokenny/kagi-rs", package = "kagi-sdk" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Choose your surface

| Use case | SDK surface | Credential |
|---|---|---|
| Kagi official API routes | `client.official_api()?` | `BotToken` (commonly from `KAGI_API_KEY`) |
| Kagi session-web routes | `client.session_web()?` | `SessionToken` (commonly from `KAGI_SESSION_TOKEN`) |

## Quickstart (canonical: official API)

```rust
use kagi_sdk::official_api::models::SearchRequest;
use kagi_sdk::{BotToken, KagiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = BotToken::new(std::env::var("KAGI_API_KEY")?)?;
    let client = KagiClient::with_bot_token(token)?;
    let api = client.official_api()?;

    let response = api.search(SearchRequest::new("rust async patterns")?).await?;
    println!("{:#?}", response.data);
    Ok(())
}
```

## Session quickstart (alternate surface)

```rust
use kagi_sdk::session_web::models::{SearchRequest, SummarizeRequest};
use kagi_sdk::{KagiClient, SessionToken};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = SessionToken::new(std::env::var("KAGI_SESSION_TOKEN")?)?;
    let client = KagiClient::with_session_token(token)?;
    let web = client.session_web()?;

    let search = web.search(SearchRequest::new("kagi rust sdk")?).await?;
    let summary = web
        .summarize(SummarizeRequest::from_url("https://example.com")?)
        .await?;

    println!("results={}, summary_len={}", search.results.len(), summary.markdown.len());
    Ok(())
}
```

## Supported endpoints

### Official API (`Authorization: Bot <token>`)

| Method | Route | SDK method |
|---|---|---|
| GET | `/api/v0/search` | `official_api.search(...)` |
| GET | `/api/v0/enrich/web` | `official_api.enrich_web(...)` |
| GET | `/api/v0/enrich/news` | `official_api.enrich_news(...)` |
| GET | `/api/v0/summarize` | `official_api.summarize_get(...)` |
| POST | `/api/v0/summarize` | `official_api.summarize_post(...)` |
| POST | `/api/v0/fastgpt` | `official_api.fastgpt(...)` |
| GET | `/api/v1/smallweb/feed` | `official_api.smallweb_feed(...)` |

### Session web (`Cookie: kagi_session=<token>`)

| Method | Route | SDK method |
|---|---|---|
| GET | `/html/search` | `session_web.search(...)` |
| GET or POST | `/mother/summary_labs` or `/mother/summary_labs/` | `session_web.summarize(...)` |
| GET or POST | `/mother/summary_labs` or `/mother/summary_labs/` | `session_web.summarize_stream(...)` *(advanced)* |

## Error handling

Most methods return `Result<_, kagi_sdk::KagiError>`.

Common categories:

- invalid credential/input/configuration
- unsupported auth/surface combination
- unauthorized bot token or invalid session
- transport failures
- parse failures
- API envelope failures

## Advanced configuration

```rust
use std::time::Duration;
use kagi_sdk::{BotToken, ClientConfig, KagiClient};

fn build_client() -> Result<KagiClient, kagi_sdk::KagiError> {
    let config = ClientConfig::default()
        .with_timeout(Duration::from_secs(10))
        .with_user_agent("my-app/0.1.0");

    KagiClient::builder()
        .config(config)
        .bot_token(BotToken::new("your-token")?)
        .build()
}
```

## Development and live-test notes

From workspace root:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Live integration tests are manual-only in v1 via `live-integration.yml`:

- `sdk/tests/live_official.rs` expects `KAGI_API_KEY` (optional `KAGI_BASE_URL`)
- `sdk/tests/live_session.rs` expects `KAGI_SESSION_TOKEN` (optional `KAGI_BASE_URL`)

For release/workflow policy, see [`../docs/releasing.md`](../docs/releasing.md).
