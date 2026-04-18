use kagi_sdk::{
    official_api::models::{
        SearchRequest as OfficialSearchRequest, SummarizeGetRequest, SummarizePostRequest,
    },
    session_web::models::{SearchRequest as SessionSearchRequest, SummarizeRequest},
    BotToken, ClientConfig, KagiClient, SessionToken,
};

use crate::{
    error::{StartupError, ToolFailure},
    normalize,
    schema::{SearchToolInput, SearchToolOutput, SummarizeToolInput, SummarizeToolOutput},
};

pub const ENV_BACKEND_MODE: &str = "KAGI_MCP_BACKEND";
pub const ENV_API_KEY: &str = "KAGI_API_KEY";
pub const ENV_SESSION_TOKEN: &str = "KAGI_SESSION_TOKEN";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendMode {
    Auto,
    Official,
    Session,
}

impl BackendMode {
    fn parse(value: Option<&str>) -> Result<Self, StartupError> {
        let Some(raw_mode) = value else {
            return Ok(Self::Auto);
        };

        match raw_mode.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "official" => Ok(Self::Official),
            "session" => Ok(Self::Session),
            _ => Err(StartupError::InvalidBackendMode {
                env_var: ENV_BACKEND_MODE,
                value: raw_mode.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EnvConfig {
    pub backend_mode: Option<String>,
    pub api_key: Option<String>,
    pub session_token: Option<String>,
}

impl EnvConfig {
    pub fn read_process() -> Self {
        Self {
            backend_mode: std::env::var(ENV_BACKEND_MODE).ok(),
            api_key: std::env::var(ENV_API_KEY).ok(),
            session_token: std::env::var(ENV_SESSION_TOKEN).ok(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BackendRuntime {
    Official(KagiClient),
    Session(KagiClient),
}

impl BackendRuntime {
    pub fn from_process_env(config: ClientConfig) -> Result<Self, StartupError> {
        Self::from_env_config(EnvConfig::read_process(), config)
    }

    pub fn from_env_config(env: EnvConfig, config: ClientConfig) -> Result<Self, StartupError> {
        let mode = BackendMode::parse(env.backend_mode.as_deref())?;
        let EnvConfig {
            backend_mode: _,
            api_key,
            session_token,
        } = env;

        match mode {
            BackendMode::Auto => {
                if let Some(api_key) = api_key {
                    return Self::build_official(api_key, config);
                }

                if let Some(session_token) = session_token {
                    return Self::build_session(session_token, config);
                }

                Err(StartupError::MissingCredential {
                    env_var: ENV_API_KEY,
                    mode: "auto",
                    hint_suffix: String::new(),
                })
            }
            BackendMode::Official => {
                let api_key = api_key.ok_or_else(|| StartupError::MissingCredential {
                    env_var: ENV_API_KEY,
                    mode: "official",
                    hint_suffix: missing_credential_hint_for_official(session_token.as_deref()),
                })?;

                Self::build_official(api_key, config)
            }
            BackendMode::Session => {
                let session_token =
                    session_token.ok_or_else(|| StartupError::MissingCredential {
                        env_var: ENV_SESSION_TOKEN,
                        mode: "session",
                        hint_suffix: missing_credential_hint_for_session(api_key.as_deref()),
                    })?;

                Self::build_session(session_token, config)
            }
        }
    }

    pub async fn search(&self, input: &SearchToolInput) -> Result<SearchToolOutput, ToolFailure> {
        match self {
            Self::Official(client) => {
                let api = client
                    .official_api()
                    .map_err(ToolFailure::from_kagi_error)?;
                let request = OfficialSearchRequest::new(input.query.clone())
                    .map_err(ToolFailure::from_kagi_error)?;

                let response = api
                    .search(request)
                    .await
                    .map_err(ToolFailure::from_kagi_error)?;

                normalize::official::normalize_search(response.data, input.limit_as_usize())
            }
            Self::Session(client) => {
                let web = client.session_web().map_err(ToolFailure::from_kagi_error)?;
                let request = SessionSearchRequest::new(input.query.clone())
                    .map_err(ToolFailure::from_kagi_error)?;

                let response = web
                    .search(request)
                    .await
                    .map_err(ToolFailure::from_kagi_error)?;

                Ok(normalize::session::normalize_search(
                    response,
                    input.limit_as_usize(),
                ))
            }
        }
    }

    pub async fn summarize(
        &self,
        input: &SummarizeToolInput,
    ) -> Result<SummarizeToolOutput, ToolFailure> {
        match self {
            Self::Official(client) => {
                let api = client
                    .official_api()
                    .map_err(ToolFailure::from_kagi_error)?;

                if let Some(url) = input.url.as_deref() {
                    let request =
                        SummarizeGetRequest::new(url).map_err(ToolFailure::from_kagi_error)?;
                    let response = api
                        .summarize_get(request)
                        .await
                        .map_err(ToolFailure::from_kagi_error)?;

                    return normalize::official::normalize_summarize(response.data, Some(url));
                }

                let text = input
                    .text
                    .as_ref()
                    .expect("SummarizeToolInput guarantees exactly one of url/text");
                let request = SummarizePostRequest::from_text(text.clone())
                    .map_err(ToolFailure::from_kagi_error)?;
                let response = api
                    .summarize_post(request)
                    .await
                    .map_err(ToolFailure::from_kagi_error)?;

                normalize::official::normalize_summarize(response.data, None)
            }
            Self::Session(client) => {
                let web = client.session_web().map_err(ToolFailure::from_kagi_error)?;

                let request = if let Some(url) = input.url.as_deref() {
                    SummarizeRequest::from_url(url).map_err(ToolFailure::from_kagi_error)?
                } else {
                    let text = input
                        .text
                        .as_ref()
                        .expect("SummarizeToolInput guarantees exactly one of url/text");

                    SummarizeRequest::from_text(text.clone())
                        .map_err(ToolFailure::from_kagi_error)?
                };

                let response = web
                    .summarize(request)
                    .await
                    .map_err(ToolFailure::from_kagi_error)?;

                Ok(normalize::session::normalize_summarize(
                    response,
                    input.url.as_deref(),
                ))
            }
        }
    }

    fn build_official(api_key: String, config: ClientConfig) -> Result<Self, StartupError> {
        let token = BotToken::new(api_key).map_err(|error| match error {
            kagi_sdk::KagiError::InvalidCredential { reason, .. } => {
                StartupError::InvalidCredential {
                    env_var: ENV_API_KEY,
                    reason,
                }
            }
            unexpected => StartupError::ClientConstruction {
                reason: unexpected.to_string(),
            },
        })?;

        let client = KagiClient::new(token.into(), config).map_err(|error| {
            StartupError::ClientConstruction {
                reason: error.to_string(),
            }
        })?;

        Ok(Self::Official(client))
    }

    fn build_session(session_token: String, config: ClientConfig) -> Result<Self, StartupError> {
        let token = SessionToken::new(session_token).map_err(|error| match error {
            kagi_sdk::KagiError::InvalidCredential { reason, .. } => {
                StartupError::InvalidCredential {
                    env_var: ENV_SESSION_TOKEN,
                    reason,
                }
            }
            unexpected => StartupError::ClientConstruction {
                reason: unexpected.to_string(),
            },
        })?;

        let client = KagiClient::new(token.into(), config).map_err(|error| {
            StartupError::ClientConstruction {
                reason: error.to_string(),
            }
        })?;

        Ok(Self::Session(client))
    }
}

fn missing_credential_hint_for_official(session_token: Option<&str>) -> String {
    if !has_non_blank_credential(session_token) {
        return String::new();
    }

    format!(
        "; `{ENV_SESSION_TOKEN}` is set, so the configured credential may belong to `session` mode. Use `{ENV_API_KEY}` for `official`, or set `{ENV_BACKEND_MODE}=session` if you intended session-web auth."
    )
}

fn missing_credential_hint_for_session(api_key: Option<&str>) -> String {
    if !has_non_blank_credential(api_key) {
        return String::new();
    }

    format!(
        "; `{ENV_API_KEY}` is set, so the configured credential may belong to `official` mode. Use `{ENV_SESSION_TOKEN}` for `session`, or set `{ENV_BACKEND_MODE}=official` if you intended bot-token auth."
    )
}

fn has_non_blank_credential(value: Option<&str>) -> bool {
    value.is_some_and(|credential| !credential.trim().is_empty())
}
