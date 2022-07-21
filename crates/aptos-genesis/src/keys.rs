// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::config::{HostAndPort, ValidatorConfiguration};
use aptos_config::{config::IdentityBlob, keys::ConfigKey};
use aptos_crypto::{bls12381, ed25519::Ed25519PrivateKey, x25519, PrivateKey};
use aptos_keygen::KeyGen;
use aptos_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
use serde::{Deserialize, Serialize};

/// Type for serializing private keys file
#[derive(Deserialize, Serialize)]
pub struct PrivateIdentity {
    pub account_address: AccountAddress,
    pub account_private_key: Ed25519PrivateKey,
    pub consensus_private_key: bls12381::PrivateKey,
    pub full_node_network_private_key: x25519::PrivateKey,
    pub validator_network_private_key: x25519::PrivateKey,
}

/// Builds validator configuration from private identity
pub fn build_validator_configuration(
    private_identity: PrivateIdentity,
    validator_host: HostAndPort,
    full_node_host: Option<HostAndPort>,
    stake_amount: u64,
) -> anyhow::Result<ValidatorConfiguration> {
    let account_address = private_identity.account_address;
    let account_public_key = private_identity.account_private_key.public_key();
    let consensus_public_key = private_identity.consensus_private_key.public_key();
    let proof_of_possession =
        bls12381::ProofOfPossession::create(&private_identity.consensus_private_key);
    let validator_network_public_key = private_identity.validator_network_private_key.public_key();

    let full_node_network_public_key = if full_node_host.is_some() {
        Some(private_identity.full_node_network_private_key.public_key())
    } else {
        None
    };

    Ok(ValidatorConfiguration {
        account_address,
        consensus_public_key,
        proof_of_possession,
        account_public_key,
        validator_network_public_key,
        validator_host,
        full_node_network_public_key,
        full_node_host,
        stake_amount,
    })
}

/// Generates objects used for a user in genesis
pub fn generate_key_objects(
    keygen: &mut KeyGen,
) -> anyhow::Result<(IdentityBlob, IdentityBlob, PrivateIdentity)> {
    let account_key = ConfigKey::new(keygen.generate_ed25519_private_key());
    let consensus_key = ConfigKey::new(keygen.generate_bls12381_private_key());
    let validator_network_key = ConfigKey::new(keygen.generate_x25519_private_key()?);
    let full_node_network_key = ConfigKey::new(keygen.generate_x25519_private_key()?);

    let account_address = AuthenticationKey::ed25519(&account_key.public_key()).derived_address();

    // Build these for use later as node identity
    let validator_blob = IdentityBlob {
        account_address: Some(account_address),
        account_private_key: Some(account_key.private_key()),
        consensus_private_key: Some(consensus_key.private_key()),
        network_private_key: validator_network_key.private_key(),
    };
    let vfn_blob = IdentityBlob {
        account_address: Some(account_address),
        account_private_key: None,
        consensus_private_key: None,
        network_private_key: full_node_network_key.private_key(),
    };

    let private_identity = PrivateIdentity {
        account_address,
        account_private_key: account_key.private_key(),
        consensus_private_key: consensus_key.private_key(),
        full_node_network_private_key: full_node_network_key.private_key(),
        validator_network_private_key: validator_network_key.private_key(),
    };

    Ok((validator_blob, vfn_blob, private_identity))
}
