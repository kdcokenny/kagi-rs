use std::fmt;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE};

use crate::error::KagiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialKind {
    BotToken,
    SessionToken,
}

impl fmt::Display for CredentialKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BotToken => formatter.write_str("BotToken"),
            Self::SessionToken => formatter.write_str("SessionToken"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BotToken(String);

impl BotToken {
    pub fn new(value: impl Into<String>) -> Result<Self, KagiError> {
        let token = parse_token(value.into(), CredentialKind::BotToken)?;
        Ok(Self(token))
    }

    pub(crate) fn as_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for BotToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BotToken(REDACTED)")
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SessionToken(String);

impl SessionToken {
    pub fn new(value: impl Into<String>) -> Result<Self, KagiError> {
        let token = parse_token(value.into(), CredentialKind::SessionToken)?;
        Ok(Self(token))
    }

    pub(crate) fn as_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionToken(REDACTED)")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Credentials {
    BotToken(BotToken),
    SessionToken(SessionToken),
}

impl Credentials {
    pub fn kind(&self) -> CredentialKind {
        match self {
            Self::BotToken(_) => CredentialKind::BotToken,
            Self::SessionToken(_) => CredentialKind::SessionToken,
        }
    }

    pub(crate) fn apply_to_headers(&self, headers: &mut HeaderMap) -> Result<(), KagiError> {
        match self {
            Self::BotToken(token) => {
                let value = HeaderValue::from_str(&format!("Bot {}", token.as_secret())).map_err(
                    |source| KagiError::InvalidCredential {
                        kind: CredentialKind::BotToken,
                        reason: format!("token could not be encoded as header: {source}"),
                    },
                )?;
                headers.insert(AUTHORIZATION, value);
            }
            Self::SessionToken(token) => {
                let value = HeaderValue::from_str(&format!("kagi_session={}", token.as_secret()))
                    .map_err(|source| KagiError::InvalidCredential {
                    kind: CredentialKind::SessionToken,
                    reason: format!("token could not be encoded as cookie: {source}"),
                })?;
                headers.insert(COOKIE, value);
            }
        }

        Ok(())
    }
}

impl From<BotToken> for Credentials {
    fn from(value: BotToken) -> Self {
        Self::BotToken(value)
    }
}

impl From<SessionToken> for Credentials {
    fn from(value: SessionToken) -> Self {
        Self::SessionToken(value)
    }
}

fn parse_token(raw_token: String, kind: CredentialKind) -> Result<String, KagiError> {
    let trimmed = raw_token.trim();
    if trimmed.is_empty() {
        return Err(KagiError::InvalidCredential {
            kind,
            reason: "token cannot be empty".to_string(),
        });
    }

    if trimmed.chars().any(char::is_whitespace) {
        return Err(KagiError::InvalidCredential {
            kind,
            reason: "token cannot contain whitespace".to_string(),
        });
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::BotToken;

    #[test]
    fn token_debug_is_redacted() {
        let token = BotToken::new("super-secret").expect("token should parse");
        assert_eq!(format!("{token:?}"), "BotToken(REDACTED)");
    }
}
