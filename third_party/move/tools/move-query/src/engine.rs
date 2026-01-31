// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! QueryEngine implementation
//!
//! Wraps GlobalEnv and provides query methods for inspecting Move packages.

use crate::types::*;
use anyhow::Result;
use aptos_framework::build_model;
use move_binary_format::file_format::Visibility as ModelVisibility;
use move_core_types::ability::Ability as MoveAbility;
use move_model::{
    metadata::LanguageVersion,
    model::{FunctionEnv, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, StructEnv},
    ty::{PrimitiveType, ReferenceKind, Type as ModelType},
};
use std::str::FromStr;
use move_package::source_package::manifest_parser::parse_move_manifest_from_file;
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// Options for building the query engine
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Named addresses to use during compilation
    pub named_addresses: BTreeMap<String, aptos_types::account_address::AccountAddress>,
    /// Development mode
    pub dev: bool,
    /// Bytecode version
    pub bytecode_version: Option<u32>,
    /// Language version
    pub language_version: Option<String>,
    /// Compiler experiments
    pub experiments: Vec<String>,
}

/// Result type for query operations
pub type QueryResult<T> = Result<T, QueryError>;

/// Error type for query operations
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Failed to build package: {0}")]
    BuildFailed(String),
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Function not found: {0}::{1}")]
    FunctionNotFound(String, String),
    #[error("Struct not found: {0}::{1}")]
    StructNotFound(String, String),
    #[error("Constant not found: {0}::{1}")]
    ConstantNotFound(String, String),
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl QueryError {
    pub fn code(&self) -> QueryErrorCode {
        match self {
            QueryError::BuildFailed(_) => QueryErrorCode::BuildFailed,
            QueryError::ModuleNotFound(_) => QueryErrorCode::ModuleNotFound,
            QueryError::FunctionNotFound(_, _) => QueryErrorCode::FunctionNotFound,
            QueryError::StructNotFound(_, _) => QueryErrorCode::StructNotFound,
            QueryError::ConstantNotFound(_, _) => QueryErrorCode::ConstantNotFound,
            QueryError::InvalidPattern(_) => QueryErrorCode::InvalidPattern,
            QueryError::InvalidArgument(_) => QueryErrorCode::InvalidArgument,
            QueryError::InternalError(_) => QueryErrorCode::InternalError,
        }
    }
}

/// Main query engine wrapping GlobalEnv
pub struct QueryEngine {
    env: GlobalEnv,
    package_path: PathBuf,
    options: QueryOptions,
}

impl QueryEngine {
    /// Build package and create query engine
    pub fn new(package_path: PathBuf, options: QueryOptions) -> QueryResult<Self> {
        let env = Self::build_env(&package_path, &options)?;
        Ok(Self {
            env,
            package_path,
            options,
        })
    }

    fn build_env(package_path: &Path, options: &QueryOptions) -> QueryResult<GlobalEnv> {
        // Parse language version if provided
        let language_version = options
            .language_version
            .as_ref()
            .and_then(|s| LanguageVersion::from_str(s).ok());

        build_model(
            options.dev,
            package_path,
            options.named_addresses.clone(),
            None, // target_filter
            options.bytecode_version,
            None, // compiler_version (use default)
            language_version,
            false, // skip_attribute_checks (use compiler default)
            BTreeSet::new(), // known_attributes (use compiler default)
            options.experiments.clone(),
        )
        .map_err(|e| QueryError::BuildFailed(format!("{:#}", e)))
    }

    /// Rebuild the model (after file changes)
    pub fn rebuild(&mut self) -> QueryResult<()> {
        self.env = Self::build_env(&self.package_path, &self.options)?;
        Ok(())
    }

    /// Get the package path
    pub fn package_path(&self) -> &Path {
        &self.package_path
    }

    // ========================================
    // Hierarchical Queries (L0 → L1 → L2)
    // ========================================

