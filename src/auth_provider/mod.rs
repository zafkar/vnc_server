use std::collections::HashMap;

use anyhow::Result;

use crate::protocol::handshake::security::SecurityResult;

#[cfg(feature = "auth_provider_file")]
pub mod file_auth;
pub mod none_provider;
#[cfg(feature = "auth_provider_pam")]
pub mod pam;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct UserPermissions {
    #[serde(default)]
    pub view: bool,
    #[serde(default)]
    pub control: bool,
}

impl UserPermissions {
    pub fn full() -> Self {
        Self {
            view: true,
            control: true,
        }
    }

    pub fn empty() -> Self {
        Self {
            view: false,
            control: false,
        }
    }

    pub fn set_view(mut self, value: bool) -> Self {
        self.view = value;
        self
    }

    pub fn set_control(mut self, value: bool) -> Self {
        self.control = value;
        self
    }
}

pub trait AuthProvider: Send + Sync {
    fn get_passwords_permissions(&self) -> Result<HashMap<String, UserPermissions>>;
    fn verify_user(&self, login: &str, password: &str) -> Result<SecurityResult>;
}
