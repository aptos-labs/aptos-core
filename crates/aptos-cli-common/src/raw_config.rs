// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Raw intermediate config types for two-phase deserialization.
//!
//! These types mirror `ProfileConfig` / `CliConfig` but keep all values as strings,
//! allowing us to detect and decrypt `enc:v1:...` fields before parsing them into
//! their typed representations.

use crate::{
    encryption::{is_encrypted, is_sensitive_field, DerivedKey, EncryptionConfig},
    CliError, CliTypedResult, Network, ProfileConfig,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    ValidCryptoMaterialStringExt,
};
use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};

/// The current profile schema version, stamped on every save.
pub const CURRENT_PROFILE_VERSION: u32 = 1;

pub fn default_profile_version() -> u32 {
    1
}

// ── RawProfileConfig ──

/// A profile where all values are strings — used as the intermediate serde layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawProfileConfig {
    #[serde(default = "default_profile_version")]
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_auth_token: Option<String>,
}

impl Default for RawProfileConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_PROFILE_VERSION,
            network: None,
            private_key: None,
            public_key: None,
            account: None,
            rest_url: None,
            faucet_url: None,
            derivation_path: None,
            node_api_key: None,
            faucet_auth_token: None,
        }
    }
}

impl RawProfileConfig {
    /// Convert to a typed `ProfileConfig`, decrypting sensitive fields if a key is provided.
    pub fn into_profile_config(self, key: Option<&DerivedKey>) -> CliTypedResult<ProfileConfig> {
        let private_key_str = maybe_decrypt(self.private_key, "private_key", key)?;
        let public_key_str = maybe_decrypt(self.public_key, "public_key", key)?;

        let private_key = private_key_str.map(|s| parse_private_key(&s)).transpose()?;
        let public_key = public_key_str.map(|s| parse_public_key(&s)).transpose()?;

        let account = self
            .account
            .map(|s| {
                AccountAddress::from_str(&s)
                    .map_err(|e| CliError::UnableToParse("account address", e.to_string()))
            })
            .transpose()?;

        let network = self.network.map(|s| Network::from_str(&s)).transpose()?;

        let node_api_key = maybe_decrypt(self.node_api_key, "node_api_key", key)?;
        let faucet_auth_token = maybe_decrypt(self.faucet_auth_token, "faucet_auth_token", key)?;

        Ok(ProfileConfig {
            version: self.version,
            network,
            private_key,
            public_key,
            account,
            rest_url: self.rest_url,
            faucet_url: self.faucet_url,
            derivation_path: self.derivation_path,
            node_api_key,
            faucet_auth_token,
        })
    }

    /// Returns true if any field in this profile is encrypted.
    pub fn has_encrypted_fields(&self) -> bool {
        [
            &self.private_key,
            &self.public_key,
            &self.node_api_key,
            &self.faucet_auth_token,
        ]
        .iter()
        .any(|f| f.as_deref().is_some_and(is_encrypted))
    }

    /// Returns true if the named field is present and encrypted.
    pub fn is_field_encrypted(&self, field_name: &str) -> bool {
        let value = match field_name {
            "private_key" => &self.private_key,
            "public_key" => &self.public_key,
            "node_api_key" => &self.node_api_key,
            "faucet_auth_token" => &self.faucet_auth_token,
            _ => return false,
        };
        value.as_deref().is_some_and(is_encrypted)
    }
}

