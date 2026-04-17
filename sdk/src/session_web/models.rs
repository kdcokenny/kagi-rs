use crate::{
    boundary::{HttpUrl, NonBlankString, NonEmptyString},
    error::KagiError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlSearchRequest {
    query: NonEmptyString,
}

impl HtmlSearchRequest {
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
pub struct HtmlSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlSearchResponse {
    pub results: Vec<HtmlSearchResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryLabsUrlRequest {
    url: HttpUrl,
}

impl SummaryLabsUrlRequest {
    pub fn new(url: impl AsRef<str>) -> Result<Self, KagiError> {
        Ok(Self {
            url: HttpUrl::new("url", url)?,
        })
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        vec![("url".to_string(), self.url.as_str().to_string())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryLabsTextRequest {
    text: NonBlankString,
}

impl SummaryLabsTextRequest {
    pub fn new(text: impl Into<String>) -> Result<Self, KagiError> {
        Ok(Self {
            text: NonBlankString::new("text", text)?,
        })
    }

    pub(crate) fn into_form(self) -> Vec<(String, String)> {
        vec![("text".to_string(), self.text.as_str().to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::SummaryLabsTextRequest;

    #[test]
    fn summary_labs_text_preserves_whitespace() {
        let form = SummaryLabsTextRequest::new("  keep exact spacing  ")
            .expect("text should parse")
            .into_form();

        assert_eq!(form[0].0, "text");
        assert_eq!(form[0].1, "  keep exact spacing  ");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryStreamResponse {
    pub chunks: Vec<String>,
    pub text: String,
}
