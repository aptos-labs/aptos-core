// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_core_types::value::LayoutTag;
use move_vm_types::loaded_data::runtime_types::StructIdentifier;
use std::collections::HashMap;

#[derive(Clone)]
pub struct NativeTypes {
    types: HashMap<StructIdentifier, LayoutTag>,
}

impl NativeTypes {
    pub fn empty() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    pub fn new(types: impl IntoIterator<Item = (StructIdentifier, LayoutTag)>) -> Self {
        Self {
            types: HashMap::from_iter(types),
        }
    }

    pub fn get_native_type_id(&self, idx: &StructIdentifier) -> Option<LayoutTag> {
        self.types.get(idx).cloned()
    }
}
