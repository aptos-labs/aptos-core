// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::AbortInfo;
use move_binary_format::{normalized::Function, CompiledModule};
use move_core_types::{
    errmap::ErrorDescription, identifier::Identifier, language_storage::ModuleId,
    metadata::Metadata,
};
use move_vm_runtime::move_vm::MoveVM;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// The minimal file format version from which the V1 metadata is supported
pub const METADATA_V1_MIN_FILE_FORMAT_VERSION: u32 = 6;

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

/// Enumeration of potentially known attributes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KnownAttribute {
    kind: u16,
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
            kind: KnownAttributeKind::ViewFunction as u16,
            args: vec![],
        }
    }

    pub fn is_view_function(&self) -> bool {
        self.kind == (KnownAttributeKind::ViewFunction as u16)
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

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("Unknown attribute ({}) for key: {}", self.attribute, self.key)]
pub struct MetadataValidationError {
    pub key: String,
    pub attribute: u16,
}

pub fn is_valid_view_function(
    functions: &BTreeMap<Identifier, Function>,
    fun: &str,
) -> Result<(), MetadataValidationError> {
    if let Ok(ident_fun) = Identifier::new(fun) {
        if let Some(mod_fun) = functions.get(&ident_fun) {
            if !mod_fun.return_.is_empty() {
                return Ok(());
            }
        }
    }

    Err(MetadataValidationError {
        key: fun.to_string(),
        attribute: KnownAttributeKind::ViewFunction as u16,
    })
}

pub fn verify_module_metadata(module: &CompiledModule) -> Result<(), MetadataValidationError> {
    let metadata = if let Some(metadata) = get_module_metadata(module) {
        metadata
    } else {
        return Ok(());
    };

    let functions = module
        .function_defs
        .iter()
        .map(|func_def| Function::new(module, func_def))
        .collect::<BTreeMap<_, _>>();
    for (fun, attrs) in &metadata.fun_attributes {
        for attr in attrs {
            if attr.is_view_function() {
                is_valid_view_function(&functions, fun)?
            } else {
                return Err(MetadataValidationError {
                    key: fun.clone(),
                    attribute: attr.kind,
                });
            }
        }
    }
    for (struct_, attrs) in &metadata.struct_attributes {
        if let Some(attr) = attrs.iter().next() {
            return Err(MetadataValidationError {
                key: struct_.clone(),
                attribute: attr.kind,
            });
        }
    }
    Ok(())
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
    pub fn downgrade(self) -> RuntimeModuleMetadata {
        RuntimeModuleMetadata {
            error_map: self.error_map,
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
            .get(&(code & 0xFFF))
            .or_else(|| self.error_map.get(&code))
            .map(|descr| AbortInfo {
                reason_name: descr.code_name.clone(),
                description: descr.code_description.clone(),
            })
    }
}
