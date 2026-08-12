// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::metadata::TxnMetadata;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use mono_move_core::{
    native::{NativeExtensions, NativeName},
    Interner,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_natives::{
    make_all_production_natives, Dispatch, EventStore, ObjectContextExtension, RandomnessContext,
    StorageUsageAtEpochBoundary, TransactionContextExtension,
};
use mono_move_runtime::{ProductionContextFamily, ProductionNativeRegistry};

/// Builds the production native registry.
pub fn production_natives(guard: &ExecutionGuard<'_>) -> ProductionNativeRegistry {
    let mut natives = ProductionNativeRegistry::new();
    natives
        .register_all(
            make_all_production_natives::<ProductionContextFamily>()
                .into_iter()
                .map(|(addr, module, function, dispatch, func)| {
                    let module = guard.module_id_of(&addr, &module);
                    let function = guard.identifier_of(&function);
                    let name = match dispatch {
                        Dispatch::Polymorphic => NativeName::Polymorphic { module, function },
                        Dispatch::Monomorphic(ty_args) => NativeName::Monomorphic {
                            module,
                            function,
                            ty_args: guard.type_list_of(ty_args),
                        },
                    };
                    (name, func)
                }),
        )
        .expect("natives have unique qualified names");
    natives
}

/// The native extensions a user transaction runs with.
///
/// `unbiasable` says whether the transaction may draw randomness, which only a call annotated
/// `#[randomness]` may — see [`RandomnessContext`].
pub(crate) fn transaction_extensions(
    txn_data: &TxnMetadata,
    usage: StateStorageUsage,
    unbiasable: bool,
) -> NativeExtensions {
    let mut extensions = NativeExtensions::new();
    // TODO(completeness): the transaction index and the reserved byte
    // (block execution vs. validation/simulation) come from the auxiliary
    // info the block coordinator has to pass in; both are hardcoded here.
    extensions.add(TransactionContextExtension::new(
        txn_data.txn_hash.clone(),
        0,
        0,
        0,
    ));
    extensions.add(ObjectContextExtension::new());
    extensions.add(StorageUsageAtEpochBoundary::new(
        usage.items() as u64,
        usage.bytes() as u64,
    ));
    extensions.add(EventStore::new());
    extensions.add(RandomnessContext::new(unbiasable));
    extensions
}
