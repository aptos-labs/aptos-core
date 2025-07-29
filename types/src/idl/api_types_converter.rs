// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Converter module for converting between api-types and gravity-aptos types
//!
//! This module provides conversion functions to transform ValidatorInfo and ValidatorConfig
//! from the api-types crate to the corresponding types in gravity-aptos.

use super::error::ValidatorInfoIdlError;
use crate::{
    account_address::AccountAddress,
    network_address::NetworkAddress,
    on_chain_config::{ConsensusScheme, ValidatorSet},
    validator_config::ValidatorConfig,
    validator_info::ValidatorInfo,
};
use anyhow::{anyhow, format_err};
use aptos_crypto::bls12381;
use bytes::Bytes;
use std::{convert::TryFrom, str::FromStr};

// Convert the bytes returned from execution layer
// In greth, it returnes one gravity_validator_set bytes
pub fn construct_and_convert_validator_set(
    bytes: &[u8],
) -> Result<ValidatorSet, ValidatorInfoIdlError> {
    let validator_set =
        bcs::from_bytes::<api_types::on_chain_config::validator_set::ValidatorSet>(bytes)
            .map_err(|e| format_err!("[on-chain config] Failed to deserialize into config: {}", e))
            .unwrap();
    let validator_set = convert_validator_set(validator_set)?;
    Ok(validator_set)
}

pub fn convert_validator_set(
    api_validator_set: api_types::on_chain_config::validator_set::ValidatorSet,
) -> Result<ValidatorSet, ValidatorInfoIdlError> {
    // Convert validator set
    let active_validators = api_validator_set
        .active_validators
        .iter()
        .map(|validator| convert_validator_info(validator))
        .collect::<Result<Vec<ValidatorInfo>, ValidatorInfoIdlError>>()?;
    let pending_inactive = api_validator_set
        .pending_inactive
        .iter()
        .map(|validator| convert_validator_info(validator))
        .collect::<Result<Vec<ValidatorInfo>, ValidatorInfoIdlError>>()?;
    let pending_active = api_validator_set
        .pending_active
        .iter()
        .map(|validator| convert_validator_info(validator))
        .collect::<Result<Vec<ValidatorInfo>, ValidatorInfoIdlError>>()?;
    let total_voting_power = api_validator_set.total_voting_power;
    let total_joining_power = api_validator_set.total_joining_power;

    Ok(ValidatorSet {
        scheme: ConsensusScheme::BLS12381,
        active_validators,
        pending_inactive,
        pending_active,
        total_voting_power,
        total_joining_power,
    })
}

/// Convert api-types ValidatorInfo to gravity-aptos ValidatorInfo
///
/// This function handles the conversion between the two different ValidatorInfo
/// implementations, ensuring type safety and proper error handling.
///
/// ## Example
///
/// ```rust
/// use aptos_types::idl::api_types_converter::convert_validator_info;
///
/// // Assuming you have an api_types::ValidatorInfo instance
/// let api_validator_info: api_types::ValidatorInfo = /* from api */;
///
/// // Convert to gravity-aptos ValidatorInfo
/// let gravity_validator_info = convert_validator_info(&api_validator_info)?;
/// ```
pub fn convert_validator_info(
    api_validator_info: &api_types::on_chain_config::validator_info::ValidatorInfo,
) -> Result<ValidatorInfo, ValidatorInfoIdlError> {
    // Convert account address
    let account_address = convert_account_address(&api_validator_info.account_address)?;

    // Convert validator config
    let config = convert_validator_config(&api_validator_info.config)?;

    // Create gravity-aptos ValidatorInfo
    Ok(ValidatorInfo::new(
        account_address,
        api_validator_info.consensus_voting_power,
        config,
    ))
}

// TODO(Gravity_alex): we should consider multi addresses in one validator config
fn parse_network_address(
    network_addresses: Vec<u8>,
) -> Result<Vec<NetworkAddress>, ValidatorInfoIdlError> {
    let address_bytes = Bytes::from(network_addresses);
    let address_string: String = bcs::from_bytes(&address_bytes).unwrap();
    let validator_network_address: NetworkAddress =
        NetworkAddress::from_str(&address_string).unwrap();
    Ok(vec![validator_network_address])
}

