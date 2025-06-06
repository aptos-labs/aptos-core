// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{LoadedFunctionOwner, Script},
    LoadedFunction, ModuleStorage,
};
use ambassador::delegatable_trait;
use move_binary_format::{
    errors::{Location, PartialVMResult, VMResult},
    file_format::CompiledScript,
};
use move_core_types::language_storage::TypeTag;
use move_vm_types::{
    code::{Code, ScriptCache},
    loaded_data::runtime_types::Type,
    sha3_256,
};
use std::sync::Arc;

/// Represents storage which in addition to modules, also caches scripts. The clients can implement
/// this trait to ensure that even script dependency is upgraded, the correct script is still
/// returned. Scripts are cached based on their hash.
#[delegatable_trait]
pub trait CodeStorage: ModuleStorage {
    /// Returns a deserialized script, either by directly deserializing it from the provided bytes
    /// (and caching it), or fetching it from the cache. Note that there are no guarantees that the
    /// returned script is verified. An error is returned if the deserialization fails.
    fn deserialize_and_cache_script(
        &self,
        serialized_script: &[u8],
    ) -> VMResult<Arc<CompiledScript>>;

    /// Returns a verified script. If not yet cached, verified from scratch and cached. An error is
    /// returned if script fails to deserialize or verify.
    fn verify_and_cache_script(&self, serialized_script: &[u8]) -> VMResult<Arc<Script>>;

    /// Loads the script:
    ///   1. Fetches it from the cache (or deserializes and verifies it if it is not cached).
    ///   2. Verifies type arguments (modules that define the type arguments are also loaded).
    /// If both steps are successful, returns a [LoadedFunction] corresponding to the script's
    /// entrypoint.
    fn load_script(
        &self,
        serialized_script: &[u8],
        ty_tag_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        // Step 1: Load script. During the loading process, if script has not been previously
        // cached, it will be verified.
        let script = self.verify_and_cache_script(serialized_script)?;

        // Step 2: Load & verify types used as type arguments passed to this script. Note that
        // arguments for scripts are verified on the client side.
        let ty_args = ty_tag_args
            .iter()
            .map(|ty_tag| self.fetch_ty(ty_tag))
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

impl<T> CodeStorage for T
where
    T: ModuleStorage
        + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>,
{
    fn deserialize_and_cache_script(
        &self,
        serialized_script: &[u8],
    ) -> VMResult<Arc<CompiledScript>> {
        let hash = sha3_256(serialized_script);
        Ok(match self.get_script(&hash) {
            Some(script) => script.deserialized().clone(),
            None => {
                let deserialized_script = self
                    .runtime_environment()
                    .deserialize_into_script(serialized_script)?;
                self.insert_deserialized_script(hash, deserialized_script)
            },
        })
    }

    fn verify_and_cache_script(&self, serialized_script: &[u8]) -> VMResult<Arc<Script>> {
        use Code::*;

        let hash = sha3_256(serialized_script);
        let deserialized_script = match self.get_script(&hash) {
            Some(Verified(script)) => return Ok(script),
            Some(Deserialized(deserialized_script)) => deserialized_script,
            None => self
                .runtime_environment()
                .deserialize_into_script(serialized_script)
                .map(Arc::new)?,
        };

        // Locally verify the script.
        let locally_verified_script = self
            .runtime_environment()
            .build_locally_verified_script(deserialized_script)?;

        // Verify the script is correct w.r.t. its dependencies.
        let immediate_dependencies = locally_verified_script
            .immediate_dependencies_iter()
            .map(|id| {
                // Since module is stored on-chain, we should not see any verification errors here.
                self.fetch_existing_verified_module(&id)
            })
            .collect::<VMResult<Vec<_>>>()?;
        let verified_script = self
            .runtime_environment()
            .build_verified_script(locally_verified_script, &immediate_dependencies)?;

        Ok(self.insert_verified_script(hash, verified_script))
    }
}
