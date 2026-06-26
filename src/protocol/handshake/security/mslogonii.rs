use anyhow::{Context, Result, anyhow};
use des::cipher::{BlockCipherDecrypt, KeyInit, consts::U8};
use num_prime::nt_funcs::is_prime;
use purecrypto::{bignum::BoxedUint, dh::DhGroup};
use rand::RngExt;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{auth_provider::AuthProvider, protocol::handshake::security::SecurityResult};

const MAX_NUM: u64 = 1 << 31;

pub async fn check<S: AsyncWrite + AsyncRead + Unpin>(
    mut stream: S,
    provider: Arc<dyn AuthProvider>,
) -> Result<SecurityResult> {
    let mut g = generate_prime();
    let mut modulus = generate_prime();

    if g > modulus {
        std::mem::swap(&mut g, &mut modulus);
    }

    let private_key = {
        let mut rng = rand::rng();
        rng.random::<u64>()
    };

    let public_key = modpow(g, private_key, modulus);

    stream.write_u64(g).await?;
    stream.write_u64(modulus).await?;
    stream.write_u64(public_key).await?;

    let client_public = stream.read_u64().await?;
    let shared_secret = modpow(client_public, private_key, modulus);

    let mut username_crypted = [0u8; 256];
    stream.read_exact(&mut username_crypted).await?;
    let mut password_crypted = [0u8; 64];
    stream.read_exact(&mut password_crypted).await?;

    let key = shared_secret.to_be_bytes();

    vnc_decrypt_bytes(&mut username_crypted, &key)?;
    let username = u8_null_to_string(&username_crypted)?;
    dbg!(&username);

    vnc_decrypt_bytes(&mut password_crypted, &key)?;
    let password = u8_null_to_string(&password_crypted)?;
    dbg!(&password);

    provider.verify_user(&username, &password)
}

fn u8_null_to_string(data: &[u8]) -> Result<String> {
    let mut result = String::new();
    for c in data.iter() {
        if *c == 0 {
            return Ok(result);
        }
        result += &char::from_u32(*c as u32)
            .context("Invalid char")?
            .to_string();
    }
    Err(anyhow!("No null byte found"))
}

fn vnc_decrypt_bytes(data: &mut [u8], key: &[u8; 8]) -> anyhow::Result<()> {
    let des = des::Des::new_from_slice(key)?;

    let mut i = 8;

    while i < data.len() {
        let mut block: des::cipher::Array<u8, U8> = des::cipher::Array::try_from(&data[i..i + 8])?;

        des.decrypt_block(&mut block);

        for j in 0..8 {
            block[j] ^= data[i + j - 8];
        }

        data[i..i + 8].copy_from_slice(&block);

        i += 8;
    }

    let mut block: des::cipher::Array<u8, U8> = des::cipher::Array::try_from(&data[0..8])?;

    des.decrypt_block(&mut block);

    for j in 0..8 {
        block[j] ^= key[j];
    }

    data[0..8].copy_from_slice(&block);

    Ok(())
}

// fn decrypt_full_array(des_crypter: &des::Des, block: &[u8]) -> Result<Vec<u8>> {
//     Ok(block
//         .chunks(8)
//         .map(|b| decrypt_8b(des_crypter, b))
//         .collect::<Result<Vec<Vec<u8>>>>()?
//         .into_iter()
//         .flatten()
//         .collect())
// }

// fn decrypt_8b(des_crypter: &des::Des, block: &[u8]) -> Result<Vec<u8>> {
//     let mut client_des_block: des::cipher::Array<u8, U8> = des::cipher::Array::try_from(block)?;
//     des_crypter.decrypt_block(&mut client_des_block);

//     Ok(client_des_block.iter().cloned().collect())
// }

fn modpow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result = 1;

    while exp > 0 {
        if exp & 1 == 1 {
            result = result * base % modulus;
        }

        base = base * base % modulus;
        exp >>= 1;
    }

    result
}

fn generate_prime() -> u64 {
    let mut rng = rand::rng();
    loop {
        let start = rng.random_range(0..MAX_NUM);

        if let Some(p) = try_to_generate_prime(start) {
            return p;
        }
    }
}

fn try_to_generate_prime(mut n: u64) -> Option<u64> {
    // make odd
    n |= 1;

    loop {
        if is_prime(&n, None).probably() {
            return Some(n);
        }

        n += 2;

        // prevent running past the intended range
        if n >= MAX_NUM {
            return None;
        }
    }
}
