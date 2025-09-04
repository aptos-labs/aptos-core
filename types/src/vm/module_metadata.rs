// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    on_chain_config::{FeatureFlag, Features},
    transaction::{AbortInfo, EntryFunction},
    vm::code::CompiledCodeMetadata,
};
use lru::LruCache;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        FunctionDefinition, FunctionHandle, IdentifierIndex, SignatureToken, StructDefinition,
        StructFieldInformation, StructHandle, TableIndex,
    },
    CompiledModule,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    errmap::ErrorDescription,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
};
use move_model::{
    metadata::{CompilationMetadata, COMPILATION_METADATA_KEY},
    model::StructEnv,
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap, env, num::NonZeroUsize, str::FromStr, sync::Arc};
use thiserror::Error;

pub mod prelude {
    pub use crate::vm::module_metadata::{
        get_compilation_metadata, get_metadata_from_compiled_code, RuntimeModuleMetadataV1,
    };
}

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
/// Velor specific metadata (`velor::` here).
pub static VELOR_METADATA_KEY: &[u8] = "velor::metadata_v0".as_bytes();
pub static VELOR_METADATA_KEY_V1: &[u8] = "velor::metadata_v1".as_bytes();

/// Velor specific metadata attached to the metadata section of file_format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeModuleMetadata {
    /// The error map containing the description of error reasons as grabbed from the source.
    /// These are typically only a few entries so no relevant size difference.
    pub error_map: BTreeMap<u64, ErrorDescription>,
}

/// V1 of Velor specific metadata attached to the metadata section of file_format.
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

const METADATA_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(1024).unwrap();

thread_local! {
    static V1_METADATA_CACHE: RefCell<LruCache<Vec<u8>, Option<Arc<RuntimeModuleMetadataV1>>>> = RefCell::new(LruCache::new(METADATA_CACHE_SIZE));

    static V0_METADATA_CACHE: RefCell<LruCache<Vec<u8>, Option<Arc<RuntimeModuleMetadataV1>>>> = RefCell::new(LruCache::new(METADATA_CACHE_SIZE));
}

