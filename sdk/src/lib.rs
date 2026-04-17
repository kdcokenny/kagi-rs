//! Rust SDK for Kagi with explicit protocol surfaces.
//!
//! ## Overview
//!
//! `kagi-sdk` models Kagi access as two distinct surfaces:
//!
//! 1. **Official API surface**
//!    - Auth header: `Authorization: Bot <token>`
//!    - Route families: `/api/v0/*` and `/api/v1/*` (in-scope subset)
//!    - Response shape: JSON envelopes
//!
//! 2. **Session web surface**
//!    - Cookie header: `Cookie: kagi_session=<token>`
//!    - Routes: `/html/search`, `/mother/summary_labs`, `/mother/summary_labs/`
//!    - Response shape:
//!      - `search(...)`: HTML parsing
//!      - `summarize(...)`: JSON parsing
//!      - `summarize_stream(...)`: framed stream parsing (advanced)
//!
//! The SDK enforces auth/surface compatibility at runtime before request execution,
//! returning explicit errors for unsupported combinations.
//!
//! ## Quickstart: official API (bot token)
//!
//! ```no_run
//! use kagi_sdk::{BotToken, KagiClient};
//! use kagi_sdk::official_api::models::SearchRequest;
//!
//! # async fn run() -> Result<(), kagi_sdk::KagiError> {
//! let token = BotToken::new(std::env::var("KAGI_BOT_TOKEN").expect("set KAGI_BOT_TOKEN"))?;
//! let client = KagiClient::with_bot_token(token)?;
//! let api = client.official_api()?;
//!
//! let response = api.search(SearchRequest::new("rust error handling")?).await?;
//! println!("{:#?}", response.data);
//! # Ok(())
//! # }
//! # fn main() {}
//! ```
//!
//! ## Quickstart: session web (session token)
//!
//! ```no_run
//! use kagi_sdk::{KagiClient, SessionToken};
//! use kagi_sdk::session_web::models::{SearchRequest, SummarizeRequest};
//!
//! # async fn run() -> Result<(), kagi_sdk::KagiError> {
//! let token =
//!     SessionToken::new(std::env::var("KAGI_SESSION_TOKEN").expect("set KAGI_SESSION_TOKEN"))?;
//! let client = KagiClient::with_session_token(token)?;
//! let web = client.session_web()?;
//!
//! let search = web.search(SearchRequest::new("kagi rust sdk")?).await?;
//! let summary = web
//!     .summarize(SummarizeRequest::from_url("https://example.com")?)
//!     .await?;
//! println!("{} {}", search.results.len(), summary.markdown.len());
//! # Ok(())
//! # }
//! # fn main() {}
//! ```
//!
//! ## Builder configuration example
//!
//! ```no_run
//! use std::time::Duration;
//! use kagi_sdk::{BotToken, ClientConfig, KagiClient};
//!
//! # fn run() -> Result<(), kagi_sdk::KagiError> {
//! let config = ClientConfig::default()
//!     .with_timeout(Duration::from_secs(15))
//!     .with_user_agent("my-app/0.1.0");
//!
//! let _client = KagiClient::builder()
//!     .config(config)
//!     .bot_token(BotToken::new("your-token")?)
//!     .build()?;
//! # Ok(())
//! # }
//! # fn main() {}
//! ```
//!
//! ## Error handling
//!
//! Most fallible operations return [`KagiError`]. Prefer `?` propagation and map errors at your
//! application boundary.
//!
//! ## Module overview
//!
//! | Module | Purpose |
//! |---|---|
//! | [`auth`] | Credential types and header/cookie application |
//! | [`client`] | `KagiClient` construction and surface entrypoints |
//! | [`config`] | Client base URL, timeout, and user-agent configuration |
//! | [`official_api`] | Typed official API requests and envelope parsing |
//! | [`session_web`] | Typed session-web requests for HTML search, JSON summarize, and advanced framed streaming |
//! | [`parsing`] | Parser helpers for session HTML search, summarize JSON, and framed streaming |
//! | [`routing`] | Endpoint metadata (surface, version, parser shape) |
//! | [`error`] | Central typed error model |
//!
//! ## Safety
//!
//! This crate forbids `unsafe` Rust (`#![forbid(unsafe_code)]`).
//!
#![forbid(unsafe_code)]

mod boundary;
mod transport;

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod official_api;
pub mod parsing;
pub mod routing;
pub mod session_web;

pub use auth::{BotToken, CredentialKind, Credentials, SessionToken};
pub use boundary::{HttpUrl, NonBlankString, NonEmptyString};
pub use client::{KagiClient, KagiClientBuilder};
pub use config::ClientConfig;
pub use error::KagiError;
