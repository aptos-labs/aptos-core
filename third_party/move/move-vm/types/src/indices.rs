// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loaded_data::runtime_types::StructIdentifier;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use parking_lot::RwLock;
use std::{collections::BTreeMap, fmt::Formatter};

struct IndexMapData<T: Clone + Ord> {
    forward_map: BTreeMap<T, u64>,
    backward_map: Vec<T>,
}

struct IndexMap<T: Clone + Ord> {
    data: RwLock<IndexMapData<T>>,
}

impl<T> IndexMap<T>
where
    T: Clone + Ord,
{
    fn empty() -> Self {
        let index_map = IndexMapData {
            forward_map: BTreeMap::new(),
            backward_map: Vec::new(),
        };
        Self {
            data: RwLock::new(index_map),
        }
    }

    fn value_to_idx(&self, value: &T) -> u64 {
        {
            let index_map = self.data.read();
            if let Some(idx) = index_map.forward_map.get(value) {
                return *idx;
            }
        }

        // Possibly need to insert, so make the copies BEFORE acquiring the write lock.
        let forward_key = value.clone();
        let backward_value = value.clone();

        let idx = {
            let mut index_map = self.data.write();

            if let Some(idx) = index_map.forward_map.get(value) {
                return *idx;
            }

            let idx = index_map.backward_map.len() as u64;
            index_map.backward_map.push(backward_value);
            index_map.forward_map.insert(forward_key, idx);
            idx
        };

        idx
    }
}

const MODULE_OFFSET: u64 = 32;
const FUNCTION_OR_STRUCT_OFFSET: u64 = 48;

const ADDRESS_MASK: u64 = 0x0000_0000_FFFF_FFFF;
const MODULE_MASK: u64 = 0x0000_FFFF_0000_0000;
#[allow(dead_code)]
const FUNCTION_OR_STRUCT_MASK: u64 = 0xFFFF_0000_0000_0000;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct StructIdx(u64);

impl StructIdx {
    pub fn new(idx: u64) -> Self {
        Self(idx)
    }

    #[allow(dead_code)]
    pub fn module_idx(&self) -> ModuleIdx {
        ModuleIdx(self.0 & (MODULE_MASK | ADDRESS_MASK))
    }

    #[allow(dead_code)]
    pub fn address_idx(&self) -> AddressIdx {
        AddressIdx(self.0 & ADDRESS_MASK)
    }
}

impl std::fmt::Display for StructIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FunctionIdx(u64);

impl FunctionIdx {
    #[allow(dead_code)]
    pub fn new(idx: u64) -> Self {
        Self(idx)
    }

    #[allow(dead_code)]
    pub fn module_idx(&self) -> ModuleIdx {
        ModuleIdx(self.0 & (MODULE_MASK | ADDRESS_MASK))
    }

    #[allow(dead_code)]
    pub fn address_idx(&self) -> AddressIdx {
        AddressIdx(self.0 & ADDRESS_MASK)
    }
}

impl std::fmt::Display for FunctionIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ModuleIdx(u64);

impl ModuleIdx {
    #[allow(dead_code)]
    pub fn new(idx: u64) -> Self {
        Self(idx)
    }

    #[allow(dead_code)]
    pub fn address_idx(&self) -> AddressIdx {
        AddressIdx(self.0 & ADDRESS_MASK)
    }
}

impl std::fmt::Display for ModuleIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct AddressIdx(u64);

impl AddressIdx {
    pub fn new(idx: u64) -> Self {
        Self(idx)
    }
}

impl std::fmt::Display for AddressIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(dead_code)]
pub struct IndexMapManager {
    address_index_map: IndexMap<AccountAddress>,
    module_name_index_map: IndexMap<Identifier>,
    function_name_index_map: IndexMap<Identifier>,
    struct_name_index_map: IndexMap<Identifier>,
}

