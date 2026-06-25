use std::collections::HashMap;

pub mod file_auth;
pub mod none_provider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityResult {
    Denied,
    Authorized(UserPermissions),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct UserPermissions {
    #[serde(default)]
    pub view: bool,
    #[serde(default)]
    pub control: bool,
}

pub trait AuthProvider: Send + Sync {
    fn get_passwords_permissions(&self) -> HashMap<String, UserPermissions>;
}
