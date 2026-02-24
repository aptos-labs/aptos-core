// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accounts::{
        account_managers::AccountRecoveryManagers, account_recovery_db::AccountRecoveryDBInterface,
    },
    error::PepperServiceError,
    vuf_keypair::{get_pepper_service_vuf_public_key_and_json, VUFKeypair},
};
use aptos_crypto::blstrs::scalar_from_uniform_be_bytes;
use aptos_keyless_pepper_common::PepperInput;
use aptos_time_service::TimeService;
use sha3::Digest;
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

/// Generates a random VUF keypair for testing purposes
pub fn create_vuf_keypair(private_key_seed: Option<[u8; 32]>) -> Arc<VUFKeypair> {
    // Get or generate a seed for the private key
    let private_key_seed = private_key_seed.unwrap_or(rand::random::<[u8; 32]>());

    // Derive the VUF private key from the seed
    let mut sha3_hasher = sha3::Sha3_512::new();
    sha3_hasher.update(private_key_seed);
    let vuf_private_key = scalar_from_uniform_be_bytes(sha3_hasher.finalize().as_slice());

    // Get the VUF public key and its JSON representation
    let (vuf_public_key, vuf_public_key_json) =
        get_pepper_service_vuf_public_key_and_json(&vuf_private_key);

    // Return the VUF keypair
    let vuf_keypair = VUFKeypair::new(vuf_private_key, vuf_public_key, vuf_public_key_json);
    Arc::new(vuf_keypair)
}

/// Returns an empty account managers and overrides instance
pub fn get_empty_account_recovery_managers() -> Arc<AccountRecoveryManagers> {
    Arc::new(AccountRecoveryManagers::new_empty())
}

/// Returns a mock account recovery DB instance
pub fn get_mock_account_recovery_db() -> Arc<dyn AccountRecoveryDBInterface + Send + Sync> {
    Arc::new(MockAccountRecoveryDB)
}
