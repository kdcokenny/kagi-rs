pub mod models;

use crate::{
    client::KagiClient,
    error::KagiError,
    parsing::{
        parse_html_search_response, parse_kagi_failure_payload, parse_summarize_response,
        parse_summary_stream_response,
    },
    routing::EndpointId,
    transport::{RequestBody, TransportResponse},
};
use scraper::{Html, Selector};

#[allow(deprecated)]
use self::models::{
    HtmlSearchRequest, HtmlSearchResponse, SummaryLabsTextRequest, SummaryLabsUrlRequest,
};
use self::models::{
    SearchRequest, SearchResponse, SummarizeRequest, SummarizeResponse, SummaryStreamResponse,
};

#[derive(Debug)]
pub struct SessionWeb<'a> {
    client: &'a KagiClient,
}

impl<'a> SessionWeb<'a> {
    pub(crate) fn new(client: &'a KagiClient) -> Self {
        Self { client }
    }

    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse, KagiError> {
        let response = self
            .request(
                EndpointId::SessionHtmlSearch,
                request.into_query(),
                RequestBody::Empty,
            )
            .await?;

        if (300..400).contains(&response.status) {
            return Err(KagiError::InvalidSession {
                endpoint: response.endpoint,
                status: response.status,
                message: format!(
                    "session-web request redirected to {:?}",
                    response.redirect_location
                ),
            });
        }

        if response.status >= 400 {
            return Err(build_http_api_failure(&response));
        }

        if let Some((code, message)) = parse_kagi_failure_payload(&response.body) {
            return Err(KagiError::ApiFailure {
                endpoint: response.endpoint,
                status: response.status,
                code,
                message,
            });
        }

        let parsed_search = parse_html_search_response(response.endpoint, &response.body);
        if parsed_search.is_ok() {
            return parsed_search;
        }

        if response_looks_like_kagi_auth_interstitial(&response) {
            return Err(KagiError::InvalidSession {
                endpoint: response.endpoint,
                status: response.status,
                message: "response matched Kagi auth interstitial structure".to_string(),
            });
        }

        parsed_search
    }

    pub async fn summarize(
        &self,
        request: SummarizeRequest,
    ) -> Result<SummarizeResponse, KagiError> {
        let response = self.request_summarize(request, false).await?;

        validate_non_search_session_response(&response)?;
        parse_summarize_response(response.endpoint, &response.body)
    }

    pub async fn summarize_stream(
        &self,
        request: SummarizeRequest,
    ) -> Result<SummaryStreamResponse, KagiError> {
        let response = self.request_summarize(request, true).await?;

        validate_non_search_session_response(&response)?;
        parse_summary_stream_response(response.endpoint, &response.body)
    }

    #[deprecated(note = "use search(SearchRequest) instead")]
    #[doc(hidden)]
    #[allow(deprecated)]
    pub async fn html_search(
        &self,
        request: HtmlSearchRequest,
    ) -> Result<HtmlSearchResponse, KagiError> {
        self.search(request).await
    }

    #[deprecated(
        note = "use summarize(...) or summarize_stream(...) with SummarizeRequest instead"
    )]
    #[doc(hidden)]
    #[allow(deprecated)]
    pub async fn summary_labs_url(
        &self,
        request: SummaryLabsUrlRequest,
    ) -> Result<SummaryStreamResponse, KagiError> {
        self.summarize_stream(request.into_summarize_request())
            .await
    }

    #[deprecated(
        note = "use summarize(...) or summarize_stream(...) with SummarizeRequest instead"
    )]
    #[doc(hidden)]
    #[allow(deprecated)]
    pub async fn summary_labs_text(
        &self,
        request: SummaryLabsTextRequest,
    ) -> Result<SummaryStreamResponse, KagiError> {
        self.summarize_stream(request.into_summarize_request())
            .await
    }

    async fn request(
        &self,
        endpoint: EndpointId,
        query: Vec<(String, String)>,
        body: RequestBody,
    ) -> Result<TransportResponse, KagiError> {
        self.client
            .transport()
            .execute(self.client.credentials(), endpoint, &query, body)
            .await
    }

    async fn request_summarize(
        &self,
        request: SummarizeRequest,
        stream: bool,
    ) -> Result<TransportResponse, KagiError> {
        if let Some(query) = request.clone().into_query(stream) {
            return self
                .request(EndpointId::SessionSummaryLabsGet, query, RequestBody::Empty)
                .await;
        }

        let form = request
            .into_form(stream)
            .expect("text summarize request must produce form fields");

        self.request(
            EndpointId::SessionSummaryLabsPost,
            Vec::new(),
            RequestBody::Form(form),
        )
        .await
    }
}

