// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use mono_move_core::native::NativeExtensions;
use mono_move_natives::{
    make_all_production_natives, EventStore, ObjectContextExtension, RistrettoPointStore,
    StorageUsageAtEpochBoundary, TransactionContextExtension,
};
use mono_move_runtime::{ProductionContextFamily, ProductionNativeRegistry};
use std::sync::LazyLock;

/// Builds the production native registry.
pub fn production_natives() -> &'static ProductionNativeRegistry {
    &PRODUCTION_NATIVES
}

static PRODUCTION_NATIVES: LazyLock<ProductionNativeRegistry> = LazyLock::new(|| {
    ProductionNativeRegistry::with_natives(make_all_production_natives::<ProductionContextFamily>())
});

/// The native extensions every transaction kind runs with, around its own
/// transaction-context extension.
pub(crate) fn extensions_with(
    txn_context: TransactionContextExtension,
    usage: StateStorageUsage,
) -> NativeExtensions {
    let mut extensions = NativeExtensions::new();
    extensions.add(txn_context);
    extensions.add(ObjectContextExtension::new());
    extensions.add(StorageUsageAtEpochBoundary::new(
        usage.items() as u64,
        usage.bytes() as u64,
    ));
    extensions.add(EventStore::new());
    extensions.add(RistrettoPointStore::new());
    extensions
}
