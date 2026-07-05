use std::{sync::Arc, time::Duration};

use anyhow::Result;

#[cfg(feature = "auth_provider_winlogon")]
use crate::auth_provider::windows_logon::WinLogonAuthProvider;
use crate::{
    auth_provider::{AuthProvider, none_provider::NoneAuthProvider},
    protocol::{handshake::security::SecurityType, pixel_format::PixelFormat},
};

#[cfg(feature = "auth_provider_file")]
use crate::auth_provider::file_auth::FileAuthProvider;

#[cfg(feature = "auth_provider_pam")]
use crate::auth_provider::pam::{PAMAuthProvider, PAMAuthProviderConfig};

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub capture: CaptureConfig,
    #[serde(default)]
    pub auth_provider: AuthProviderConfig,
    #[cfg(feature = "management")]
    #[serde(default)]
    pub management: Option<ManagmentServerConfig>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum AuthProviderConfig {
    #[cfg(feature = "auth_provider_file")]
    File { path: String },
    None {
        login: Option<String>,
        password: String,
    },
    #[cfg(feature = "auth_provider_pam")]
    PAM(PAMAuthProviderConfig),
    #[cfg(feature = "auth_provider_winlogon")]
    WinLogon(WinLogonAuthProviderConfig),
}

impl Default for AuthProviderConfig {
    fn default() -> Self {
        AuthProviderConfig::None {
            login: Some("admin".to_string()),
            password: "password".to_string(),
        }
    }
}

impl AuthProviderConfig {
    pub async fn init(&self) -> Result<Arc<dyn AuthProvider>> {
        match self {
            #[cfg(feature = "auth_provider_file")]
            AuthProviderConfig::File { path } => Ok(Arc::new(FileAuthProvider::load(path).await?)),
            AuthProviderConfig::None { password, login } => Ok(Arc::new(NoneAuthProvider {
                password: password.clone(),
                login: login.clone(),
            })),
            #[cfg(feature = "auth_provider_pam")]
            AuthProviderConfig::PAM(pamauth_provider_config) => Ok(Arc::new(
                PAMAuthProvider::start(pamauth_provider_config.clone())?,
            )),
            #[cfg(feature = "auth_provider_winlogon")]
            AuthProviderConfig::WinLogon(winlogon_auth_config) => {
                Ok(Arc::new(WinLogonAuthProvider(winlogon_auth_config)))
            }
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

#[cfg(feature = "management")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ManagmentServerConfig {
    #[serde(default = "default_mgmt_bind_address")]
    pub bind_address: String,
}

#[cfg(feature = "management")]
impl Default for ManagmentServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_mgmt_bind_address(),
        }
    }
}

#[cfg(feature = "management")]
fn default_mgmt_bind_address() -> String {
    "127.0.0.1:5899".to_string()
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