/// Convert a typed `ProfileConfig` into a `RawProfileConfig`, encrypting sensitive fields
/// if a key is provided.
pub fn profile_config_to_raw(
    config: &ProfileConfig,
    key: Option<&DerivedKey>,
) -> CliTypedResult<RawProfileConfig> {
    let private_key_str = config
        .private_key
        .as_ref()
        .map(|k| {
            k.to_aip_80_string().map_err(|e| {
                CliError::UnexpectedError(format!("Failed to encode private key: {}", e))
            })
        })
        .transpose()?;

    let public_key_str = config
        .public_key
        .as_ref()
        .map(|k| {
            k.to_aip_80_string().map_err(|e| {
                CliError::UnexpectedError(format!("Failed to encode public key: {}", e))
            })
        })
        .transpose()?;

    let account_str = config.account.map(|a| a.to_standard_string());
    // Use serde serialization format for network (e.g. "Devnet") to match existing config files
    let network_str = config.network.map(|n| {
        serde_yaml::to_value(n)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| n.to_string())
    });

    Ok(RawProfileConfig {
        version: CURRENT_PROFILE_VERSION,
        network: network_str,
        private_key: maybe_encrypt(private_key_str, "private_key", key)?,
        public_key: public_key_str, // public_key is never encrypted
        account: account_str,
        rest_url: config.rest_url.clone(),
        faucet_url: config.faucet_url.clone(),
        derivation_path: config.derivation_path.clone(),
        node_api_key: maybe_encrypt(config.node_api_key.clone(), "node_api_key", key)?,
        faucet_auth_token: maybe_encrypt(
            config.faucet_auth_token.clone(),
            "faucet_auth_token",
            key,
        )?,
    })
}

// ── RawCliConfig ──

/// Top-level config with raw string profiles and optional encryption metadata.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RawCliConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<EncryptionConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles: Option<BTreeMap<String, RawProfileConfig>>,
}

impl RawCliConfig {
    /// Returns true if any profile contains encrypted fields.
    pub fn has_any_encrypted_fields(&self) -> bool {
        self.profiles
            .as_ref()
            .map(|profiles| profiles.values().any(|p| p.has_encrypted_fields()))
            .unwrap_or(false)
    }
}

// ── Helpers ──

/// Decrypt a field value if it has the `enc:v1:` prefix and a key is available.
fn maybe_decrypt(
    value: Option<String>,
    field_name: &str,
    key: Option<&DerivedKey>,
) -> CliTypedResult<Option<String>> {
    match value {
        Some(v) if is_encrypted(&v) => match key {
            Some(key) => Ok(Some(key.decrypt_field(&v, field_name)?)),
            None => Ok(None), // encrypted field, no key → skip
        },
        other => Ok(other),
    }
}

/// Encrypt a field value if a key is provided and the field is sensitive.
fn maybe_encrypt(
    value: Option<String>,
    field_name: &str,
    key: Option<&DerivedKey>,
) -> CliTypedResult<Option<String>> {
    match (value, key) {
        (Some(v), Some(key)) if is_sensitive_field(field_name) => {
            Ok(Some(key.encrypt_field(&v, field_name)?))
        },
        (v, _) => Ok(v),
    }
}

fn parse_private_key(s: &str) -> CliTypedResult<Ed25519PrivateKey> {
    // Handle AIP-80 prefix
    let stripped = crate::strip_private_key_prefix(s)?;
    Ed25519PrivateKey::from_encoded_string(stripped)
        .map_err(|e| CliError::UnableToParse("Ed25519PrivateKey", e.to_string()))
}