impl IndexMapManager {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            address_index_map: IndexMap::empty(),
            module_name_index_map: IndexMap::empty(),
            function_name_index_map: IndexMap::empty(),
            struct_name_index_map: IndexMap::empty(),
        }
    }

    #[allow(dead_code)]
    pub fn module_idx_from_id(&self, module_id: &ModuleId) -> ModuleIdx {
        self.module_idx(module_id.address(), &module_id.name)
    }

    #[allow(dead_code)]
    pub fn module_idx(&self, address: &AccountAddress, module_name: &Identifier) -> ModuleIdx {
        let address_idx = self.address_index_map.value_to_idx(address);
        let module_name_idx = self.module_name_index_map.value_to_idx(module_name);

        ModuleIdx((module_name_idx << MODULE_OFFSET) | address_idx)
    }

    pub fn struct_idx(
        &self,
        address: &AccountAddress,
        module_name: &Identifier,
        struct_name: &Identifier,
    ) -> StructIdx {
        let address_idx = self.address_index_map.value_to_idx(address);
        let module_name_idx = self.module_name_index_map.value_to_idx(module_name);
        let struct_name_idx = self.struct_name_index_map.value_to_idx(struct_name);

        StructIdx(
            (struct_name_idx << FUNCTION_OR_STRUCT_OFFSET)
                | (module_name_idx << MODULE_OFFSET)
                | address_idx,
        )
    }

    #[allow(dead_code)]
    pub fn struct_idx_from_struct_tag(&self, struct_tag: &StructTag) -> StructIdx {
        self.struct_idx(&struct_tag.address, &struct_tag.module, &struct_tag.name)
    }

    // FIXME
    pub fn struct_id_from_idx(&self, idx: &StructIdx) -> StructIdentifier {
        let address =
            self.address_index_map.data.read().backward_map[(idx.0 & ADDRESS_MASK) as usize];
        let module_name = self.module_name_index_map.data.read().backward_map
            [((idx.0 & MODULE_MASK) >> MODULE_OFFSET) as usize]
            .to_owned();
        let name = self.struct_name_index_map.data.read().backward_map
            [((idx.0 & FUNCTION_OR_STRUCT_MASK) >> FUNCTION_OR_STRUCT_OFFSET) as usize]
            .to_owned();

        StructIdentifier {
            module: ModuleId::new(address, module_name),
            name,
        }
    }

    pub fn function_name_from_idx(&self, idx: &FunctionIdx) -> Identifier {
        self.function_name_index_map.data.read().backward_map
            [((idx.0 & FUNCTION_OR_STRUCT_MASK) >> FUNCTION_OR_STRUCT_OFFSET) as usize]
            .to_owned()
    }

    pub fn struct_name_from_idx(&self, idx: &StructIdx) -> Identifier {
        self.struct_name_index_map.data.read().backward_map
            [((idx.0 & FUNCTION_OR_STRUCT_MASK) >> FUNCTION_OR_STRUCT_OFFSET) as usize]
            .to_owned()
    }

    // FIXME
    pub fn module_id_from_idx(&self, idx: &FunctionIdx) -> ModuleId {
        let address =
            self.address_index_map.data.read().backward_map[(idx.0 & ADDRESS_MASK) as usize];
        let module = self.module_name_index_map.data.read().backward_map
            [((idx.0 & MODULE_MASK) >> MODULE_OFFSET) as usize]
            .to_owned();
        ModuleId::new(address, module)
    }

    // FIXME
    pub fn module_addr_name_from_module_idx(
        &self,
        idx: &ModuleIdx,
    ) -> (AccountAddress, Identifier) {
        let address =
            self.address_index_map.data.read().backward_map[(idx.0 & ADDRESS_MASK) as usize];
        let module = self.module_name_index_map.data.read().backward_map
            [((idx.0 & MODULE_MASK) >> MODULE_OFFSET) as usize]
            .to_owned();
        (address, module)
    }

    // FIXME
    pub fn module_id_from_struct_idx(&self, idx: &StructIdx) -> ModuleId {
        let address =
            self.address_index_map.data.read().backward_map[(idx.0 & ADDRESS_MASK) as usize];
        let module = self.module_name_index_map.data.read().backward_map
            [((idx.0 & MODULE_MASK) >> MODULE_OFFSET) as usize]
            .to_owned();
        ModuleId::new(address, module)
    }

    // FIXME
    pub fn struct_tag_from_idx(&self, idx: &StructIdx, type_args: Vec<TypeTag>) -> StructTag {
        let address =
            self.address_index_map.data.read().backward_map[(idx.0 & ADDRESS_MASK) as usize];
        let module = self.module_name_index_map.data.read().backward_map
            [((idx.0 & MODULE_MASK) >> MODULE_OFFSET) as usize]
            .to_owned();
        let name = self.struct_name_index_map.data.read().backward_map
            [((idx.0 & FUNCTION_OR_STRUCT_MASK) >> FUNCTION_OR_STRUCT_OFFSET) as usize]
            .to_owned();

        StructTag {
            address,
            module,
            name,
            type_args,
        }
    }

    #[allow(dead_code)]
    pub fn function_idx(
        &self,
        address: &AccountAddress,
        module_name: &Identifier,
        function_name: &Identifier,
    ) -> FunctionIdx {
        let address_idx = self.address_index_map.value_to_idx(address);
        let module_name_idx = self.module_name_index_map.value_to_idx(module_name);
        let function_name_idx = self.function_name_index_map.value_to_idx(function_name);

        FunctionIdx(
            (function_name_idx << FUNCTION_OR_STRUCT_OFFSET)
                | (module_name_idx << MODULE_OFFSET)
                | address_idx,
        )
    }
}
