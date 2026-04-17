use std::time::Duration;

use url::Url;

use crate::error::KagiError;

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub base_url: Url,
    pub timeout: Duration,
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: Url::parse("https://kagi.com").expect("static URL is valid"),
            timeout: Duration::from_secs(20),
            user_agent: format!("kagi-sdk-rust/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl ClientConfig {
    pub fn with_base_url(mut self, base_url: Url) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub(crate) fn validate(&self) -> Result<(), KagiError> {
        if !matches!(self.base_url.scheme(), "http" | "https") {
            return Err(KagiError::InvalidClientConfiguration {
                reason: "base_url must use http or https".to_string(),
            });
        }

        if self.timeout.is_zero() {
            return Err(KagiError::InvalidClientConfiguration {
                reason: "timeout must be greater than zero".to_string(),
            });
        }

        if self.user_agent.trim().is_empty() {
            return Err(KagiError::InvalidClientConfiguration {
                reason: "user_agent cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}
