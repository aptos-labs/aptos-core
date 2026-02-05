// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! QueryEngine for programmatic Move package introspection.
//!
//! Provides [`QueryEngine`], a high-level API for querying Move packages.
//! Wraps move-model's [`GlobalEnv`] with a hierarchical query interface:
//!
//! ```text
//! Package → Module → Function / Struct / Constant
//! ```
//!
//! See `tests/testsuite.rs` for usage examples.

use crate::types::*;
use move_command_line_common::address::NumericalAddress;
use move_model::{
    ast::ModuleName,
    model::{FunctionEnv, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, StructEnv, TypeParameter},
};
use move_package::{
    source_package::{
        manifest_parser::parse_move_manifest_from_file, parsed_manifest::Dependencies,
    },
    BuildConfig, ModelConfig,
};
use std::{collections::BTreeMap, path::PathBuf};

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
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Main query engine wrapping GlobalEnv
pub struct QueryEngine {
    env: GlobalEnv,
    package_path: PathBuf,
    build_config: BuildConfig,
    model_config: ModelConfig,
}

impl QueryEngine {
    /// Build package and create query engine
    pub fn new(
        package_path: PathBuf,
        build_config: BuildConfig,
        model_config: ModelConfig,
    ) -> QueryResult<Self> {
        let env = build_config
            .clone()
            .move_model_for_package(&package_path, model_config.clone())
            .map_err(|e| QueryError::BuildFailed(format!("{:#}", e)))?;
        Ok(Self {
            env,
            package_path,
            build_config,
            model_config,
        })
    }

    /// Rebuild the model (after file changes)
    pub fn rebuild(&mut self) -> QueryResult<()> {
        self.env = self
            .build_config
            .clone()
            .move_model_for_package(&self.package_path, self.model_config.clone())
            .map_err(|e| QueryError::BuildFailed(format!("{:#}", e)))?;
        Ok(())
    }

    // ========================================
    // Package -> Module -> Item Queries
    // ========================================

    /// Get package info with module names.
    pub fn get_package(&self) -> QueryResult<Package> {
        let manifest_path = self.package_path.join("Move.toml");

        // Parse manifest for package name and dependencies
        let manifest = parse_move_manifest_from_file(&manifest_path)?;
        let name = manifest.package.name.to_string();

        // Build dependency info with modules from GlobalEnv
        let dependencies = self.build_dependency_info(&manifest.dependencies);

        let modules: Vec<String> = self
            .iter_modules(true)
            .map(|m| m.get_full_name_str())
            .collect();

        Ok(Package {
            name,
            path: self.package_path.display().to_string(),
            manifest_path: manifest_path.display().to_string(),
            dependencies,
            modules,
        })
    }

    /// Get module info with function/struct/constant names.
    /// Name format: "0x1::module".
    pub fn get_module(&self, name: &str) -> QueryResult<Module> {
        let module_env = self.find_module(name)?;
        Ok(self.format_module(&module_env))
    }

    /// Get function details.
    /// Name format: "0x1::module::function".
    pub fn get_function(&self, name: &str) -> QueryResult<Function> {
        let (module, item) = Self::parse_full_name(name)?;
        let module_env = self.find_module(module)?;
        let func_env = self.find_function(&module_env, item)?;
        Ok(self.format_function(&func_env))
    }

    /// Get struct details.
    /// Name format: "0x1::module::struct".
    pub fn get_struct(&self, name: &str) -> QueryResult<Struct> {
        let (module, item) = Self::parse_full_name(name)?;
        let module_env = self.find_module(module)?;
        let struct_env = self.find_struct(&module_env, item)?;
        Ok(self.format_struct(&struct_env))
    }

    /// Get constant details.
    /// Name format: "0x1::module::CONSTANT".
    pub fn get_constant(&self, name: &str) -> QueryResult<Constant> {
        let (module, item) = Self::parse_full_name(name)?;
        let module_env = self.find_module(module)?;
        let const_env = self.find_constant(&module_env, item)?;
        Ok(self.format_constant(&const_env))
    }

    /// Parse full name like "0x1::coin::transfer" into ("0x1::coin", "transfer").
    fn parse_full_name(name: &str) -> QueryResult<(&str, &str)> {
        name.rsplit_once("::").ok_or_else(|| {
            QueryError::Other(anyhow::anyhow!(
                "Invalid name '{}': expected format like 0x1::module::item",
                name
            ))
        })
    }

    /// Parse module name like "0x1::coin" into ("0x1", "coin").
    /// Returns None if no "::" separator found.
    fn parse_module_name(name: &str) -> Option<(&str, &str)> {
        name.rsplit_once("::")
    }

