// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::extended_checks::ResourceGroupScope;
use aptos_types::{
    on_chain_config::{FeatureFlag, Features, TimedFeatureFlag, TimedFeatures},
    transaction::AbortInfo,
};
use lru::LruCache;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        Ability, AbilitySet, CompiledScript, FunctionDefinition, FunctionHandle, IdentifierIndex,
        SignatureToken, StructDefinition, StructFieldInformation, StructHandle, TableIndex,
    },
    CompiledModule,
};
use move_core_types::{
    errmap::ErrorDescription,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
};
use move_model::metadata::{CompilationMetadata, COMPILATION_METADATA_KEY};
use move_vm_runtime::move_vm::MoveVM;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap, env, sync::Arc};
use thiserror::Error;

/// The minimal file format version from which the V1 metadata is supported
pub const METADATA_V1_MIN_FILE_FORMAT_VERSION: u32 = 6;

// For measuring complexity of a CompiledModule w.r.t. to metadata evaluation.
// This is for the size of types.
/// Cost of one node in a type.
const NODE_COST: usize = 10;
/// Cost of one character in the name of struct referred from a type node.
const IDENT_CHAR_COST: usize = 1;
/// Overall budget for module complexity, calibrated via tests
const COMPLEXITY_BUDGET: usize = 200000000;

/// The keys used to identify the metadata in the metadata section of the module bytecode.
/// This is more or less arbitrary, besides we should use some unique key to identify
/// Aptos specific metadata (`aptos::` here).
pub static APTOS_METADATA_KEY: &[u8] = "aptos::metadata_v0".as_bytes();
pub static APTOS_METADATA_KEY_V1: &[u8] = "aptos::metadata_v1".as_bytes();

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
    kind: u8,
    args: Vec<String>,
}

/// Enumeration of known attributes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KnownAttributeKind {
    // An older compiler placed view functions at 0. This was then published to
    // Testnet, and now we need to recognize this as a legacy index.
    LegacyViewFunction = 0,
    ViewFunction = 1,
    ResourceGroup = 2,
    ResourceGroupMember = 3,
    Event = 4,
    Randomness = 5,
}

impl KnownAttribute {
    pub fn view_function() -> Self {
        Self {
            kind: KnownAttributeKind::ViewFunction as u8,
            args: vec![],
        }
    }

    pub fn is_view_function(&self) -> bool {
        self.kind == (KnownAttributeKind::LegacyViewFunction as u8)
            || self.kind == (KnownAttributeKind::ViewFunction as u8)
    }

    pub fn resource_group(scope: ResourceGroupScope) -> Self {
        Self {
            kind: KnownAttributeKind::ResourceGroup as u8,
            args: vec![scope.as_str().to_string()],
        }
    }

    pub fn is_resource_group(&self) -> bool {
        self.kind == KnownAttributeKind::ResourceGroup as u8
    }

    pub fn get_resource_group(&self) -> Option<ResourceGroupScope> {
        if self.kind == KnownAttributeKind::ResourceGroup as u8 {
            self.args.first().and_then(|scope| str::parse(scope).ok())
        } else {
            None
        }
    }

    pub fn resource_group_member(container: String) -> Self {
        Self {
            kind: KnownAttributeKind::ResourceGroupMember as u8,
            args: vec![container],
        }
    }

    pub fn get_resource_group_member(&self) -> Option<StructTag> {
        if self.kind == KnownAttributeKind::ResourceGroupMember as u8 {
            self.args.first()?.parse().ok()
        } else {
            None
        }
    }

    pub fn is_resource_group_member(&self) -> bool {
        self.kind == KnownAttributeKind::ResourceGroupMember as u8
    }

    pub fn event() -> Self {
        Self {
            kind: KnownAttributeKind::Event as u8,
            args: vec![],
        }
    }

    pub fn is_event(&self) -> bool {
        self.kind == KnownAttributeKind::Event as u8
    }

