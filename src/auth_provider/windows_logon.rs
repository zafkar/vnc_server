use std::marker::PhantomData;

use anyhow::{Result, anyhow};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{
            CheckTokenMembership, LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT, LogonUserW,
            LookupAccountNameW,
        },
    },
    core::PCWSTR,
};

use crate::{
    auth_provider::{AuthProvider, UserPermissions},
    protocol::handshake::security::SecurityResult,
};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct WinLogon<T>(T);

impl WinLogon<()> {
    pub fn new() -> Self {
        Self(())
    }

    pub fn login(
        self,
        username: &str,
        domain: Option<&str>,
        password: &str,
    ) -> Result<WinLogon<HANDLE>> {
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
            Ok(WinLogon(token))
        } else {
            Err(anyhow!("Failed to connect"))
        }
    }
}

fn lookup_group_sid(system: Option<&str>, group: &str) -> Result<Vec<u8>> {
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
            PWSTR::null(),
            &mut domain_size,
            &mut sid_use,
        );
    }

    let mut sid = vec![0u8; sid_size as usize];
    let mut domain = vec![0u16; domain_size as usize];

    unsafe {
        LookupAccountNameW(
            system_ptr,
            PCWSTR(group.as_ptr()),
            Some(sid.as_mut_ptr() as _),
            &mut sid_size,
            PWSTR(domain.as_mut_ptr()),
            &mut domain_size,
            &mut sid_use,
        )?;
    }

    Ok(sid)
}

impl WinLogon<HANDLE> {
    pub fn is_member(&self, system: Option<&str>, group: &str) -> Result<bool> {
        let sid = lookup_group_sid(system, group)?;

        let mut member = false.into();

        unsafe {
            CheckTokenMembership(self.0, sid.as_ptr() as _, &mut member)?;
        }

        Ok(member.as_bool())
    }
}

impl Drop for WinLogon<HANDLE> {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
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

pub struct WinLogonAuthProvider(WinLogonAuthProviderConfig);

impl AuthProvider for WinLogonAuthProvider {
    fn get_passwords_permissions(
        &self,
    ) -> Result<std::collections::HashMap<String, super::UserPermissions>> {
        Err(anyhow!(
            "This WinLogonAuthProvider doesn't support password recovery"
        ))
    }

    fn verify_user(&self, login: &str, password: &str) -> Result<SecurityResult> {
        let winlogon = WinLogon::new();

        let (domain, username) = if let Some((domain, username)) = login.split_once("\\") {
            (Some(domain), username)
        } else {
            (None, username)
        };

        let Ok(winlogon_logged) = winlogon.login(username, domain, password) else {
            return Ok(SecurityResult::Denied("Wrong password or user".to_string()));
        };

        let mut permissions = UserPermissions::empty();
        if winlogon_logged.is_member(self.0.target_system, self.0.view_group_name) {
            permissions.set_view(true);
        }

        if winlogon_logged.is_member(self.0.target_system, self.0.control_group_name) {
            permissions.set_control(true);
        }

        Ok(SecurityResult::Authorized(permissions))
    }
}