    // ========================================
    // Source Code Query
    // ========================================

    /// Get source code for a location (1-indexed lines, returns full lines).
    /// Column information is ignored; full lines are always returned.
    pub fn get_source(&self, location: &Location) -> QueryResult<String> {
        if location.start_line == 0 || location.end_line == 0 {
            return Err(anyhow::anyhow!(
                "line numbers must be >= 1 (got start_line={}, end_line={})",
                location.start_line,
                location.end_line
            )
            .into());
        }
        if location.start_line > location.end_line {
            return Err(anyhow::anyhow!(
                "start_line ({}) must be <= end_line ({})",
                location.start_line,
                location.end_line
            )
            .into());
        }
        let content = std::fs::read_to_string(&location.file)
            .map_err(|e| anyhow::anyhow!("failed to read file {}: {}", location.file, e))?;
        let source = content
            .lines()
            .skip((location.start_line - 1) as usize)
            .take((location.end_line - location.start_line + 1) as usize)
            .collect::<Vec<_>>()
            .join("\n");
        Ok(source)
    }

    // ========================================
    // Formatting
    // ========================================

    fn build_dependency_info(&self, deps: &Dependencies) -> Vec<Dependency> {
        use std::collections::HashMap;

        // Map canonical paths to dep names for O(1) ancestor lookup
        let path_to_dep: HashMap<PathBuf, String> = deps
            .iter()
            .filter_map(|(name, info)| {
                let canonical = self.package_path.join(&info.local).canonicalize().ok()?;
                Some((canonical, name.as_str().to_string()))
            })
            .collect();

        // Group modules by dependency
        let mut modules_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for module in self.iter_modules(false) {
            let file = self.format_location(&module.get_loc()).file;
            let Ok(module_path) = PathBuf::from(&file).canonicalize() else {
                continue;
            };
            for ancestor in module_path.ancestors() {
                if let Some(dep_name) = path_to_dep.get(ancestor) {
                    modules_map
                        .entry(dep_name.clone())
                        .or_default()
                        .push(module.get_full_name_str());
                    break;
                }
            }
        }

        // Build result
        deps.iter()
            .map(|(name, info)| Dependency {
                name: name.as_str().to_string(),
                path: info.local.display().to_string(),
                modules: modules_map.remove(name.as_str()).unwrap_or_default(),
            })
            .collect()
    }

    fn format_module(&self, module: &ModuleEnv<'_>) -> Module {
        let functions: Vec<String> = module
            .get_functions()
            .map(|f| f.get_name_str().to_string())
            .collect();

        let structs: Vec<String> = module
            .get_structs()
            .map(|s| s.get_name_str().to_string())
            .collect();

        let constants: Vec<String> = module
            .get_named_constants()
            .map(|c| c.get_name().display(self.env.symbol_pool()).to_string())
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

        Module {
            address: self.env.display(module.self_address()).to_string(),
            name: module.get_name().display(&self.env).to_string(),
            doc: module.get_doc().to_string(),
            location: self.format_location(&module.get_loc()),
            is_target: module.is_target(),
            friends,
            uses,
            used_by,
            functions,
            structs,
            constants,
        }
    }

    fn format_function(&self, func: &FunctionEnv<'_>) -> Function {
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

        let attributes: Vec<String> = func
            .get_attributes()
            .iter()
            .map(|attr| self.format_attribute(attr))
            .collect();

        Function {
            module: func.module_env.get_full_name_str(),
            name: func.get_name_str().to_string(),
            signature: func.get_header_string(),
            doc: func.get_doc().to_string(),
            location: self.format_location(&func.get_loc()),
            visibility: func.visibility_str().trim().to_string(),
            is_entry: func.is_entry(),
            is_view: attributes.iter().any(|a| a == "view"),
            is_inline: func.is_inline(),
            is_native: func.is_native(),
            attributes,
            acquires,
        }
    }

    fn format_struct(&self, struct_env: &StructEnv<'_>) -> Struct {
        let tctx = struct_env.get_type_display_ctx();

        // Format type parameters as strings like "T: copy + drop" or "phantom U"
        let type_parameters: Vec<String> = struct_env
            .get_type_parameters()
            .iter()
            .map(|tp| self.format_type_parameter(tp))
            .collect();

        let abilities: Vec<String> = struct_env
            .get_abilities()
            .into_iter()
            .map(|a| a.to_string())
            .collect();

        let is_enum = struct_env.has_variants();
        let (fields, variants) = self.format_fields_and_variants(struct_env, &tctx);

        let attributes: Vec<String> = struct_env
            .get_attributes()
            .iter()
            .map(|attr| self.format_attribute(attr))
            .collect();

        Struct {
            module: struct_env.module_env.get_full_name_str(),
            name: struct_env
                .get_name()
                .display(self.env.symbol_pool())
                .to_string(),
            doc: struct_env.get_doc().to_string(),
            location: self.format_location(&struct_env.get_loc()),
            abilities,
            is_resource: struct_env.has_memory(),
            is_native: struct_env.is_native(),
            is_enum,
            attributes,
            type_parameters,
            fields,
            variants,
        }
    }

