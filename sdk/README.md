# kagi-sdk

Rust SDK for Kagi with an explicit two-surface model:

- **Official API surface**: `Authorization: Bot <token>`
- **Session web surface**: `Cookie: kagi_session=<token>`

The crate is designed to make those protocol boundaries obvious in code through separate namespaces and typed request models.

## Install

### A) Local development in this repository

`path` is resolved relative to the consuming crate's `Cargo.toml`.

For a sibling crate in this repo (for example future `cli/` or `mcp/`), use:

```toml
[dependencies]
kagi-sdk = { path = "../sdk" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

If your crate is outside this repository, point `path` to this `sdk/` directory using an appropriate relative or absolute path.

### B) External usage from GitHub (today)

```toml
[dependencies]
kagi-sdk = { git = "https://github.com/kdcokenny/kagi-rs", package = "kagi-sdk" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Optionally pin to a revision for reproducibility:

```toml
kagi-sdk = { git = "https://github.com/kdcokenny/kagi-rs", package = "kagi-sdk", rev = "<commit-hash>" }
```

### C) crates.io (future)

`kagi-sdk` is not published to crates.io yet. When it is published, this README will include the exact crates.io dependency line.

## Quickstart: official API (bot token)

```rust
use kagi_sdk::{BotToken, KagiClient};
use kagi_sdk::official_api::models::SearchRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = BotToken::new(std::env::var("KAGI_BOT_TOKEN")?)?;
    let client = KagiClient::with_bot_token(token)?;
    let api = client.official_api()?;

    let response = api.search(SearchRequest::new("rust async patterns")?).await?;
    println!("{:#?}", response.data);
    Ok(())
}
```

## Quickstart: session web (session token)

```rust
use kagi_sdk::{KagiClient, SessionToken};
use kagi_sdk::session_web::models::HtmlSearchRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = SessionToken::new(std::env::var("KAGI_SESSION_TOKEN")?)?;
    let client = KagiClient::with_session_token(token)?;
    let web = client.session_web()?;

    let response = web.html_search(HtmlSearchRequest::new("kagi rust sdk")?).await?;
    println!("{} results", response.results.len());
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
| GET | `/html/search` | `session_web.html_search(...)` |
| GET | `/mother/summary_labs` | `session_web.summary_labs_url(...)` |
| POST | `/mother/summary_labs/` | `session_web.summary_labs_text(...)` |

## Why two surfaces?

Kagi uses two distinct protocols with different auth, routes, and response formats.

- Official API is JSON-envelope based and bot-token authenticated.
- Session web is session-cookie authenticated and returns HTML or stream responses.

The SDK keeps those surfaces separate and performs runtime pre-request checks so unsupported credential/surface combinations fail loudly.

## Error handling

Most methods return `Result<_, kagi_sdk::KagiError>`.

Common categories include:

- invalid credential/input/configuration
- unsupported auth/surface combinations
- unauthorized bot token or invalid session
- transport failures
- parse failures
- API-domain envelope failures

```rust
use kagi_sdk::{BotToken, KagiClient, KagiError};
use kagi_sdk::official_api::models::SearchRequest;

async fn run() -> Result<(), KagiError> {
    let client = KagiClient::with_bot_token(BotToken::new("your-token")?)?;
    let api = client.official_api()?;
    let _response = api.search(SearchRequest::new("tokio")?).await?;
    Ok(())
}
```

## Builder and config example

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

## Development

From workspace root:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
