// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accounts::{
        account_managers::AccountRecoveryManagers, account_recovery_db::AccountRecoveryDBInterface,
    },
    error::PepperServiceError,
};
use aptos_keyless_pepper_common::{
    vuf::{bls12381_g1_bls::Bls12381G1Bls, slip_10::ed25519_dalek::Digest, VUF},
    PepperInput, PepperV0VufPubKey,
};
use aptos_time_service::TimeService;
use ark_ec::CurveGroup;
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use std::{sync::Arc, time::Duration};

/// A mock implementation of the account recovery DB that does nothing
struct MockAccountRecoveryDB;

#[async_trait::async_trait]
impl AccountRecoveryDBInterface for MockAccountRecoveryDB {
    async fn update_db_with_pepper_input(
        &self,
        _pepper_input: &PepperInput,
    ) -> Result<(), PepperServiceError> {
        Ok(()) // Do nothing
    }
}

/// Advances the mock time service by the given number of seconds
pub async fn advance_time_secs(time_service: TimeService, seconds: u64) {
    let mock_time_service = time_service.into_mock();
    mock_time_service
        .advance_async(Duration::from_secs(seconds))
        .await;
}

/// Generates a random VUF public and private keypair for testing purposes
pub fn create_vuf_public_private_keypair() -> (String, Arc<ark_bls12_381::Fr>) {
    // Generate a random VUF seed
    let private_key_seed = rand::random::<[u8; 32]>();

    // Derive the VUF private key from the seed
    let mut sha3_hasher = sha3::Sha3_512::new();
    sha3_hasher.update(private_key_seed);
    let vuf_private_key =
        ark_bls12_381::Fr::from_be_bytes_mod_order(sha3_hasher.finalize().as_slice());

    // Derive the VUF public key from the private key
    let vuf_public_key = Bls12381G1Bls::pk_from_sk(&vuf_private_key).unwrap();

    // Create the pepper public key object
    let mut public_key_buf = vec![];
    vuf_public_key
        .into_affine()
        .serialize_compressed(&mut public_key_buf)
        .unwrap();
    let pepper_vuf_public_key = PepperV0VufPubKey::new(public_key_buf);

    // Transform the public key object to a pretty JSON string
    let vuf_public_key_string = serde_json::to_string_pretty(&pepper_vuf_public_key).unwrap();

    (vuf_public_key_string, Arc::new(vuf_private_key))
}

/// Returns an empty account managers and overrides instance
pub fn get_empty_account_recovery_managers() -> Arc<AccountRecoveryManagers> {
    Arc::new(AccountRecoveryManagers::new_empty())
}

/// Returns a mock account recovery DB instance
pub fn get_mock_account_recovery_db() -> Arc<dyn AccountRecoveryDBInterface + Send + Sync> {
    Arc::new(MockAccountRecoveryDB)
}
