use anyhow::{Result, anyhow};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{
            CheckTokenMembership, LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT, LogonUserW,
            LookupAccountNameW, PSID, SID_NAME_USE,
        },
    },
    core::{PCWSTR, PWSTR},
};

use crate::{
    auth_provider::{AuthProvider, UserPermissions},
    protocol::handshake::security::SecurityResult,
};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct WinLogon(Option<HANDLE>);

impl WinLogon {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn login(&mut self, username: &str, domain: Option<&str>, password: &str) -> Result<bool> {
        let user = to_wide(username);
        let domain = to_wide(domain.unwrap_or("."));
        let pass = to_wide(password);

        let mut token = Default::default();

        let ok = unsafe {
            LogonUserW(
                PCWSTR(user.as_ptr()),
                PCWSTR(domain.as_ptr()),
                PCWSTR(pass.as_ptr()),
                LOGON32_LOGON_INTERACTIVE,
                LOGON32_PROVIDER_DEFAULT,
                &mut token,
            )
        };

        if ok.is_ok() {
            self.0 = Some(token);
            Ok(true)
        } else {
            Err(anyhow!("Failed to connect"))
        }
    }

    pub fn is_member(&self, system: Option<&str>, group: &str) -> Result<bool> {
        let Some(token) = self.0 else {
            return Err(anyhow!("User is not logged in"));
        };
        let sid = lookup_group_sid(system, group)?;

        let mut member = false.into();

        unsafe {
            CheckTokenMembership(Some(token), sid, &mut member)?;
        }

        Ok(member.as_bool())
    }
}

fn lookup_group_sid(system: Option<&str>, group: &str) -> Result<PSID> {
    let system = system.map(to_wide);
    let group = to_wide(group);

    let system_ptr = system
        .as_ref()
        .map(|s| PCWSTR(s.as_ptr()))
        .unwrap_or(PCWSTR::null());

    let mut sid_size = 0u32;
    let mut domain_size = 0u32;
    let mut sid_use = SID_NAME_USE(0);

    // First call gets required buffer sizes.
    unsafe {
        let _ = LookupAccountNameW(
            system_ptr,
            PCWSTR(group.as_ptr()),
            None,
            &mut sid_size,
            None,
            &mut domain_size,
            &mut sid_use,
        );
    }

    let sid = PSID::default();
    let mut domain = vec![0u16; domain_size as usize];

    unsafe {
        LookupAccountNameW(
            system_ptr,
            PCWSTR(group.as_ptr()),
            Some(sid),
            &mut sid_size,
            Some(PWSTR(domain.as_mut_ptr())),
            &mut domain_size,
            &mut sid_use,
        )?;
    }

    Ok(sid)
}

impl Drop for WinLogon {
    fn drop(&mut self) {
        if let Some(token) = self.0 {
            unsafe {
                let _ = CloseHandle(token);
            };
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct WinLogonAuthProviderConfig {
    #[serde(default = "default_system")]
    target_system: String,
    control_group_name: String,
    view_group_name: String,
}

fn default_system() -> String {
    ".".to_string()
}

pub struct WinLogonAuthProvider(pub WinLogonAuthProviderConfig);

impl AuthProvider for WinLogonAuthProvider {
    fn get_passwords_permissions(
        &self,
    ) -> Result<std::collections::HashMap<String, super::UserPermissions>> {
        Err(anyhow!(
            "This WinLogonAuthProvider doesn't support password recovery"
        ))
    }

    fn verify_user(&self, login: &str, password: &str) -> Result<SecurityResult> {
        let mut winlogon = WinLogon::new();

        let (domain, username) = if let Some((domain, username)) = login.split_once("\\") {
            (Some(domain), username)
        } else {
            (None, login)
        };

        if !winlogon.login(username, domain, password)? {
            return Ok(SecurityResult::Denied("Wrong password or user".to_string()));
        };

        let mut permissions = UserPermissions::empty();
        if winlogon.is_member(Some(&self.0.target_system), &self.0.view_group_name)? {
            permissions = permissions.set_view(true);
        }

        if winlogon.is_member(Some(&self.0.target_system), &self.0.control_group_name)? {
            permissions = permissions.set_control(true);
        }

        Ok(SecurityResult::Authorized(permissions))
    }
}
