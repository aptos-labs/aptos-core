// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{loader::Script, ModuleStorage};
use move_binary_format::{errors::VMResult, file_format::CompiledScript};
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

/// Returns the hash (SHA-3-256) of the script.
pub fn script_hash(serialized_script: &[u8]) -> [u8; 32] {
    let mut sha3_256 = Sha3_256::new();
    sha3_256.update(serialized_script);
    sha3_256.finalize().into()
}

/// Represents storage which in addition to modules, also caches scripts. The
/// clients can implement this trait to ensure that even script dependency is
/// upgraded, the correct script is still returned. Scripts are cached based
/// on their hash.
pub trait CodeStorage: ModuleStorage {
    /// Returns a deserialized script, either by directly deserializing it from the
    /// provided bytes, or fetching it from the storage (if it has been cached). Note
    /// that there are no guarantees that the returned script is verified. An error
    /// is returned if the deserialization fails.
    fn fetch_deserialized_script(&self, serialized_script: &[u8]) -> VMResult<Arc<CompiledScript>>;

    /// Returns a verified script. The script can either be cached, or deserialized and/or
    /// verified from scratch. An error is returned if script fails to deserialize or verify.
    fn fetch_verified_script(&self, serialized_script: &[u8]) -> VMResult<Arc<Script>>;
}
