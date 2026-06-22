use anyhow::Result;
use num_enum::{FromPrimitive, IntoPrimitive};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::protocol::{RecvFrom, SendInto};

mod vnc_authent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
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
        password: &str,
    ) -> Result<bool> {
        match self {
            SecurityType::Invalid => Ok(false),
            SecurityType::None => Ok(true),
            SecurityType::VNCAuthentication => vnc_authent::check(stream, password).await,
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
pub enum SecurityResult {
    Ok = 0,
    #[default]
    Failed = 1,
}

impl SendInto for SecurityResult {
    async fn send<S: AsyncWrite + Unpin>(&self, mut stream: S) -> Result<()> {
        Ok(stream.write_u32((*self).into()).await?)
    }
}
