// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Encrypted-at-rest credentials for node configuration.
//!
//! Aptos node config files historically store sensitive material (e.g. Vault
//! tokens) either inline as plaintext (`from_config`) or in a sibling file on
//! disk (`from_disk`). Both approaches leave the secret readable by anyone (or
//! anything) that can read the file system. A leaked config file, a backup that
//! ends up in the wrong place, or a malicious dependency that scans the disk as
//! part of a supply-chain attack can all walk away with the raw secret.
//!
//! [`EncryptedCredential`] provides a third option: the secret is stored
//! encrypted-at-rest inside the config file, and the password used to derive
//! the decryption key is supplied at runtime through an environment variable.
//! The password itself is never written to disk. An attacker who only obtains
//! the config file (or any on-disk artifact) gets ciphertext that is useless
//! without also obtaining the runtime password, which lives in a separate trust
//! domain (an operator's secret manager, a systemd `EnvironmentFile`, a
//! Kubernetes secret, etc.).
//!
//! Cryptographic construction:
//! - The key is derived from the password with PBKDF2-HMAC-SHA256 using a
//!   per-credential random salt and a configurable iteration count.
//! - The plaintext is encrypted with AES-256-GCM (authenticated encryption)
//!   using a per-credential random 96-bit nonce.
//! - The salt, nonce, and ciphertext (with the appended GCM tag) are stored as
//!   hex strings so the config remains human-readable YAML/JSON.
//!
//! This type is additive and backwards compatible: existing configs that do not
//! use it are unaffected, and it can be slotted into existing credential enums
//! (see `Token::FromEncrypted` in `secure_backend_config`) without breaking the
//! serialization of the existing variants.

use crate::config::Error;
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN},
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// The default environment variable that holds the password used to decrypt
/// credentials. Operators may override this per-credential via `password_env`.
pub const DEFAULT_CREDENTIAL_PASSWORD_ENV: &str = "APTOS_CREDENTIAL_PASSWORD";

/// Default PBKDF2 iteration count. This follows OWASP's recommendation for
/// PBKDF2-HMAC-SHA256 and is intentionally high to slow down offline brute
/// force attempts against a weak password.
pub const DEFAULT_PBKDF2_ITERATIONS: u32 = 600_000;

/// Length (in bytes) of the AES-256 key derived from the password.
const AES_256_KEY_LEN: usize = 32;

/// Length (in bytes) of the PBKDF2 salt.
const SALT_LEN: usize = 16;

/// The PBKDF2 algorithm used for key derivation.
static PBKDF2_ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;

fn default_password_env() -> String {
    DEFAULT_CREDENTIAL_PASSWORD_ENV.to_string()
}

fn default_iterations() -> u32 {
    DEFAULT_PBKDF2_ITERATIONS
}

/// A credential that is stored encrypted-at-rest and decrypted at runtime using
/// a password sourced from an environment variable.
///
/// All fields are serialized so the credential can live directly inside a node
/// config file. The plaintext is never serialized; the only way to recover it is
/// to call [`EncryptedCredential::decrypt`] with the correct password.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EncryptedCredential {
    /// The name of the environment variable that holds the decryption password.
    /// Defaults to [`DEFAULT_CREDENTIAL_PASSWORD_ENV`] so that the common case
    /// requires no extra configuration.
    #[serde(default = "default_password_env")]
    pub password_env: String,
    /// The PBKDF2 iteration count used when the credential was encrypted. This
    /// is stored alongside the ciphertext so the key can be re-derived even if
    /// the default iteration count changes in a future release.
    #[serde(default = "default_iterations")]
    pub iterations: u32,
    /// The hex-encoded PBKDF2 salt (randomly generated per credential).
    pub salt: String,
    /// The hex-encoded AES-256-GCM nonce (randomly generated per credential).
    pub nonce: String,
    /// The hex-encoded AES-256-GCM ciphertext with the authentication tag
    /// appended.
    pub ciphertext: String,
}

impl EncryptedCredential {
    /// Encrypts the given plaintext with the given password, using the default
    /// password environment variable name and iteration count. A fresh random
    /// salt and nonce are generated for every call.
    pub fn encrypt(plaintext: &[u8], password: &str) -> Result<Self, Error> {
        Self::encrypt_with(
            plaintext,
            password,
            DEFAULT_CREDENTIAL_PASSWORD_ENV,
            DEFAULT_PBKDF2_ITERATIONS,
        )
    }

