use std::collections::HashMap;

use anyhow::Result;

use crate::auth_provider::{AuthProvider, UserPermissions};

pub struct NoneAuthProvider {
    pub login: Option<String>,
    pub password: String,
}

impl AuthProvider for NoneAuthProvider {
    fn get_passwords_permissions(
        &self,
    ) -> Result<std::collections::HashMap<String, super::UserPermissions>> {
        let mut users = HashMap::new();
        users.insert(
            self.password.clone(),
            UserPermissions {
                view: true,
                control: true,
            },
        );
        Ok(users)
    }

    fn verify_user(&self, login: &str, password: &str) -> anyhow::Result<super::SecurityResult> {
        if let Some(own_login) = &self.login
            && own_login == login
            && password == self.password
        {
            Ok(super::SecurityResult::Authorized(UserPermissions::full()))
        } else {
            Ok(super::SecurityResult::Denied("Wrong password".to_string()))
        }
    }
}
