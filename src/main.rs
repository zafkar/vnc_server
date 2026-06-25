use anyhow::Result;
use tokio::fs;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use vnc_server::{config::Config, server::VNCServer};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer())
        .init();

    let config = match load_config().await {
        Ok(conf) => conf,
        Err(err) => {
            warn!("{err} : The configuration couldn't be loaded, resorting to default config");
            Config::default()
        }
    };
    info!("Starting with config : {config:#?}");
    VNCServer { config }.start().await
}

async fn load_config() -> Result<Config> {
    let text = fs::read_to_string("config.ron").await?;
    Ok(ron::from_str(&text)?)
}
