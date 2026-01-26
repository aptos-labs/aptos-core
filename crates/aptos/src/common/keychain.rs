// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! System keychain integration for secure credential storage.
//!
//! This module provides platform-native secure storage for private keys:
//! - macOS: Keychain Access
//! - Windows: Credential Manager
//!
//! Using the system keychain provides better security than file-based encryption
//! because the credentials are protected by the OS security mechanisms and can
//! integrate with biometrics, secure enclaves, etc.
//!
//! Note: Linux Secret Service support is not currently available due to build
//! dependency requirements. Use passphrase encryption instead on Linux.

use crate::common::types::CliError;

/// The service name used to store credentials in the system keychain
#[cfg(any(target_os = "macos", target_os = "windows"))]
const KEYCHAIN_SERVICE: &str = "aptos-cli";

/// Check if the system keychain is available on this platform
///
/// Currently supports:
/// - macOS: Keychain Access
/// - Windows: Credential Manager
pub fn is_keychain_available() -> bool {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        // Try to create an entry to check if the keychain is available
        // We use a test key that we immediately discard
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, "__aptos_keychain_test__");
        match entry {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // On Linux and other platforms, keychain is not supported
        false
    }
}

/// Store a private key in the system keychain
///
/// The key is stored as a hex-encoded string under the given profile name.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn store_private_key(profile_name: &str, private_key_bytes: &[u8]) -> Result<(), CliError> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, profile_name).map_err(|e| {
        CliError::UnexpectedError(format!("Failed to create keychain entry: {}", e))
    })?;

    // Store the private key as hex-encoded string
    let hex_key = hex::encode(private_key_bytes);

    entry
        .set_password(&hex_key)
        .map_err(|e| CliError::UnexpectedError(format!("Failed to store key in keychain: {}", e)))
}

/// Store a private key in the system keychain (stub for unsupported platforms)
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn store_private_key(_profile_name: &str, _private_key_bytes: &[u8]) -> Result<(), CliError> {
    Err(CliError::UnexpectedError(
        "System keychain is not supported on this platform. Use passphrase encryption instead."
            .to_string(),
    ))
}

/// Retrieve a private key from the system keychain
///
/// Returns the private key bytes for the given profile name.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn retrieve_private_key(profile_name: &str) -> Result<Vec<u8>, CliError> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, profile_name).map_err(|e| {
        CliError::UnexpectedError(format!("Failed to access keychain entry: {}", e))
    })?;

    let hex_key = entry.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => CliError::CommandArgumentError(format!(
            "No private key found in keychain for profile '{}'",
            profile_name
        )),
        keyring::Error::Ambiguous(_) => CliError::UnexpectedError(format!(
            "Multiple entries found in keychain for profile '{}'. Please remove duplicates.",
            profile_name
        )),
        _ => CliError::UnexpectedError(format!("Failed to retrieve key from keychain: {}", e)),
    })?;

    hex::decode(&hex_key).map_err(|e| {
        CliError::UnexpectedError(format!("Failed to decode private key from keychain: {}", e))
    })
}

/// Retrieve a private key from the system keychain (stub for unsupported platforms)
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn retrieve_private_key(_profile_name: &str) -> Result<Vec<u8>, CliError> {
    Err(CliError::UnexpectedError(
        "System keychain is not supported on this platform. Use passphrase encryption instead."
            .to_string(),
    ))
}

/// Delete a private key from the system keychain
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn delete_private_key(profile_name: &str) -> Result<(), CliError> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, profile_name).map_err(|e| {
        CliError::UnexpectedError(format!("Failed to access keychain entry: {}", e))
    })?;

    entry.delete_credential().map_err(|e| match e {
        keyring::Error::NoEntry => CliError::CommandArgumentError(format!(
            "No private key found in keychain for profile '{}'",
            profile_name
        )),
        _ => CliError::UnexpectedError(format!("Failed to delete key from keychain: {}", e)),
    })
}

/// Delete a private key from the system keychain (stub for unsupported platforms)
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn delete_private_key(_profile_name: &str) -> Result<(), CliError> {
    Err(CliError::UnexpectedError(
        "System keychain is not supported on this platform. Use passphrase encryption instead."
            .to_string(),
    ))
}

/// Check if a private key exists in the system keychain for the given profile
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn has_private_key(profile_name: &str) -> bool {
    let entry = match keyring::Entry::new(KEYCHAIN_SERVICE, profile_name) {
        Ok(e) => e,
        Err(_) => return false,
    };

    entry.get_password().is_ok()
}

/// Check if a private key exists in the system keychain (stub for unsupported platforms)
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn has_private_key(_profile_name: &str) -> bool {
    false
}

#[cfg(test)]
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod tests {
    use super::*;

    // Note: These tests may fail on systems without a keychain or in CI environments.
    // They are primarily for local development testing.

    #[test]
    #[ignore] // Ignore by default as it requires system keychain
    fn test_keychain_roundtrip() {
        let profile = "aptos_test_profile_roundtrip";
        let private_key = b"test_private_key_data_32_bytes!!";

        // Clean up any existing entry
        let _ = delete_private_key(profile);

        // Store and retrieve
        store_private_key(profile, private_key).expect("Failed to store");
        let retrieved = retrieve_private_key(profile).expect("Failed to retrieve");
        assert_eq!(private_key.as_slice(), retrieved.as_slice());

        // Clean up
        delete_private_key(profile).expect("Failed to delete");
    }

    #[test]
    #[ignore] // Ignore by default as it requires system keychain
    fn test_keychain_not_found() {
        let result = retrieve_private_key("nonexistent_profile_12345");
        assert!(result.is_err());
    }
}
