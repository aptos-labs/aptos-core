// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loaded_data::runtime_types::StructIdentifier;
use move_binary_format::errors::PartialVMResult;
use move_core_types::language_storage::{StructTag, TypeTag};
use parking_lot::RwLock;
use std::{collections::BTreeMap, fmt::Formatter, sync::Arc};

macro_rules! panic_error {
    ($msg:expr) => {{
        println!("[Error] panic detected: {}", $msg);
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
        )
        .with_message(format!("Panic detected: {:?}", $msg))
    }};
}

/// Represents a unique identifier for the struct name. Note that this index has no public
/// constructor - the only way to construct it is via [StructNameIndexMap].
#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructNameIndex(usize);

impl StructNameIndex {
    /// Creates a new index for testing purposes only. For production, indices must always be
    /// created by the data structure that uses them to intern struct names.
    #[cfg(any(test, feature = "testing"))]
    pub fn new(idx: usize) -> Self {
        Self(idx)
    }
}

impl std::fmt::Display for StructNameIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
struct IndexMap<T: Clone + Ord> {
    forward_map: BTreeMap<T, usize>,
    backward_map: Vec<Arc<T>>,
}

/// A data structure to cache struct identifiers (address, module name, struct name) and use
/// indices instead, to save on the memory consumption and avoid unnecessary cloning. It
/// guarantees that the same struct name identifier always corresponds to a unique index.
pub struct StructNameIndexMap(RwLock<IndexMap<StructIdentifier>>);

impl StructNameIndexMap {
    /// Returns an empty map with no entries.
    pub fn empty() -> Self {
        Self(RwLock::new(IndexMap {
            forward_map: BTreeMap::new(),
            backward_map: vec![],
        }))
    }

    /// Flushes the cached struct names and indices.
    pub fn flush(&self) {
        let mut index_map = self.0.write();
        index_map.backward_map.clear();
        index_map.forward_map.clear();
    }

    /// Maps the struct identifier into an index. If the identifier already exists returns the
    /// corresponding index. This function guarantees that for any struct identifiers A and B,
    /// if A == B, they have the same indices.
    pub fn struct_name_to_idx(
        &self,
        struct_name: &StructIdentifier,
    ) -> PartialVMResult<StructNameIndex> {
        {
            let index_map = self.0.read();
            if let Some(idx) = index_map.forward_map.get(struct_name) {
                return Ok(StructNameIndex(*idx));
            }
        }

        // Possibly need to insert, so make the copies outside of the lock.
        let forward_key = struct_name.clone();
        let backward_value = Arc::new(struct_name.clone());

        let idx = {
            let mut index_map = self.0.write();

            if let Some(idx) = index_map.forward_map.get(struct_name) {
                return Ok(StructNameIndex(*idx));
            }

            let idx = index_map.backward_map.len();
            index_map.backward_map.push(backward_value);
            index_map.forward_map.insert(forward_key, idx);
            idx
        };

        Ok(StructNameIndex(idx))
    }

