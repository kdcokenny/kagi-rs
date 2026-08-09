#![forbid(unsafe_code)]

mod backend;
mod error;
pub mod normalize;
mod schema;

use backend::BackendRuntime;
pub use backend::{ENV_API_KEY, ENV_BACKEND_MODE, ENV_SESSION_TOKEN};
pub use error::StartupError;
use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
pub use schema::{SearchResultCard, SearchToolOutput, SummarizeToolOutput};

#[derive(Debug, Clone)]
pub struct KagiMcpServer {
    backend: BackendRuntime,
}

impl KagiMcpServer {
    pub fn from_env() -> Result<Self, StartupError> {
        let backend = BackendRuntime::from_process_env(kagi_sdk::ClientConfig::default())?;
        Ok(Self::from_backend(backend))
    }

    fn from_backend(backend: BackendRuntime) -> Self {
        Self { backend }
    }

    pub async fn serve_stdio(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let running = self.serve(rmcp::transport::stdio()).await?;
        let _ = running.waiting().await?;
        Ok(())
    }
}

#[tool_router]
impl KagiMcpServer {
    #[tool(
        name = "kagi_search",
        description = "Search Kagi and return normalized result cards.",
        annotations(read_only_hint = true, idempotent_hint = true)
    )]
    async fn kagi_search(
        &self,
        Parameters(input): Parameters<schema::SearchToolInput>,
    ) -> Result<Json<schema::SearchToolOutput>, String> {
        self.backend
            .search(&input)
            .await
            .map(Json)
            .map_err(|error| error.message().to_string())
    }

    #[tool(
        name = "kagi_summarize",
        description = "Summarize a URL or raw text with Kagi.",
        annotations(read_only_hint = true, idempotent_hint = true)
    )]
    async fn kagi_summarize(
        &self,
        Parameters(input): Parameters<schema::SummarizeToolInput>,
    ) -> Result<Json<schema::SummarizeToolOutput>, String> {
        self.backend
            .summarize(&input)
            .await
            .map(Json)
            .map_err(|error| error.message().to_string())
    }
}

#[tool_handler(name = "kagi-mcp")]
impl ServerHandler for KagiMcpServer {}

#[cfg(test)]
mod tests;
