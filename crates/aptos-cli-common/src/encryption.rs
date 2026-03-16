// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Password-based encryption for sensitive CLI config fields.
//!
//! Sensitive values (private keys, API keys, auth tokens) are encrypted with AES-256-GCM
//! using a key derived from a user password via Argon2id. Encrypted fields are stored in
//! `config.yaml` with the prefix `enc:v1:<base64(nonce ∥ ciphertext ∥ tag)>`.

use crate::{CliError, CliTypedResult};
use aes_gcm::{aead::Aead, Aes256Gcm, Nonce};
use argon2::Argon2;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{path::Path, sync::OnceLock};
use zeroize::{Zeroize, Zeroizing};

/// Prefix that identifies an encrypted field value.
const ENC_PREFIX: &str = "enc:v1:";

/// Fields that should be encrypted when encryption is enabled.
pub const SENSITIVE_FIELDS: &[&str] = &["private_key", "node_api_key", "faucet_auth_token"];

/// Default Argon2 parameters — ~2-5s on modern hardware with 128 MiB memory.
const DEFAULT_ARGON2_T_COST: u32 = 2;
const DEFAULT_ARGON2_M_COST: u32 = 131072; // 128 MiB
const DEFAULT_ARGON2_P_COST: u32 = 1;

/// Salt length in bytes.
const SALT_LEN: usize = 16;

/// AES-GCM nonce length.
const NONCE_LEN: usize = 12;

/// Derived key length for AES-256.
const KEY_LEN: usize = 32;

/// Domain-separation tag for the key-check HMAC.
const KEY_CHECK_TAG: &[u8] = b"aptos-cli-config-key-check";

/// Process-level cache so we don't re-prompt within a single invocation.
/// Wrapped in `Zeroizing` so the password is zeroed during process teardown.
static PASSWORD_CACHE: OnceLock<Zeroizing<String>> = OnceLock::new();

// ── EncryptionConfig ──

/// Metadata stored in the `encryption:` section of `config.yaml`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub version: u32,
    /// 16-byte salt encoded as hex.
    pub salt: String,
    pub argon2_t_cost: u32,
    pub argon2_m_cost: u32,
    pub argon2_p_cost: u32,
    /// HMAC-SHA256 of the derived key, hex-encoded. Used for fast password verification.
    pub key_check: String,
    #[serde(default)]
    pub use_keyring: bool,
}

impl EncryptionConfig {
    /// Create a new config with a random salt and derive a key check from the password.
    pub fn new(password: &str, use_keyring: bool) -> CliTypedResult<Self> {
        let salt = generate_random_salt();
        let salt_hex = hex::encode(salt);

        let config = EncryptionConfig {
            version: 1,
            salt: salt_hex,
            argon2_t_cost: DEFAULT_ARGON2_T_COST,
            argon2_m_cost: DEFAULT_ARGON2_M_COST,
            argon2_p_cost: DEFAULT_ARGON2_P_COST,
            key_check: String::new(),
            use_keyring,
        };

        let derived = DerivedKey::derive(password, &config)?;
        let key_check = derived.compute_key_check();

        Ok(EncryptionConfig {
            key_check,
            ..config
        })
    }
}

// ── DerivedKey ──

/// A 32-byte AES-256 key derived from a user password via Argon2id.
///
/// The key material is zeroed on drop to minimize time secrets reside in memory.
pub struct DerivedKey {
    key: [u8; KEY_LEN],
}

impl Drop for DerivedKey {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl DerivedKey {
    /// Derive the AES-256 key from a password and the encryption config.
    pub fn derive(password: &str, config: &EncryptionConfig) -> CliTypedResult<Self> {
        let salt = hex::decode(&config.salt).map_err(|e| {
            CliError::EncryptionError(format!("Invalid salt in encryption config: {}", e))
        })?;

        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                config.argon2_m_cost,
                config.argon2_t_cost,
                config.argon2_p_cost,
                Some(KEY_LEN),
            )
            .map_err(|e| CliError::EncryptionError(format!("Invalid Argon2 params: {}", e)))?,
        );