    pub fn randomness(claimed_gas: Option<u64>) -> Self {
        Self {
            kind: KnownAttributeKind::Randomness as u8,
            args: if let Some(amount) = claimed_gas {
                vec![amount.to_string()]
            } else {
                vec![]
            },
        }
    }

    pub fn is_randomness(&self) -> bool {
        self.kind == KnownAttributeKind::Randomness as u8
    }

    pub fn try_as_randomness_annotation(&self) -> Option<RandomnessAnnotation> {
        if self.kind == KnownAttributeKind::Randomness as u8 {
            if let Some(arg) = self.args.first() {
                let max_gas = arg.parse::<u64>().ok();
                Some(RandomnessAnnotation::new(max_gas))
            } else {
                Some(RandomnessAnnotation::default())
            }
        } else {
            None
        }
    }
}

const METADATA_CACHE_SIZE: usize = 1024;

thread_local! {
    static V1_METADATA_CACHE: RefCell<LruCache<Vec<u8>, Option<Arc<RuntimeModuleMetadataV1>>>> = RefCell::new(LruCache::new(METADATA_CACHE_SIZE));

    static V0_METADATA_CACHE: RefCell<LruCache<Vec<u8>, Option<Arc<RuntimeModuleMetadataV1>>>> = RefCell::new(LruCache::new(METADATA_CACHE_SIZE));
}

/// Extract metadata from the VM, upgrading V0 to V1 representation as needed
pub fn get_metadata(md: &[Metadata]) -> Option<Arc<RuntimeModuleMetadataV1>> {
    if let Some(data) = md.iter().find(|md| md.key == APTOS_METADATA_KEY_V1) {
        V1_METADATA_CACHE.with(|ref_cell| {
            let mut cache = ref_cell.borrow_mut();
            if let Some(meta) = cache.get(&data.value) {
                meta.clone()
            } else {
                let meta = bcs::from_bytes::<RuntimeModuleMetadataV1>(&data.value)
                    .ok()
                    .map(Arc::new);
                cache.put(data.value.clone(), meta.clone());
                meta
            }
        })
    } else {
        get_metadata_v0(md)
    }
}

pub fn get_metadata_v0(md: &[Metadata]) -> Option<Arc<RuntimeModuleMetadataV1>> {
    if let Some(data) = md.iter().find(|md| md.key == APTOS_METADATA_KEY) {
        V0_METADATA_CACHE.with(|ref_cell| {
            let mut cache = ref_cell.borrow_mut();
            if let Some(meta) = cache.get(&data.value) {
                meta.clone()
            } else {
                let meta = bcs::from_bytes::<RuntimeModuleMetadata>(&data.value)
                    .ok()
                    .map(RuntimeModuleMetadata::upgrade)
                    .map(Arc::new);
                cache.put(data.value.clone(), meta.clone());
                meta
            }
        })
    } else {
        None
    }
}

/// Extract metadata from the VM, upgrading V0 to V1 representation as needed
pub fn get_vm_metadata(vm: &MoveVM, module_id: &ModuleId) -> Option<Arc<RuntimeModuleMetadataV1>> {
    vm.with_module_metadata(module_id, get_metadata)
}

/// Extract metadata from the VM, legacy V0 format upgraded to V1
pub fn get_vm_metadata_v0(
    vm: &MoveVM,
    module_id: &ModuleId,
) -> Option<Arc<RuntimeModuleMetadataV1>> {
    vm.with_module_metadata(module_id, get_metadata_v0)
}