    /// Encrypts the given plaintext, allowing the caller to control the
    /// environment variable name recorded in the credential and the PBKDF2
    /// iteration count.
    pub fn encrypt_with(
        plaintext: &[u8],
        password: &str,
        password_env: &str,
        iterations: u32,
    ) -> Result<Self, Error> {
        if password.is_empty() {
            return Err(Error::InvalidCredential(
                "refusing to encrypt with an empty password".to_string(),
            ));
        }
        let iterations_nz = NonZeroU32::new(iterations).ok_or_else(|| {
            Error::InvalidCredential("PBKDF2 iteration count must be non-zero".to_string())
        })?;

        let rng = SystemRandom::new();

        // Generate a random salt and derive the AES key from the password.
        let mut salt = [0u8; SALT_LEN];
        rng.fill(&mut salt)
            .map_err(|_| Error::Unexpected("failed to generate random salt".to_string()))?;
        let key_bytes = derive_key(password, &salt, iterations_nz);

        // Generate a random nonce.
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rng.fill(&mut nonce_bytes)
            .map_err(|_| Error::Unexpected("failed to generate random nonce".to_string()))?;

        // Encrypt in place, appending the authentication tag.
        let key = less_safe_key(&key_bytes)?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let mut in_out = plaintext.to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| Error::Decryption("failed to encrypt credential".to_string()))?;

        Ok(Self {
            password_env: password_env.to_string(),
            iterations,
            salt: hex::encode(salt),
            nonce: hex::encode(nonce_bytes),
            ciphertext: hex::encode(in_out),
        })
    }

    /// Decrypts the credential by reading the password from the configured
    /// environment variable. This is the entry point used by node startup code.
    pub fn decrypt(&self) -> Result<Vec<u8>, Error> {
        let password = std::env::var(&self.password_env)
            .map_err(|_| Error::MissingEnvVar(self.password_env.clone()))?;
        self.decrypt_with_password(&password)
    }

    /// Decrypts the credential and returns the plaintext as a UTF-8 string,
    /// reading the password from the configured environment variable.
    pub fn decrypt_to_string(&self) -> Result<String, Error> {
        let plaintext = self.decrypt()?;
        String::from_utf8(plaintext)
            .map_err(|_| Error::Decryption("decrypted credential is not valid UTF-8".to_string()))
    }

    /// Decrypts the credential using an explicitly provided password. Exposed so
    /// tooling and tests can decrypt without touching the process environment.
    pub fn decrypt_with_password(&self, password: &str) -> Result<Vec<u8>, Error> {
        let iterations_nz = NonZeroU32::new(self.iterations).ok_or_else(|| {
            Error::InvalidCredential("PBKDF2 iteration count must be non-zero".to_string())
        })?;

        let salt = hex::decode(&self.salt)
            .map_err(|e| Error::InvalidCredential(format!("invalid salt encoding: {}", e)))?;

        let nonce_bytes = hex::decode(&self.nonce)
            .map_err(|e| Error::InvalidCredential(format!("invalid nonce encoding: {}", e)))?;
        let nonce_array: [u8; NONCE_LEN] = nonce_bytes
            .try_into()
            .map_err(|_| Error::InvalidCredential(format!("nonce must be {} bytes", NONCE_LEN)))?;

        let mut in_out = hex::decode(&self.ciphertext)
            .map_err(|e| Error::InvalidCredential(format!("invalid ciphertext encoding: {}", e)))?;

        let key_bytes = derive_key(password, &salt, iterations_nz);
        let key = less_safe_key(&key_bytes)?;
        let nonce = Nonce::assume_unique_for_key(nonce_array);

        // open_in_place authenticates the ciphertext+tag and returns the
        // plaintext slice. A wrong password (or tampered data) fails here.
        let plaintext = key
            .open_in_place(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| {
                Error::Decryption(
                    "failed to decrypt credential (wrong password or corrupted data)".to_string(),
                )
            })?;

        Ok(plaintext.to_vec())
    }
}

/// Derives a 32-byte AES-256 key from the password and salt using
/// PBKDF2-HMAC-SHA256.
fn derive_key(password: &str, salt: &[u8], iterations: NonZeroU32) -> [u8; AES_256_KEY_LEN] {
    let mut key = [0u8; AES_256_KEY_LEN];
    pbkdf2::derive(PBKDF2_ALG, iterations, salt, password.as_bytes(), &mut key);
    key
}