        let mut key = [0u8; KEY_LEN];
        argon2
            .hash_password_into(password.as_bytes(), &salt, &mut key)
            .map_err(|e| {
                CliError::EncryptionError(format!("Argon2 key derivation failed: {}", e))
            })?;

        Ok(DerivedKey { key })
    }

    /// Compute an HMAC-SHA256 key check value for fast password verification.
    pub fn compute_key_check(&self) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC accepts any key size");
        mac.update(KEY_CHECK_TAG);
        hex::encode(mac.finalize().into_bytes())
    }

    /// Verify the derived key against the stored key check.
    ///
    /// Uses constant-time comparison to prevent timing side-channel attacks.
    pub fn verify_key_check(&self, config: &EncryptionConfig) -> bool {
        use subtle::ConstantTimeEq;
        let computed = self.compute_key_check();
        computed
            .as_bytes()
            .ct_eq(config.key_check.as_bytes())
            .into()
    }

    /// Encrypt a plaintext string, returning `enc:v1:<base64(nonce ∥ ciphertext ∥ tag)>`.
    ///
    /// `field_name` is bound as AAD so ciphertexts can't be swapped between fields.
    pub fn encrypt_field(&self, plaintext: &str, field_name: &str) -> CliTypedResult<String> {
        use aes_gcm::{aead::Payload, KeyInit as _};

        let cipher = Aes256Gcm::new((&self.key).into());
        let nonce_bytes = generate_random_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let payload = Payload {
            msg: plaintext.as_bytes(),
            aad: field_name.as_bytes(),
        };
        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|e| CliError::EncryptionError(format!("AES-GCM encryption failed: {}", e)))?;

        // Wire format: nonce ∥ ciphertext (which already includes the 16-byte auth tag)
        let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(format!("{}{}", ENC_PREFIX, base64::encode(&combined)))
    }

    /// Decrypt an `enc:v1:...` value back to plaintext.
    ///
    /// `field_name` must match the value used during encryption (AAD binding).
    pub fn decrypt_field(&self, encrypted: &str, field_name: &str) -> CliTypedResult<String> {
        use aes_gcm::{aead::Payload, KeyInit as _};

        let encoded = encrypted
            .strip_prefix(ENC_PREFIX)
            .ok_or_else(|| CliError::EncryptionError("Missing enc:v1: prefix".to_string()))?;

        let combined = base64::decode(encoded).map_err(|e| {
            CliError::EncryptionError(format!("Invalid base64 in encrypted field: {}", e))
        })?;

        if combined.len() < NONCE_LEN + 16 {
            // At minimum: 12-byte nonce + 16-byte auth tag
            return Err(CliError::EncryptionError(
                "Encrypted field too short".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new((&self.key).into());
        let payload = Payload {
            msg: ciphertext,
            aad: field_name.as_bytes(),
        };
        let plaintext = cipher
            .decrypt(nonce, payload)
            .map_err(|_| CliError::WrongPassword)?;

        String::from_utf8(plaintext)
            .map_err(|e| CliError::EncryptionError(format!("Decrypted value is not UTF-8: {}", e)))
    }
}

// ── Password retrieval ──

/// Get the encryption password for an encrypted config.
///
/// Priority: process cache → env var → OS keyring → interactive prompt.
///
/// `config_dir` is the resolved `.aptos/` directory — used to scope keyring
/// entries so different project configs don't collide.
///
/// Returns a `Zeroizing<String>` so the password is zeroed when the caller drops it.
/// The process-level cache retains a copy for the lifetime of the process.
pub fn get_password(
    config: &EncryptionConfig,
    config_dir: &Path,
) -> CliTypedResult<Zeroizing<String>> {
    // 1. Process-level cache
    if let Some(cached) = PASSWORD_CACHE.get() {
        return Ok(cached.clone());
    }

    // 2. Environment variable
    if let Ok(pw) = std::env::var("APTOS_CONFIG_PASSWORD") {
        cache_password(pw);
        return Ok(PASSWORD_CACHE.get().unwrap().clone());
    }

    // 3. OS keyring
    #[cfg(feature = "keyring-cache")]
    if config.use_keyring
        && let Some(pw) = try_keyring_get(config_dir)
    {
        cache_password(pw);
        return Ok(PASSWORD_CACHE.get().unwrap().clone());
    }

    // Suppress unused variable warnings when keyring feature is disabled
    let _ = config;
    let _ = config_dir;

    // 4. Interactive prompt
    let pw = prompt_password("Enter config password: ")?;
    cache_password(pw.to_string());
    Ok(PASSWORD_CACHE.get().unwrap().clone())
}

/// Get or prompt for a new password with confirmation.
///
/// Priority: process cache → `APTOS_CONFIG_PASSWORD` env var → interactive prompt (with confirm).
/// The env-var path skips confirmation since CI environments can't do interactive prompts.
pub fn prompt_new_password() -> CliTypedResult<Zeroizing<String>> {
    // Process cache (e.g. already set earlier in this invocation)
    if let Some(cached) = PASSWORD_CACHE.get() {
        return Ok(cached.clone());
    }

    // Environment variable — trusted; skip confirmation
    if let Ok(pw) = std::env::var("APTOS_CONFIG_PASSWORD") {
        if pw.is_empty() {
            return Err(CliError::EncryptionError(
                "APTOS_CONFIG_PASSWORD is set but empty".to_string(),
            ));
        }
        cache_password(pw);
        return Ok(PASSWORD_CACHE.get().unwrap().clone());
    }

    // Interactive prompt with confirmation
    let pw = prompt_password("Create config password: ")?;
    let confirm = prompt_password("Confirm config password: ")?;
    if *pw != *confirm {
        return Err(CliError::EncryptionError(
            "Passwords do not match".to_string(),
        ));
    }
    if pw.is_empty() {
        return Err(CliError::EncryptionError(
            "Password cannot be empty".to_string(),
        ));
    }
    cache_password(pw.to_string());
    Ok(PASSWORD_CACHE.get().unwrap().clone())
}

fn prompt_password(prompt: &str) -> CliTypedResult<Zeroizing<String>> {
    rpassword::prompt_password(prompt)
        .map(Zeroizing::new)
        .map_err(|e| CliError::EncryptionError(format!("Failed to read password: {}", e)))
}

fn cache_password(pw: String) {
    let _ = PASSWORD_CACHE.set(Zeroizing::new(pw));
}

// ── Keyring helpers ──

/// Build a keyring entry name scoped to the given `.aptos/` directory so that
/// different project configs don't collide in the OS credential store.
#[cfg(feature = "keyring-cache")]
fn keyring_key(config_dir: &Path) -> String {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    let canonical = config_dir
        .canonicalize()
        .unwrap_or_else(|_| config_dir.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    format!("aptos-config-password-{:x}", hasher.finish())
}

#[cfg(feature = "keyring-cache")]
fn try_keyring_get(config_dir: &Path) -> Option<String> {
    let entry = keyring::Entry::new("aptos-cli", &keyring_key(config_dir)).ok()?;
    entry.get_password().ok()
}

#[cfg(feature = "keyring-cache")]
pub fn keyring_store(password: &str, config_dir: &Path) -> CliTypedResult<()> {
    let entry = keyring::Entry::new("aptos-cli", &keyring_key(config_dir))
        .map_err(|e| CliError::EncryptionError(format!("Keyring error: {}", e)))?;
    entry.set_password(password).map_err(|e| {
        CliError::EncryptionError(format!("Failed to store password in keyring: {}", e))
    })
}

#[cfg(feature = "keyring-cache")]
pub fn keyring_clear(config_dir: &Path) -> CliTypedResult<()> {
    let entry = keyring::Entry::new("aptos-cli", &keyring_key(config_dir))
        .map_err(|e| CliError::EncryptionError(format!("Keyring error: {}", e)))?;
    // Ignore "not found" errors — the entry may not exist.
    let _ = entry.delete_credential();
    Ok(())
}

// ── Predicate helpers ──

/// Returns true if the value is an encrypted field (`enc:v1:...`).
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENC_PREFIX)
}

/// Returns true if the field name should be encrypted.
pub fn is_sensitive_field(name: &str) -> bool {
    SENSITIVE_FIELDS.contains(&name)
}

// ── Random helpers ──

fn generate_random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut salt);
    salt
}

