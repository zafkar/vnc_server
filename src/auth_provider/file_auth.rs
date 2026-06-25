use std::collections::HashMap;

use anyhow::Result;
use tokio::fs;

use crate::{
    auth_provider::{AuthProvider, UserPermissions},
    protocol::handshake::security::SecurityResult,
};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FileAuthProvider {
    users: Vec<User>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    login: Option<String>,
    password: String,
    permission: UserPermissions,
}

impl User {
    pub fn new(login: Option<String>, password: String, permission: UserPermissions) -> Self {
        Self {
            login,
            password,
            permission,
        }
    }
}

impl FileAuthProvider {
    pub async fn load(path: &str) -> Result<Self> {
        let text = fs::read_to_string(path).await?;
        Ok(ron::from_str(&text)?)
    }

    pub fn new(users: &[User]) -> Self {
        Self {
            users: users.to_vec(),
        }
    }
}

impl AuthProvider for FileAuthProvider {
    fn get_passwords_permissions(&self) -> Result<HashMap<String, UserPermissions>> {
        Ok(self.users.iter().fold(HashMap::new(), |mut acc, user| {
            acc.insert(user.password.clone(), user.permission);
            acc
        }))
    }

    fn verify_user(&mut self, login: &str, password: &str) -> Result<SecurityResult> {
        for user in self.users.iter() {
            if user.login == Some(login.to_string()) && user.password == password {
                return Ok(SecurityResult::Authorized(user.permission));
            }
        }

        Ok(SecurityResult::Denied("Wrong password".to_string()))
    }
}
