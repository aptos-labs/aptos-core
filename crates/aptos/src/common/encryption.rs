// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Credential encryption at rest for the Aptos CLI.
//!
//! This module provides password-based encryption for private keys stored in the CLI configuration.
//! It uses PBKDF2 for key derivation and AES-256-GCM for authenticated encryption.
//!
//! The encrypted format is: version (1 byte) || salt (32 bytes) || nonce (12 bytes) || ciphertext
//! where ciphertext includes the AES-GCM authentication tag.

use crate::common::types::CliError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use pbkdf2::pbkdf2_hmac_array;
use serde::{Deserialize, Serialize};
use sha2_0_10_6::Sha256;

/// Current version of the encryption format
const ENCRYPTION_VERSION: u8 = 1;

/// Number of PBKDF2 iterations - 600,000 as recommended by OWASP for SHA-256
const PBKDF2_ITERATIONS: u32 = 600_000;

/// Salt size in bytes
const SALT_SIZE: usize = 32;

/// AES-GCM nonce size in bytes
const NONCE_SIZE: usize = 12;

/// AES-256 key size in bytes
const KEY_SIZE: usize = 32;

/// Encrypted private key stored in the config file
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedPrivateKey {
    /// Base64-encoded encrypted data (version || salt || nonce || ciphertext)
    pub ciphertext: String,
}

impl EncryptedPrivateKey {
    /// Encrypt a private key with a passphrase
    pub fn encrypt(private_key_bytes: &[u8], passphrase: &str) -> Result<Self, CliError> {
        if passphrase.is_empty() {
            return Err(CliError::CommandArgumentError(
                "Passphrase cannot be empty".to_string(),
            ));
        }

        // Generate random salt and nonce
        let salt: [u8; SALT_SIZE] = rand::random();
        let nonce_bytes: [u8; NONCE_SIZE] = rand::random();

        // Derive key using PBKDF2-HMAC-SHA256
        let key = derive_key(passphrase, &salt);

        // Encrypt using AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| CliError::UnexpectedError(format!("Failed to create cipher: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, private_key_bytes).map_err(|e| {
            CliError::UnexpectedError(format!("Failed to encrypt private key: {}", e))
        })?;

        // Combine version, salt, nonce, and ciphertext
        let mut combined = Vec::with_capacity(1 + SALT_SIZE + NONCE_SIZE + ciphertext.len());
        combined.push(ENCRYPTION_VERSION);
        combined.extend_from_slice(&salt);
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(Self {
            ciphertext: base64::encode(&combined),
        })
    }

    /// Decrypt a private key with a passphrase
    pub fn decrypt(&self, passphrase: &str) -> Result<Vec<u8>, CliError> {
        let combined = base64::decode(&self.ciphertext).map_err(|e| {
            CliError::UnexpectedError(format!("Failed to decode encrypted data: {}", e))
        })?;

        // Check minimum length (version + salt + nonce + at least 1 byte of ciphertext + 16 byte tag)
        let min_length = 1 + SALT_SIZE + NONCE_SIZE + 17;
        if combined.len() < min_length {
            return Err(CliError::UnexpectedError(
                "Encrypted data is too short".to_string(),
            ));
        }

        // Parse the version
        let version = combined[0];
        if version != ENCRYPTION_VERSION {
            return Err(CliError::UnexpectedError(format!(
                "Unsupported encryption version: {}",
                version
            )));
        }

        // Extract salt, nonce, and ciphertext
        let salt = &combined[1..1 + SALT_SIZE];
        let nonce_bytes = &combined[1 + SALT_SIZE..1 + SALT_SIZE + NONCE_SIZE];
        let ciphertext = &combined[1 + SALT_SIZE + NONCE_SIZE..];

        // Derive key using PBKDF2-HMAC-SHA256
        let key = derive_key(passphrase, salt);

        // Decrypt using AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| CliError::UnexpectedError(format!("Failed to create cipher: {}", e)))?;
        let nonce = Nonce::from_slice(nonce_bytes);

        cipher.decrypt(nonce, ciphertext).map_err(|_| {
            CliError::CommandArgumentError(
                "Failed to decrypt private key. Incorrect passphrase or corrupted data."
                    .to_string(),
            )
        })
    }

    /// Check if this encrypted key can be decrypted with the given passphrase
    pub fn verify_passphrase(&self, passphrase: &str) -> bool {
        self.decrypt(passphrase).is_ok()
    }
}

/// Derive an encryption key from a passphrase and salt using PBKDF2-HMAC-SHA256
fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_SIZE] {
    pbkdf2_hmac_array::<Sha256, KEY_SIZE>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS)
}

/// Environment variable name for the encryption passphrase
pub const APTOS_CLI_PASSPHRASE_ENV: &str = "APTOS_CLI_PASSPHRASE";

/// Check if credential encryption is enabled via environment variable
pub fn get_passphrase_from_env() -> Option<String> {
    std::env::var(APTOS_CLI_PASSPHRASE_ENV).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let private_key = b"test_private_key_data_32_bytes!!";
        let passphrase = "test_passphrase_123";

        let encrypted = EncryptedPrivateKey::encrypt(private_key, passphrase).unwrap();
        let decrypted = encrypted.decrypt(passphrase).unwrap();

        assert_eq!(private_key.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_passphrase_fails() {
        let private_key = b"test_private_key_data_32_bytes!!";
        let passphrase = "correct_passphrase";
        let wrong_passphrase = "wrong_passphrase";

        let encrypted = EncryptedPrivateKey::encrypt(private_key, passphrase).unwrap();
        let result = encrypted.decrypt(wrong_passphrase);

        assert!(result.is_err());
    }

    #[test]
    fn test_empty_passphrase_fails() {
        let private_key = b"test_private_key_data";
        let result = EncryptedPrivateKey::encrypt(private_key, "");

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_passphrase() {
        let private_key = b"test_private_key_data_32_bytes!!";
        let passphrase = "test_passphrase_123";

        let encrypted = EncryptedPrivateKey::encrypt(private_key, passphrase).unwrap();

        assert!(encrypted.verify_passphrase(passphrase));
        assert!(!encrypted.verify_passphrase("wrong_passphrase"));
    }

    #[test]
    fn test_encrypted_data_is_base64() {
        let private_key = b"test_private_key_data";
        let passphrase = "test_passphrase";

        let encrypted = EncryptedPrivateKey::encrypt(private_key, passphrase).unwrap();

        // Verify it's valid base64
        assert!(base64::decode(&encrypted.ciphertext).is_ok());
    }

    #[test]
    fn test_different_encryptions_produce_different_ciphertext() {
        let private_key = b"test_private_key_data";
        let passphrase = "test_passphrase";

        let encrypted1 = EncryptedPrivateKey::encrypt(private_key, passphrase).unwrap();
        let encrypted2 = EncryptedPrivateKey::encrypt(private_key, passphrase).unwrap();

        // Different random salt and nonce should produce different ciphertext
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);

        // But both should decrypt to the same plaintext
        assert_eq!(
            encrypted1.decrypt(passphrase).unwrap(),
            encrypted2.decrypt(passphrase).unwrap()
        );
    }
}