fn validate_non_search_session_response(response: &TransportResponse) -> Result<(), KagiError> {
    if (300..400).contains(&response.status) {
        return Err(KagiError::InvalidSession {
            endpoint: response.endpoint,
            status: response.status,
            message: format!(
                "session-web request redirected to {:?}",
                response.redirect_location
            ),
        });
    }

    if response_looks_like_kagi_auth_interstitial(response) {
        return Err(KagiError::InvalidSession {
            endpoint: response.endpoint,
            status: response.status,
            message: "response matched Kagi auth interstitial structure".to_string(),
        });
    }

    if let Some((code, message)) = parse_kagi_failure_payload(&response.body) {
        return Err(KagiError::ApiFailure {
            endpoint: response.endpoint,
            status: response.status,
            code,
            message,
        });
    }

    if response.status >= 400 {
        return Err(build_http_api_failure(response));
    }

    Ok(())
}

fn build_http_api_failure(response: &TransportResponse) -> KagiError {
    if let Some((code, message)) = parse_kagi_failure_payload(&response.body) {
        return KagiError::ApiFailure {
            endpoint: response.endpoint,
            status: response.status,
            code,
            message,
        };
    }

    let fallback_message = if response.body.trim().is_empty() {
        format!(
            "HTTP {} returned without parseable Kagi failure payload",
            response.status
        )
    } else {
        response.body.clone()
    };

    KagiError::ApiFailure {
        endpoint: response.endpoint,
        status: response.status,
        code: None,
        message: fallback_message,
    }
}

fn response_looks_like_kagi_auth_interstitial(response: &TransportResponse) -> bool {
    if !response_looks_like_html(response) {
        return false;
    }

    let document = Html::parse_document(&response.body);
    let login_form_selector =
        Selector::parse("form[action='/auth/login'], form[action*='/auth/login']")
            .expect("static selector must compile");
    let password_input_selector = Selector::parse("input[type='password'], input[name='password']")
        .expect("static selector must compile");
    let auth_link_selector = Selector::parse("a[href='/auth/login'], a[href*='/auth/login']")
        .expect("static selector must compile");
    let auth_heading_selector =
        Selector::parse("title, h1, h2").expect("static selector must compile");

    let has_auth_form_with_password = document
        .select(&login_form_selector)
        .any(|form| form.select(&password_input_selector).next().is_some());
    if has_auth_form_with_password {
        return true;
    }

    let has_auth_link = document.select(&auth_link_selector).next().is_some();
    if !has_auth_link {
        return false;
    }

    document.select(&auth_heading_selector).any(|node| {
        let heading_text = node
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();

        heading_text.contains("sign in")
            || heading_text.contains("log in")
            || heading_text.contains("session expired")
            || heading_text.contains("invalid session")
    })
}

fn response_looks_like_html(response: &TransportResponse) -> bool {
    if response
        .content_type
        .as_deref()
        .is_some_and(|value| value.to_ascii_lowercase().contains("text/html"))
    {
        return true;
    }

    let trimmed_body = response.body.trim_start().to_ascii_lowercase();
    trimmed_body.starts_with("<!doctype html") || trimmed_body.starts_with("<html")
}
