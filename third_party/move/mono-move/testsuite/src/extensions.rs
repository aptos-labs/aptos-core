// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared native-extension fixtures for the mono-move test harnesses.

use mono_move_core::native::NativeExtensions;
use mono_move_natives::{
    EventStore, ObjectContextExtension, StorageUsageAtEpochBoundary, TransactionContextExtension,
};

// Fixed inputs seeded into both VMs' native extensions, so extension-backed
// natives (AUID generation, state-storage usage, ...) produce matching output
// across the two engines.
pub(crate) const TEST_TXN_HASH: [u8; 32] = [7u8; 32];
pub(crate) const TEST_STATE_ITEMS: u64 = 100;
pub(crate) const TEST_STATE_BYTES: u64 = 2000;
// Inputs to the monotonically-increasing counter. The reserved byte is 0,
// matching `TransactionIndexKind::BlockExecution`.
pub(crate) const TEST_SESSION_COUNTER: u8 = 2;
pub(crate) const TEST_TXN_INDEX: u32 = 5;
pub(crate) const TEST_RESERVED_BYTE: u8 = 0;

/// Builds the per-transaction native extensions seeded with the fixed inputs above.
pub(crate) fn seed_extensions() -> NativeExtensions {
    let mut extensions = NativeExtensions::new();
    extensions.add(TransactionContextExtension::new(
        TEST_TXN_HASH.to_vec(),
        TEST_SESSION_COUNTER,
        TEST_TXN_INDEX,
        TEST_RESERVED_BYTE,
    ));
    extensions.add(ObjectContextExtension::new());
    extensions.add(StorageUsageAtEpochBoundary::new(
        TEST_STATE_ITEMS,
        TEST_STATE_BYTES,
    ));
    extensions.add(EventStore::new());
    extensions
}