fn parse_public_key(s: &str) -> CliTypedResult<Ed25519PublicKey> {
    // Handle AIP-80 prefix: strip "ed25519-pub-" if present
    let stripped = s.strip_prefix("ed25519-pub-").unwrap_or(s);
    Ed25519PublicKey::from_encoded_string(stripped)
        .map_err(|e| CliError::UnableToParse("Ed25519PublicKey", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::EncryptionConfig;

    #[test]
    fn test_plaintext_round_trip() {
        let raw = RawProfileConfig {
            network: Some("devnet".to_string()),
            rest_url: Some("https://fullnode.devnet.aptoslabs.com".to_string()),
            ..Default::default()
        };

        let typed = raw.into_profile_config(None).unwrap();
        assert_eq!(typed.network, Some(Network::Devnet));
        assert_eq!(
            typed.rest_url.as_deref(),
            Some("https://fullnode.devnet.aptoslabs.com")
        );
    }

    #[test]
    fn test_encrypted_round_trip() {
        let config = EncryptionConfig::new("test-pw", false).unwrap();
        let key = DerivedKey::derive("test-pw", &config).unwrap();

        // Build a profile and convert to raw (encrypting)
        let profile = ProfileConfig {
            network: Some(Network::Devnet),
            rest_url: Some("https://example.com".to_string()),
            node_api_key: Some("secret-api-key".to_string()),
            ..Default::default()
        };

        let raw = profile_config_to_raw(&profile, Some(&key)).unwrap();

        // node_api_key should be encrypted
        assert!(raw.node_api_key.as_ref().unwrap().starts_with("enc:v1:"));
        // rest_url should NOT be encrypted
        assert_eq!(raw.rest_url.as_deref(), Some("https://example.com"));
        // network should be plain (serde format uses capitalized variant name)
        assert_eq!(raw.network.as_deref(), Some("Devnet"));

        // Round-trip back
        let restored = raw.into_profile_config(Some(&key)).unwrap();
        assert_eq!(restored.node_api_key.as_deref(), Some("secret-api-key"));
        assert_eq!(restored.network, Some(Network::Devnet));
    }

    #[test]
    fn test_has_encrypted_fields() {
        let raw = RawProfileConfig {
            private_key: Some("enc:v1:AAAA".to_string()),
            ..Default::default()
        };
        assert!(raw.has_encrypted_fields());

        let raw_plain = RawProfileConfig {
            private_key: Some("ed25519-priv-0xabc".to_string()),
            ..Default::default()
        };
        assert!(!raw_plain.has_encrypted_fields());
    }

    /// Full YAML round-trip: build a RawCliConfig with encrypted fields,
    /// serialize to YAML, deserialize back, and decrypt — verifying all
    /// sensitive fields survive and non-sensitive fields stay readable.
    #[test]
    fn test_yaml_round_trip_with_encryption() {
        let password = "yaml-round-trip-pw";
        let enc_config = EncryptionConfig::new(password, false).unwrap();
        let key = DerivedKey::derive(password, &enc_config).unwrap();

        // Build a typed profile with both sensitive and non-sensitive fields
        let profile = ProfileConfig {
            network: Some(Network::Testnet),
            rest_url: Some("https://fullnode.testnet.aptoslabs.com".to_string()),
            faucet_url: Some("https://faucet.testnet.aptoslabs.com".to_string()),
            node_api_key: Some("my-secret-api-key".to_string()),
            faucet_auth_token: Some("my-faucet-token".to_string()),
            ..Default::default()
        };

        // Convert to raw (encrypts sensitive fields)
        let raw_profile = profile_config_to_raw(&profile, Some(&key)).unwrap();

        let raw_config = RawCliConfig {
            encryption: Some(enc_config.clone()),
            profiles: Some([("default".to_string(), raw_profile)].into_iter().collect()),
        };

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&raw_config).unwrap();

        // Verify: sensitive fields are encrypted in the YAML
        assert!(
            yaml.contains("enc:v1:"),
            "YAML should contain encrypted fields"
        );
        assert!(
            !yaml.contains("my-secret-api-key"),
            "Plaintext API key must NOT appear in YAML"
        );
        assert!(
            !yaml.contains("my-faucet-token"),
            "Plaintext faucet token must NOT appear in YAML"
        );
        // Non-sensitive fields remain readable
        assert!(yaml.contains("fullnode.testnet.aptoslabs.com"));
        assert!(yaml.contains("faucet.testnet.aptoslabs.com"));

        // Deserialize back to raw
        let loaded_raw: RawCliConfig = serde_yaml::from_str(&yaml).unwrap();
        assert!(loaded_raw.has_any_encrypted_fields());
        assert!(loaded_raw.encryption.is_some());

        // Derive key from the same password (simulating APTOS_CONFIG_PASSWORD env var path)
        let loaded_enc = loaded_raw.encryption.as_ref().unwrap();
        let loaded_key = DerivedKey::derive(password, loaded_enc).unwrap();
        assert!(loaded_key.verify_key_check(loaded_enc));

        // Decrypt profiles
        let loaded_profiles = loaded_raw.profiles.unwrap();
        let default_raw = loaded_profiles.get("default").unwrap();
        let restored = default_raw
            .clone()
            .into_profile_config(Some(&loaded_key))
            .unwrap();

        assert_eq!(restored.network, Some(Network::Testnet));
        assert_eq!(
            restored.rest_url.as_deref(),
            Some("https://fullnode.testnet.aptoslabs.com")
        );
        assert_eq!(restored.node_api_key.as_deref(), Some("my-secret-api-key"));
        assert_eq!(
            restored.faucet_auth_token.as_deref(),
            Some("my-faucet-token")
        );
    }

    /// Verifying that a wrong password fails key_check before even attempting decryption.
    #[test]
    fn test_wrong_password_detected_by_key_check() {
        let enc_config = EncryptionConfig::new("correct-pw", false).unwrap();
        let wrong_key = DerivedKey::derive("wrong-pw", &enc_config).unwrap();
        assert!(
            !wrong_key.verify_key_check(&enc_config),
            "Wrong password should fail key_check"
        );
    }

    #[test]
    fn test_missing_version_defaults_to_1() {
        // Simulate an existing config.yaml that predates the version field
        let yaml = r#"
network: Devnet
rest_url: "https://example.com"
"#;
        let raw: RawProfileConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.version, 1);

        let typed = raw.into_profile_config(None).unwrap();
        assert_eq!(typed.version, 1);
    }

    #[test]
    fn test_profile_config_to_raw_stamps_current_version() {
        let profile = ProfileConfig {
            version: 0, // intentionally wrong — save should overwrite
            network: Some(Network::Devnet),
            ..Default::default()
        };
        let raw = profile_config_to_raw(&profile, None).unwrap();
        assert_eq!(raw.version, CURRENT_PROFILE_VERSION);
    }

    #[test]
    fn test_version_survives_yaml_round_trip() {
        let profile = ProfileConfig {
            network: Some(Network::Testnet),
            rest_url: Some("https://example.com".to_string()),
            ..Default::default()
        };
        let raw = profile_config_to_raw(&profile, None).unwrap();
        assert_eq!(raw.version, CURRENT_PROFILE_VERSION);

        // Serialize to YAML and back
        let yaml = serde_yaml::to_string(&raw).unwrap();
        assert!(
            yaml.contains("version: 1"),
            "YAML should contain version field"
        );

        let raw2: RawProfileConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(raw2.version, CURRENT_PROFILE_VERSION);

        let typed = raw2.into_profile_config(None).unwrap();
        assert_eq!(typed.version, CURRENT_PROFILE_VERSION);
    }

    /// Plaintext config (no encryption section) loads without needing a password.
    #[test]
    fn test_plaintext_config_needs_no_password() {
        let raw_config = RawCliConfig {
            encryption: None,
            profiles: Some(
                [("default".to_string(), RawProfileConfig {
                    network: Some("Devnet".to_string()),
                    rest_url: Some("https://example.com".to_string()),
                    ..Default::default()
                })]
                .into_iter()
                .collect(),
            ),
        };

        // Serialize + deserialize
        let yaml = serde_yaml::to_string(&raw_config).unwrap();
        let loaded: RawCliConfig = serde_yaml::from_str(&yaml).unwrap();

        assert!(!loaded.has_any_encrypted_fields());
        assert!(loaded.encryption.is_none());

        // Should convert without any key
        let profiles = loaded.profiles.unwrap();
        let restored = profiles
            .get("default")
            .unwrap()
            .clone()
            .into_profile_config(None)
            .unwrap();
        assert_eq!(restored.network, Some(Network::Devnet));
    }
}
