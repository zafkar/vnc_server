use std::collections::HashMap;

use anyhow::Result;
use tokio::fs;

use crate::auth_provider::{AuthProvider, UserPermissions};

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
    fn get_passwords_permissions(&self) -> HashMap<String, UserPermissions> {
        self.users.iter().fold(HashMap::new(), |mut acc, user| {
            acc.insert(user.password.clone(), user.permission);
            acc
        })
    }
}
