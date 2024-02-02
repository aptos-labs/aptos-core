// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_vm_types::loaded_data::runtime_types::StructIdentifier;
use std::collections::BTreeMap;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NativeTypeID(u64);

impl NativeTypeID {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone)]
pub struct NativeTypes {
    // check which ds is faster for these keys
    types: BTreeMap<StructIdentifier, NativeTypeID>,
}

impl NativeTypes {
    pub fn empty() -> Self {
        Self {
            types: BTreeMap::new(),
        }
    }

    pub fn new(types: impl IntoIterator<Item = (StructIdentifier, NativeTypeID)>) -> Self {
        Self {
            // TODO: Allow failures? on same IDs?
            types: BTreeMap::from_iter(types),
        }
    }

    pub fn get_native_type_id(&self, idx: &StructIdentifier) -> Option<NativeTypeID> {
        self.types.get(idx).copied()
    }
}
