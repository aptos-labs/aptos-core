// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validating the keyless authenticators a transaction carries, before the
//! prologue runs. The validation itself lives in `aptos-keyless-validation`;
//! this supplies the chain state it reads.

use crate::errors::DiscardReason;
use aptos_keyless_validation::KeylessStateView;
use aptos_types::{
    jwks::{FederatedJWKs, PatchedJWKs},
    on_chain_config::CurrentTimeMicroseconds,
    transaction::SignedTransaction,
    vm_status::{StatusCode, VMStatus},
};
use aptos_vm_environment::environment::AptosEnvironment;
use move_core_types::account_address::AccountAddress;

/// Serves keyless validation out of the block's environment.
//
// TODO(correctness): the timestamp and the JWKs are read once per block, so a
// transaction sees the values from before its own block's prologue ran.
// AptosVM reads both per transaction. Needs a resource read on the executor's
// own provider.
struct EnvironmentStateView<'a> {
    env: &'a AptosEnvironment,
}

impl KeylessStateView for EnvironmentStateView<'_> {
    fn current_time(&self) -> Result<CurrentTimeMicroseconds, VMStatus> {
        self.env.current_time().cloned().ok_or_else(|| {
            VMStatus::error(
                StatusCode::VALUE_DESERIALIZATION_ERROR,
                Some("could not fetch CurrentTimeMicroseconds on-chain config".to_string()),
            )
        })
    }

    fn patched_jwks(&self) -> Result<PatchedJWKs, VMStatus> {
        self.env
            .patched_jwks_bytes()
            .and_then(|bytes| bcs::from_bytes(bytes).ok())
            .ok_or_else(|| {
                VMStatus::error(
                    StatusCode::VALUE_DESERIALIZATION_ERROR,
                    Some("could not deserialize PatchedJWKs".to_string()),
                )
            })
    }

    // TODO(completeness): a federated account nominates an arbitrary address,
    // which cannot be preloaded into the environment.
    fn federated_jwks(&self, _jwk_addr: &AccountAddress) -> Result<FederatedJWKs, VMStatus> {
        Err(VMStatus::error(
            StatusCode::VALUE_DESERIALIZATION_ERROR,
            Some("federated keyless accounts are not supported yet".to_string()),
        ))
    }
}

/// Validates every keyless authenticator `txn` carries. A transaction with
/// none is untouched.
pub(crate) fn validate_keyless_authenticators(
    txn: &SignedTransaction,
    env: &AptosEnvironment,
) -> Result<(), DiscardReason> {
    let authenticators = aptos_types::keyless::get_authenticators(txn)
        .map_err(|_| DiscardReason::InvalidSignature)?;
    if authenticators.is_empty() {
        return Ok(());
    }
    aptos_keyless_validation::validate_authenticators(
        env.keyless_pvk(),
        env.keyless_configuration(),
        &authenticators,
        env.features(),
        &EnvironmentStateView { env },
    )
    .map_err(|status| DiscardReason::KeylessValidation(status.status_code()))
}
