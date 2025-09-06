// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValueMetadata;
use aptos_vm_types::module_and_script_storage::module_storage::AptosModuleStorage;
use move_binary_format::errors::{Location, VMResult};
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{module_traversal::TraversalContext, EagerLoader, LazyLoader, Loader};

pub trait AptosLoader: Loader {
    fn load_module_state_value_metadata(
        &self,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> VMResult<Option<StateValueMetadata>>;
}

impl<'a, T> AptosLoader for LazyLoader<'a, T>
where
    T: AptosModuleStorage,
{
    fn load_module_state_value_metadata(
        &self,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> VMResult<Option<StateValueMetadata>> {
        let addr = module_id.address();
        let name = module_id.name();

        traversal_context
            .check_is_special_or_visited(addr, name)
            .map_err(|err| err.finish(Location::Undefined))?;
        self.as_unmetered_module_storage()
            .unmetered_get_module_state_value_metadata(addr, name)
            .map_err(|err| err.finish(Location::Undefined))
    }
}

impl<'a, T> AptosLoader for EagerLoader<'a, T>
where
    T: AptosModuleStorage,
{
    fn load_module_state_value_metadata(
        &self,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> VMResult<Option<StateValueMetadata>> {
        let addr = module_id.address();
        let name = module_id.name();

        traversal_context
            .check_is_special_or_visited(addr, name)
            .map_err(|err| err.finish(Location::Undefined))?;
        self.as_unmetered_module_storage()
            .unmetered_get_module_state_value_metadata(addr, name)
            .map_err(|err| err.finish(Location::Undefined))
    }
}
