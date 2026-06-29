use std::net::SocketAddr;

use serde::{Serialize, Serializer};
use tokio::{sync, task::AbortHandle};
use uuid::Uuid;

use crate::{
    auth_provider::UserPermissions,
    mgmt_server::stats::Stats,
    protocol::{
        encodings::EncodingType, handshake::security::SecurityType, pixel_format::PixelFormat,
    },
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Client {
    pub uuid: Uuid,
    #[serde(skip_serializing)]
    pub abort_handle: AbortHandle,
    #[serde(serialize_with = "serialize_watch_client")]
    pub client_info: sync::watch::Receiver<ClientInfo>,
}

fn serialize_watch_client<S>(
    watcher: &sync::watch::Receiver<ClientInfo>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    watcher.borrow().serialize(serializer)
}

impl Client {
    pub fn new(abort_handle: AbortHandle, client_info: sync::watch::Receiver<ClientInfo>) -> Self {
        Self {
            uuid: Uuid::now_v7(),
            abort_handle,
            client_info,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ClientInfo {
    pub status: ClientStatus,
    pub addr: SocketAddr,
    pub auth_type: Option<SecurityType>,
    pub permissions: Option<UserPermissions>,
    pub encoding: Option<EncodingType>,
    pub pixel_format: Option<PixelFormat>,
    pub time_for_frame_stats: Stats,
}

impl ClientInfo {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            status: ClientStatus::default(),
            addr,
            auth_type: None,
            permissions: None,
            encoding: None,
            pixel_format: None,
            time_for_frame_stats: Stats::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ClientStatus {
    #[default]
    Starting,
    Authorized,
    Initialized,
    Running,
    Dead,
}
