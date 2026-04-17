pub mod models;

use crate::{
    client::KagiClient,
    error::KagiError,
    parsing::{parse_html_search_response, parse_summary_stream_response},
    routing::{EndpointId, ParserShape},
    transport::{RequestBody, TransportResponse},
};

use self::models::{
    HtmlSearchRequest, HtmlSearchResponse, SummaryLabsTextRequest, SummaryLabsUrlRequest,
    SummaryStreamResponse,
};

#[derive(Debug)]
pub struct SessionWeb<'a> {
    client: &'a KagiClient,
}

impl<'a> SessionWeb<'a> {
    pub(crate) fn new(client: &'a KagiClient) -> Self {
        Self { client }
    }

    pub async fn html_search(
        &self,
        request: HtmlSearchRequest,
    ) -> Result<HtmlSearchResponse, KagiError> {
        let response = self
            .request(
                EndpointId::SessionHtmlSearch,
                request.into_query(),
                RequestBody::Empty,
            )
            .await?;
        ensure_session_response_shape(&response)?;
        parse_html_search_response(response.endpoint, &response.body)
    }

    pub async fn summary_labs_url(
        &self,
        request: SummaryLabsUrlRequest,
    ) -> Result<SummaryStreamResponse, KagiError> {
        let response = self
            .request(
                EndpointId::SessionSummaryLabsGet,
                request.into_query(),
                RequestBody::Empty,
            )
            .await?;
        ensure_session_response_shape(&response)?;
        parse_summary_stream_response(response.endpoint, &response.body)
    }

    pub async fn summary_labs_text(
        &self,
        request: SummaryLabsTextRequest,
    ) -> Result<SummaryStreamResponse, KagiError> {
        let response = self
            .request(
                EndpointId::SessionSummaryLabsPost,
                Vec::new(),
                RequestBody::Form(request.into_form()),
            )
            .await?;
        ensure_session_response_shape(&response)?;
        parse_summary_stream_response(response.endpoint, &response.body)
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
}

fn ensure_session_response_shape(response: &TransportResponse) -> Result<(), KagiError> {
    if response.status < 400 {
        let spec = response.endpoint.spec();

        if (300..400).contains(&response.status) {
            return Err(KagiError::InvalidSession {
                endpoint: response.endpoint,
                status: response.status,
                message: format!(
                    "session-web request was redirected to {:?}; expected endpoint route {}",
                    response.redirect_location, spec.route,
                ),
            });
        }

        // Redirect following is disabled in transport, so route-based validation adds little
        // protection against session/login interstitial pages. We fail fast on redirect status
        // and then validate content shape for the requested endpoint.
        ensure_expected_content_shape(response)?;
        return Ok(());
    }

    Err(KagiError::ApiFailure {
        endpoint: response.endpoint,
        status: response.status,
        code: None,
        message: response.body.clone(),
    })
}

fn ensure_expected_content_shape(response: &TransportResponse) -> Result<(), KagiError> {
    match response.endpoint {
        EndpointId::SessionHtmlSearch => Ok(()),
        EndpointId::SessionSummaryLabsGet | EndpointId::SessionSummaryLabsPost => {
            if body_looks_like_html(&response.body) {
                return Err(KagiError::InvalidSession {
                    endpoint: response.endpoint,
                    status: response.status,
                    message: "summary endpoint returned HTML instead of stream data".to_string(),
                });
            }

            if response
                .content_type
                .as_deref()
                .is_some_and(|value| value.to_ascii_lowercase().contains("text/html"))
            {
                return Err(KagiError::InvalidSession {
                    endpoint: response.endpoint,
                    status: response.status,
                    message: "summary endpoint returned text/html instead of stream data"
                        .to_string(),
                });
            }

            if response.body.lines().any(looks_like_sse_line) {
                return Ok(());
            }

            Err(KagiError::ResponseParse {
                endpoint: response.endpoint,
                parser: ParserShape::Stream,
                reason: "summary endpoint response did not contain SSE-style lines".to_string(),
            })
        }
        _ => Err(KagiError::InvalidClientConfiguration {
            reason: format!(
                "session validation received non-session endpoint {}",
                response.endpoint
            ),
        }),
    }
}

fn body_looks_like_html(body: &str) -> bool {
    let normalized = body.trim_start().to_ascii_lowercase();
    normalized.starts_with("<!doctype html") || normalized.starts_with("<html")
}

fn looks_like_sse_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("data:")
        || trimmed.starts_with("event:")
        || trimmed.starts_with("id:")
        || trimmed.starts_with("retry:")
        || trimmed.starts_with(':')
}
