use std::sync::Arc;

use anyhow::Result;
use num_enum::{FromPrimitive, IntoPrimitive};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    auth_provider::{AuthProvider, UserPermissions},
    protocol::{RecvFrom, SendInto, handshake::write_handshake_error},
};

mod vnc_authent;

#[cfg(feature = "auth_method_mslogonii")]
mod mslogonii;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    IntoPrimitive,
    serde::Deserialize,
    serde::Serialize,
)]
#[repr(u8)]
pub enum SecurityType {
    #[default]
    Invalid = 0,
    None = 1,
    VNCAuthentication = 2,
    #[cfg(feature = "auth_method_mslogonii")]
    MSLogonII = 113,
}

impl SecurityType {
    pub async fn check_password<S: AsyncWrite + AsyncRead + Unpin>(
        &self,
        stream: S,
        provider: Arc<dyn AuthProvider>,
    ) -> Result<SecurityResult> {
        match self {
            SecurityType::Invalid => Ok(SecurityResult::Denied(
                "Invalid or unknown Authentication Type".to_string(),
            )),
            SecurityType::None => Ok(SecurityResult::Authorized(UserPermissions {
                view: true,
                control: true,
            })),
            SecurityType::VNCAuthentication => vnc_authent::check(stream, provider).await,
            #[cfg(feature = "auth_method_mslogonii")]
            SecurityType::MSLogonII => mslogonii::check(stream, provider).await,
        }
    }
}

impl SendInto for Vec<SecurityType> {
    async fn send<S: AsyncWrite + Unpin>(&self, mut stream: S) -> Result<()> {
        let mut data = vec![self.len() as u8];
        data.extend(self.iter().map(|s| u8::from(*s)));
        Ok(stream.write_all(&data).await?)
    }
}

impl RecvFrom for SecurityType {
    async fn recv<S: tokio::io::AsyncRead + Unpin>(mut stream: S) -> Result<Self> {
        Ok(stream.read_u8().await?.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityResult {
    Denied(String),
    Authorized(UserPermissions),
}

impl SecurityResult {
    pub fn get_value(&self) -> u32 {
        match self {
            SecurityResult::Denied(..) => 1,
            SecurityResult::Authorized(..) => 0,
        }
    }

    pub fn is_denied(&self) -> bool {
        if let Self::Denied(_) = self {
            true
        } else {
            false
        }
    }

    pub fn get_permissions(&self) -> UserPermissions {
        match self {
            SecurityResult::Denied(_) => UserPermissions::empty(),
            SecurityResult::Authorized(user_permissions) => *user_permissions,
        }
    }
}

impl SendInto for SecurityResult {
    async fn send<S: AsyncWrite + Unpin>(&self, mut stream: S) -> Result<()> {
        stream.write_u32(self.get_value()).await?;
        if let Self::Denied(msg) = self {
            write_handshake_error(&mut stream, msg).await?;
        }
        Ok(())
    }
}

impl From<anyhow::Error> for SecurityResult {
    fn from(value: anyhow::Error) -> Self {
        SecurityResult::Denied(format!("Authentication Denied : {value}"))
    }
}