    fn idx_to_struct_name_helper<'a>(
        index_map: &'a parking_lot::RwLockReadGuard<IndexMap<StructIdentifier>>,
        idx: StructNameIndex,
    ) -> PartialVMResult<&'a Arc<StructIdentifier>> {
        index_map.backward_map.get(idx.0).ok_or_else(|| {
            let msg = format!(
                "Index out of bounds when accessing struct name reference \
                     at index {}, backward map length: {}",
                idx.0,
                index_map.backward_map.len()
            );
            panic_error!(msg)
        })
    }

    /// Returns the reference of the struct name corresponding to the index. Here, we wrap the
    /// name into an [Arc] to ensure that the lock is released.
    pub fn idx_to_struct_name_ref(
        &self,
        idx: StructNameIndex,
    ) -> PartialVMResult<Arc<StructIdentifier>> {
        let index_map = self.0.read();
        Ok(Self::idx_to_struct_name_helper(&index_map, idx)?.clone())
    }

    /// Returns the clone of the struct name corresponding to the index. The clone ensures that the
    /// lock is released before the control returns to the caller.
    pub fn idx_to_struct_name(&self, idx: StructNameIndex) -> PartialVMResult<StructIdentifier> {
        let index_map = self.0.read();
        Ok(Self::idx_to_struct_name_helper(&index_map, idx)?
            .as_ref()
            .clone())
    }

    /// Returns the struct tag corresponding to the struct name and the provided type arguments.
    pub fn idx_to_struct_tag(
        &self,
        idx: StructNameIndex,
        ty_args: Vec<TypeTag>,
    ) -> PartialVMResult<StructTag> {
        let index_map = self.0.read();
        let struct_name = Self::idx_to_struct_name_helper(&index_map, idx)?.as_ref();
        Ok(StructTag {
            address: *struct_name.module.address(),
            module: struct_name.module.name().to_owned(),
            name: struct_name.name.clone(),
            type_args: ty_args,
        })
    }

    /// Returns the number of cached entries. Asserts that the number of cached indices is equal to
    /// the number of cached struct names.
    pub fn checked_len(&self) -> PartialVMResult<usize> {
        let (forward_map_len, backward_map_len) = {
            let index_map = self.0.read();
            (index_map.forward_map.len(), index_map.backward_map.len())
        };

        if forward_map_len != backward_map_len {
            let msg = format!(
                "Indexed map maps size mismatch: forward map has length {}, \
                 but backward map has length {}",
                forward_map_len, backward_map_len
            );
            return Err(panic_error!(msg));
        }

        Ok(forward_map_len)
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
    use claims::{assert_err, assert_ok};
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };
    use proptest::{arbitrary::any, collection::vec, proptest, strategy::Strategy};
    use std::sync::Arc;

    fn make_struct_name(module_name: &str, struct_name: &str) -> StructIdentifier {
        let module = ModuleId::new(AccountAddress::ONE, Identifier::new(module_name).unwrap());
        let name = Identifier::new(struct_name).unwrap();
        StructIdentifier { module, name }
    }

    #[test]
    fn test_index_map_must_contain_idx() {
        let struct_name_idx_map = StructNameIndexMap::empty();
        assert_err!(struct_name_idx_map.idx_to_struct_name_ref(StructNameIndex::new(0)));
    }

    #[test]
    fn test_index_map() {
        let struct_name_idx_map = StructNameIndexMap::empty();

        // First-time access.

        let foo = make_struct_name("foo", "Foo");
        let foo_idx = assert_ok!(struct_name_idx_map.struct_name_to_idx(&foo));
        assert_eq!(foo_idx.0, 0);

        let bar = make_struct_name("bar", "Bar");
        let bar_idx = assert_ok!(struct_name_idx_map.struct_name_to_idx(&bar));
        assert_eq!(bar_idx.0, 1);

        // Check that struct names actually correspond to indices.
        let returned_foo = assert_ok!(struct_name_idx_map.idx_to_struct_name_ref(foo_idx));
        assert_eq!(returned_foo.as_ref(), &foo);
        let returned_bar = assert_ok!(struct_name_idx_map.idx_to_struct_name_ref(bar_idx));
        assert_eq!(returned_bar.as_ref(), &bar);

        // Re-check indices on second access.
        let foo_idx = assert_ok!(struct_name_idx_map.struct_name_to_idx(&foo));
        assert_eq!(foo_idx.0, 0);
        let bar_idx = assert_ok!(struct_name_idx_map.struct_name_to_idx(&bar));
        assert_eq!(bar_idx.0, 1);

        let len = assert_ok!(struct_name_idx_map.checked_len());
        assert_eq!(len, 2);
    }

    fn struct_name_strategy() -> impl Strategy<Value = StructIdentifier> {
        let address = any::<AccountAddress>();
        let module_identifier = any::<Identifier>();
        let struct_identifier = any::<Identifier>();
        (address, module_identifier, struct_identifier).prop_map(|(a, m, name)| StructIdentifier {
            module: ModuleId::new(a, m),
            name,
        })
    }

    proptest! {
        #[test]
        fn test_index_map_concurrent_arbitrary_struct_names(struct_names in vec(struct_name_strategy(), 30..100),
        ) {
            let struct_name_idx_map = Arc::new(StructNameIndexMap::empty());

            // Each thread caches a struct name, and reads it to check if the cached result is
            // still the same.
            std::thread::scope(|s| {
                for struct_name in &struct_names {
                    s.spawn({
                        let struct_name_idx_map = struct_name_idx_map.clone();
                        move || {
                            let idx = assert_ok!(struct_name_idx_map.struct_name_to_idx(struct_name));
                            let actual_struct_name = assert_ok!(struct_name_idx_map.idx_to_struct_name_ref(idx));
                            assert_eq!(actual_struct_name.as_ref(), struct_name);
                        }
                    });
                }
            });
        }
    }

    #[test]
    fn test_index_map_concurrent_single_struct_name() {
        let struct_name_idx_map = Arc::new(StructNameIndexMap::empty());
        let struct_name = make_struct_name("foo", "Foo");

        // Each threads tries to cache the same struct name.
        std::thread::scope(|s| {
            for _ in 0..50 {
                s.spawn({
                    let struct_name_idx_map = struct_name_idx_map.clone();
                    let struct_name = struct_name.clone();
                    move || {
                        assert_ok!(struct_name_idx_map.struct_name_to_idx(&struct_name));
                    }
                });
            }
        });

        // Only a single struct name mast be cached!
        let len = assert_ok!(struct_name_idx_map.checked_len());
        assert_eq!(len, 1);
    }
}
