// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_dependencies_and_charge_gas,
    module_traversal::TraversalContext,
    storage::loader::traits::{
        Loader, ModuleMetadataLoader, NativeModuleLoader, StructDefinitionLoader,
    },
    ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{language_storage::ModuleId, metadata::Metadata};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{runtime_types::StructType, struct_name_indexing::StructNameIndex},
};
use std::sync::Arc;

/// Eager loader implementation used prior to lazy loading. It uses eager module verification by
/// loading and verifying the transitive closure of module's dependencies and friends. The gas is
/// metered at "entrypoints" (entry function or a script, dynamic dispatch) for the whole closure
/// at once.
pub struct EagerLoader<'a, T> {
    module_storage: &'a T,
}

impl<'a, T> EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    /// Returns a new eager loader.
    pub fn new(module_storage: &'a T) -> Self {
        Self { module_storage }
    }
}

impl<'a, T> WithRuntimeEnvironment for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.module_storage.runtime_environment()
    }
}

impl<'a, T> StructDefinitionLoader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn is_lazy_loading_enabled(&self) -> bool {
        debug_assert!(!self.runtime_environment().vm_config().enable_lazy_loading);
        false
    }

    fn load_struct_definition(
        &self,
        _gas_meter: &mut impl DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)?;

        self.module_storage.unmetered_get_struct_definition(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }
}

impl<'a, T> NativeModuleLoader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn charge_native_result_load_module(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<()> {
        let arena_id = traversal_context
            .referenced_module_ids
            .alloc(module_id.clone());
        check_dependencies_and_charge_gas(self.module_storage, gas_meter, traversal_context, [(
            arena_id.address(),
            arena_id.name(),
        )])
        .map_err(|err| {
            err.to_partial().append_message_with_separator(
                '.',
                format!(
                    "Failed to charge transitive dependency for {}. Does this module exist?",
                    module_id
                ),
            )
        })?;
        Ok(())
    }
}

impl<'a, T> ModuleMetadataLoader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_module_metadata(
        &self,
        _gas_meter: &mut impl DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<Vec<Metadata>> {
        // Note:
        //   For backwards compatibility, metadata accesses were never metered.
        self.module_storage
            .unmetered_get_existing_module_metadata(module_id.address(), module_id.name())
            .map_err(|err| err.to_partial())
    }
}

impl<'a, T> Loader for EagerLoader<'a, T> where T: ModuleStorage {}