/// Check if the metadata has unknown key/data types
pub fn check_metadata_format(
    module: &CompiledModule,
    features: &Features,
) -> Result<(), MalformedError> {
    let mut exist = false;
    let mut compilation_key_exist = false;
    for data in module.metadata.iter() {
        if data.key == *APTOS_METADATA_KEY || data.key == *APTOS_METADATA_KEY_V1 {
            if exist {
                return Err(MalformedError::DuplicateKey);
            }
            exist = true;

            if data.key == *APTOS_METADATA_KEY {
                bcs::from_bytes::<RuntimeModuleMetadata>(&data.value)
                    .map_err(|e| MalformedError::DeserializedError(data.key.clone(), e))?;
            } else if data.key == *APTOS_METADATA_KEY_V1 {
                bcs::from_bytes::<RuntimeModuleMetadataV1>(&data.value)
                    .map_err(|e| MalformedError::DeserializedError(data.key.clone(), e))?;
            }
        } else if features.is_enabled(FeatureFlag::REJECT_UNSTABLE_BYTECODE)
            && data.key == *COMPILATION_METADATA_KEY
        {
            if compilation_key_exist {
                return Err(MalformedError::DuplicateKey);
            }
            compilation_key_exist = true;
            bcs::from_bytes::<CompilationMetadata>(&data.value)
                .map_err(|e| MalformedError::DeserializedError(data.key.clone(), e))?;
        } else {
            return Err(MalformedError::UnknownKey(data.key.clone()));
        }
    }

    Ok(())
}

/// Extract metadata from a compiled module, upgrading V0 to V1 representation as needed.
pub fn get_metadata_from_compiled_module(
    module: &CompiledModule,
) -> Option<RuntimeModuleMetadataV1> {
    if let Some(data) = find_metadata(module, APTOS_METADATA_KEY_V1) {
        let mut metadata = bcs::from_bytes::<RuntimeModuleMetadataV1>(&data.value).ok();
        // Clear out metadata for v5, since it shouldn't have existed in the first place and isn't
        // being used. Note, this should have been gated in the verify module metadata.
        if module.version == 5 {
            if let Some(metadata) = metadata.as_mut() {
                metadata.struct_attributes.clear();
                metadata.fun_attributes.clear();
            }
        }
        metadata
    } else if let Some(data) = find_metadata(module, APTOS_METADATA_KEY) {
        // Old format available, upgrade to new one on the fly
        let data_v0 = bcs::from_bytes::<RuntimeModuleMetadata>(&data.value).ok()?;
        Some(data_v0.upgrade())
    } else {
        None
    }
}

/// Extract compilation metadata from a compiled module
pub fn get_compilation_metadata_from_compiled_module(
    module: &CompiledModule,
) -> Option<CompilationMetadata> {
    if let Some(data) = find_metadata(module, COMPILATION_METADATA_KEY) {
        bcs::from_bytes::<CompilationMetadata>(&data.value).ok()
    } else {
        None
    }
}

/// Extract compilation metadata from a compiled script
pub fn get_compilation_metadata_from_compiled_script(
    module: &CompiledScript,
) -> Option<CompilationMetadata> {
    if let Some(data) = find_metadata_in_script(module, COMPILATION_METADATA_KEY) {
        bcs::from_bytes::<CompilationMetadata>(&data.value).ok()
    } else {
        None
    }
}

