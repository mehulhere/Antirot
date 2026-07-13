use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;

use crate::config::Config;
use crate::error::{AppError, AppResult};

const ENCRYPTED_PREFIX: &str = "enc:v1:";

pub fn encrypt_byok_key(config: &Config, plaintext: &str) -> AppResult<String> {
    let key = config.byok_encryption_key.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "ANTIROT_BYOK_ENCRYPTION_KEY_HEX is required before storing BYOK credentials"
                .to_string(),
        )
    })?;
    encrypt_with_key(&key.0, plaintext)
}

fn encrypt_with_key(key: &[u8; 32], plaintext: &str) -> AppResult<String> {
    if plaintext.trim().is_empty() {
        return Err(AppError::BadRequest("BYOK key cannot be empty".to_string()));
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| AppError::BadRequest("BYOK encryption is unavailable".to_string()))?;
    let mut nonce_bytes = [0_u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|_| AppError::BadRequest("BYOK encryption failed".to_string()))?;
    Ok(format!(
        "{ENCRYPTED_PREFIX}{}:{}",
        URL_SAFE_NO_PAD.encode(nonce_bytes),
        URL_SAFE_NO_PAD.encode(ciphertext)
    ))
}

pub fn decrypt_byok_key(config: &Config, stored: &str) -> AppResult<String> {
    let key = config.byok_encryption_key.as_ref().ok_or_else(|| {
        AppError::BadRequest("BYOK credential decryption is not configured".to_string())
    })?;
    decrypt_with_key(&key.0, stored)
}

fn decrypt_with_key(key: &[u8; 32], stored: &str) -> AppResult<String> {
    let encoded = stored.strip_prefix(ENCRYPTED_PREFIX).ok_or_else(|| {
        AppError::BadRequest(
            "stored BYOK credential uses the retired plaintext format; re-save it in Settings"
                .to_string(),
        )
    })?;
    let (nonce, ciphertext) = encoded
        .split_once(':')
        .ok_or_else(|| AppError::BadRequest("stored BYOK credential is invalid".to_string()))?;
    let nonce = URL_SAFE_NO_PAD
        .decode(nonce)
        .map_err(|_| AppError::BadRequest("stored BYOK credential is invalid".to_string()))?;
    let ciphertext = URL_SAFE_NO_PAD
        .decode(ciphertext)
        .map_err(|_| AppError::BadRequest("stored BYOK credential is invalid".to_string()))?;
    if nonce.len() != 12 {
        return Err(AppError::BadRequest(
            "stored BYOK credential is invalid".to_string(),
        ));
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| AppError::BadRequest("BYOK decryption is unavailable".to_string()))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| {
            AppError::BadRequest("stored BYOK credential could not be decrypted".to_string())
        })?;
    String::from_utf8(plaintext)
        .map_err(|_| AppError::BadRequest("stored BYOK credential is invalid".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byok_encryption_round_trip_does_not_store_plaintext() {
        let key = [7_u8; 32];
        let encrypted = encrypt_with_key(&key, "secret-provider-key").unwrap();
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert!(!encrypted.contains("secret-provider-key"));
        assert_eq!(
            decrypt_with_key(&key, &encrypted).unwrap(),
            "secret-provider-key"
        );
    }

    #[test]
    fn plaintext_credentials_are_rejected() {
        let error = decrypt_with_key(&[7_u8; 32], "secret-provider-key").unwrap_err();
        assert!(error.to_string().contains("retired plaintext format"));
    }
}