fn generate_random_nonce() -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
    nonce
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let config = EncryptionConfig::new("test-password", false).unwrap();
        let key = DerivedKey::derive("test-password", &config).unwrap();

        let plaintext = "ed25519-priv-0xabcdef1234567890";
        let encrypted = key.encrypt_field(plaintext, "private_key").unwrap();

        assert!(is_encrypted(&encrypted));
        assert!(encrypted.starts_with(ENC_PREFIX));

        let decrypted = key.decrypt_field(&encrypted, "private_key").unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_password() {
        let config = EncryptionConfig::new("correct-password", false).unwrap();
        let correct_key = DerivedKey::derive("correct-password", &config).unwrap();
        let wrong_key = DerivedKey::derive("wrong-password", &config).unwrap();

        let encrypted = correct_key.encrypt_field("secret", "private_key").unwrap();

        // Wrong key should fail decryption
        let result = wrong_key.decrypt_field(&encrypted, "private_key");
        assert!(result.is_err());
    }

    #[test]
    fn test_aad_field_binding() {
        let config = EncryptionConfig::new("password", false).unwrap();
        let key = DerivedKey::derive("password", &config).unwrap();

        let encrypted = key.encrypt_field("secret-value", "private_key").unwrap();

        // Decrypting with the correct field name succeeds
        assert!(key.decrypt_field(&encrypted, "private_key").is_ok());

        // Decrypting with a different field name fails (AAD mismatch)
        assert!(key.decrypt_field(&encrypted, "node_api_key").is_err());
    }

    #[test]
    fn test_key_check_verification() {
        let config = EncryptionConfig::new("my-password", false).unwrap();

        let correct = DerivedKey::derive("my-password", &config).unwrap();
        assert!(correct.verify_key_check(&config));

        let wrong = DerivedKey::derive("wrong-password", &config).unwrap();
        assert!(!wrong.verify_key_check(&config));
    }

    #[test]
    fn test_key_derivation_determinism() {
        let config = EncryptionConfig {
            version: 1,
            salt: hex::encode([1u8; SALT_LEN]),
            argon2_t_cost: DEFAULT_ARGON2_T_COST,
            argon2_m_cost: DEFAULT_ARGON2_M_COST,
            argon2_p_cost: DEFAULT_ARGON2_P_COST,
            key_check: String::new(),
            use_keyring: false,
        };

        let key1 = DerivedKey::derive("password", &config).unwrap();
        let key2 = DerivedKey::derive("password", &config).unwrap();
        assert_eq!(key1.key, key2.key);
    }

    #[test]
    fn test_is_encrypted() {
        assert!(is_encrypted("enc:v1:SGVsbG8="));
        assert!(!is_encrypted("ed25519-priv-0xabc"));
        assert!(!is_encrypted("plain-text"));
    }

    #[test]
    fn test_is_sensitive_field() {
        assert!(is_sensitive_field("private_key"));
        assert!(is_sensitive_field("node_api_key"));
        assert!(is_sensitive_field("faucet_auth_token"));
        assert!(!is_sensitive_field("network"));
        assert!(!is_sensitive_field("rest_url"));
    }

    #[test]
    fn test_encrypted_field_format() {
        let config = EncryptionConfig::new("pw", false).unwrap();
        let key = DerivedKey::derive("pw", &config).unwrap();
        let encrypted = key.encrypt_field("hello", "private_key").unwrap();

        // Should have prefix
        let encoded = encrypted.strip_prefix(ENC_PREFIX).unwrap();
        let decoded = base64::decode(encoded).unwrap();

        // At least 12 (nonce) + 5 (plaintext "hello") + 16 (tag) = 33 bytes
        assert!(decoded.len() >= 33);
    }
}
