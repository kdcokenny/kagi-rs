use reqwest::header::{HeaderMap, CONTENT_TYPE, LOCATION};
use serde_json::Value;

use crate::{
    auth::Credentials,
    config::ClientConfig,
    error::KagiError,
    routing::{EndpointId, ProtocolSurface},
};

#[derive(Debug, Clone)]
pub(crate) struct Transport {
    http: reqwest::Client,
    config: ClientConfig,
}

#[derive(Debug, Clone)]
pub(crate) struct TransportResponse {
    pub endpoint: EndpointId,
    pub status: u16,
    pub body: String,
    pub content_type: Option<String>,
    pub redirect_location: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum RequestBody {
    Empty,
    Json(Value),
    Form(Vec<(String, String)>),
}

impl Transport {
    pub(crate) fn new(config: ClientConfig) -> Result<Self, KagiError> {
        config.validate()?;

        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(config.user_agent.clone())
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|source| KagiError::InvalidClientConfiguration {
                reason: format!("failed to build reqwest client: {source}"),
            })?;

        Ok(Self { http, config })
    }

    pub(crate) async fn execute(
        &self,
        credentials: &Credentials,
        endpoint: EndpointId,
        query: &[(String, String)],
        body: RequestBody,
    ) -> Result<TransportResponse, KagiError> {
        let spec = endpoint.spec();
        let provided_kind = credentials.kind();

        if provided_kind != spec.allowed_credential {
            return Err(KagiError::UnsupportedCapability {
                endpoint,
                credential: provided_kind,
                expected: spec.allowed_credential,
            });
        }

        let mut headers = HeaderMap::new();
        credentials.apply_to_headers(&mut headers)?;

        let url = self.config.base_url.join(spec.route).map_err(|source| {
            KagiError::InvalidClientConfiguration {
                reason: format!("invalid route join for {endpoint}: {source}"),
            }
        })?;

        let mut request = self
            .http
            .request(spec.method.as_reqwest(), url)
            .headers(headers);
        if !query.is_empty() {
            request = request.query(query);
        }

        request = match body {
            RequestBody::Empty => request,
            RequestBody::Json(json) => request.json(&json),
            RequestBody::Form(form) => request.form(&form),
        };

        let response = request
            .send()
            .await
            .map_err(|source| KagiError::Transport { endpoint, source })?;

        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let redirect_location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let body = response
            .text()
            .await
            .map_err(|source| KagiError::Transport { endpoint, source })?;

        if matches!(status, 401 | 403) {
            return match spec.surface {
                ProtocolSurface::OfficialApi => Err(KagiError::UnauthorizedBotToken {
                    endpoint,
                    message: body,
                }),
                ProtocolSurface::SessionWeb => Err(KagiError::InvalidSession {
                    endpoint,
                    status,
                    message: body,
                }),
            };
        }

        Ok(TransportResponse {
            endpoint,
            status,
            body,
            content_type,
            redirect_location,
        })
    }
}
