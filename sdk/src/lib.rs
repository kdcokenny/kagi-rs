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
pub use client::{KagiClient, KagiClientBuilder};
pub use config::ClientConfig;
pub use error::KagiError;
