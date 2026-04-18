use kagi_sdk::{CredentialKind, KagiError};

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error(
        "invalid `{env_var}` value `{value}`; expected one of `auto`, `official`, or `session`"
    )]
    InvalidBackendMode {
        env_var: &'static str,
        value: String,
    },

    #[error("missing required credential `{env_var}` for `{mode}` backend mode{hint_suffix}")]
    MissingCredential {
        env_var: &'static str,
        mode: &'static str,
        hint_suffix: String,
    },

    #[error("invalid credential in `{env_var}`: {reason}")]
    InvalidCredential {
        env_var: &'static str,
        reason: String,
    },

    #[error("failed to construct Kagi client: {reason}")]
    ClientConstruction { reason: String },
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ToolFailure {
    message: String,
}

impl ToolFailure {
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn parse_drift(reason: impl Into<String>) -> Self {
        Self {
            message: format!(
                "Kagi returned an unexpected response shape for this capability ({})",
                reason.into()
            ),
        }
    }

    pub fn from_kagi_error(error: KagiError) -> Self {
        match error {
            KagiError::InvalidCredential { kind, reason } => Self {
                message: format!(
                    "Server credential is invalid for {kind}. Update the credential and restart ({reason})."
                ),
            },
            KagiError::MissingCredentialConfiguration { reason }
            | KagiError::InvalidClientConfiguration { reason } => Self {
                message: format!("Server startup configuration is invalid ({reason})."),
            },
            KagiError::ConflictingCredentialConfiguration { .. } => Self {
                message: "Server startup configuration is invalid (conflicting credential configuration)."
                    .to_string(),
            },
            KagiError::InvalidInput { field, reason } => Self {
                message: format!(
                    "The server generated an invalid upstream request for `{field}` ({reason})."
                ),
            },
            KagiError::UnsupportedAuthSurface { .. } | KagiError::UnsupportedCapability { .. } => {
                Self {
                    message: "Selected backend mode does not support this capability. Update `KAGI_MCP_BACKEND` and restart.".to_string(),
                }
            }
            KagiError::UnauthorizedBotToken { .. } => Self {
                message: auth_failure_message(Some(CredentialKind::BotToken)),
            },
            KagiError::InvalidSession { .. } => Self {
                message: auth_failure_message(Some(CredentialKind::SessionToken)),
            },
            KagiError::Transport { source, .. } => {
                if source.is_timeout() {
                    return Self {
                        message: "Kagi request timed out. Retry shortly.".to_string(),
                    };
                }

                Self {
                    message: "Kagi transport request failed. Check network connectivity and retry.".to_string(),
                }
            }
            KagiError::ResponseParse { reason, .. } => Self::parse_drift(reason),
            KagiError::ApiFailure {
                endpoint,
                status,
                code,
                message,
                ..
            } => {
                if api_failure_indicates_auth_failure(status, code.as_deref(), &message) {
                    return Self {
                        message: auth_failure_message(Some(endpoint.spec().allowed_credential)),
                    };
                }

                if status == 429 {
                    return Self {
                        message:
                            "Kagi rate-limited this request (HTTP 429). Retry after a short delay."
                                .to_string(),
                    };
                }

                if status >= 500 {
                    return Self {
                        message: format!(
                            "Kagi upstream service is currently failing (HTTP {status}). Retry later."
                        ),
                    };
                }

                if status < 400 {
                    let detail = code
                        .map(|code| format!("{code}: {message}"))
                        .unwrap_or(message);

                    return Self {
                        message: format!(
                            "Kagi reported an application-level failure (HTTP {status}): {detail}"
                        ),
                    };
                }

                Self {
                    message: format!(
                        "Kagi rejected the upstream request (HTTP {status}). Verify input and retry."
                    ),
                }
            }
        }
    }
}

fn auth_failure_message(expected_kind: Option<CredentialKind>) -> String {
    let base =
        "Authentication failed with Kagi. Verify the configured credential and restart the server.";

    let guidance = match expected_kind {
        Some(CredentialKind::BotToken) => {
            " This backend expects an official bot token in `KAGI_API_KEY`; the configured value may belong to session-web auth (`KAGI_SESSION_TOKEN`) instead."
        }
        Some(CredentialKind::SessionToken) => {
            " This backend expects a session-web token in `KAGI_SESSION_TOKEN`; the configured value may belong to official bot-token auth (`KAGI_API_KEY`) instead."
        }
        None => {
            " `KAGI_API_KEY` should be used only for official bot tokens, and `KAGI_SESSION_TOKEN` should be used only for session-web tokens."
        }
    };

    format!("{base}{guidance}")
}

fn api_failure_indicates_auth_failure(status: u16, code: Option<&str>, message: &str) -> bool {
    if matches!(status, 401 | 403) {
        return true;
    }

    if code.is_some_and(|raw_code| {
        let normalized = raw_code.trim().to_ascii_lowercase();
        matches!(normalized.as_str(), "unauthorized" | "invalid_session")
    }) {
        return true;
    }

    let normalized_message = message.trim().to_ascii_lowercase();
    normalized_message == "unauthorized"
        || normalized_message == "unauthorized: unauthorized"
        || normalized_message.contains("invalid session")
}
