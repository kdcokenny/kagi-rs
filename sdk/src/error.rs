use thiserror::Error;

use crate::{
    auth::CredentialKind,
    routing::{EndpointId, ParserShape, ProtocolSurface},
};

#[derive(Debug, Error)]
pub enum KagiError {
    #[error("invalid {kind} credential: {reason}")]
    InvalidCredential {
        kind: CredentialKind,
        reason: String,
    },

    #[error("missing credential configuration: {reason}")]
    MissingCredentialConfiguration { reason: String },

    #[error(
        "conflicting credential configuration: attempted to set {attempted} after {already_set}"
    )]
    ConflictingCredentialConfiguration {
        already_set: CredentialKind,
        attempted: CredentialKind,
    },

    #[error("invalid input for `{field}`: {reason}")]
    InvalidInput { field: &'static str, reason: String },

    #[error("invalid client configuration: {reason}")]
    InvalidClientConfiguration { reason: String },

    #[error(
        "unsupported credential/surface combination: {credential} cannot access {surface}; expected {expected}"
    )]
    UnsupportedAuthSurface {
        surface: ProtocolSurface,
        credential: CredentialKind,
        expected: CredentialKind,
    },

    #[error("unsupported capability for {endpoint}: provided {credential}, expected {expected}")]
    UnsupportedCapability {
        endpoint: EndpointId,
        credential: CredentialKind,
        expected: CredentialKind,
    },

    #[error("network transport failed for {endpoint}: {source}")]
    Transport {
        endpoint: EndpointId,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to parse {parser:?} response for {endpoint}: {reason}")]
    ResponseParse {
        endpoint: EndpointId,
        parser: ParserShape,
        reason: String,
    },

    #[error("bot token unauthorized for {endpoint}: {message}")]
    UnauthorizedBotToken {
        endpoint: EndpointId,
        message: String,
    },

    #[error("session token invalid or expired for {endpoint} (HTTP {status}): {message}")]
    InvalidSession {
        endpoint: EndpointId,
        status: u16,
        message: String,
    },

    #[error("kagi API failure for {endpoint} (HTTP {status}, code {code:?}): {message}")]
    ApiFailure {
        endpoint: EndpointId,
        status: u16,
        code: Option<String>,
        message: String,
    },
}