/// Extract metadata from the VM, upgrading V0 to V1 representation as needed
pub fn get_metadata(md: &[Metadata]) -> Option<Arc<RuntimeModuleMetadataV1>> {
    if let Some(data) = find_metadata(md, VELOR_METADATA_KEY_V1) {
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
    } else if let Some(data) = find_metadata(md, VELOR_METADATA_KEY) {
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

/// For the specified entry function, tries to find randomness attribute in its metadata. If it
/// does not exist, [None] is returned.
pub fn get_randomness_annotation_for_entry_function(
    entry_func: &EntryFunction,
    metadata: &[Metadata],
) -> Option<RandomnessAnnotation> {
    get_metadata(metadata).and_then(|metadata| {
        metadata
            .fun_attributes
            .get(entry_func.function().as_str())
            .map(|attrs| {
                attrs
                    .iter()
                    .filter_map(KnownAttribute::try_as_randomness_annotation)
                    .next()
            })
            .unwrap_or(None)
    })
}

/// Check if the metadata has unknown key/data types
fn check_metadata_format(module: &CompiledModule) -> Result<(), MalformedError> {
    let mut exist = false;
    let mut compilation_key_exist = false;
    for data in module.metadata.iter() {
        if data.key == *VELOR_METADATA_KEY || data.key == *VELOR_METADATA_KEY_V1 {
            if exist {
                return Err(MalformedError::DuplicateKey);
            }
            exist = true;

            if data.key == *VELOR_METADATA_KEY {
                bcs::from_bytes::<RuntimeModuleMetadata>(&data.value)
                    .map_err(|e| MalformedError::DeserializedError(data.key.clone(), e))?;
            } else if data.key == *VELOR_METADATA_KEY_V1 {
                bcs::from_bytes::<RuntimeModuleMetadataV1>(&data.value)
                    .map_err(|e| MalformedError::DeserializedError(data.key.clone(), e))?;
            }
        } else if data.key == *COMPILATION_METADATA_KEY {
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

/// Extract metadata from a compiled module or a script, upgrading V0 to V1 representation as
/// needed.
pub fn get_metadata_from_compiled_code(
    code: &impl CompiledCodeMetadata,
) -> Option<RuntimeModuleMetadataV1> {
    if let Some(data) = find_metadata(code.metadata(), VELOR_METADATA_KEY_V1) {
        let mut metadata = bcs::from_bytes::<RuntimeModuleMetadataV1>(&data.value).ok();
        // Clear out metadata for v5, since it shouldn't have existed in the first place and isn't
        // being used. Note, this should have been gated in the verify module metadata.
        if code.version() == 5 {
            if let Some(metadata) = metadata.as_mut() {
                metadata.struct_attributes.clear();
                metadata.fun_attributes.clear();
            }
        }
        metadata
    } else if let Some(data) = find_metadata(code.metadata(), VELOR_METADATA_KEY) {
        // Old format available, upgrade to new one on the fly
        let data_v0 = bcs::from_bytes::<RuntimeModuleMetadata>(&data.value).ok()?;
        Some(data_v0.upgrade())
    } else {
        None
    }
}

/// Extract compilation metadata from a compiled module or script.
pub fn get_compilation_metadata(code: &impl CompiledCodeMetadata) -> Option<CompilationMetadata> {
    if let Some(data) = find_metadata(code.metadata(), COMPILATION_METADATA_KEY) {
        bcs::from_bytes::<CompilationMetadata>(&data.value).ok()
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

pub fn verify_module_metadata_for_module_publishing(
    module: &CompiledModule,
    features: &Features,
) -> Result<(), MetaDataValidationError> {
    if features.is_enabled(FeatureFlag::SAFER_METADATA) {
        check_module_complexity(module)?;
    }

    if features.are_resource_groups_enabled() {
        check_metadata_format(module)?;
    }
    let metadata = if let Some(metadata) = get_metadata_from_compiled_code(module) {
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

fn find_metadata<'a>(metadata: &'a [Metadata], key: &[u8]) -> Option<&'a Metadata> {
    metadata.iter().find(|md| md.key == key)
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

#[derive(Debug, Eq, PartialEq)]
pub enum ResourceGroupScope {
    Global,
    Address,
    Module,
}

impl ResourceGroupScope {
    pub fn is_less_strict(&self, other: &ResourceGroupScope) -> bool {
        match self {
            ResourceGroupScope::Global => other != self,
            ResourceGroupScope::Address => other == &ResourceGroupScope::Module,
            ResourceGroupScope::Module => false,
        }
    }

    pub fn are_equal_envs(&self, resource: &StructEnv, group: &StructEnv) -> bool {
        match self {
            ResourceGroupScope::Global => true,
            ResourceGroupScope::Address => {
                resource.module_env.get_name().addr() == group.module_env.get_name().addr()
            },
            ResourceGroupScope::Module => {
                resource.module_env.get_name() == group.module_env.get_name()
            },
        }
    }

    pub fn are_equal_module_ids(&self, resource: &ModuleId, group: &ModuleId) -> bool {
        match self {
            ResourceGroupScope::Global => true,
            ResourceGroupScope::Address => resource.address() == group.address(),
            ResourceGroupScope::Module => resource == group,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceGroupScope::Global => "global",
            ResourceGroupScope::Address => "address",
            ResourceGroupScope::Module => "module_",
        }
    }
}

impl FromStr for ResourceGroupScope {
    type Err = ResourceGroupScopeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "global" => Ok(ResourceGroupScope::Global),
            "address" => Ok(ResourceGroupScope::Address),
            "module_" => Ok(ResourceGroupScope::Module),
            _ => Err(ResourceGroupScopeError(s.to_string())),
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid resource group scope: {0}")]
pub struct ResourceGroupScopeError(String);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_less_strict() {
        let less_strict = [
            (ResourceGroupScope::Global, ResourceGroupScope::Address),
            (ResourceGroupScope::Global, ResourceGroupScope::Module),
            (ResourceGroupScope::Address, ResourceGroupScope::Module),
        ];
        for (scope, other_scope) in less_strict {
            assert!(scope.is_less_strict(&other_scope));
        }

        let more_or_as_strict = [
            (ResourceGroupScope::Global, ResourceGroupScope::Global),
            (ResourceGroupScope::Address, ResourceGroupScope::Global),
            (ResourceGroupScope::Address, ResourceGroupScope::Address),
            (ResourceGroupScope::Module, ResourceGroupScope::Global),
            (ResourceGroupScope::Module, ResourceGroupScope::Address),
            (ResourceGroupScope::Module, ResourceGroupScope::Module),
        ];
        for (scope, other_scope) in more_or_as_strict {
            assert!(!scope.is_less_strict(&other_scope));
        }
    }

    #[test]
    fn test_are_equal_module_ids() {
        let id = |s: &str| -> ModuleId { ModuleId::from_str(s).unwrap() };

        let global_scope = ResourceGroupScope::Global;
        assert!(global_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x1::foo")));
        assert!(global_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x1::bar")));
        assert!(global_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x2::foo")));
        assert!(global_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x2::bar")));

        let address_scope = ResourceGroupScope::Address;
        assert!(address_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x1::foo")));
        assert!(address_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x1::bar")));
        assert!(!address_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x2::foo")));
        assert!(!address_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x2::bar")));

        let module_scope = ResourceGroupScope::Module;
        assert!(module_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x1::foo")));
        assert!(!module_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x1::bar")));
        assert!(!module_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x2::foo")));
        assert!(!module_scope.are_equal_module_ids(&id("0x1::foo"), &id("0x2::bar")));
    }
}