    fn format_constant(&self, const_env: &NamedConstantEnv<'_>) -> Constant {
        let tctx = const_env.module_env.get_type_display_ctx();
        Constant {
            module: const_env.module_env.get_full_name_str(),
            name: const_env
                .get_name()
                .display(self.env.symbol_pool())
                .to_string(),
            doc: const_env.get_doc().to_string(),
            location: self.format_location(&const_env.get_loc()),
            type_: const_env.get_type().display(&tctx).to_string(),
            value: Some(self.env.display(&const_env.get_value()).to_string()),
        }
    }

    fn format_location(&self, loc: &Loc) -> Location {
        if let Some((file, cs_loc)) = self.env.get_file_and_location(loc) {
            let end_loc = self.env.get_location(&loc.at_end()).unwrap_or(cs_loc);
            // Why `+ 1`: Convert to 1-indexed line/column numbers for display.
            Location {
                file,
                start_line: cs_loc.line.0 + 1,
                start_column: cs_loc.column.0 + 1,
                end_line: end_loc.line.0 + 1,
                end_column: end_loc.column.0 + 1,
            }
        } else {
            Location::default()
        }
    }

    /// Format fields (for structs) or variants (for enums) from a struct environment.
    fn format_fields_and_variants(
        &self,
        struct_env: &StructEnv<'_>,
        tctx: &move_model::ty::TypeDisplayContext,
    ) -> (Option<Vec<Field>>, Option<Vec<Variant>>) {
        if struct_env.is_native() {
            return (None, None);
        }

        if struct_env.has_variants() {
            // Enum: collect variants with their fields
            let variants = struct_env
                .get_variants()
                .map(|variant_sym| Variant {
                    name: variant_sym.display(self.env.symbol_pool()).to_string(),
                    fields: struct_env
                        .get_fields_of_variant(variant_sym)
                        .map(|f| Field {
                            name: f.get_name().display(self.env.symbol_pool()).to_string(),
                            type_: f.get_type().display(tctx).to_string(),
                        })
                        .collect(),
                })
                .collect();
            (None, Some(variants))
        } else {
            // Struct: collect fields
            let fields = struct_env
                .get_fields()
                .map(|f| Field {
                    name: f.get_name().display(self.env.symbol_pool()).to_string(),
                    type_: f.get_type().display(tctx).to_string(),
                })
                .collect();
            (Some(fields), None)
        }
    }

    fn format_type_parameter(&self, tp: &TypeParameter) -> String {
        let name = tp.0.display(self.env.symbol_pool()).to_string();
        let phantom = if tp.1.is_phantom { "phantom " } else { "" };
        if tp.1.abilities.is_empty() {
            format!("{}{}", phantom, name)
        } else {
            format!("{}{}: {}", phantom, name, tp.1.abilities)
        }
    }

    fn format_attribute(&self, attr: &move_model::ast::Attribute) -> String {
        use move_model::ast::{Attribute, AttributeValue};
        match attr {
            Attribute::Apply(_, name, args) => {
                let name_str = name.display(self.env.symbol_pool());
                if args.is_empty() {
                    name_str.to_string()
                } else {
                    let args_str: Vec<String> =
                        args.iter().map(|a| self.format_attribute(a)).collect();
                    format!("{}({})", name_str, args_str.join(", "))
                }
            },
            Attribute::Assign(_, name, value) => {
                let value_str = match value {
                    AttributeValue::Value(_, v) => self.env.display(v).to_string(),
                    AttributeValue::Name(_, module, sym) => {
                        if let Some(m) = module {
                            format!(
                                "{}::{}",
                                m.display_full(&self.env),
                                sym.display(self.env.symbol_pool())
                            )
                        } else {
                            sym.display(self.env.symbol_pool()).to_string()
                        }
                    },
                };
                format!("{} = {}", name.display(self.env.symbol_pool()), value_str)
            },
        }
    }

    // ========================================
    // Helper Methods - Finding Items
    // ========================================

