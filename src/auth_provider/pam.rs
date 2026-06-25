use crate::auth_provider::{AuthProvider, SecurityResult, UserPermissions};
use anyhow::{Context, Result, anyhow};
use tokio::{sync, task::spawn_blocking};
use tracing::{error, warn};

struct PAMThreadRequest {
    user: String,
    password: String,
    reply: sync::oneshot::Sender<SecurityResult>,
}

pub struct PAMAuthProvider {
    sender: sync::mpsc::Sender<PAMThreadRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PAMAuthProviderConfig {
    channel_size: usize,
    service: String,
    control_group_name: String,
    view_group_name: String,
}

impl PAMAuthProvider {
    pub fn start(config: PAMAuthProviderConfig) -> Result<Self> {
        let (request_sender, request_receiver) =
            sync::mpsc::channel::<PAMThreadRequest>(config.channel_size);

        spawn_blocking(
            move || match pam_auth_provider_thread(config, request_receiver) {
                Ok(_) => warn!("PAMAuthProvider thread completed"),
                Err(err) => error!("PAMAuthProvider thread crashed : {err}"),
            },
        );

        Ok(Self {
            sender: request_sender,
        })
    }
}

fn pam_auth_provider_thread(
    config: PAMAuthProviderConfig,
    mut request_receiver: sync::mpsc::Receiver<PAMThreadRequest>,
) -> Result<()> {
    while let Some(request) = request_receiver.blocking_recv() {
        let mut pam_context = pam_client::Context::new(
            &config.service,
            Some(&request.user),
            pam_client::conv_mock::Conversation::with_credentials(&request.user, &request.password),
        )?;

        match pam_context.authenticate(pam_client::Flag::NONE) {
            Ok(_) => (),
            Err(err) => match err.code() {
                pam_client::ErrorCode::AUTH_ERR | pam_client::ErrorCode::USER_UNKNOWN => {
                    request
                        .reply
                        .send(SecurityResult::Denied("Wrong password".to_string()))
                        .map_err(|err| anyhow!("PAMAuthProvider : {err:?}"))?;
                    continue;
                }
                _ => return Err(anyhow!("PAMAuthProvider : {err}")),
            },
        };

        match pam_context.acct_mgmt(pam_client::Flag::NONE) {
            Ok(_) => (),
            Err(err) => match err.code() {
                pam_client::ErrorCode::AUTH_ERR | pam_client::ErrorCode::PERM_DENIED => {
                    request
                        .reply
                        .send(SecurityResult::Denied("Access denied".to_string()))
                        .map_err(|err| anyhow!("PAMAuthProvider : {err:?}"))?;
                    continue;
                }
                pam_client::ErrorCode::ACCT_EXPIRED => {
                    request
                        .reply
                        .send(SecurityResult::Denied("Account expired".to_string()))
                        .map_err(|err| anyhow!("PAMAuthProvider : {err:?}"))?;
                    continue;
                }
                _ => return Err(anyhow!("PAMAuthProvider : {err}")),
            },
        };

        let mut user_permission = UserPermissions::empty();

        let user = uzers::get_user_by_name(&request.user).context("Coudln't find user")?;
        for group in uzers::get_user_groups(user.name(), user.primary_group_id())
            .context("Couldn't gather groups for user")?
        {
            if group.name().to_string_lossy() == config.control_group_name {
                user_permission = user_permission.set_control(true);
            }

            if group.name().to_string_lossy() == config.view_group_name {
                user_permission = user_permission.set_view(true);
            }
        }

        request
            .reply
            .send(SecurityResult::Authorized(user_permission))
            .map_err(|err| anyhow!("PAMAuthProvider : {err:?}"))?;
    }

    Ok(())
}

impl AuthProvider for PAMAuthProvider {
    fn get_passwords_permissions(
        &self,
    ) -> Result<std::collections::HashMap<String, super::UserPermissions>> {
        Err(anyhow!(
            "Unsupported authetication method for this Auth Provider"
        ))
    }

    fn verify_user(
        &mut self,
        login: &str,
        password: &str,
    ) -> anyhow::Result<super::SecurityResult> {
        let (reply_sender, reply_receiver) = sync::oneshot::channel();

        self.sender.blocking_send(PAMThreadRequest {
            user: login.to_string(),
            password: password.to_string(),
            reply: reply_sender,
        })?;

        reply_receiver
            .blocking_recv()
            .map_err(|err| anyhow!("verify_user : Couldn't receive : {err}"))
    }
}
