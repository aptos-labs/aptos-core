// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_value::{StateValue, StateValueMetadata};
use bytes::Bytes;
use move_vm_types::{
    code::{WithBytes, WithHash},
    sha3_256,
};

/// Additional data stored alongside deserialized or verified modules.
pub struct VelorModuleExtension {
    /// Serialized representation of the module.
    bytes: Bytes,
    /// Module's hash.
    hash: [u8; 32],
    /// The state value metadata associated with the module, when read from or
    /// written to storage.
    state_value_metadata: StateValueMetadata,
}

impl VelorModuleExtension {
    /// Creates new extension based on [StateValue].
    pub fn new(state_value: StateValue) -> Self {
        let (state_value_metadata, bytes) = state_value.unpack();
        let hash = sha3_256(&bytes);
        Self {
            bytes,
            hash,
            state_value_metadata,
        }
    }

    /// Returns the state value metadata stored in extension.
    pub fn state_value_metadata(&self) -> &StateValueMetadata {
        &self.state_value_metadata
    }
}

impl WithBytes for VelorModuleExtension {
    fn bytes(&self) -> &Bytes {
        &self.bytes
    }
}

impl WithHash for VelorModuleExtension {
    fn hash(&self) -> &[u8; 32] {
        &self.hash
    }
}
