// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validating the keyless authenticators a transaction carries, before the
//! prologue runs. The validation itself lives in `aptos-keyless-validation`;
//! this supplies the chain state it reads.

use crate::{
    errors::DiscardReason,
    providers::{read_config, read_resource, AptosDataProvider},
};
use aptos_keyless_validation::KeylessStateView;
use aptos_types::{
    jwks::{FederatedJWKs, PatchedJWKs},
    on_chain_config::CurrentTimeMicroseconds,
    transaction::SignedTransaction,
};
use aptos_vm_environment::environment::AptosEnvironment;
use mono_move_global_context::ExecutionGuard;
use move_core_types::account_address::AccountAddress;

/// Serves keyless validation the chain state it reads, as execution would
/// see it.
struct ProviderStateView<'a, 'guard> {
    guard: &'a ExecutionGuard<'guard>,
    provider: &'a dyn AptosDataProvider,
}

impl KeylessStateView for ProviderStateView<'_, '_> {
    fn current_time(&self) -> Option<CurrentTimeMicroseconds> {
        read_config::<CurrentTimeMicroseconds>(self.guard, self.provider)
            .ok()
            .flatten()
    }

    fn patched_jwks(&self) -> Option<PatchedJWKs> {
        read_config::<PatchedJWKs>(self.guard, self.provider)
            .ok()
            .flatten()
    }

    fn federated_jwks(&self, jwk_addr: &AccountAddress) -> Option<FederatedJWKs> {
        read_resource::<FederatedJWKs>(self.guard, self.provider, *jwk_addr)
            .ok()
            .flatten()
    }
}

/// Validates every keyless authenticator `txn` carries. A transaction with
/// none is untouched.
pub(crate) fn validate_keyless_authenticators(
    txn: &SignedTransaction,
    env: &AptosEnvironment,
    guard: &ExecutionGuard<'_>,
    provider: &dyn AptosDataProvider,
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
        &ProviderStateView { guard, provider },
    )
    .map_err(DiscardReason::KeylessValidationFailure)
}
