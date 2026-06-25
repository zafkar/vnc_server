pub mod file_auth;

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
