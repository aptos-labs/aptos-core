// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Serves the resources of V1's [`InMemoryStorage`] to the MonoMove test
//! harnesses.

use aptos_types::on_chain_config::{Features, OnChainConfig};
use mono_move_core::storage::resource_provider::{ResourceProvider, ResourceProviderError};
use mono_move_global_context::ExecutionGuard;
use mono_move_unit_test_backend::LazyResourceProvider;
use move_core_types::account_address::AccountAddress;
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::resolver::ResourceResolver;

/// A provider serving `storage`'s resources to MonoMove.
pub(crate) fn storage_resource_provider<'guard>(
    guard: &'guard ExecutionGuard<'_>,
    storage: &'guard InMemoryStorage,
) -> impl ResourceProvider + 'guard {
    LazyResourceProvider::new(guard, move |address, tag| {
        let (blob, _) = storage
            .get_resource_bytes_with_metadata_and_layout(&address, &tag, &[], None)
            .map_err(|err| ResourceProviderError::InvariantViolation(err.to_string()))?;
        Ok(blob)
    })
}

/// Returns storage with default features for testing enabled.
pub(crate) fn storage_with_default_features_for_test() -> InMemoryStorage {
    let mut storage = InMemoryStorage::new();
    storage.publish_or_overwrite_resource(
        AccountAddress::ONE,
        Features::struct_tag(),
        features_blob(),
    );
    storage
}

/// BCS bytes of the `Features` resource the test harnesses seed at `0x1`.
pub(crate) fn features_blob() -> Vec<u8> {
    bcs::to_bytes(&Features::default_for_tests()).expect("Features serializes")
}
