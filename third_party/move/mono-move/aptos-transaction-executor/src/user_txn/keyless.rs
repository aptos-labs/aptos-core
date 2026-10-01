// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validating the keyless authenticators a transaction carries, before the
//! prologue runs. The validation itself lives in `aptos-keyless-validation`;
//! this supplies the chain state it reads.

use crate::{
    errors::DiscardReason,
    providers::{read_config, read_resource},
    symbols::FrameworkSymbols,
};
use aptos_keyless_validation::KeylessStateView;
use aptos_types::{
    jwks::{FederatedJWKs, PatchedJWKs},
    on_chain_config::CurrentTimeMicroseconds,
    transaction::SignedTransaction,
};
use aptos_vm_environment::environment::AptosEnvironment;
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::InterpreterContext;
use move_core_types::account_address::AccountAddress;
use std::cell::RefCell;

/// Serves keyless validation the chain state it reads, through the
/// interpreter so every read is recorded.
struct InterpreterStateView<'a, 'guard> {
    // The validation reads through `&self`, so each read borrows the
    // interpreter mutably for its duration.
    interp: RefCell<&'a mut InterpreterContext<'guard>>,
    guard: &'a ExecutionGuard<'guard>,
    symbols: &'a FrameworkSymbols,
}

impl KeylessStateView for InterpreterStateView<'_, '_> {
    fn current_time(&self) -> Option<CurrentTimeMicroseconds> {
        read_config::<CurrentTimeMicroseconds>(
            &mut self.interp.borrow_mut(),
            self.guard,
            self.symbols.current_time_microseconds,
        )
        .ok()
        .flatten()
    }

    fn patched_jwks(&self) -> Option<PatchedJWKs> {
        read_config::<PatchedJWKs>(
            &mut self.interp.borrow_mut(),
            self.guard,
            self.symbols.patched_jwks,
        )
        .ok()
        .flatten()
    }

    fn federated_jwks(&self, jwk_addr: &AccountAddress) -> Option<FederatedJWKs> {
        read_resource::<FederatedJWKs>(
            &mut self.interp.borrow_mut(),
            self.guard,
            self.symbols.federated_jwks,
            *jwk_addr,
        )
        .ok()
        .flatten()
    }
}

/// Validates every keyless authenticator `txn` carries. A transaction with
/// none is untouched.
pub(crate) fn validate_keyless_authenticators<'guard>(
    txn: &SignedTransaction,
    env: &AptosEnvironment,
    interp: &mut InterpreterContext<'guard>,
    guard: &ExecutionGuard<'guard>,
    symbols: &FrameworkSymbols,
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
        &InterpreterStateView {
            interp: RefCell::new(interp),
            guard,
            symbols,
        },
    )
    .map_err(DiscardReason::KeylessValidationFailure)
}
