use std::sync::Arc;

use anyhow::Result;
use des::cipher::{BlockCipherEncrypt, KeyInit, consts::U8};
use rand::RngExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::debug;

use crate::auth_provider::{SecurityResult, file_auth::FileAuthProvider};

pub async fn check<S: AsyncWrite + AsyncRead + Unpin>(
    mut stream: S,
    provider: Arc<FileAuthProvider>,
) -> Result<SecurityResult> {
    let challenge: [u8; 16] = {
        let mut rng = rand::rng();
        rng.random()
    };

    debug!("Challenge : {challenge:?}");

    stream.write_all(&challenge).await?;

    let mut client_challenge = [0u8; 16];
    stream.read_exact(&mut client_challenge).await?;

    for (password, permissions) in provider.get_passwords_permissions() {
        if encrypt_challenge(challenge.clone(), &password)? == client_challenge {
            return Ok(SecurityResult::Authorized(permissions));
        }
    }
    Ok(SecurityResult::Denied)
}

fn encrypt_8b(des_crypter: &des::Des, block: &[u8]) -> Result<Vec<u8>> {
    let mut client_des_block: des::cipher::Array<u8, U8> = des::cipher::Array::try_from(block)?;
    des_crypter.encrypt_block(&mut client_des_block);

    Ok(client_des_block.iter().cloned().collect())
}

fn encrypt_challenge(challenge: [u8; 16], password: &str) -> Result<Vec<u8>> {
    let password_bytes = password.as_bytes();
    let des_password = if password_bytes.len() > 8 {
        password_bytes[0..8].to_vec()
    } else {
        let mut unpadded = password_bytes.to_vec();
        unpadded.extend_from_slice(&vec![0; 8 - unpadded.len()]);
        unpadded
    }
    .iter()
    .map(|b| b.reverse_bits())
    .collect::<Vec<u8>>();
    debug!("des_key :  {des_password:?}");

    let des_crypter = des::Des::new_from_slice(&des_password)?;

    let mut encrypted_challenge = encrypt_8b(&des_crypter, &challenge[0..8])?;
    encrypted_challenge.extend_from_slice(&encrypt_8b(&des_crypter, &challenge[8..16])?);

    Ok(encrypted_challenge)
}
