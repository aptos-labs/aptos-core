// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! ValidatorInfo IDL implementation for cross-language serialization

use crate::{
    account_address::AccountAddress,
    validator_config::ValidatorConfig,
    validator_info::ValidatorInfo,
};
use aptos_crypto::bls12381;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use super::error::ValidatorInfoIdlError;

/// JSON IDL representation of ValidatorInfo for cross-language serialization
/// 
/// This struct represents ValidatorInfo in a JSON-friendly format that can be
/// easily serialized and deserialized across different programming languages.
/// 
/// ## JSON Format
/// ```json
/// {
///   "account_address": "0x1234567890abcdef...",
///   "consensus_voting_power": 1000,
///   "config": {
///     "consensus_public_key": "0xabcdef1234567890...",
///     "validator_network_addresses": "base64_encoded_bcs_bytes",
///     "fullnode_network_addresses": "base64_encoded_bcs_bytes",
///     "validator_index": 42
///   }
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorInfoIdl {
    /// Account address in hex format (0x-prefixed)
    pub account_address: String,
    /// Voting power for consensus
    pub consensus_voting_power: u64,
    /// Validator configuration
    pub config: ValidatorConfigIdl,
}

/// JSON IDL representation of ValidatorConfig for cross-language serialization
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorConfigIdl {
    /// Consensus public key in hex format
    pub consensus_public_key: String,
    /// Validator network addresses as base64-encoded BCS bytes
    pub validator_network_addresses: String,
    /// Fullnode network addresses as base64-encoded BCS bytes
    pub fullnode_network_addresses: String,
    /// Validator index in the validator set
    pub validator_index: u64,
}

impl From<&ValidatorInfo> for ValidatorInfoIdl {
    fn from(validator_info: &ValidatorInfo) -> Self {
        ValidatorInfoIdl {
            account_address: validator_info.account_address.to_hex(),
            consensus_voting_power: validator_info.consensus_voting_power(),
            config: ValidatorConfigIdl::from(validator_info.config()),
        }
    }
}

impl From<&ValidatorConfig> for ValidatorConfigIdl {
    fn from(config: &ValidatorConfig) -> Self {
        ValidatorConfigIdl {
            consensus_public_key: hex::encode(config.consensus_public_key.to_bytes()),
            validator_network_addresses: base64::encode(&config.validator_network_addresses),
            fullnode_network_addresses: base64::encode(&config.fullnode_network_addresses),
            validator_index: config.validator_index,
        }
    }
}

impl TryFrom<ValidatorInfoIdl> for ValidatorInfo {
    type Error = ValidatorInfoIdlError;

    fn try_from(idl: ValidatorInfoIdl) -> Result<Self, Self::Error> {
        let account_address = AccountAddress::from_hex_literal(&idl.account_address)
            .map_err(|e| ValidatorInfoIdlError::AccountAddressError(e.to_string()))?;
        
        let config = idl.config.try_into()?;

        Ok(ValidatorInfo::new(
            account_address,
            idl.consensus_voting_power,
            config,
        ))
    }
}

impl TryFrom<ValidatorConfigIdl> for ValidatorConfig {
    type Error = ValidatorInfoIdlError;

    fn try_from(idl: ValidatorConfigIdl) -> Result<Self, Self::Error> {
        let consensus_public_key_bytes = hex::decode(&idl.consensus_public_key)
            .map_err(|e| ValidatorInfoIdlError::HexError(e.to_string()))?;
        
        let consensus_public_key = bls12381::PublicKey::try_from(consensus_public_key_bytes.as_slice())
            .map_err(|e| ValidatorInfoIdlError::ConsensusPublicKeyError(e.to_string()))?;

        let validator_network_addresses = base64::decode(&idl.validator_network_addresses)
            .map_err(|e| ValidatorInfoIdlError::Base64Error(e.to_string()))?;

        let fullnode_network_addresses = base64::decode(&idl.fullnode_network_addresses)
            .map_err(|e| ValidatorInfoIdlError::Base64Error(e.to_string()))?;

        Ok(ValidatorConfig::new(
            consensus_public_key,
            validator_network_addresses,
            fullnode_network_addresses,
            idl.validator_index,
        ))
    }
}

// Extension trait for ValidatorInfo to add IDL functionality
pub trait ValidatorInfoIdlExt {
    /// Serialize ValidatorInfo to JSON string for IDL purposes
    fn to_json_idl(&self) -> Result<String, serde_json::Error>;
    
    /// Deserialize ValidatorInfo from JSON string for IDL purposes
    fn from_json_idl(json: &str) -> Result<Self, ValidatorInfoIdlError> where Self: Sized;
}

impl ValidatorInfoIdlExt for ValidatorInfo {
    fn to_json_idl(&self) -> Result<String, serde_json::Error> {
        let idl_representation = ValidatorInfoIdl::from(self);
        serde_json::to_string_pretty(&idl_representation)
    }

    fn from_json_idl(json: &str) -> Result<Self, ValidatorInfoIdlError> {
        let idl_representation: ValidatorInfoIdl = serde_json::from_str(json)
            .map_err(|e| ValidatorInfoIdlError::JsonDeserializationError(e.to_string()))?;
        idl_representation.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_address::NetworkAddress;

    #[test]
    fn test_validator_info_idl_roundtrip() {
        let validator_info = ValidatorInfo::new(
            AccountAddress::from_hex_literal("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap(),
            1000,
            ValidatorConfig::new(
                bls12381::PublicKey::try_from(&[42u8; 48][..]).unwrap(),
                bcs::to_bytes(&vec![NetworkAddress::mock()]).unwrap(),
                bcs::to_bytes(&vec![NetworkAddress::mock()]).unwrap(),
                42,
            ),
        );

        // Test serialization to JSON IDL
        let json_idl = validator_info.to_json_idl().unwrap();
        println!("JSON IDL: {}", json_idl);

        // Test deserialization from JSON IDL
        let deserialized = ValidatorInfo::from_json_idl(&json_idl).unwrap();
        assert_eq!(validator_info, deserialized);
    }

    #[test]
    fn test_validator_info_idl_format() {
        let validator_info = ValidatorInfo::new(
            AccountAddress::from_hex_literal("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap(),
            1000,
            ValidatorConfig::new(
                bls12381::PublicKey::try_from(&[42u8; 48][..]).unwrap(),
                bcs::to_bytes(&vec![NetworkAddress::mock()]).unwrap(),
                bcs::to_bytes(&vec![NetworkAddress::mock()]).unwrap(),
                42,
            ),
        );

        let json_idl = validator_info.to_json_idl().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_idl).unwrap();
        
        // Verify the structure
        assert!(parsed.get("account_address").is_some());
        assert!(parsed.get("consensus_voting_power").is_some());
        assert!(parsed.get("config").is_some());
        
        let config = parsed.get("config").unwrap();
        assert!(config.get("consensus_public_key").is_some());
        assert!(config.get("validator_network_addresses").is_some());
        assert!(config.get("fullnode_network_addresses").is_some());
        assert!(config.get("validator_index").is_some());
    }

    #[test]
    fn test_validator_info_idl_error_handling() {
        // Test invalid JSON
        let invalid_json = "{ invalid json }";
        let result = ValidatorInfo::from_json_idl(invalid_json);
        assert!(result.is_err());

        // Test missing required fields
        let incomplete_json = r#"{"account_address": "0x123"}"#;
        let result = ValidatorInfo::from_json_idl(incomplete_json);
        assert!(result.is_err());
    }
} 