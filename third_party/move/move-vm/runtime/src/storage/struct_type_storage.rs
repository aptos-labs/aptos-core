// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::ModuleStorageAdapter,
    storage::{loader::LoaderV2, verifier::Verifier},
    ModuleStorage,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{identifier::IdentStr, language_storage::ModuleId};
use move_vm_types::loaded_data::runtime_types::StructType;
use std::sync::Arc;

/// We use this trait so that we can fetch struct types differently for V1 and V2 loaders.
pub(crate) trait StructTypeStorage {
    /// Returns the struct type defined in the given module, for the specified name.
    fn fetch_struct_ty(
        &self,
        module_id: &ModuleId,
        struct_name: &IdentStr,
    ) -> PartialVMResult<Arc<StructType>>;
}

pub(crate) struct LoaderV2StructTypeStorage<'a, V: Clone + Verifier> {
    pub(crate) loader: &'a LoaderV2<V>,
    pub(crate) module_storage: &'a dyn ModuleStorage,
}

impl<'a, V: Clone + Verifier> StructTypeStorage for LoaderV2StructTypeStorage<'a, V> {
    fn fetch_struct_ty(
        &self,
        module_id: &ModuleId,
        struct_name: &IdentStr,
    ) -> PartialVMResult<Arc<StructType>> {
        self.loader.load_struct_ty(
            self.module_storage,
            module_id.address(),
            module_id.name(),
            struct_name,
        )
    }
}

pub(crate) struct LoaderV1StructTypeStorage<'a> {
    pub(crate) module_store: &'a ModuleStorageAdapter,
}

impl<'a> StructTypeStorage for LoaderV1StructTypeStorage<'a> {
    fn fetch_struct_ty(
        &self,
        module_id: &ModuleId,
        struct_name: &IdentStr,
    ) -> PartialVMResult<Arc<StructType>> {
        self.module_store
            .get_struct_type_by_identifier(struct_name, module_id)
    }
}
