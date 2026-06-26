use anyhow::{Context, Result, anyhow};
use des::cipher::{BlockCipherDecrypt, KeyInit, consts::U8};
use num_prime::nt_funcs::is_prime;
use rand::RngExt;
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    task::spawn_blocking,
};

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

    let handle = spawn_blocking(move || provider.verify_user(&username, &password));

    handle.await?
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
    let mut reversed_key = [0u8; 8];
    for (i, byte) in key.iter().enumerate() {
        reversed_key[i] = byte.reverse_bits();
    }
    let des = des::Des::new_from_slice(&reversed_key)?;

    let mut i = data.len();
    while i > 8 {
        i -= 8;

        let mut current_ciphertext = [0u8; 8];
        current_ciphertext.copy_from_slice(&data[i..i + 8]);
        let mut block: des::cipher::Array<u8, U8> =
            des::cipher::Array::try_from(current_ciphertext)?;

        des.decrypt_block(&mut block);

        let prev_ciphertext = &data[i - 8..i];
        for j in 0..8 {
            block[j] ^= prev_ciphertext[j];
        }

        data[i..i + 8].copy_from_slice(&block);
    }

    let mut block: des::cipher::Array<u8, U8> = des::cipher::Array::try_from(&data[0..8])?;

    des.decrypt_block(&mut block);

    for j in 0..8 {
        block[j] ^= key[j];
    }

    data[0..8].copy_from_slice(&block);

    Ok(())
}

#[cfg(test)]
fn decrypt_cpp_style(data: &mut [u8], key: &[u8; 8]) -> Vec<u8> {
    let mut result = data.to_vec();
    vnc_decrypt_bytes(&mut result, key).unwrap();
    result
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    fn hex_to_bytes(hex: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(hex.len() / 2);
        for chunk in hex.as_bytes().chunks(2) {
            let byte_str = std::str::from_utf8(chunk).unwrap();
            bytes.push(u8::from_str_radix(byte_str, 16).unwrap());
        }
        bytes
    }

    #[test]
    fn decrypts_ciphertext_generated_by_cpp_demo() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let binary = manifest_dir.join("target").join("test-auth-demo");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();

        let build = Command::new("g++")
            .arg("-std=c++17")
            .arg("-Iultravnc")
            .arg("ultravnc/main.cpp")
            .arg("ultravnc/encrypt.cpp")
            .arg("ultravnc/dh.cpp")
            .arg("-o")
            .arg(&binary)
            .current_dir(manifest_dir)
            .status()
            .unwrap();
        assert!(build.success(), "failed to build the C++ demo");

        let run = Command::new(&binary)
            .current_dir(manifest_dir)
            .output()
            .unwrap();
        assert!(run.status.success(), "failed to run the C++ demo");

        let stdout = String::from_utf8(run.stdout).unwrap();
        let key_line = stdout
            .lines()
            .find(|line| line.contains("After DH:"))
            .unwrap();
        let key_value = key_line
            .split("key=")
            .nth(1)
            .unwrap()
            .trim()
            .parse::<u64>()
            .unwrap();
        let key: [u8; 8] = key_value.to_be_bytes();

        let user_line = stdout
            .lines()
            .find(|line| line.contains("[WriteExact user]"))
            .unwrap();
        let password_line = stdout
            .lines()
            .find(|line| line.contains("[WriteExact passwd]"))
            .unwrap();

        let mut user_ciphertext = hex_to_bytes(user_line.split_once("] ").unwrap().1.trim());
        let mut password_ciphertext =
            hex_to_bytes(password_line.split_once("] ").unwrap().1.trim());

        let user_plaintext = decrypt_cpp_style(&mut user_ciphertext, &key);
        let password_plaintext = decrypt_cpp_style(&mut password_ciphertext, &key);

        assert_eq!(u8_null_to_string(&user_plaintext).unwrap(), "admin");
        assert_eq!(u8_null_to_string(&password_plaintext).unwrap(), "password");
    }

    #[test]
    fn converts_null_terminated_bytes_to_string() {
        let bytes = [b'u', b's', b'e', b'r', 0, b'x'];
        assert_eq!(u8_null_to_string(&bytes).unwrap(), "user");
    }

    #[test]
    fn rejects_missing_null_terminator() {
        let bytes = [b'u', b's', b'e', b'r'];
        assert!(u8_null_to_string(&bytes).is_err());
    }
}
