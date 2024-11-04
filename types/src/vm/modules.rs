// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_value::{StateValue, StateValueMetadata};
use bytes::Bytes;
use move_vm_types::{
    code::{WithBytes, WithHash},
    sha3_256,
};

/// Additional data stored alongside deserialized or verified modules.
pub struct AptosModuleExtension {
    /// Module's hash.
    hash: [u8; 32],
    /// The original state value associated with the module, when read from or written to storage.
    state_value: StateValue,
}

impl AptosModuleExtension {
    /// Creates new extension based on [StateValue].
    pub fn new(state_value: StateValue) -> Self {
        let hash = sha3_256(state_value.bytes());
        Self { hash, state_value }
    }

    /// Returns state value metadata stored in extension.
    pub fn state_value_metadata(&self) -> &StateValueMetadata {
        self.state_value.metadata()
    }
}

impl WithBytes for AptosModuleExtension {
    fn bytes(&self) -> &Bytes {
        self.state_value.bytes()
    }
}

impl WithHash for AptosModuleExtension {
    fn hash(&self) -> &[u8; 32] {
        &self.hash
    }
}

impl PartialEq for AptosModuleExtension {
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash) && self.state_value_metadata().eq(other.state_value_metadata())
    }
}

impl Eq for AptosModuleExtension {}
