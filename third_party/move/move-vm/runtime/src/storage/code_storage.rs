// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{loader::Script, logging::expect_no_verification_errors, ModuleStorage};
use ambassador::delegatable_trait;
use move_binary_format::{errors::VMResult, file_format::CompiledScript};
use move_vm_types::{
    code::{Code, ScriptCache},
    module_linker_error,
};
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

/// Returns the hash (SHA-3-256) of the bytes. Used for both modules and scripts.
pub fn compute_code_hash(bytes: &[u8]) -> [u8; 32] {
    let mut sha3_256 = Sha3_256::new();
    sha3_256.update(bytes);
    sha3_256.finalize().into()
}

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
        let hash = compute_code_hash(serialized_script);
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

        let hash = compute_code_hash(serialized_script);
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
            .map(|(addr, name)| {
                // Since module is stored on-chain, we should not see any verification errors here.
                self.fetch_verified_module(addr, name)
                    .map_err(expect_no_verification_errors)?
                    .ok_or_else(|| module_linker_error!(addr, name))
            })
            .collect::<VMResult<Vec<_>>>()?;
        let verified_script = self
            .runtime_environment()
            .build_verified_script(locally_verified_script, &immediate_dependencies)?;

        Ok(self.insert_verified_script(hash, verified_script))
    }
}
