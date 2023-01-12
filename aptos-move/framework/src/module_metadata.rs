// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::AbortInfo;
use move_binary_format::CompiledModule;
use move_core_types::errmap::ErrorDescription;
use move_core_types::language_storage::ModuleId;
use move_core_types::metadata::Metadata;
use move_vm_runtime::move_vm::MoveVM;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The keys used to identify the metadata in the metadata section of the module bytecode.
/// This is more or less arbitrary, besides we should use some unique key to identify
/// Aptos specific metadata (`aptos::` here).
pub static APTOS_METADATA_KEY: Lazy<Vec<u8>> =
    Lazy::new(|| "aptos::metadata_v0".as_bytes().to_vec());
pub static APTOS_METADATA_KEY_V1: Lazy<Vec<u8>> =
    Lazy::new(|| "aptos::metadata_v1".as_bytes().to_vec());

/// Aptos specific metadata attached to the metadata section of file_format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeModuleMetadata {
    /// The error map containing the description of error reasons as grabbed from the source.
    /// These are typically only a few entries so no relevant size difference.
    pub error_map: BTreeMap<u64, ErrorDescription>,
}

/// V1 of Aptos specific metadata attached to the metadata section of file_format.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeModuleMetadataV1 {
    /// The error map containing the description of error reasons as grabbed from the source.
    /// These are typically only a few entries so no relevant size difference.
    pub error_map: BTreeMap<u64, ErrorDescription>,

    /// Attributes attached to structs.
    pub struct_attributes: BTreeMap<String, Vec<KnownAttribute>>,

    /// Attributes attached to functions, by definition index.
    pub fun_attributes: BTreeMap<String, Vec<KnownAttribute>>,
}

/// Enumeration of known attributes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KnownAttribute {
    kind: KnownAttributeKind,
    args: Vec<String>,
}

/// Enumeration of known attributes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KnownAttributeKind {
    ViewFunction = 1,
}

impl KnownAttribute {
    pub fn view_function() -> Self {
        Self {
            kind: KnownAttributeKind::ViewFunction,
            args: vec![],
        }
    }

    pub fn is_view_function(&self) -> bool {
        self.kind == KnownAttributeKind::ViewFunction
    }
}

/// Extract metadata from the VM, upgrading V0 to V1 representation as needed
pub fn get_vm_metadata(vm: &MoveVM, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1> {
    if let Some(data) = vm.get_module_metadata(module_id.clone(), &APTOS_METADATA_KEY_V1) {
        bcs::from_bytes::<RuntimeModuleMetadataV1>(&data.value).ok()
    } else if let Some(data) = vm.get_module_metadata(module_id, &APTOS_METADATA_KEY) {
        // Old format available, upgrade to new one on the fly
        let data_v0 = bcs::from_bytes::<RuntimeModuleMetadata>(&data.value).ok()?;
        Some(data_v0.upgrade())
    } else {
        None
    }
}

/// Extract metadata from the VM, legacy V0 format upgraded to V1
pub fn get_vm_metadata_v0(vm: &MoveVM, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1> {
    if let Some(data) = vm.get_module_metadata(module_id, &APTOS_METADATA_KEY) {
        let data_v0 = bcs::from_bytes::<RuntimeModuleMetadata>(&data.value).ok()?;
        Some(data_v0.upgrade())
    } else {
        None
    }
}

/// Extract metadata from a compiled module, upgrading V0 to V1 representation as needed.
pub fn get_module_metadata(module: &CompiledModule) -> Option<RuntimeModuleMetadataV1> {
    if let Some(data) = find_metadata(module, &APTOS_METADATA_KEY_V1) {
        bcs::from_bytes::<RuntimeModuleMetadataV1>(&data.value).ok()
    } else if let Some(data) = find_metadata(module, &APTOS_METADATA_KEY) {
        // Old format available, upgrade to new one on the fly
        let data_v0 = bcs::from_bytes::<RuntimeModuleMetadata>(&data.value).ok()?;
        Some(data_v0.upgrade())
    } else {
        None
    }
}

fn find_metadata<'a>(module: &'a CompiledModule, key: &[u8]) -> Option<&'a Metadata> {
    module.metadata.iter().find(|md| md.key == key)
}

impl RuntimeModuleMetadata {
    pub fn upgrade(self) -> RuntimeModuleMetadataV1 {
        RuntimeModuleMetadataV1 {
            error_map: self.error_map,
            ..RuntimeModuleMetadataV1::default()
        }
    }
}

impl RuntimeModuleMetadataV1 {
    pub fn is_empty(&self) -> bool {
        self.error_map.is_empty()
            && self.fun_attributes.is_empty()
            && self.struct_attributes.is_empty()
    }

    pub fn extract_abort_info(&self, code: u64) -> Option<AbortInfo> {
        self.error_map
            .get(&(code & 0xfff))
            .or_else(|| self.error_map.get(&code))
            .map(|descr| AbortInfo {
                reason_name: descr.code_name.clone(),
                description: descr.code_description.clone(),
            })
    }
}