// This is mostly a copy paste of the existing function
// get_metadata_from_compiled_module. In the API types there is a unifying trait for
// modules and scripts called Bytecode that could help eliminate this duplication,
// since all we need is a common way to access the metadata, but we'd have to move
// that trait outside of the API types and into somewhere more reasonable for the
// framework to access. There is currently no other trait that both CompiledModule
// and CompiledScript implement. This stands as a future improvement, if we end
// up needing more functions that work similarly for both of these types..
//
/// Extract metadata from a compiled module, upgrading V0 to V1 representation as needed.
pub fn get_metadata_from_compiled_script(
    script: &CompiledScript,
) -> Option<RuntimeModuleMetadataV1> {
    if let Some(data) = find_metadata_in_script(script, APTOS_METADATA_KEY_V1) {
        let mut metadata = bcs::from_bytes::<RuntimeModuleMetadataV1>(&data.value).ok();
        // Clear out metadata for v5, since it shouldn't have existed in the first place and isn't
        // being used. Note, this should have been gated in the verify module metadata.
        if script.version == 5 {
            if let Some(metadata) = metadata.as_mut() {
                metadata.struct_attributes.clear();
                metadata.fun_attributes.clear();
            }
        }
        metadata
    } else if let Some(data) = find_metadata_in_script(script, APTOS_METADATA_KEY) {
        // Old format available, upgrade to new one on the fly
        let data_v0 = bcs::from_bytes::<RuntimeModuleMetadata>(&data.value).ok()?;
        Some(data_v0.upgrade())
    } else {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum MetaDataValidationError {
    #[error(transparent)]
    Malformed(MalformedError),
    #[error(transparent)]
    InvalidAttribute(AttributeValidationError),
}

impl From<MalformedError> for MetaDataValidationError {
    fn from(value: MalformedError) -> Self {
        MetaDataValidationError::Malformed(value)
    }
}

impl From<AttributeValidationError> for MetaDataValidationError {
    fn from(value: AttributeValidationError) -> Self {
        MetaDataValidationError::InvalidAttribute(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum MalformedError {
    #[error("Unknown key found: {0:?}")]
    UnknownKey(Vec<u8>),
    #[error("Unable to deserialize value for {0:?}: {1}")]
    DeserializedError(Vec<u8>, bcs::Error),
    #[error("Duplicate key for metadata")]
    DuplicateKey,
    #[error("Module too complex")]
    ModuleTooComplex,
    #[error("Index out of range")]
    IndexOutOfRange,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("Unknown attribute ({}) for key: {}", self.attribute, self.key)]
pub struct AttributeValidationError {
    pub key: String,
    pub attribute: u8,
}

pub fn is_valid_unbiasable_function(
    functions: &BTreeMap<&IdentStr, (&FunctionHandle, &FunctionDefinition)>,
    fun: &str,
) -> Result<(), AttributeValidationError> {
    if let Ok(ident_fun) = Identifier::new(fun) {
        if let Some((_func_handle, func_def)) = functions.get(ident_fun.as_ident_str()) {
            if func_def.is_entry && !func_def.visibility.is_public() {
                return Ok(());
            }
        }
    }

    Err(AttributeValidationError {
        key: fun.to_string(),
        attribute: KnownAttributeKind::Randomness as u8,
    })
}

pub fn is_valid_view_function(
    module: &CompiledModule,
    functions: &BTreeMap<&IdentStr, (&FunctionHandle, &FunctionDefinition)>,
    fun: &str,
) -> Result<(), AttributeValidationError> {
    if let Ok(ident_fun) = Identifier::new(fun) {
        if let Some((func_handle, _func_def)) = functions.get(ident_fun.as_ident_str()) {
            let sig = module.signature_at(func_handle.return_);
            if !sig.0.is_empty() {
                return Ok(());
            }
        }
    }

    Err(AttributeValidationError {
        key: fun.to_string(),
        attribute: KnownAttributeKind::ViewFunction as u8,
    })
}

pub fn is_valid_resource_group(
    structs: &BTreeMap<&IdentStr, (&StructHandle, &StructDefinition)>,
    struct_: &str,
) -> Result<(), AttributeValidationError> {
    if let Ok(ident_struct) = Identifier::new(struct_) {
        if let Some((struct_handle, struct_def)) = structs.get(ident_struct.as_ident_str()) {
            let num_fields = match &struct_def.field_information {
                StructFieldInformation::Native | StructFieldInformation::DeclaredVariants(_) => 0,
                StructFieldInformation::Declared(fields) => fields.len(),
            };
            if struct_handle.abilities == AbilitySet::EMPTY
                && struct_handle.type_parameters.is_empty()
                && num_fields == 1
            {
                return Ok(());
            }
        }
    }

    Err(AttributeValidationError {
        key: struct_.to_string(),
        attribute: KnownAttributeKind::ViewFunction as u8,
    })
}

pub fn is_valid_resource_group_member(
    structs: &BTreeMap<&IdentStr, (&StructHandle, &StructDefinition)>,
    struct_: &str,
) -> Result<(), AttributeValidationError> {
    if let Ok(ident_struct) = Identifier::new(struct_) {
        if let Some((struct_handle, _struct_def)) = structs.get(ident_struct.as_ident_str()) {
            if struct_handle.abilities.has_ability(Ability::Key) {
                return Ok(());
            }
        }
    }

    Err(AttributeValidationError {
        key: struct_.to_string(),
        attribute: KnownAttributeKind::ViewFunction as u8,
    })
}

pub fn verify_module_metadata(
    module: &CompiledModule,
    features: &Features,
    timed_features: &TimedFeatures,
) -> Result<(), MetaDataValidationError> {
    if features.is_enabled(FeatureFlag::SAFER_METADATA)
        && timed_features.is_enabled(TimedFeatureFlag::ModuleComplexityCheck)
    {
        check_module_complexity(module)?;
    }

    if features.are_resource_groups_enabled() {
        check_metadata_format(module, features)?;
    }
    let metadata = if let Some(metadata) = get_metadata_from_compiled_module(module) {
        metadata
    } else {
        return Ok(());
    };

    let functions = module
        .function_defs
        .iter()
        .map(|func_def| {
            let func_handle = module.function_handle_at(func_def.function);
            let name = module.identifier_at(func_handle.name);
            (name, (func_handle, func_def))
        })
        .collect::<BTreeMap<_, _>>();

    for (fun, attrs) in &metadata.fun_attributes {
        for attr in attrs {
            if attr.is_view_function() {
                is_valid_view_function(module, &functions, fun)?;
            } else if attr.is_randomness() {
                is_valid_unbiasable_function(&functions, fun)?;
            } else {
                return Err(AttributeValidationError {
                    key: fun.clone(),
                    attribute: attr.kind,
                }
                .into());
            }
        }
    }

    let structs = module
        .struct_defs
        .iter()
        .map(|struct_def| {
            let struct_handle = module.struct_handle_at(struct_def.struct_handle);
            let name = module.identifier_at(struct_handle.name);
            (name, (struct_handle, struct_def))
        })
        .collect::<BTreeMap<_, _>>();

    for (struct_, attrs) in &metadata.struct_attributes {
        for attr in attrs {
            if features.are_resource_groups_enabled() {
                if attr.is_resource_group() && attr.get_resource_group().is_some() {
                    is_valid_resource_group(&structs, struct_)?;
                    continue;
                } else if attr.is_resource_group_member()
                    && attr.get_resource_group_member().is_some()
                {
                    is_valid_resource_group_member(&structs, struct_)?;
                    continue;
                }
            }
            if features.is_module_event_enabled() && attr.is_event() {
                continue;
            }
            return Err(AttributeValidationError {
                key: struct_.clone(),
                attribute: attr.kind,
            }
            .into());
        }
    }
    Ok(())
}

fn find_metadata<'a>(module: &'a CompiledModule, key: &[u8]) -> Option<&'a Metadata> {
    module.metadata.iter().find(|md| md.key == key)
}

fn find_metadata_in_script<'a>(script: &'a CompiledScript, key: &[u8]) -> Option<&'a Metadata> {
    script.metadata.iter().find(|md| md.key == key)
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

/// Checks the complexity of a module.
fn check_module_complexity(module: &CompiledModule) -> Result<(), MetaDataValidationError> {
    let mut meter: usize = 0;
    for sig in module.signatures() {
        for tok in &sig.0 {
            check_sigtok_complexity(module, &mut meter, tok)?
        }
    }
    for handle in module.function_handles() {
        check_ident_complexity(module, &mut meter, handle.name)?;
        for tok in &safe_get_table(module.signatures(), handle.parameters.0)?.0 {
            check_sigtok_complexity(module, &mut meter, tok)?
        }
        for tok in &safe_get_table(module.signatures(), handle.return_.0)?.0 {
            check_sigtok_complexity(module, &mut meter, tok)?
        }
    }
    for handle in module.struct_handles() {
        check_ident_complexity(module, &mut meter, handle.name)?;
    }
    for def in module.struct_defs() {
        match &def.field_information {
            StructFieldInformation::Native => {},
            StructFieldInformation::Declared(fields) => {
                for field in fields {
                    check_ident_complexity(module, &mut meter, field.name)?;
                    check_sigtok_complexity(module, &mut meter, &field.signature.0)?
                }
            },
            StructFieldInformation::DeclaredVariants(variants) => {
                for variant in variants {
                    check_ident_complexity(module, &mut meter, variant.name)?;
                    for field in &variant.fields {
                        check_ident_complexity(module, &mut meter, field.name)?;
                        check_sigtok_complexity(module, &mut meter, &field.signature.0)?
                    }
                }
            },
        }
    }
    for def in module.function_defs() {
        if let Some(unit) = &def.code {
            for tok in &safe_get_table(module.signatures(), unit.locals.0)?.0 {
                check_sigtok_complexity(module, &mut meter, tok)?
            }
        }
    }
    Ok(())
}

// Iterate -- without recursion -- through the nodes of a signature token. Any sub-nodes are
// dealt with via the iterator
fn check_sigtok_complexity(
    module: &CompiledModule,
    meter: &mut usize,
    tok: &SignatureToken,
) -> Result<(), MetaDataValidationError> {
    for node in tok.preorder_traversal() {
        // Count the node.
        *meter = meter.saturating_add(NODE_COST);
        match node {
            SignatureToken::Struct(idx) | SignatureToken::StructInstantiation(idx, _) => {
                let shandle = safe_get_table(module.struct_handles(), idx.0)?;
                let mhandle = safe_get_table(module.module_handles(), shandle.module.0)?;
                // Count identifier sizes
                check_ident_complexity(module, meter, shandle.name)?;
                check_ident_complexity(module, meter, mhandle.name)?
            },
            _ => {},
        }
        check_budget(*meter)?
    }
    Ok(())
}

fn check_ident_complexity(
    module: &CompiledModule,
    meter: &mut usize,
    idx: IdentifierIndex,
) -> Result<(), MetaDataValidationError> {
    *meter = meter.saturating_add(
        safe_get_table(module.identifiers(), idx.0)?
            .len()
            .saturating_mul(IDENT_CHAR_COST),
    );
    check_budget(*meter)
}

fn safe_get_table<A>(table: &[A], idx: TableIndex) -> Result<&A, MetaDataValidationError> {
    let idx = idx as usize;
    if idx < table.len() {
        Ok(&table[idx])
    } else {
        Err(MetaDataValidationError::Malformed(
            MalformedError::IndexOutOfRange,
        ))
    }
}

fn check_budget(meter: usize) -> Result<(), MetaDataValidationError> {
    let mut budget = COMPLEXITY_BUDGET;
    if cfg!(feature = "testing") {
        if let Ok(b) = env::var("METADATA_BUDGET_CAL") {
            budget = b.parse::<usize>().unwrap()
        }
    }
    if meter > budget {
        Err(MetaDataValidationError::Malformed(
            MalformedError::ModuleTooComplex,
        ))
    } else {
        Ok(())
    }
}

/// The randomness consuming options specified by developers for their entry function.
/// Examples: `#[randomness(max_gas = 99999)]`, `#[randomness]`.
#[derive(Default)]
pub struct RandomnessAnnotation {
    pub max_gas: Option<u64>,
}

impl RandomnessAnnotation {
    pub fn new(max_gas: Option<u64>) -> Self {
        Self { max_gas }
    }
}
