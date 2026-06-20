use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use vnc_server::server::start_server;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer())
        .init();

    start_server().await
}