/// Convert api-types ValidatorConfig to gravity-aptos ValidatorConfig
pub fn convert_validator_config(
    api_config: &api_types::on_chain_config::validator_config::ValidatorConfig,
) -> Result<ValidatorConfig, ValidatorInfoIdlError> {
    // Convert consensus public key from Vec<u8> to bls12381::PublicKey
    // let consensus_public_key =
    //     bls12381::PublicKey::try_from(api_config.consensus_public_key.as_slice())
    //         .map_err(|e| ValidatorInfoIdlError::ConsensusPublicKeyError(e.to_string()))?;

    let consensus_public_key = bls12381::PublicKey::try_from(
        hex::decode(&api_config.consensus_public_key)
            .unwrap()
            .as_slice(),
    )
    .map_err(|e| ValidatorInfoIdlError::ConsensusPublicKeyError(e.to_string()))?;

    let validator_network_addresses = bcs::to_bytes(
        &parse_network_address(api_config.validator_network_addresses.clone()).unwrap(),
    )
    .unwrap();
    let fullnode_network_addresses = bcs::to_bytes(
        &parse_network_address(api_config.fullnode_network_addresses.clone()).unwrap(),
    )
    .unwrap();

    Ok(ValidatorConfig::new(
        consensus_public_key,
        validator_network_addresses,
        fullnode_network_addresses,
        api_config.validator_index,
    ))
}

/// Convert api-types AccountAddress to gravity-aptos AccountAddress
pub fn convert_account_address(
    api_address: &api_types::u256_define::AccountAddress,
) -> Result<AccountAddress, ValidatorInfoIdlError> {
    // api_types::AccountAddress is a wrapper around [u8; 32]
    // gravity-aptos::AccountAddress is also [u8; 32]
    // We can convert directly
    let bytes: [u8; 32] = api_address.bytes();
    Ok(AccountAddress::new(bytes))
}

/// Convert gravity-aptos ValidatorInfo to api-types ValidatorInfo
///
/// This is the reverse conversion, useful when you need to convert back
/// to api-types format for API responses.
pub fn convert_to_api_validator_info(
    validator_info: &ValidatorInfo,
) -> api_types::on_chain_config::validator_info::ValidatorInfo {
    api_types::on_chain_config::validator_info::ValidatorInfo {
        account_address: convert_to_api_account_address(validator_info.account_address()),
        consensus_voting_power: validator_info.consensus_voting_power(),
        config: convert_to_api_validator_config(validator_info.config()),
    }
}

/// Convert gravity-aptos ValidatorConfig to api-types ValidatorConfig
pub fn convert_to_api_validator_config(
    validator_config: &ValidatorConfig,
) -> api_types::on_chain_config::validator_config::ValidatorConfig {
    api_types::on_chain_config::validator_config::ValidatorConfig {
        consensus_public_key: validator_config.consensus_public_key.to_bytes().to_vec(),
        validator_network_addresses: validator_config.validator_network_addresses.clone(),
        fullnode_network_addresses: validator_config.fullnode_network_addresses.clone(),
        validator_index: validator_config.validator_index,
    }
}

/// Convert gravity-aptos AccountAddress to api-types AccountAddress
pub fn convert_to_api_account_address(
    gravity_address: &AccountAddress,
) -> api_types::u256_define::AccountAddress {
    api_types::u256_define::AccountAddress::new(gravity_address.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_address::NetworkAddress;

    #[test]
    fn test_convert_validator_info_roundtrip() {
        // Create a gravity-aptos ValidatorInfo
        let validator_info = ValidatorInfo::new(
            AccountAddress::from_hex_literal(
                "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            )
            .unwrap(),
            1000,
            ValidatorConfig::new(
                bls12381::PublicKey::try_from(&[42u8; 48][..]).unwrap(),
                bcs::to_bytes(&vec![NetworkAddress::mock()]).unwrap(),
                bcs::to_bytes(&vec![NetworkAddress::mock()]).unwrap(),
                42,
            ),
        );

        // Convert to api-types
        let api_validator_info = convert_to_api_validator_info(&validator_info);

        // Convert back to gravity-aptos
        let converted_back = convert_validator_info(&api_validator_info).unwrap();

        // Should be equal
        assert_eq!(validator_info, converted_back);
    }

    #[test]
    fn test_convert_validator_config() {
        let validator_config = ValidatorConfig::new(
            bls12381::PublicKey::try_from(&[42u8; 48][..]).unwrap(),
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            42,
        );

        let api_config = convert_to_api_validator_config(&validator_config);
        let converted_back = convert_validator_config(&api_config).unwrap();

        assert_eq!(validator_config, converted_back);
    }

    #[test]
    fn test_convert_account_address() {
        let gravity_address = AccountAddress::from_hex_literal(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        )
        .unwrap();

        let api_address = convert_to_api_account_address(&gravity_address);
        let converted_back = convert_account_address(&api_address).unwrap();

        assert_eq!(gravity_address, converted_back);
    }
}