    fn iter_modules(&self, is_target: bool) -> impl Iterator<Item = ModuleEnv<'_>> {
        self.env
            .get_modules()
            .filter(move |m| m.is_target() == is_target)
    }

    fn find_module(&self, name: &str) -> QueryResult<ModuleEnv<'_>> {
        let (addr, module_name) = Self::parse_module_name(name).ok_or_else(|| {
            QueryError::Other(anyhow::anyhow!(
                "Invalid module name '{}': expected format 0x1::module",
                name
            ))
        })?;
        // Validate address format to avoid panic in ModuleName::from_str.
        // NumericalAddress::parse_str only accepts numeric addresses (0x...), not named addresses.
        NumericalAddress::parse_str(addr).map_err(|_| {
            QueryError::Other(anyhow::anyhow!(
                "Invalid address '{}': expected numeric address (e.g., 0x1), not a named address",
                addr
            ))
        })?;
        let symbol = self.env.symbol_pool().make(module_name);
        let module_name = ModuleName::from_str(addr, symbol);
        self.env
            .find_module(&module_name)
            .ok_or_else(|| QueryError::ModuleNotFound(name.to_string()))
    }

    fn find_function<'a>(
        &'a self,
        module: &'a ModuleEnv<'a>,
        name: &str,
    ) -> QueryResult<FunctionEnv<'a>> {
        let symbol = self.env.symbol_pool().make(name);
        module.find_function(symbol).ok_or_else(|| {
            QueryError::FunctionNotFound(module.get_full_name_str(), name.to_string())
        })
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
        module.find_named_constant(symbol).ok_or_else(|| {
            QueryError::ConstantNotFound(module.get_full_name_str(), name.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Location;
    use move_model::metadata::{CompilerVersion, LanguageVersion};

    fn create_engine() -> QueryEngine {
        let package_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_packages/single_mod");
        let model_config = ModelConfig {
            all_files_as_targets: false,
            target_filter: None,
            compiler_version: CompilerVersion::default(),
            language_version: LanguageVersion::default(),
        };
        let build_config = BuildConfig {
            compiler_config: move_package::CompilerConfig {
                skip_attribute_checks: true,
                ..Default::default()
            },
            ..Default::default()
        };
        QueryEngine::new(package_dir, build_config, model_config).unwrap()
    }

    #[test]
    fn test_invalid_module_name_format() {
        let engine = create_engine();
        let err = engine.get_module("invalid_module").unwrap_err();
        assert!(err.to_string().contains("Invalid module name"));
    }

    #[test]
    fn test_invalid_address_format() {
        let engine = create_engine();
        let err = engine.get_module("named_addr::module").unwrap_err();
        assert!(err.to_string().contains("Invalid address"));
    }

    #[test]
    fn test_module_not_found() {
        let engine = create_engine();
        let err = engine.get_module("0x999::nonexistent").unwrap_err();
        assert!(err.to_string().contains("Module not found"));
    }

    #[test]
    fn test_function_not_found() {
        let engine = create_engine();
        let err = engine
            .get_function("0x42::comprehensive::nonexistent")
            .unwrap_err();
        assert!(err.to_string().contains("Function not found"));
    }

    #[test]
    fn test_struct_not_found() {
        let engine = create_engine();
        let err = engine
            .get_struct("0x42::comprehensive::NonexistentStruct")
            .unwrap_err();
        assert!(err.to_string().contains("Struct not found"));
    }

    #[test]
    fn test_constant_not_found() {
        let engine = create_engine();
        let err = engine
            .get_constant("0x42::comprehensive::NONEXISTENT")
            .unwrap_err();
        assert!(err.to_string().contains("Constant not found"));
    }

    #[test]
    fn test_get_source_invalid_line_zero() {
        let engine = create_engine();
        let loc = Location {
            file: "test.move".to_string(),
            start_line: 0,
            start_column: 1,
            end_line: 5,
            end_column: 1,
        };
        let err = engine.get_source(&loc).unwrap_err();
        assert!(err.to_string().contains("line numbers must be >= 1"));
    }

    #[test]
    fn test_get_source_start_greater_than_end() {
        let engine = create_engine();
        let loc = Location {
            file: "test.move".to_string(),
            start_line: 10,
            start_column: 1,
            end_line: 5,
            end_column: 1,
        };
        let err = engine.get_source(&loc).unwrap_err();
        assert!(err.to_string().contains("start_line"));
    }

    #[test]
    fn test_get_source_file_not_found() {
        let engine = create_engine();
        let loc = Location {
            file: "/nonexistent/path.move".to_string(),
            start_line: 1,
            start_column: 1,
            end_line: 5,
            end_column: 1,
        };
        let err = engine.get_source(&loc).unwrap_err();
        assert!(err.to_string().contains("failed to read file"));
    }
}
