use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_tracing();

    let server = kagi_mcp::KagiMcpServer::from_env()?;
    server.serve_stdio().await
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    let _ = fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();
}
