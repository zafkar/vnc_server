use std::sync::Arc;

use anyhow::Result;
use num_enum::{FromPrimitive, IntoPrimitive};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    auth_provider::{AuthProvider, UserPermissions},
    protocol::{RecvFrom, SendInto},
};

mod vnc_authent;

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
}

impl SecurityType {
    pub async fn check_password<S: AsyncWrite + AsyncRead + Unpin>(
        &self,
        stream: S,
        provider: Arc<dyn AuthProvider>,
    ) -> Result<crate::auth_provider::SecurityResult> {
        match self {
            SecurityType::Invalid => Ok(crate::auth_provider::SecurityResult::Denied),
            SecurityType::None => Ok(crate::auth_provider::SecurityResult::Authorized(
                UserPermissions {
                    view: true,
                    control: true,
                },
            )),
            SecurityType::VNCAuthentication => vnc_authent::check(stream, provider).await,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum SecurityResultPacket {
    Ok = 0,
    #[default]
    Failed = 1,
}

impl SendInto for SecurityResultPacket {
    async fn send<S: AsyncWrite + Unpin>(&self, mut stream: S) -> Result<()> {
        Ok(stream.write_u32((*self).into()).await?)
    }
}

impl From<crate::auth_provider::SecurityResult> for SecurityResultPacket {
    fn from(value: crate::auth_provider::SecurityResult) -> Self {
        match value {
            crate::auth_provider::SecurityResult::Denied => SecurityResultPacket::Failed,
            crate::auth_provider::SecurityResult::Authorized(..) => SecurityResultPacket::Ok,
        }
    }
}
