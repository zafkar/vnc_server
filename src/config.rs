use std::{sync::Arc, time::Duration};

use anyhow::Result;

use crate::{
    auth_provider::{AuthProvider, file_auth::FileAuthProvider, none_provider::NoneAuthProvider},
    protocol::{handshake::security::SecurityType, pixel_format::PixelFormat},
};

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub capture: CaptureConfig,
    #[serde(default)]
    pub auth_provider: AuthProviderConfig,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum AuthProviderConfig {
    File { path: String },
    None { password: String },
}

impl Default for AuthProviderConfig {
    fn default() -> Self {
        AuthProviderConfig::None {
            password: "password".to_string(),
        }
    }
}

impl AuthProviderConfig {
    pub async fn init(&self) -> Result<Arc<dyn AuthProvider>> {
        match self {
            AuthProviderConfig::File { path } => Ok(Arc::new(FileAuthProvider::load(path).await?)),
            AuthProviderConfig::None { password } => Ok(Arc::new(NoneAuthProvider {
                password: password.clone(),
            })),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_channel_size")]
    pub channel_size: usize,
    #[serde(default = "default_auth_protocols")]
    pub auth_protocols: Vec<SecurityType>,
    #[serde(default)]
    pub pixel_format: PixelFormat,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            channel_size: default_channel_size(),
            auth_protocols: default_auth_protocols(),
            pixel_format: Default::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CaptureConfig {
    #[serde(default = "default_delay_between_frames")]
    pub time_between_frame: Duration,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            time_between_frame: Duration::from_millis(15),
        }
    }
}

fn default_bind_address() -> String {
    "0.0.0.0:5900".to_string()
}

fn default_channel_size() -> usize {
    128
}

fn default_auth_protocols() -> Vec<SecurityType> {
    vec![SecurityType::VNCAuthentication]
}

fn default_delay_between_frames() -> Duration {
    Duration::from_millis(15)
}
