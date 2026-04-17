use crate::{
    boundary::{HttpUrl, NonBlankString, NonEmptyString},
    error::KagiError,
};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub query: NonEmptyString,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummarizeRequest {
    pub input: SummarizeInput,
    pub options: SummarizeOptions,
}

impl SummarizeRequest {
    pub fn from_url(url: impl AsRef<str>) -> Result<Self, KagiError> {
        Ok(Self {
            input: SummarizeInput::Url(HttpUrl::new("url", url)?),
            options: SummarizeOptions::default(),
        })
    }

    pub fn from_text(text: impl Into<String>) -> Result<Self, KagiError> {
        Ok(Self {
            input: SummarizeInput::Text(NonBlankString::new("text", text)?),
            options: SummarizeOptions::default(),
        })
    }

    pub fn with_summary_type(mut self, summary_type: SummaryType) -> Self {
        self.options.summary_type = Some(summary_type);
        self
    }

    pub fn with_target_language(
        mut self,
        target_language: impl Into<String>,
    ) -> Result<Self, KagiError> {
        self.options.target_language =
            Some(NonEmptyString::new("target_language", target_language)?);
        Ok(self)
    }

    pub(crate) fn into_query(self, stream: bool) -> Option<Vec<(String, String)>> {
        let SummarizeInput::Url(url) = self.input else {
            return None;
        };

        let mut query = vec![("url".to_string(), url.as_str().to_string())];
        query.extend(self.options.into_params());
        if stream {
            query.push(("stream".to_string(), "1".to_string()));
        }

        Some(query)
    }

    pub(crate) fn into_form(self, stream: bool) -> Option<Vec<(String, String)>> {
        let SummarizeInput::Text(text) = self.input else {
            return None;
        };

        let mut form = vec![("text".to_string(), text.as_str().to_string())];
        form.extend(self.options.into_params());
        if stream {
            form.push(("stream".to_string(), "1".to_string()));
        }

        Some(form)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummarizeInput {
    Url(HttpUrl),
    Text(NonBlankString),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SummarizeOptions {
    pub summary_type: Option<SummaryType>,
    pub target_language: Option<NonEmptyString>,
}

impl SummarizeOptions {
    fn into_params(self) -> Vec<(String, String)> {
        let mut params = Vec::new();

        if let Some(summary_type) = self.summary_type {
            params.push((
                "summary_type".to_string(),
                summary_type.as_param().to_string(),
            ));
        }

        if let Some(target_language) = self.target_language {
            params.push((
                "target_language".to_string(),
                target_language.as_str().to_string(),
            ));
        }

        params
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryType {
    Summary,
    Takeaway,
}

impl SummaryType {
    fn as_param(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Takeaway => "takeaway",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SummarizeResponse {
    pub markdown: String,
    pub text: Option<String>,
    pub status: Option<String>,
    pub metadata: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryStreamResponse {
    pub chunks: Vec<String>,
    pub text: String,
}

#[deprecated(note = "use SearchRequest and SessionWeb::search(...) instead")]
#[doc(hidden)]
pub type HtmlSearchRequest = SearchRequest;

#[deprecated(note = "use SearchResponse returned by SessionWeb::search(...) instead")]
#[doc(hidden)]
pub type HtmlSearchResponse = SearchResponse;

#[deprecated(note = "use SearchResult returned by SessionWeb::search(...) instead")]
#[doc(hidden)]
pub type HtmlSearchResult = SearchResult;

#[deprecated(note = "use SummarizeRequest with SessionWeb::summarize(...) instead")]
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryLabsUrlRequest {
    url: HttpUrl,
}

#[allow(deprecated)]
impl SummaryLabsUrlRequest {
    pub fn new(url: impl AsRef<str>) -> Result<Self, KagiError> {
        Ok(Self {
            url: HttpUrl::new("url", url)?,
        })
    }

    pub(crate) fn into_summarize_request(self) -> SummarizeRequest {
        SummarizeRequest {
            input: SummarizeInput::Url(self.url),
            options: SummarizeOptions::default(),
        }
    }
}

#[deprecated(note = "use SummarizeRequest with SessionWeb::summarize(...) instead")]
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryLabsTextRequest {
    text: NonBlankString,
}

#[allow(deprecated)]
impl SummaryLabsTextRequest {
    pub fn new(text: impl Into<String>) -> Result<Self, KagiError> {
        Ok(Self {
            text: NonBlankString::new("text", text)?,
        })
    }

    pub(crate) fn into_summarize_request(self) -> SummarizeRequest {
        SummarizeRequest {
            input: SummarizeInput::Text(self.text),
            options: SummarizeOptions::default(),
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::{SummarizeRequest, SummaryLabsTextRequest, SummaryType};

    #[test]
    fn summary_labs_text_preserves_whitespace() {
        let form = SummaryLabsTextRequest::new("  keep exact spacing  ")
            .expect("text should parse")
            .into_summarize_request()
            .into_form(false)
            .expect("text request should produce form");

        assert_eq!(form[0].0, "text");
        assert_eq!(form[0].1, "  keep exact spacing  ");
    }

    #[test]
    fn summarize_options_are_encoded_into_request_params() {
        let request = SummarizeRequest::from_url("https://example.com")
            .expect("url should parse")
            .with_summary_type(SummaryType::Takeaway)
            .with_target_language("es")
            .expect("target language should parse");

        let query = request
            .into_query(true)
            .expect("url request should produce query params");

        assert!(query
            .iter()
            .any(|(key, value)| key == "summary_type" && value == "takeaway"));
        assert!(query
            .iter()
            .any(|(key, value)| key == "target_language" && value == "es"));
        assert!(query
            .iter()
            .any(|(key, value)| key == "stream" && value == "1"));
    }
}
