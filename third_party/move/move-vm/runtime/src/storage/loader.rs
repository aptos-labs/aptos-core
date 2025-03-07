// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{config::VMConfig, loader::LoadedFunctionOwner, CodeStorage, LoadedFunction};
use move_binary_format::errors::{Location, PartialVMResult, VMResult};
use move_core_types::language_storage::TypeTag;
use move_vm_types::loaded_data::runtime_types::{Type, TypeBuilder};

/// V2 implementation of loader, which is stateless - i.e., it does not contain module or script
/// cache. Instead, module and script storages are passed to all APIs by reference.
pub(crate) struct LoaderV2 {
    vm_config: VMConfig,
}

impl LoaderV2 {
    pub(crate) fn new(vm_config: VMConfig) -> Self {
        Self { vm_config }
    }

    pub(crate) fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    pub(crate) fn ty_builder(&self) -> &TypeBuilder {
        &self.vm_config.ty_builder
    }

    /// Loads the script:
    ///   1. Fetches it from the cache (or deserializes and verifies it if it is not cached).
    ///   2. Verifies type arguments (modules that define the type arguments are also loaded).
    /// If both steps are successful, returns a [LoadedFunction] corresponding to the script's
    /// entrypoint.
    pub(crate) fn load_script(
        &self,
        code_storage: &impl CodeStorage,
        serialized_script: &[u8],
        ty_tag_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        // Step 1: Load script. During the loading process, if script has not been previously
        // cached, it will be verified.
        let script = code_storage.verify_and_cache_script(serialized_script)?;

        // Step 2: Load & verify types used as type arguments passed to this script. Note that
        // arguments for scripts are verified on the client side.
        let ty_args = ty_tag_args
            .iter()
            .map(|ty_tag| code_storage.fetch_ty(ty_tag))
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| err.finish(Location::Script))?;

        let main = script.entry_point();
        Type::verify_ty_arg_abilities(main.ty_param_abilities(), &ty_args)
            .map_err(|err| err.finish(Location::Script))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Script(script),
            ty_args,
            function: main,
        })
    }
}
