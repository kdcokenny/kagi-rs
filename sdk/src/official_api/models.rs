use serde_json::{json, Value};

use crate::{
    boundary::{HttpUrl, NonBlankString, NonEmptyString},
    error::KagiError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    query: NonEmptyString,
}

impl SearchRequest {
    pub fn new(query: impl Into<String>) -> Result<Self, KagiError> {
        Ok(Self {
            query: NonEmptyString::new("query", query)?,
        })
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        vec![("q".to_string(), self.query.as_str().to_string())]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResponse {
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrichWebRequest {
    url: HttpUrl,
}

impl EnrichWebRequest {
    pub fn new(url: impl AsRef<str>) -> Result<Self, KagiError> {
        Ok(Self {
            url: HttpUrl::new("url", url)?,
        })
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        vec![("url".to_string(), self.url.as_str().to_string())]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnrichWebResponse {
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrichNewsRequest {
    url: HttpUrl,
}

impl EnrichNewsRequest {
    pub fn new(url: impl AsRef<str>) -> Result<Self, KagiError> {
        Ok(Self {
            url: HttpUrl::new("url", url)?,
        })
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        vec![("url".to_string(), self.url.as_str().to_string())]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnrichNewsResponse {
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummarizeGetRequest {
    url: HttpUrl,
}

impl SummarizeGetRequest {
    pub fn new(url: impl AsRef<str>) -> Result<Self, KagiError> {
        Ok(Self {
            url: HttpUrl::new("url", url)?,
        })
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        vec![("url".to_string(), self.url.as_str().to_string())]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SummarizeGetResponse {
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummarizePostRequest {
    input: SummarizePostInput,
}

impl SummarizePostRequest {
    pub fn from_url(url: impl AsRef<str>) -> Result<Self, KagiError> {
        Ok(Self {
            input: SummarizePostInput::from_url(url)?,
        })
    }

    pub fn from_text(text: impl Into<String>) -> Result<Self, KagiError> {
        Ok(Self {
            input: SummarizePostInput::from_text(text)?,
        })
    }

    pub(crate) fn into_json(self) -> Value {
        self.input.into_json()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummarizePostInput {
    Url(HttpUrl),
    Text(NonBlankString),
}

impl SummarizePostInput {
    pub fn from_url(url: impl AsRef<str>) -> Result<Self, KagiError> {
        Ok(Self::Url(HttpUrl::new("url", url)?))
    }

    pub fn from_text(text: impl Into<String>) -> Result<Self, KagiError> {
        Ok(Self::Text(NonBlankString::new("text", text)?))
    }

    pub(crate) fn into_json(self) -> Value {
        match self {
            Self::Url(url) => json!({ "url": url.as_str() }),
            Self::Text(text) => json!({ "text": text.as_str() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SummarizePostInput;

    #[test]
    fn summarize_post_text_preserves_whitespace() {
        let payload = SummarizePostInput::from_text("  keep exact spacing  ")
            .expect("text should parse")
            .into_json();

        assert_eq!(payload["text"], "  keep exact spacing  ");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SummarizePostResponse {
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastGptRequest {
    query: NonEmptyString,
}

impl FastGptRequest {
    pub fn new(query: impl Into<String>) -> Result<Self, KagiError> {
        Ok(Self {
            query: NonEmptyString::new("query", query)?,
        })
    }

    pub(crate) fn into_json(self) -> Value {
        json!({ "query": self.query.as_str() })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FastGptResponse {
    pub data: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SmallwebFeedRequest {
    limit: Option<u8>,
}

impl SmallwebFeedRequest {
    pub fn with_limit(limit: u8) -> Result<Self, KagiError> {
        if limit == 0 {
            return Err(KagiError::InvalidInput {
                field: "limit",
                reason: "limit must be greater than zero".to_string(),
            });
        }

        Ok(Self { limit: Some(limit) })
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        self.limit
            .map(|limit| vec![("limit".to_string(), limit.to_string())])
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmallwebFeedResponse {
    pub data: Value,
}