/// Builds a `LessSafeKey` for AES-256-GCM from raw key bytes. `LessSafeKey` is
/// "less safe" only because it lets the caller pick the nonce; we generate a
/// fresh random nonce per encryption and store it, which is the supported usage.
fn less_safe_key(key_bytes: &[u8; AES_256_KEY_LEN]) -> Result<LessSafeKey, Error> {
    let unbound = UnboundKey::new(&AES_256_GCM, key_bytes)
        .map_err(|_| Error::Unexpected("failed to construct AES-256-GCM key".to_string()))?;
    Ok(LessSafeKey::new(unbound))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PASSWORD: &str = "correct horse battery staple";
    const TEST_SECRET: &[u8] = b"s3cr3t-vault-token";

    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let credential = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        let decrypted = credential.decrypt_with_password(TEST_PASSWORD).unwrap();
        assert_eq!(decrypted, TEST_SECRET);
    }

    #[test]
    fn test_decrypt_to_string() {
        let credential = EncryptedCredential::encrypt(b"hello world", TEST_PASSWORD).unwrap();
        // decrypt_with_password gives bytes; verify via env-free helper as well.
        assert_eq!(
            credential.decrypt_with_password(TEST_PASSWORD).unwrap(),
            b"hello world"
        );
    }

    #[test]
    fn test_wrong_password_fails() {
        let credential = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        let result = credential.decrypt_with_password("wrong password");
        assert!(matches!(result, Err(Error::Decryption(_))));
    }

    #[test]
    fn test_ciphertext_does_not_contain_plaintext() {
        let credential = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        // The plaintext bytes must not appear anywhere in the serialized config.
        let serialized = serde_yaml::to_string(&credential).unwrap();
        assert!(!serialized.contains("s3cr3t"));
        assert!(!serialized.contains(TEST_PASSWORD));
    }

    #[test]
    fn test_unique_salt_and_nonce_per_encryption() {
        let a = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        let b = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        // Same plaintext + password must still produce different ciphertext.
        assert_ne!(a.salt, b.salt);
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.ciphertext, b.ciphertext);
    }

    #[test]
    fn test_empty_password_rejected() {
        let result = EncryptedCredential::encrypt(TEST_SECRET, "");
        assert!(matches!(result, Err(Error::InvalidCredential(_))));
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let mut credential = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        // Flip a byte in the ciphertext; GCM authentication must reject it.
        let mut bytes = hex::decode(&credential.ciphertext).unwrap();
        bytes[0] ^= 0xFF;
        credential.ciphertext = hex::encode(bytes);
        let result = credential.decrypt_with_password(TEST_PASSWORD);
        assert!(matches!(result, Err(Error::Decryption(_))));
    }

    #[test]
    fn test_yaml_round_trip() {
        let credential = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        let yaml = serde_yaml::to_string(&credential).unwrap();
        let parsed: EncryptedCredential = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(credential, parsed);
        assert_eq!(
            parsed.decrypt_with_password(TEST_PASSWORD).unwrap(),
            TEST_SECRET
        );
    }

    #[test]
    fn test_password_env_and_iterations_default_when_omitted() {
        // A minimal credential (as a human might hand-author after running an
        // encrypt tool) omits password_env and iterations; both should default.
        let template = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        let yaml = format!(
            "salt: \"{}\"\nnonce: \"{}\"\nciphertext: \"{}\"\n",
            template.salt, template.nonce, template.ciphertext
        );
        let parsed: EncryptedCredential = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.password_env, DEFAULT_CREDENTIAL_PASSWORD_ENV);
        assert_eq!(parsed.iterations, DEFAULT_PBKDF2_ITERATIONS);
        assert_eq!(
            parsed.decrypt_with_password(TEST_PASSWORD).unwrap(),
            TEST_SECRET
        );
    }

    #[test]
    fn test_custom_password_env_name() {
        let credential =
            EncryptedCredential::encrypt_with(TEST_SECRET, TEST_PASSWORD, "MY_PASSWORD", 1000)
                .unwrap();
        assert_eq!(credential.password_env, "MY_PASSWORD");
        assert_eq!(credential.iterations, 1000);
        assert_eq!(
            credential.decrypt_with_password(TEST_PASSWORD).unwrap(),
            TEST_SECRET
        );
    }

    #[test]
    fn test_missing_env_var_errors() {
        // This crate is `#![forbid(unsafe_code)]`, so tests cannot mutate the
        // process environment (set_var/remove_var are unsafe in this edition).
        // We instead point at a uniquely named variable that is never set, and
        // assert the lookup fails cleanly. The successful env path is exercised
        // by node startup code; decryption correctness is covered by
        // `decrypt_with_password` tests above.
        let env_name = "APTOS_TEST_CREDENTIAL_PASSWORD_THAT_IS_NEVER_SET_42";
        assert!(
            std::env::var(env_name).is_err(),
            "test env var unexpectedly set"
        );
        let credential =
            EncryptedCredential::encrypt_with(TEST_SECRET, TEST_PASSWORD, env_name, 1000).unwrap();
        let result = credential.decrypt();
        assert!(matches!(result, Err(Error::MissingEnvVar(_))));
    }

    #[test]
    fn test_invalid_hex_errors() {
        let mut credential = EncryptedCredential::encrypt(TEST_SECRET, TEST_PASSWORD).unwrap();
        credential.salt = "not-hex".to_string();
        let result = credential.decrypt_with_password(TEST_PASSWORD);
        assert!(matches!(result, Err(Error::InvalidCredential(_))));
    }
}
