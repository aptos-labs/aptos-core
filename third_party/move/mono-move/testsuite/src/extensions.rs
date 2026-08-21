// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared native-extension fixtures for the mono-move test harnesses.

use aptos_types::transaction::user_transaction_context::{
    TransactionIndexKind, UserTransactionContext,
};
use mono_move_core::native::NativeExtensions;
use mono_move_natives::{
    EventStore, ObjectContextExtension, RistrettoPointStore, StorageUsageAtEpochBoundary,
    TransactionContextExtension,
};
use move_core_types::account_address::AccountAddress;

// Fixed inputs seeded into both VMs' native extensions, so extension-backed
// natives (AUID generation, state-storage usage, ...) produce matching output
// across the two engines.
pub(crate) const TEST_TXN_HASH: [u8; 32] = [7u8; 32];
pub(crate) const TEST_SCRIPT_HASH: [u8; 1] = [1u8];
pub(crate) const TEST_CHAIN_ID: u8 = 4;
pub(crate) const TEST_STATE_ITEMS: u64 = 100;
pub(crate) const TEST_STATE_BYTES: u64 = 2000;
pub(crate) const TEST_SESSION_COUNTER: u8 = 2;
pub(crate) const TEST_TXN_INDEX: u32 = 5;

/// Builds the per-transaction native extensions initialized with the fixed
/// dummy inputs.
pub(crate) fn seed_extensions(user_transaction_context: bool) -> NativeExtensions {
    let mut extensions = NativeExtensions::new();
    extensions.add(TransactionContextExtension::new(
        TEST_TXN_HASH,
        TEST_SCRIPT_HASH.to_vec(),
        TEST_CHAIN_ID,
        TEST_SESSION_COUNTER,
        user_transaction_context.then(test_user_transaction_context),
    ));
    extensions.add(ObjectContextExtension::new());
    extensions.add(StorageUsageAtEpochBoundary::new(
        TEST_STATE_ITEMS,
        TEST_STATE_BYTES,
    ));
    extensions.add(EventStore::new());
    extensions.add(RistrettoPointStore::new());
    extensions
}

/// The user transaction context both VMs use.
pub(crate) fn test_user_transaction_context() -> UserTransactionContext {
    UserTransactionContext::new(
        AccountAddress::ZERO,
        vec![],
        AccountAddress::ZERO,
        0,
        0,
        TEST_CHAIN_ID,
        None,
        None,
        TransactionIndexKind::BlockExecution {
            transaction_index: TEST_TXN_INDEX,
        },
        false,
        false,
    )
}
