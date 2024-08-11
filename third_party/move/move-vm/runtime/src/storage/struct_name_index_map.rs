// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_vm_types::loaded_data::runtime_types::{StructIdentifier, StructNameIndex};
use parking_lot::RwLock;
use std::collections::BTreeMap;

#[derive(Clone)]
struct IndexMap<T: Clone + Ord> {
    forward_map: BTreeMap<T, usize>,
    backward_map: Vec<T>,
}

/// A data structure to cache struct identifiers (address, module name, struct name) and
/// use indices instead, to save on the memory consumption and avoid unnecessary cloning.
pub(crate) struct StructNameIndexMap(RwLock<IndexMap<StructIdentifier>>);

impl StructNameIndexMap {
    pub(crate) fn empty() -> Self {
        Self(RwLock::new(IndexMap {
            forward_map: BTreeMap::new(),
            backward_map: vec![],
        }))
    }

    pub(crate) fn struct_name_to_idx(&self, struct_name: StructIdentifier) -> StructNameIndex {
        let mut index_map = self.0.write();
        if let Some(idx) = index_map.forward_map.get(&struct_name) {
            return StructNameIndex(*idx);
        }
        let idx = index_map.backward_map.len();
        index_map.forward_map.insert(struct_name.clone(), idx);
        index_map.backward_map.push(struct_name);
        StructNameIndex(idx)
    }

    pub(crate) fn idx_to_struct_name(&self, idx: StructNameIndex) -> StructIdentifier {
        // TODO(loader_v2): Avoid cloning here, changed for now to avoid deadlocks.
        self.0.read().backward_map[idx.0].clone()
    }
}

impl Clone for StructNameIndexMap {
    fn clone(&self) -> Self {
        Self(RwLock::new(self.0.read().clone()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };

    fn make_struct_name(module_name: &str, struct_name: &str) -> StructIdentifier {
        let module = ModuleId::new(AccountAddress::ONE, Identifier::new(module_name).unwrap());
        let name = Identifier::new(struct_name).unwrap();
        StructIdentifier { module, name }
    }

    #[test]
    #[should_panic]
    fn test_index_map_must_contain_idx() {
        let struct_name_idx_map = StructNameIndexMap::empty();
        let _ = struct_name_idx_map.idx_to_struct_name(StructNameIndex(0));
    }

    #[test]
    fn test_index_map() {
        let struct_name_idx_map = StructNameIndexMap::empty();

        // First-time access.

        let foo = make_struct_name("foo", "Foo");
        let foo_idx = struct_name_idx_map.struct_name_to_idx(foo.clone());
        assert_eq!(foo_idx.0, 0);

        let bar = make_struct_name("bar", "Bar");
        let bar_idx = struct_name_idx_map.struct_name_to_idx(bar.clone());
        assert_eq!(bar_idx.0, 1);

        // Check that struct names actually correspond to indices.
        let returned_foo = struct_name_idx_map.idx_to_struct_name(foo_idx);
        assert_eq!(&returned_foo, &foo);
        let returned_bar = struct_name_idx_map.idx_to_struct_name(bar_idx);
        assert_eq!(&returned_bar, &bar);

        // Re-check indices on second access.
        let foo_idx = struct_name_idx_map.struct_name_to_idx(foo);
        assert_eq!(foo_idx.0, 0);
        let bar_idx = struct_name_idx_map.struct_name_to_idx(bar);
        assert_eq!(bar_idx.0, 1);
    }
}
