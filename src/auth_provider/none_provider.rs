use std::collections::HashMap;

use crate::auth_provider::{AuthProvider, UserPermissions};

pub struct NoneAuthProvider {
    pub password: String,
}

impl AuthProvider for NoneAuthProvider {
    fn get_passwords_permissions(
        &self,
    ) -> std::collections::HashMap<String, super::UserPermissions> {
        let mut users = HashMap::new();
        users.insert(
            self.password.clone(),
            UserPermissions {
                view: true,
                control: true,
            },
        );
        users
    }
}
