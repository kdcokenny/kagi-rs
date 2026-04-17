pub mod models;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    client::KagiClient,
    error::KagiError,
    routing::EndpointId,
    transport::{RequestBody, TransportResponse},
};

use self::models::{
    EnrichNewsRequest, EnrichNewsResponse, EnrichWebRequest, EnrichWebResponse, FastGptRequest,
    FastGptResponse, SearchRequest, SearchResponse, SmallwebFeedRequest, SmallwebFeedResponse,
    SummarizeGetRequest, SummarizeGetResponse, SummarizePostRequest, SummarizePostResponse,
};

#[derive(Debug)]
pub struct OfficialApi<'a> {
    client: &'a KagiClient,
}

impl<'a> OfficialApi<'a> {
    pub(crate) fn new(client: &'a KagiClient) -> Self {
        Self { client }
    }

    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse, KagiError> {
        let response = self
            .request(
                EndpointId::OfficialSearch,
                request.into_query(),
                RequestBody::Empty,
            )
            .await?;
        let data = parse_json_envelope::<Value>(response)?;
        Ok(SearchResponse { data })
    }

    pub async fn enrich_web(
        &self,
        request: EnrichWebRequest,
    ) -> Result<EnrichWebResponse, KagiError> {
        let response = self
            .request(
                EndpointId::OfficialEnrichWeb,
                request.into_query(),
                RequestBody::Empty,
            )
            .await?;
        let data = parse_json_envelope::<Value>(response)?;
        Ok(EnrichWebResponse { data })
    }

    pub async fn enrich_news(
        &self,
        request: EnrichNewsRequest,
    ) -> Result<EnrichNewsResponse, KagiError> {
        let response = self
            .request(
                EndpointId::OfficialEnrichNews,
                request.into_query(),
                RequestBody::Empty,
            )
            .await?;
        let data = parse_json_envelope::<Value>(response)?;
        Ok(EnrichNewsResponse { data })
    }

    pub async fn summarize_get(
        &self,
        request: SummarizeGetRequest,
    ) -> Result<SummarizeGetResponse, KagiError> {
        let response = self
            .request(
                EndpointId::OfficialSummarizeGet,
                request.into_query(),
                RequestBody::Empty,
            )
            .await?;
        let data = parse_json_envelope::<Value>(response)?;
        Ok(SummarizeGetResponse { data })
    }

    pub async fn summarize_post(
        &self,
        request: SummarizePostRequest,
    ) -> Result<SummarizePostResponse, KagiError> {
        let response = self
            .request(
                EndpointId::OfficialSummarizePost,
                Vec::new(),
                RequestBody::Json(request.into_json()),
            )
            .await?;
        let data = parse_json_envelope::<Value>(response)?;
        Ok(SummarizePostResponse { data })
    }

    pub async fn fastgpt(&self, request: FastGptRequest) -> Result<FastGptResponse, KagiError> {
        let response = self
            .request(
                EndpointId::OfficialFastGpt,
                Vec::new(),
                RequestBody::Json(request.into_json()),
            )
            .await?;
        let data = parse_json_envelope::<Value>(response)?;
        Ok(FastGptResponse { data })
    }

    pub async fn smallweb_feed(
        &self,
        request: SmallwebFeedRequest,
    ) -> Result<SmallwebFeedResponse, KagiError> {
        let response = self
            .request(
                EndpointId::OfficialSmallwebFeed,
                request.into_query(),
                RequestBody::Empty,
            )
            .await?;
        let data = parse_json_envelope::<Value>(response)?;
        Ok(SmallwebFeedResponse { data })
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

#[derive(Debug, serde::Deserialize)]
struct ApiEnvelope<T> {
    data: Option<T>,
    #[serde(default)]
    error: Option<Value>,
}

fn parse_json_envelope<T>(response: TransportResponse) -> Result<T, KagiError>
where
    T: DeserializeOwned,
{
    let endpoint = response.endpoint;
    let status = response.status;
    let json_body: Value =
        serde_json::from_str(&response.body).map_err(|source| KagiError::ResponseParse {
            endpoint,
            parser: endpoint.spec().parser,
            reason: format!("response body is not valid JSON: {source}"),
        })?;

    if status >= 400 {
        let (code, message) = extract_error_details(&json_body).unwrap_or((
            None,
            format!("HTTP {status} returned without parseable envelope message"),
        ));

        return Err(KagiError::ApiFailure {
            endpoint,
            status,
            code,
            message,
        });
    }

    let envelope: ApiEnvelope<T> =
        serde_json::from_value(json_body.clone()).map_err(|source| KagiError::ResponseParse {
            endpoint,
            parser: endpoint.spec().parser,
            reason: format!("invalid JSON envelope shape: {source}"),
        })?;

    if envelope.error.as_ref().is_some_and(is_failure_marker) {
        let (code, message) = extract_error_details(&json_body).unwrap_or((
            None,
            "API envelope reported failure without details".to_string(),
        ));

        return Err(KagiError::ApiFailure {
            endpoint,
            status,
            code,
            message,
        });
    }

    if let Some(data) = envelope.data {
        return Ok(data);
    }

    Err(KagiError::ResponseParse {
        endpoint,
        parser: endpoint.spec().parser,
        reason: "API envelope did not contain `data`".to_string(),
    })
}

fn extract_error_details(value: &Value) -> Option<(Option<String>, String)> {
    let object = value.as_object()?;

    let message = object
        .get("message")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            object
                .get("error")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            object
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "unknown API failure".to_string());

    let code = object
        .get("code")
        .and_then(value_to_code)
        .or_else(|| object.get("error").and_then(value_to_code));

    Some((code, message))
}

fn value_to_code(value: &Value) -> Option<String> {
    if let Some(code) = value.as_str() {
        return Some(code.to_string());
    }

    if let Some(code) = value.as_i64() {
        return Some(code.to_string());
    }

    value
        .as_object()
        .and_then(|object| object.get("code"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn is_failure_marker(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(is_error) => *is_error,
        Value::String(message) => !message.trim().is_empty(),
        Value::Number(number) => match number.as_i64() {
            Some(value) => value != 0,
            None => true,
        },
        Value::Array(items) => !items.is_empty(),
        Value::Object(_) => true,
    }
}
