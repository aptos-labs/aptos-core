// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{loader::Script, ModuleStorage};
use ambassador::delegatable_trait;
use move_binary_format::{
    deserializer::DeserializerConfig,
    errors::{Location, PartialVMError, VMResult},
    file_format::CompiledScript,
};
use move_core_types::vm_status::StatusCode;
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

/// Returns the hash (SHA-3-256) of the script.
pub fn script_hash(serialized_script: &[u8]) -> [u8; 32] {
    let mut sha3_256 = Sha3_256::new();
    sha3_256.update(serialized_script);
    sha3_256.finalize().into()
}

pub fn deserialize_script(
    serialized_script: &[u8],
    deserializer_config: &DeserializerConfig,
) -> VMResult<CompiledScript> {
    CompiledScript::deserialize_with_config(serialized_script, deserializer_config).map_err(|err| {
        let msg = format!("[VM] deserializer for script returned error: {:?}", err);
        PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
            .with_message(msg)
            .finish(Location::Script)
    })
}

/// Represents storage which in addition to modules, also caches scripts. The
/// clients can implement this trait to ensure that even script dependency is
/// upgraded, the correct script is still returned. Scripts are cached based
/// on their hash.
#[delegatable_trait]
pub trait CodeStorage: ModuleStorage {
    /// Returns a deserialized script, either by directly deserializing it from the
    /// provided bytes (and caching it), or fetching it from the cache. Note that
    /// there are no guarantees that the returned script is verified. An error is
    /// returned if the deserialization fails.
    fn deserialize_and_cache_script(
        &self,
        serialized_script: &[u8],
    ) -> VMResult<Arc<CompiledScript>>;

    /// Returns a verified script. If not yet cached, verified from scratch and cached.
    /// An error is returned if script fails to deserialize or verify.
    fn verify_and_cache_script(&self, serialized_script: &[u8]) -> VMResult<Arc<Script>>;
}
