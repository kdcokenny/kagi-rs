use std::fmt;

use url::Url;

use crate::error::KagiError;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(field: &'static str, value: impl Into<String>) -> Result<Self, KagiError> {
        let candidate = value.into();
        let trimmed = candidate.trim();

        if trimmed.is_empty() {
            return Err(KagiError::InvalidInput {
                field,
                reason: "value cannot be empty".to_string(),
            });
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for NonEmptyString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("NonEmptyString")
            .field(&self.0)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NonBlankString(String);

impl NonBlankString {
    pub fn new(field: &'static str, value: impl Into<String>) -> Result<Self, KagiError> {
        let candidate = value.into();
        if candidate.trim().is_empty() {
            return Err(KagiError::InvalidInput {
                field,
                reason: "value cannot be blank".to_string(),
            });
        }

        Ok(Self(candidate))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for NonBlankString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("NonBlankString")
            .field(&self.0)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct HttpUrl(String);

impl HttpUrl {
    pub fn new(field: &'static str, value: impl AsRef<str>) -> Result<Self, KagiError> {
        let parsed = Url::parse(value.as_ref()).map_err(|source| KagiError::InvalidInput {
            field,
            reason: format!("invalid URL: {source}"),
        })?;

        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(KagiError::InvalidInput {
                field,
                reason: "URL must use http or https".to_string(),
            });
        }

        Ok(Self(parsed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for HttpUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("HttpUrl").field(&self.0).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{HttpUrl, NonBlankString, NonEmptyString};

    #[test]
    fn non_empty_string_rejects_blank_values() {
        let result = NonEmptyString::new("query", "   ");
        assert!(result.is_err());
    }

    #[test]
    fn http_url_rejects_non_http_scheme() {
        let result = HttpUrl::new("url", "ftp://example.com");
        assert!(result.is_err());
    }

    #[test]
    fn non_blank_string_preserves_original_whitespace() {
        let parsed = NonBlankString::new("text", "  keep me  ").expect("should parse");
        assert_eq!(parsed.as_str(), "  keep me  ");
    }
}