    /// L0: Get package info with module summaries
    pub fn get_package(&self) -> QueryResult<PackageInfo> {
        let manifest_path = self.package_path.join("Move.toml");

        // Parse manifest for package name and dependencies
        let (name, dependencies, named_addresses) =
            if let Ok(manifest) = parse_move_manifest_from_file(&manifest_path) {
                let name = manifest.package.name.to_string();
                let deps: Vec<String> = manifest
                    .dependencies
                    .into_iter()
                    .map(|(name, _)| name.to_string())
                    .collect();
                let addrs: BTreeMap<String, String> = manifest
                    .addresses
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(name, addr)| {
                        let addr_str = addr
                            .map(|a| format!("0x{}", a))
                            .unwrap_or_else(|| "_".to_string());
                        (name.to_string(), addr_str)
                    })
                    .collect();
                (name, deps, addrs)
            } else {
                (
                    self.package_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    vec![],
                    BTreeMap::new(),
                )
            };

        let modules: Vec<ModuleSummary> = self
            .iter_modules()
            .map(|m| self.module_to_summary(&m))
            .collect();

        Ok(PackageInfo {
            name,
            path: self.package_path.display().to_string(),
            manifest_path: manifest_path.display().to_string(),
            named_addresses,
            dependencies,
            modules,
        })
    }

    /// L1: Get module info with function/struct/constant summaries
    pub fn get_module(&self, name: &str) -> QueryResult<ModuleInfo> {
        let module_env = self.find_module(name)?;
        Ok(self.module_to_info(&module_env))
    }

    /// L2: Get full function details
    pub fn get_function(&self, module: &str, name: &str) -> QueryResult<FunctionInfo> {
        let module_env = self.find_module(module)?;
        let func_env = self.find_function(&module_env, name)?;
        Ok(self.function_to_info(&func_env))
    }

    /// L2: Get full struct details
    pub fn get_struct(&self, module: &str, name: &str) -> QueryResult<StructInfo> {
        let module_env = self.find_module(module)?;
        let struct_env = self.find_struct(&module_env, name)?;
        Ok(self.struct_to_info(&struct_env))
    }

    /// L2: Get full constant details
    pub fn get_constant(&self, module: &str, name: &str) -> QueryResult<ConstantInfo> {
        let module_env = self.find_module(module)?;
        let const_env = self.find_constant(&module_env, name)?;
        Ok(self.constant_to_info(&const_env))
    }

    // ========================================
    // Source Code Query
    // ========================================

    /// Get source code for an item
    pub fn get_source(
        &self,
        module: &str,
        name: Option<&str>,
        item_type: Option<ItemType>,
    ) -> QueryResult<SourceInfo> {
        let module_env = self.find_module(module)?;

        match (name, item_type) {
            // Module source
            (None, None) | (None, Some(ItemType::Module)) => {
                let loc = module_env.get_loc();
                let location = self.loc_to_location(&loc);
                let source = self.extract_source(&loc)?;
                Ok(SourceInfo {
                    path: module_env.get_full_name_str(),
                    item_type: ItemType::Module,
                    location,
                    source,
                })
            }
            // Function source
            (Some(name), Some(ItemType::Function)) | (Some(name), None) => {
                // Try function first, then struct, then constant
                if let Ok(func_env) = self.find_function(&module_env, name) {
                    let loc = func_env.get_loc();
                    let location = self.loc_to_location(&loc);
                    let source = self.extract_source(&loc)?;
                    Ok(SourceInfo {
                        path: format!("{}::{}", module_env.get_full_name_str(), name),
                        item_type: ItemType::Function,
                        location,
                        source,
                    })
                } else if let Ok(struct_env) = self.find_struct(&module_env, name) {
                    let loc = struct_env.get_loc();
                    let location = self.loc_to_location(&loc);
                    let source = self.extract_source(&loc)?;
                    Ok(SourceInfo {
                        path: format!("{}::{}", module_env.get_full_name_str(), name),
                        item_type: ItemType::Struct,
                        location,
                        source,
                    })
                } else if let Ok(const_env) = self.find_constant(&module_env, name) {
                    let loc = const_env.get_loc();
                    let location = self.loc_to_location(&loc);
                    let source = self.extract_source(&loc)?;
                    Ok(SourceInfo {
                        path: format!("{}::{}", module_env.get_full_name_str(), name),
                        item_type: ItemType::Constant,
                        location,
                        source,
                    })
                } else {
                    Err(QueryError::InvalidArgument(format!(
                        "Item '{}' not found in module '{}'",
                        name, module
                    )))
                }
            }
            (Some(name), Some(ItemType::Struct)) => {
                let struct_env = self.find_struct(&module_env, name)?;
                let loc = struct_env.get_loc();
                let location = self.loc_to_location(&loc);
                let source = self.extract_source(&loc)?;
                Ok(SourceInfo {
                    path: format!("{}::{}", module_env.get_full_name_str(), name),
                    item_type: ItemType::Struct,
                    location,
                    source,
                })
            }
            (Some(name), Some(ItemType::Constant)) => {
                let const_env = self.find_constant(&module_env, name)?;
                let loc = const_env.get_loc();
                let location = self.loc_to_location(&loc);
                let source = self.extract_source(&loc)?;
                Ok(SourceInfo {
                    path: format!("{}::{}", module_env.get_full_name_str(), name),
                    item_type: ItemType::Constant,
                    location,
                    source,
                })
            }
            (Some(_), Some(ItemType::Module)) => {
                // Module type doesn't need a name, return the whole module
                let loc = module_env.get_loc();
                let location = self.loc_to_location(&loc);
                let source = self.extract_source(&loc)?;
                Ok(SourceInfo {
                    path: module_env.get_full_name_str(),
                    item_type: ItemType::Module,
                    location,
                    source,
                })
            }
            (None, Some(ItemType::Function))
            | (None, Some(ItemType::Struct))
            | (None, Some(ItemType::Constant)) => Err(QueryError::InvalidArgument(
                "Name is required for function, struct, or constant types".to_string(),
            )),
        }
    }

    // ========================================
    // Cross-Cutting Queries
    // ========================================

    /// All entry functions across all modules
    pub fn list_entry_points(&self) -> Vec<FunctionSummary> {
        self.iter_modules()
            .flat_map(|m| {
                m.get_functions()
                    .filter(|f| f.is_entry())
                    .map(|f| self.function_to_summary(&f))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// All resource types (structs with key ability)
    pub fn list_resources(&self) -> Vec<StructSummary> {
        self.iter_modules()
            .flat_map(|m| {
                m.get_structs()
                    .filter(|s| s.has_memory())
                    .map(|s| self.struct_to_summary(&s))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Search by pattern across all levels
    pub fn search(
        &self,
        pattern: &str,
        level: Option<SearchLevel>,
        limit: usize,
    ) -> QueryResult<Vec<SearchResult>> {
        let regex =
            Regex::new(pattern).map_err(|e| QueryError::InvalidPattern(e.to_string()))?;

        let mut results = Vec::new();

        for module_env in self.iter_modules() {
            if results.len() >= limit {
                break;
            }

            let module_name = module_env.get_full_name_str();

            // Search modules
            if level.is_none() || level == Some(SearchLevel::Module) {
                if regex.is_match(&module_name) {
                    results.push(SearchResult {
                        level: SearchLevel::Module,
                        path: module_name.clone(),
                        location: self.loc_to_location(&module_env.get_loc()),
                    });
                }
            }

            // Search functions
            if level.is_none() || level == Some(SearchLevel::Function) {
                for func_env in module_env.get_functions() {
                    if results.len() >= limit {
                        break;
                    }
                    let func_name = func_env.get_name_str().to_string();
                    let full_name = format!("{}::{}", module_name, func_name);
                    if regex.is_match(&func_name) || regex.is_match(&full_name) {
                        results.push(SearchResult {
                            level: SearchLevel::Function,
                            path: full_name,
                            location: self.loc_to_location(&func_env.get_loc()),
                        });
                    }
                }
            }

            // Search structs
            if level.is_none() || level == Some(SearchLevel::Struct) {
                for struct_env in module_env.get_structs() {
                    if results.len() >= limit {
                        break;
                    }
                    let struct_name = struct_env.get_name().display(self.env.symbol_pool()).to_string();
                    let full_name = format!("{}::{}", module_name, struct_name);
                    if regex.is_match(&struct_name) || regex.is_match(&full_name) {
                        results.push(SearchResult {
                            level: SearchLevel::Struct,
                            path: full_name,
                            location: self.loc_to_location(&struct_env.get_loc()),
                        });
                    }
                }
            }

            // Search constants
            if level.is_none() || level == Some(SearchLevel::Constant) {
                for const_env in module_env.get_named_constants() {
                    if results.len() >= limit {
                        break;
                    }
                    let const_name = const_env.get_name().display(self.env.symbol_pool()).to_string();
                    let full_name = format!("{}::{}", module_name, const_name);
                    if regex.is_match(&const_name) || regex.is_match(&full_name) {
                        results.push(SearchResult {
                            level: SearchLevel::Constant,
                            path: full_name,
                            location: self.loc_to_location(&const_env.get_loc()),
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    // ========================================
    // Relationship Queries
    // ========================================

    /// Get call graph for a function
    pub fn get_call_graph(&self, function: &str) -> QueryResult<CallGraph> {
        // Parse function path (module::function)
        let parts: Vec<&str> = function.rsplitn(2, "::").collect();
        if parts.len() != 2 {
            return Err(QueryError::InvalidArgument(format!(
                "Function path must be in format 'module::function', got: {}",
                function
            )));
        }
        let func_name = parts[0];
        let module_name = parts[1];

        let module_env = self.find_module(module_name)?;
        let func_env = self.find_function(&module_env, func_name)?;

        let calls: Vec<String> = func_env
            .get_called_functions()
            .map(|set| {
                set.iter()
                    .map(|qid| {
                        let callee = self.env.get_function(*qid);
                        format!(
                            "{}::{}",
                            callee.module_env.get_full_name_str(),
                            callee.get_name_str()
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let called_by: Vec<String> = func_env
            .get_calling_functions()
            .map(|set| {
                set.iter()
                    .map(|qid| {
                        let caller = self.env.get_function(*qid);
                        format!(
                            "{}::{}",
                            caller.module_env.get_full_name_str(),
                            caller.get_name_str()
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(CallGraph {
            function: format!("{}::{}", module_env.get_full_name_str(), func_name),
            calls,
            called_by,
        })
    }

    /// Get module dependencies
    pub fn get_deps(&self, module: &str) -> QueryResult<ModuleDeps> {
        let module_env = self.find_module(module)?;
        let module_id = module_env.get_id();

        let uses: Vec<String> = module_env
            .get_used_modules(false)
            .into_iter()
            .map(|id| self.env.get_module(id).get_full_name_str())
            .collect();

        let used_by: Vec<String> = module_env
            .get_using_modules(false)
            .into_iter()
            .filter(|id| *id != module_id)
            .map(|id| self.env.get_module(id).get_full_name_str())
            .collect();

        let friends: Vec<String> = module_env
            .get_friend_modules()
            .into_iter()
            .map(|id| self.env.get_module(id).get_full_name_str())
            .collect();

        Ok(ModuleDeps {
            module: module_env.get_full_name_str(),
            uses,
            used_by,
            friends,
        })
    }

    // ========================================
    // Helper Methods - Finding Items
    // ========================================

    fn iter_modules(&self) -> impl Iterator<Item = ModuleEnv<'_>> {
        // Only iterate over target modules (not dependencies)
        self.env.get_modules().filter(|m| m.is_target())
    }

    fn find_module(&self, name: &str) -> QueryResult<ModuleEnv<'_>> {
        // Try to find by full name first (e.g., "0x1::coin")
        for module in self.env.get_modules() {
            if module.get_full_name_str() == name {
                return Ok(module);
            }
        }

        // Try to find by simple name
        let symbol = self.env.symbol_pool().make(name);
        if let Some(module) = self.env.find_module_by_name(symbol) {
            return Ok(module);
        }

        // Try partial match on the simple name
        for module in self.env.get_modules() {
            let simple_name = module
                .get_name()
                .name()
                .display(self.env.symbol_pool())
                .to_string();
            if simple_name == name {
                return Ok(module);
            }
        }

        Err(QueryError::ModuleNotFound(name.to_string()))
    }

    fn find_function<'a>(
        &'a self,
        module: &'a ModuleEnv<'a>,
        name: &str,
    ) -> QueryResult<FunctionEnv<'a>> {
        let symbol = self.env.symbol_pool().make(name);
        module
            .find_function(symbol)
            .ok_or_else(|| QueryError::FunctionNotFound(module.get_full_name_str(), name.to_string()))
    }

    fn find_struct<'a>(
        &'a self,
        module: &'a ModuleEnv<'a>,
        name: &str,
    ) -> QueryResult<StructEnv<'a>> {
        let symbol = self.env.symbol_pool().make(name);
        module
            .find_struct(symbol)
            .ok_or_else(|| QueryError::StructNotFound(module.get_full_name_str(), name.to_string()))
    }

    fn find_constant<'a>(
        &'a self,
        module: &'a ModuleEnv<'a>,
        name: &str,
    ) -> QueryResult<NamedConstantEnv<'a>> {
        let symbol = self.env.symbol_pool().make(name);
        module
            .find_named_constant(symbol)
            .ok_or_else(|| QueryError::ConstantNotFound(module.get_full_name_str(), name.to_string()))
    }

    // ========================================
    // Helper Methods - Conversions
    // ========================================

    fn loc_to_location(&self, loc: &Loc) -> Location {
        if let Some((file, cs_loc)) = self.env.get_file_and_location(loc) {
            // Get end location
            let end_loc = self
                .env
                .get_location(&loc.at_end())
                .unwrap_or_else(|| cs_loc.clone());

            Location {
                file,
                // LineIndex and ColumnIndex are 0-indexed, convert to 1-indexed
                start_line: cs_loc.line.0 + 1,
                start_column: cs_loc.column.0 + 1,
                end_line: end_loc.line.0 + 1,
                end_column: end_loc.column.0 + 1,
            }
        } else {
            Location::default()
        }
    }

    fn extract_source(&self, loc: &Loc) -> QueryResult<String> {
        self.env
            .get_source(loc)
            .map(|s| s.to_string())
            .map_err(|e| QueryError::InternalError(format!("Failed to extract source: {}", e)))
    }

    fn visibility_to_query(&self, vis: ModelVisibility) -> Visibility {
        match vis {
            ModelVisibility::Public => Visibility::Public,
            ModelVisibility::Friend => Visibility::PublicFriend,
            ModelVisibility::Private => Visibility::Private,
        }
    }

    fn ability_to_query(&self, ability: MoveAbility) -> Ability {
        match ability {
            MoveAbility::Copy => Ability::Copy,
            MoveAbility::Drop => Ability::Drop,
            MoveAbility::Store => Ability::Store,
            MoveAbility::Key => Ability::Key,
        }
    }

    fn type_to_query(&self, ty: &ModelType, type_params: &[String]) -> Type {
        match ty {
            ModelType::Primitive(p) => Type::Primitive {
                name: self.primitive_to_string(p),
            },
            ModelType::Vector(inner) => Type::Vector {
                element: Box::new(self.type_to_query(inner, type_params)),
            },
            ModelType::Struct(module_id, struct_id, type_args) => {
                let module = self.env.get_module(*module_id);
                let struct_env = module.get_struct(*struct_id);
                let type_args: Vec<Type> = type_args
                    .iter()
                    .map(|t| self.type_to_query(t, type_params))
                    .collect();
                Type::Struct {
                    module: module.get_full_name_str(),
                    name: struct_env
                        .get_name()
                        .display(self.env.symbol_pool())
                        .to_string(),
                    type_args,
                }
            }
            ModelType::Reference(kind, inner) => Type::Reference {
                is_mutable: *kind == ReferenceKind::Mutable,
                referred: Box::new(self.type_to_query(inner, type_params)),
            },
            ModelType::TypeParameter(idx) => {
                let name = type_params
                    .get(*idx as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("T{}", idx));
                Type::TypeParameter { name }
            }
            ModelType::Tuple(elems) => {
                if elems.is_empty() {
                    Type::Unit
                } else {
                    Type::Tuple {
                        elements: elems
                            .iter()
                            .map(|t| self.type_to_query(t, type_params))
                            .collect(),
                    }
                }
            }
            _ => Type::Primitive {
                name: "unknown".to_string(),
            },
        }
    }

    fn primitive_to_string(&self, p: &PrimitiveType) -> String {
        match p {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::U8 => "u8".to_string(),
            PrimitiveType::U16 => "u16".to_string(),
            PrimitiveType::U32 => "u32".to_string(),
            PrimitiveType::U64 => "u64".to_string(),
            PrimitiveType::U128 => "u128".to_string(),
            PrimitiveType::U256 => "u256".to_string(),
            PrimitiveType::I8 => "i8".to_string(),
            PrimitiveType::I16 => "i16".to_string(),
            PrimitiveType::I32 => "i32".to_string(),
            PrimitiveType::I64 => "i64".to_string(),
            PrimitiveType::I128 => "i128".to_string(),
            PrimitiveType::I256 => "i256".to_string(),
            PrimitiveType::Address => "address".to_string(),
            PrimitiveType::Signer => "signer".to_string(),
            PrimitiveType::Num => "num".to_string(),
            PrimitiveType::Range => "range".to_string(),
            PrimitiveType::EventStore => "event_store".to_string(),
        }
    }

    fn address_to_string(&self, addr: &move_model::ast::Address) -> String {
        use move_model::ast::Address;
        match addr {
            Address::Numerical(account_addr) => format!("0x{}", account_addr),
            Address::Symbolic(sym) => sym.display(self.env.symbol_pool()).to_string(),
        }
    }

    // ========================================
    // Helper Methods - Summary Builders
    // ========================================

    fn module_to_summary(&self, module: &ModuleEnv<'_>) -> ModuleSummary {
        ModuleSummary {
            id: module.get_full_name_str(),
            name: module
                .get_name()
                .name()
                .display(self.env.symbol_pool())
                .to_string(),
            is_target: module.is_target(),
            location: self.loc_to_location(&module.get_loc()),
            function_count: module.get_function_count(),
            struct_count: module.get_struct_count(),
            constant_count: module.get_named_constant_count(),
        }
    }

    fn module_to_info(&self, module: &ModuleEnv<'_>) -> ModuleInfo {
        let functions: Vec<FunctionSummary> = module
            .get_functions()
            .map(|f| self.function_to_summary(&f))
            .collect();

        let structs: Vec<StructSummary> = module
            .get_structs()
            .map(|s| self.struct_to_summary(&s))
            .collect();

        let constants: Vec<ConstantSummary> = module
            .get_named_constants()
            .map(|c| self.constant_to_summary(&c))
            .collect();

        let friends: Vec<String> = module
            .get_friend_modules()
            .into_iter()
            .map(|id| self.env.get_module(id).get_full_name_str())
            .collect();

        let uses: Vec<String> = module
            .get_used_modules(false)
            .into_iter()
            .map(|id| self.env.get_module(id).get_full_name_str())
            .collect();

        let used_by: Vec<String> = module
            .get_using_modules(false)
            .into_iter()
            .filter(|id| *id != module.get_id())
            .map(|id| self.env.get_module(id).get_full_name_str())
            .collect();

        ModuleInfo {
            id: module.get_full_name_str(),
            address: self.address_to_string(module.self_address()),
            name: module
                .get_name()
                .name()
                .display(self.env.symbol_pool())
                .to_string(),
            doc: module.get_doc().to_string(),
            location: self.loc_to_location(&module.get_loc()),
            is_target: module.is_target(),
            friends,
            uses,
            used_by,
            functions,
            structs,
            constants,
        }
    }

    fn function_to_summary(&self, func: &FunctionEnv<'_>) -> FunctionSummary {
        let type_params: Vec<String> = func
            .get_type_parameters()
            .iter()
            .map(|tp| tp.0.display(self.env.symbol_pool()).to_string())
            .collect();

        let params: Vec<String> = func
            .get_parameters()
            .iter()
            .map(|p| {
                let ty = self.type_to_query(&p.1, &type_params);
                format!("{}: {}", p.0.display(self.env.symbol_pool()), ty.to_simple_string())
            })
            .collect();

        let return_type = self.type_to_query(&func.get_result_type(), &type_params);

        let signature = if type_params.is_empty() {
            format!(
                "{}({}): {}",
                func.get_name_str(),
                params.join(", "),
                return_type.to_simple_string()
            )
        } else {
            format!(
                "{}<{}>({}): {}",
                func.get_name_str(),
                type_params.join(", "),
                params.join(", "),
                return_type.to_simple_string()
            )
        };

        FunctionSummary {
            name: func.get_name_str().to_string(),
            module: func.module_env.get_full_name_str(),
            visibility: self.visibility_to_query(func.visibility()),
            is_entry: func.is_entry(),
            is_view: func.is_pragma_true("view", || false),
            location: self.loc_to_location(&func.get_loc()),
            type_parameter_count: func.get_type_parameter_count(),
            parameter_count: func.get_parameter_count(),
            signature,
        }
    }

    fn function_to_info(&self, func: &FunctionEnv<'_>) -> FunctionInfo {
        let type_params: Vec<String> = func
            .get_type_parameters()
            .iter()
            .map(|tp| tp.0.display(self.env.symbol_pool()).to_string())
            .collect();

        let type_parameters: Vec<TypeParameter> = func
            .get_type_parameters()
            .iter()
            .map(|tp| TypeParameter {
                name: tp.0.display(self.env.symbol_pool()).to_string(),
                is_phantom: tp.1.is_phantom,
                constraints: tp.1.abilities.into_iter().map(|a| self.ability_to_query(a)).collect(),
            })
            .collect();

        let parameters: Vec<Parameter> = func
            .get_parameters()
            .iter()
            .map(|p| Parameter {
                name: p.0.display(self.env.symbol_pool()).to_string(),
                type_: self.type_to_query(&p.1, &type_params),
            })
            .collect();

        let return_type = self.type_to_query(&func.get_result_type(), &type_params);

        let acquires: Vec<String> = func
            .get_acquires_global_resources()
            .map(|sids| {
                sids.iter()
                    .map(|sid| {
                        let struct_env = func.module_env.get_struct(*sid);
                        struct_env
                            .get_name()
                            .display(self.env.symbol_pool())
                            .to_string()
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Extract attributes
        let attributes: Vec<String> = func
            .get_attributes()
            .iter()
            .map(|attr| format!("{:?}", attr))
            .collect();

        FunctionInfo {
            module: func.module_env.get_full_name_str(),
            name: func.get_name_str().to_string(),
            doc: func.get_doc().to_string(),
            location: self.loc_to_location(&func.get_loc()),
            visibility: self.visibility_to_query(func.visibility()),
            is_entry: func.is_entry(),
            is_view: func.is_pragma_true("view", || false),
            is_inline: func.is_inline(),
            is_native: func.is_native(),
            attributes,
            type_parameters,
            parameters,
            return_type,
            acquires,
            spec: None, // TODO: Extract spec if needed
        }
    }

    fn struct_to_summary(&self, struct_env: &StructEnv<'_>) -> StructSummary {
        let abilities: Vec<Ability> = struct_env
            .get_abilities()
            .into_iter()
            .map(|a| self.ability_to_query(a))
            .collect();

        StructSummary {
            name: struct_env
                .get_name()
                .display(self.env.symbol_pool())
                .to_string(),
            module: struct_env.module_env.get_full_name_str(),
            abilities,
            is_resource: struct_env.has_memory(),
            location: self.loc_to_location(&struct_env.get_loc()),
            type_parameter_count: struct_env.get_type_parameters().len(),
            field_count: struct_env.get_field_count(),
        }
    }

    fn struct_to_info(&self, struct_env: &StructEnv<'_>) -> StructInfo {
        let type_params: Vec<String> = struct_env
            .get_type_parameters()
            .iter()
            .map(|tp| tp.0.display(self.env.symbol_pool()).to_string())
            .collect();

        let type_parameters: Vec<TypeParameter> = struct_env
            .get_type_parameters()
            .iter()
            .map(|tp| TypeParameter {
                name: tp.0.display(self.env.symbol_pool()).to_string(),
                is_phantom: tp.1.is_phantom,
                constraints: tp.1.abilities.into_iter().map(|a| self.ability_to_query(a)).collect(),
            })
            .collect();

        let abilities: Vec<Ability> = struct_env
            .get_abilities()
            .into_iter()
            .map(|a| self.ability_to_query(a))
            .collect();

        let fields: Option<Vec<Field>> = if struct_env.is_native() {
            None
        } else {
            Some(
                struct_env
                    .get_fields()
                    .map(|f| Field {
                        name: f.get_name().display(self.env.symbol_pool()).to_string(),
                        type_: self.type_to_query(&f.get_type(), &type_params),
                        doc: f.get_doc().to_string(),
                    })
                    .collect(),
            )
        };

        // Check for variants (enums)
        let variants: Option<Vec<Variant>> = if struct_env.has_variants() {
            Some(
                struct_env
                    .get_variants()
                    .map(|v| {
                        let variant_fields: Vec<Field> = struct_env
                            .get_fields_of_variant(v)
                            .map(|f| Field {
                                name: f.get_name().display(self.env.symbol_pool()).to_string(),
                                type_: self.type_to_query(&f.get_type(), &type_params),
                                doc: f.get_doc().to_string(),
                            })
                            .collect();
                        Variant {
                            name: v.display(self.env.symbol_pool()).to_string(),
                            fields: variant_fields,
                            doc: String::new(), // TODO: Get variant doc if available
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };

        // Extract attributes
        let attributes: Vec<String> = struct_env
            .get_attributes()
            .iter()
            .map(|attr| format!("{:?}", attr))
            .collect();

        StructInfo {
            module: struct_env.module_env.get_full_name_str(),
            name: struct_env
                .get_name()
                .display(self.env.symbol_pool())
                .to_string(),
            doc: struct_env.get_doc().to_string(),
            location: self.loc_to_location(&struct_env.get_loc()),
            abilities,
            is_resource: struct_env.has_memory(),
            is_native: struct_env.is_native(),
            attributes,
            type_parameters,
            fields,
            variants,
            invariants: vec![], // TODO: Extract invariants if needed
        }
    }

    fn constant_to_summary(&self, const_env: &NamedConstantEnv<'_>) -> ConstantSummary {
        let ty = const_env.get_type();
        ConstantSummary {
            name: const_env
                .get_name()
                .display(self.env.symbol_pool())
                .to_string(),
            module: const_env.module_env.get_full_name_str(),
            location: self.loc_to_location(&const_env.get_loc()),
            type_: self.type_to_query(&ty, &[]).to_simple_string(),
            value: Some(format!("{:?}", const_env.get_value())),
        }
    }

    fn constant_to_info(&self, const_env: &NamedConstantEnv<'_>) -> ConstantInfo {
        let ty = const_env.get_type();
        ConstantInfo {
            module: const_env.module_env.get_full_name_str(),
            name: const_env
                .get_name()
                .display(self.env.symbol_pool())
                .to_string(),
            doc: const_env.get_doc().to_string(),
            location: self.loc_to_location(&const_env.get_loc()),
            type_: self.type_to_query(&ty, &[]),
            value: Some(format!("{:?}", const_env.get_value())),
        }
    }
}
