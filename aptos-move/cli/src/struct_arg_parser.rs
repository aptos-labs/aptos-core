// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Parser for struct and enum transaction arguments.
//!
//! This module enables the Aptos CLI to accept public copy structs and enums as transaction
//! arguments in JSON format, automatically encoding them to BCS without requiring manual encoding.

use aptos_api_types::{MoveModuleBytecode, MoveStructField, MoveStructTag, MoveType};
use aptos_cli_common::{load_account_arg, CliError, CliTypedResult};
use aptos_rest_client::Client;
use async_recursion::async_recursion;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CompiledModule, StructDefinition, StructFieldInformation},
};
use move_core_types::{
    int256::{I256, U256},
    language_storage::{
        ModuleId, StructTag, FIXED_POINT32_TYPE_STR, FIXED_POINT64_TYPE_STR, MODULE_SEPARATOR,
        OBJECT_TYPE_STR, STRING_TYPE_STR,
    },
};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, str::FromStr, sync::RwLock};

/// Maximum nesting depth for structs, enums, and vectors.
/// This matches the vector depth limit in the existing CLI (mod.rs line 2942).
/// Prevents stack overflow and excessively complex arguments.
const MAX_NESTING_DEPTH: u8 = 7;

/// Cached module information including both bytecode and optionally deserialized representation.
///
/// The `compiled` field is lazily populated when first needed, avoiding repeated
/// deserialization of the same module bytecode.
struct CachedModule {
    /// Raw module bytecode from chain
    bytecode: MoveModuleBytecode,
    /// Deserialized module (lazily computed on first access when ABI is unavailable)
    compiled: Option<CompiledModule>,
}

/// Parser for struct and enum arguments that queries on-chain module metadata
/// and encodes arguments to BCS format.
///
/// Includes a module cache to avoid repeated fetches and deserialization of the same module.
pub struct StructArgParser {
    rest_client: Client,
    /// Cache of fetched modules with both bytecode and deserialized form.
    /// Uses RwLock for thread-safe caching (required since parser is shared across async tasks).
    module_cache: RwLock<HashMap<ModuleId, CachedModule>>,
}

impl StructArgParser {
    /// Create a new parser with the given REST client.
    pub fn new(rest_client: Client) -> Self {
        Self {
            rest_client,
            module_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Convert Option<T> legacy array format to enum variant and fields.
    ///
    /// This is a helper to avoid duplicating the conversion logic across multiple locations.
    /// The legacy format uses:
    /// - [] for None
    /// - [value] for Some(value)
    ///
    /// Returns (variant_name, fields_map) where fields_map has "0" key for Some variant.
    pub fn convert_option_array_to_enum_format(
        array: &[JsonValue],
    ) -> CliTypedResult<(&'static str, serde_json::Map<String, JsonValue>)> {
        if array.is_empty() {
            Ok(("None", serde_json::Map::new()))
        } else if array.len() == 1 {
            let mut map = serde_json::Map::new();
            map.insert("0".to_string(), array[0].clone());
            Ok(("Some", map))
        } else {
            Err(CliError::CommandArgumentError(format!(
                "Option<T> as vector must have 0 or 1 elements, got {}",
                array.len()
            )))
        }
    }

    /// Verify that a struct exists on-chain and retrieve its metadata.
    ///
    /// Uses a cache to avoid repeated fetches of the same module. This addresses
    /// the review comment about repeated work between verify_struct_exists and
    /// subsequent parsing which both need module bytecode.
    pub async fn verify_struct_exists(&self, struct_tag: &StructTag) -> CliTypedResult<()> {
        let module_id = ModuleId::new(struct_tag.address, struct_tag.module.clone());

        // Check cache first (read lock for concurrent access).
        // Returns either a cached MoveModuleBytecode (for deserialization without a network fetch)
        // or None if the module isn't cached yet and must be fetched.
        let cached_bytecode: Option<MoveModuleBytecode> = {
            let cache_read = self.module_cache.read().map_err(|e| {
                CliError::CommandArgumentError(format!("Failed to acquire cache read lock: {}", e))
            })?;

            if let Some(cached) = cache_read.get(&module_id) {
                // Check compiled module first if available (has complete information)
                if let Some(compiled) = &cached.compiled {
                    let struct_exists = compiled.struct_defs.iter().any(|def| {
                        let handle = ModuleAccess::struct_handle_at(compiled, def.struct_handle);
                        ModuleAccess::identifier_at(compiled, handle.name).as_str()
                            == struct_tag.name.as_str()
                    });

                    // Compiled module has complete info - return definitive answer
                    if struct_exists {
                        return Ok(());
                    } else {
                        return Err(CliError::CommandArgumentError(format!(
                            "Struct {} not found in module {}::{}",
                            struct_tag.name, struct_tag.address, struct_tag.module
                        )));
                    }
                }

                // No compiled module cached, check ABI if available
                if let Some(abi) = &cached.bytecode.abi {
                    let found_in_abi = abi
                        .structs
                        .iter()
                        .any(|s| s.name.as_str() == struct_tag.name.as_str());

                    if found_in_abi {
                        // Found in ABI - definitely exists
                        return Ok(());
                    }
                    // Not found in ABI, but ABI might be incomplete (e.g., doesn't include private structs).
                    // Fall through to deserialize the cached bytecode for a definitive check.
                }

                // Module is cached but its compiled form is not yet available.
                // Clone the bytecode so we can deserialize it after releasing the read lock,
                // avoiding an unnecessary network re-fetch.
                Some(cached.bytecode.clone())
            } else {
                // Module not in cache, need to fetch from network
                None
            }
        }; // Release read lock

        // Obtain the module bytecode: use the cached copy when available, otherwise fetch from chain.
        let is_cached = cached_bytecode.is_some();
        let module = if let Some(bytecode) = cached_bytecode {
            bytecode
        } else {
            self.rest_client
                .get_account_module(struct_tag.address, struct_tag.module.as_str())
                .await
                .map_err(|e| {
                    CliError::CommandArgumentError(format!(
                        "Failed to fetch module {}::{}: {}",
                        struct_tag.address, struct_tag.module, e
                    ))
                })?
                .into_inner()
        };

        // For freshly-fetched modules, check the ABI first as a fast path (avoids deserialization).
        // For cached modules, the ABI was already checked in the block above and the struct wasn't
        // found there, so skip straight to deserialization for a definitive check.
        if !is_cached {
            if let Some(abi) = &module.abi {
                if abi
                    .structs
                    .iter()
                    .any(|s| s.name.as_str() == struct_tag.name.as_str())
                {
                    // Found in ABI of freshly-fetched module - cache without deserializing and return.
                    self.module_cache
                        .write()
                        .map_err(|e| {
                            CliError::CommandArgumentError(format!(
                                "Failed to acquire cache write lock: {}",
                                e
                            ))
                        })?
                        .insert(module_id, CachedModule {
                            bytecode: module,
                            compiled: None,
                        });
                    return Ok(());
                }
                // Not found in ABI of fresh module — fall through to deserialization.
            }
        }

        // Deserialize the bytecode for a definitive struct-existence check.
        // Reached when: (a) the module was cached but the struct wasn't in its ABI, or
        // (b) the module was freshly fetched and has no ABI (or the ABI didn't have the struct).
        let compiled_module = Self::deserialize_module(&module, struct_tag)?;
        let struct_exists = compiled_module.struct_defs.iter().any(|def| {
            let handle = ModuleAccess::struct_handle_at(&compiled_module, def.struct_handle);
            ModuleAccess::identifier_at(&compiled_module, handle.name).as_str()
                == struct_tag.name.as_str()
        });
        let compiled_opt = Some(compiled_module);

        if !struct_exists {
            return Err(CliError::CommandArgumentError(format!(
                "Struct {} not found in module {}::{}",
                struct_tag.name, struct_tag.address, struct_tag.module
            )));
        }

        // Cache the result with deserialized module if we already have it
        self.module_cache
            .write()
            .map_err(|e| {
                CliError::CommandArgumentError(format!("Failed to acquire cache write lock: {}", e))
            })?
            .insert(module_id, CachedModule {
                bytecode: module,
                compiled: compiled_opt,
            });

        Ok(())
    }

    /// Get module from cache and ensure it's deserialized.
    ///
    /// Returns both the bytecode (for ABI access) and the compiled module.
    /// Lazily deserializes the module if not already deserialized in cache.
    ///
    /// # Precondition
    /// `verify_struct_exists` must have been called first to ensure the module is cached.
    fn get_cached_module(
        &self,
        struct_tag: &StructTag,
    ) -> CliTypedResult<(MoveModuleBytecode, Option<CompiledModule>)> {
        let module_id = ModuleId::new(struct_tag.address, struct_tag.module.clone());

        // Try read lock first for the common case where module is already deserialized
        {
            let cache_read = self.module_cache.read().map_err(|e| {
                CliError::CommandArgumentError(format!("Failed to acquire cache read lock: {}", e))
            })?;

            if let Some(cached) = cache_read.get(&module_id) {
                if let Some(compiled) = &cached.compiled {
                    // Already deserialized - return immediately
                    return Ok((cached.bytecode.clone(), Some(compiled.clone())));
                }
                // If only ABI is present (compiled is None), continue to deserialization.
                // Enums always need the compiled module for variant information.
            } else {
                return Err(CliError::CommandArgumentError(format!(
                    "Module {}::{} not found in cache. verify_struct_exists must be called first.",
                    struct_tag.address, struct_tag.module
                )));
            }
        } // Release read lock

        // Need to deserialize - acquire write lock
        let mut cache_write = self.module_cache.write().map_err(|e| {
            CliError::CommandArgumentError(format!("Failed to acquire cache write lock: {}", e))
        })?;

        // Double-check: another thread might have deserialized while we waited for write lock
        if let Some(cached) = cache_write.get(&module_id) {
            if let Some(compiled) = &cached.compiled {
                return Ok((cached.bytecode.clone(), Some(compiled.clone())));
            }

            // Deserialize now
            let compiled = Self::deserialize_module(&cached.bytecode, struct_tag)?;
            let bytecode = cached.bytecode.clone();

            // Update cache with deserialized module
            cache_write.insert(module_id, CachedModule {
                bytecode: bytecode.clone(),
                compiled: Some(compiled.clone()),
            });

            Ok((bytecode, Some(compiled)))
        } else {
            Err(CliError::CommandArgumentError(format!(
                "Module {}::{} disappeared from cache unexpectedly",
                struct_tag.address, struct_tag.module
            )))
        }
    }

    /// Check nesting depth limit to prevent stack overflow.
    fn check_depth(depth: u8, type_name: &str) -> CliTypedResult<()> {
        if depth > MAX_NESTING_DEPTH {
            return Err(CliError::CommandArgumentError(format!(
                "`{}` nesting depth {} exceeds maximum allowed depth of {}",
                type_name, depth, MAX_NESTING_DEPTH
            )));
        }
        Ok(())
    }

    /// Check if a StructTag represents an enum type (has variants).
    ///
    /// This allows us to determine whether to attempt enum or struct parsing
    /// without the overhead of a failed parsing attempt.
    ///
    /// # Precondition
    /// `verify_struct_exists` must have been called first to ensure the module is cached.
    ///
    /// # Returns
    /// - `Ok(true)` if the type is an enum (has DeclaredVariants)
    /// - `Ok(false)` if the type is a struct (has Declared fields)
    /// - `Err` if the type is Native or not found
    pub async fn is_enum_type(&self, struct_tag: &StructTag) -> CliTypedResult<bool> {
        // Verify the struct exists and is cached
        self.verify_struct_exists(struct_tag).await?;

        let (_module, compiled_opt) = self.get_cached_module(struct_tag)?;
        let compiled_module = compiled_opt.ok_or_else(|| {
            CliError::CommandArgumentError(format!(
                "Module {}::{} should have been deserialized but wasn't",
                struct_tag.address, struct_tag.module
            ))
        })?;

        let struct_def = Self::find_struct_def(&compiled_module, struct_tag)?;

        match &struct_def.field_information {
            StructFieldInformation::DeclaredVariants(_) => Ok(true),
            StructFieldInformation::Declared(_) => Ok(false),
            StructFieldInformation::Native => Err(CliError::CommandArgumentError(format!(
                "Type {} is a native type and cannot be used as a transaction argument",
                struct_tag.name
            ))),
        }
    }

    /// Parse Option<T> value which can be in two formats:
    /// 1. Legacy array format: [] for None, [value] for Some(value)
    /// 2. New enum format: {"None": {}} or {"Some": {"0": value}}
    ///
    /// This helper extracts repeated logic for Option handling that appears in
    /// multiple places (types.rs and parse_value_by_type).
    async fn parse_option_value(
        &self,
        struct_tag: &StructTag,
        value: &JsonValue,
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        if value.is_array() {
            // Legacy vector format
            let array = value.as_array().ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Expected array for Option type, got: {}",
                    value
                ))
            })?;
            let (variant, fields_map) = Self::convert_option_array_to_enum_format(array)?;
            self.construct_enum_argument(struct_tag, variant, &fields_map, depth + 1)
                .await
        } else if value.is_object() {
            // New enum format: {"None": {}} or {"Some": {"0": value}}
            let obj = value.as_object().ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Expected object for Option enum format, got: {}",
                    value
                ))
            })?;
            if obj.len() == 1 {
                let (variant_name, variant_fields) = obj.iter().next().ok_or_else(|| {
                    CliError::CommandArgumentError("Unexpected empty object for Option".to_string())
                })?;
                if let Some(fields_obj) = variant_fields.as_object() {
                    return self
                        .construct_enum_argument(struct_tag, variant_name, fields_obj, depth + 1)
                        .await;
                }
            }
            Err(CliError::CommandArgumentError(format!(
                "Invalid Option format. Expected {{\"None\": {{}}}} or {{\"Some\": {{\"0\": value}}}}, got {}",
                value
            )))
        } else {
            Err(CliError::CommandArgumentError(format!(
                "Invalid Option value. Expected array or object, got {}",
                value
            )))
        }
    }

    /// Deserialize module bytecode to CompiledModule.
    fn deserialize_module(
        module: &MoveModuleBytecode,
        struct_tag: &StructTag,
    ) -> CliTypedResult<CompiledModule> {
        CompiledModule::deserialize(module.bytecode.inner()).map_err(|e| {
            CliError::CommandArgumentError(format!(
                "Failed to deserialize module {}::{}: {}",
                struct_tag.address, struct_tag.module, e
            ))
        })
    }

    /// Find struct/enum definition in compiled module by name.
    fn find_struct_def<'a>(
        compiled_module: &'a CompiledModule,
        struct_tag: &StructTag,
    ) -> CliTypedResult<&'a StructDefinition> {
        compiled_module
            .struct_defs
            .iter()
            .find(|def| {
                let handle = ModuleAccess::struct_handle_at(compiled_module, def.struct_handle);
                ModuleAccess::identifier_at(compiled_module, handle.name).as_str()
                    == struct_tag.name.as_str()
            })
            .ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Type {} not found in module {}::{}",
                    struct_tag.name, struct_tag.address, struct_tag.module
                ))
            })
    }

    /// Construct a struct argument by parsing fields and encoding to BCS.
    ///
    /// # Why REST API access is necessary
    ///
    /// Unlike primitive types where the BCS encoding rules are fixed and known at compile time,
    /// struct/enum types require querying on-chain module bytecode to:
    /// 1. Verify the type exists and is accessible (public visibility)
    /// 2. Get field names, types, and order for correct BCS encoding
    /// 3. Support generic type instantiation (e.g., Option<T>, vector<T>)
    /// 4. Handle enum variant tags and field layouts
    ///
    /// The BCS encoding must exactly match the on-chain type definition, which can vary
    /// between deployments and cannot be determined from the type string alone.
    pub async fn construct_struct_argument(
        &self,
        struct_tag: &StructTag,
        field_values: &serde_json::Map<String, JsonValue>,
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        // Check nesting depth limit
        Self::check_depth(depth, "Struct")?;

        // Verify struct exists and cache module
        self.verify_struct_exists(struct_tag).await?;

        // Get cached module (with lazy deserialization)
        let (module, compiled_opt) = self.get_cached_module(struct_tag)?;

        // Get struct field information - first try from ABI, fall back to deserialized bytecode
        let fields = if let Some(abi) = &module.abi {
            // Use ABI if available
            let struct_def = abi
                .structs
                .iter()
                .find(|s| s.name.as_str() == struct_tag.name.as_str())
                .ok_or_else(|| {
                    CliError::CommandArgumentError(format!(
                        "Struct {} not found in module {}::{}",
                        struct_tag.name, struct_tag.address, struct_tag.module
                    ))
                })?;

            // Check if this is actually an enum (enums have empty fields in ABI due to TODO(#13806))
            // If it's an enum, we must reject it here to avoid silently returning empty BCS bytes
            if struct_def.is_enum {
                return Err(CliError::CommandArgumentError(format!(
                    "Type {} is an enum.",
                    struct_tag.name
                )));
            }

            struct_def.fields.clone()
        } else {
            // Use already-deserialized module from cache
            let compiled_module = compiled_opt.ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Module {}::{} should have been deserialized but wasn't",
                    struct_tag.address, struct_tag.module
                ))
            })?;

            // Find the struct definition
            let struct_def = Self::find_struct_def(&compiled_module, struct_tag)?;

            // Extract fields from struct definition
            match &struct_def.field_information {
                StructFieldInformation::Declared(field_defs) => field_defs
                    .iter()
                    .map(|f| convert_field_to_move_struct_field(&compiled_module, f))
                    .collect(),
                StructFieldInformation::Native => {
                    return Err(CliError::CommandArgumentError(format!(
                        "Struct {} is a native struct and cannot be used as a transaction argument",
                        struct_tag.name
                    )));
                },
                StructFieldInformation::DeclaredVariants(_) => {
                    return Err(CliError::CommandArgumentError(format!(
                        "Struct {} is an enum. Use enum variant syntax instead.",
                        struct_tag.name
                    )));
                },
            }
        };

        // Validate that all provided fields exist in the struct definition
        // This catches typos and malformed JSON that could produce unintended transactions
        let expected_field_names: std::collections::HashSet<&str> =
            fields.iter().map(|f| f.name.as_str()).collect();

        for provided_field_name in field_values.keys() {
            if !expected_field_names.contains(provided_field_name.as_str()) {
                let valid_fields: Vec<&str> = expected_field_names.iter().copied().collect();
                return Err(CliError::CommandArgumentError(format!(
                    "Unknown field '{}' for struct {}. Valid fields are: {}",
                    provided_field_name,
                    struct_tag.name,
                    valid_fields.join(", ")
                )));
            }
        }

        // Parse and encode each field
        let mut encoded_fields = Vec::new();

        for field in &fields {
            let field_name = field.name.as_str();
            let field_value = field_values.get(field_name).ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Missing field '{}' for struct {}",
                    field_name, struct_tag.name
                ))
            })?;

            // Substitute type parameters if this is a generic struct
            let field_type = substitute_type_params(&field.typ, struct_tag)?;

            let encoded_value = self
                .parse_value_by_type(&field_type, field_value, depth)
                .await?;
            encoded_fields.extend(encoded_value);
        }

        Ok(encoded_fields)
    }

    /// Construct an enum argument by encoding variant index and fields.
    ///
    /// For backward compatibility, Option<T> is encoded as a vector:
    /// - None → vec[] (empty vector, length 0)
    /// - Some(v) → vec[v] (vector with one element, length 1)
    pub async fn construct_enum_argument(
        &self,
        struct_tag: &StructTag,
        variant: &str,
        field_values: &serde_json::Map<String, JsonValue>,
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        // Check nesting depth limit
        Self::check_depth(depth, "Enum")?;

        // Special handling for Option<T> for backward compatibility (uses vector encoding)
        // Check full module path to ensure it's std::option::Option, not a custom enum named "Option"
        if struct_tag.is_option() {
            // Convert field_values map to array for Option
            let fields_array = if field_values.is_empty() {
                vec![]
            } else if variant == "None" {
                return Err(CliError::CommandArgumentError(
                    "Option::None should not have any fields".to_string(),
                ));
            } else {
                // For Option::Some, expect a single field named "0"
                if field_values.len() != 1 {
                    return Err(CliError::CommandArgumentError(format!(
                        "Option::Some expects exactly 1 field, got {}",
                        field_values.len()
                    )));
                }

                // Validate that the field name is "0"
                let field_value = field_values.get("0").ok_or_else(|| {
                    let actual_field = field_values.keys().next().unwrap();
                    CliError::CommandArgumentError(format!(
                        "Option::Some field must be named \"0\", got \"{}\"",
                        actual_field
                    ))
                })?;

                vec![field_value.clone()]
            };
            return self
                .construct_option_argument_from_array(struct_tag, variant, &fields_array, depth)
                .await;
        }

        // Verify enum exists and get cached/deserialized module
        self.verify_struct_exists(struct_tag).await?;
        let (_module, compiled_opt) = self.get_cached_module(struct_tag)?;
        let compiled_module = compiled_opt.ok_or_else(|| {
            CliError::CommandArgumentError(format!(
                "Module {}::{} should have been deserialized but wasn't",
                struct_tag.address, struct_tag.module
            ))
        })?;

        // Find the enum definition
        let enum_def = Self::find_struct_def(&compiled_module, struct_tag)?;

        // Extract variant definitions
        let variants = match &enum_def.field_information {
            StructFieldInformation::DeclaredVariants(variants) => variants,
            StructFieldInformation::Native => {
                return Err(CliError::CommandArgumentError(format!(
                    "Type {} is a native type and cannot be used as a transaction argument",
                    struct_tag.name
                )));
            },
            StructFieldInformation::Declared(_) => {
                return Err(CliError::StructNotEnumError(struct_tag.name.to_string()));
            },
        };

        // Find the variant by name and get its index
        let (variant_index, variant_def) = variants
            .iter()
            .enumerate()
            .find(|(_, v)| {
                ModuleAccess::identifier_at(&compiled_module, v.name).as_str() == variant
            })
            .ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Variant '{}' not found in enum {}::{}::{}. Available variants: {}",
                    variant,
                    struct_tag.address,
                    struct_tag.module,
                    struct_tag.name,
                    variants
                        .iter()
                        .map(|v| ModuleAccess::identifier_at(&compiled_module, v.name).as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;

        // Start encoding: variant index (ULEB128) + fields
        let mut encoded = Vec::new();
        encode_uleb128(variant_index as u64, &mut encoded);

        // Validate that all provided fields exist in the variant definition
        // This catches typos and malformed JSON that could produce unintended transactions
        let expected_field_names: std::collections::HashSet<&str> = variant_def
            .fields
            .iter()
            .map(|f| ModuleAccess::identifier_at(&compiled_module, f.name).as_str())
            .collect();

        for provided_field_name in field_values.keys() {
            if !expected_field_names.contains(provided_field_name.as_str()) {
                let valid_fields: Vec<&str> = expected_field_names.iter().copied().collect();
                return Err(CliError::CommandArgumentError(format!(
                    "Unknown field '{}' for variant {}::{}. Valid fields are: {}",
                    provided_field_name,
                    struct_tag.name,
                    variant,
                    valid_fields.join(", ")
                )));
            }
        }

        // Parse and encode each field
        for field_def in &variant_def.fields {
            let field_name = ModuleAccess::identifier_at(&compiled_module, field_def.name);
            let field_value = field_values.get(field_name.as_str()).ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Missing field '{}' for variant {}::{}",
                    field_name, struct_tag.name, variant
                ))
            })?;

            // Convert field signature to MoveType
            let field_type =
                convert_signature_token_to_move_type(&compiled_module, &field_def.signature.0);

            // Substitute type parameters if this is a generic enum
            let field_type = substitute_type_params(&field_type, struct_tag)?;

            // Encode the field value
            let encoded_value = self
                .parse_value_by_type(&field_type, field_value, depth)
                .await?;
            encoded.extend(encoded_value);
        }

        Ok(encoded)
    }

    /// Construct an Option<T> argument using vector encoding for backward compatibility.
    async fn construct_option_argument_from_array(
        &self,
        struct_tag: &StructTag,
        variant: &str,
        field_values: &[JsonValue],
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        // Get the type parameter T from Option<T>
        if struct_tag.type_args.is_empty() {
            return Err(CliError::CommandArgumentError(
                "Option must have a type parameter".to_string(),
            ));
        }

        let inner_type = MoveType::from(&struct_tag.type_args[0]);

        match variant {
            "None" => {
                // None is encoded as an empty vector
                if !field_values.is_empty() {
                    return Err(CliError::CommandArgumentError(
                        "Option::None should not have any fields".to_string(),
                    ));
                }

                let mut result = Vec::new();
                encode_uleb128(0, &mut result); // Vector length = 0
                Ok(result)
            },
            "Some" => {
                // Some(v) is encoded as a vector with one element
                if field_values.len() != 1 {
                    return Err(CliError::CommandArgumentError(format!(
                        "Option::Some requires exactly 1 field, got {}",
                        field_values.len()
                    )));
                }

                let mut result = Vec::new();
                encode_uleb128(1, &mut result); // Vector length = 1

                let encoded_value = self
                    .parse_value_by_type(&inner_type, &field_values[0], depth)
                    .await?;
                result.extend(encoded_value);
                Ok(result)
            },
            _ => Err(CliError::CommandArgumentError(format!(
                "Unknown Option variant '{}'. Expected 'None' or 'Some'.",
                variant
            ))),
        }
    }

    /// Parse primitive numeric types (U8, U16, U32, U64, U128, U256, I8, I16, I32, I64, I128, I256).
    fn parse_primitive_number(
        &self,
        type_name: &str,
        value: &JsonValue,
    ) -> CliTypedResult<Vec<u8>> {
        match type_name {
            "u8" => {
                let v = parse_number::<u8>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("u8", e))
            },
            "u16" => {
                let v = parse_number::<u16>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("u16", e))
            },
            "u32" => {
                let v = parse_number::<u32>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("u32", e))
            },
            "u64" => {
                let v = parse_number::<u64>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("u64", e))
            },
            "u128" => {
                let v = parse_number::<u128>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("u128", e))
            },
            "u256" => {
                let v = parse_u256(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("u256", e))
            },
            "i8" => {
                let v = parse_number::<i8>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("i8", e))
            },
            "i16" => {
                let v = parse_number::<i16>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("i16", e))
            },
            "i32" => {
                let v = parse_number::<i32>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("i32", e))
            },
            "i64" => {
                let v = parse_number::<i64>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("i64", e))
            },
            "i128" => {
                let v = parse_number::<i128>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("i128", e))
            },
            _ => Err(CliError::CommandArgumentError(format!(
                "Unknown numeric type: {}",
                type_name
            ))),
        }
    }

    /// Parse address from JSON string value.
    fn parse_address(&self, value: &JsonValue) -> CliTypedResult<Vec<u8>> {
        let addr_str = value.as_str().ok_or_else(|| {
            CliError::UnableToParse("address", format!("expected string, got {}", value))
        })?;
        let addr = load_account_arg(addr_str)
            .map_err(|e| CliError::UnableToParse("address", e.to_string()))?;
        bcs::to_bytes(&addr).map_err(|e| CliError::BCS("address", e))
    }

    /// Parse vector type recursively.
    #[async_recursion]
    async fn parse_vector(
        &self,
        items: &MoveType,
        value: &JsonValue,
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        let array = value.as_array().ok_or_else(|| {
            CliError::UnableToParse("vector", format!("expected array, got {}", value))
        })?;

        let mut result = Vec::new();
        // Encode vector length
        encode_uleb128(array.len() as u64, &mut result);

        // Encode each element (increment depth for nested vectors)
        for elem in array {
            let encoded = self.parse_value_by_type(items, elem, depth + 1).await?;
            result.extend(encoded);
        }

        Ok(result)
    }

    /// Parse struct types with special handling for framework types.
    ///
    /// Handles:
    /// - Option<T>: Delegated to parse_option_value
    /// - String (0x1::string::String): UTF-8 string encoding
    /// - Object<T> (0x1::object::Object): Address wrapper
    /// - FixedPoint32/64: Numeric encoding
    /// - Regular structs: Field-by-field parsing
    #[async_recursion]
    async fn parse_struct(
        &self,
        struct_tag: &MoveStructTag,
        value: &JsonValue,
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        // Build qualified name for special type checking
        let qualified_name = format!(
            "{}{}{}{}{}",
            struct_tag.address,
            MODULE_SEPARATOR,
            struct_tag.module,
            MODULE_SEPARATOR,
            struct_tag.name
        );

        // Convert MoveStructTag to StructTag for further processing
        let tag: StructTag = struct_tag.try_into()?;

        // Special handling for Option<T> - can appear as nested field type
        if tag.is_option() {
            return self.parse_option_value(&tag, value, depth).await;
        }

        // Special handling for well-known framework types.
        //
        // These types from std/aptos_std require special parsing logic that differs
        // from generic struct handling:
        // - String (0x1::string::String): UTF-8 encoded string, not a generic struct
        // - Object (0x1::object::Object<T>): Address wrapper with phantom type parameter
        //
        // TODO: Consider a more systematic registration mechanism for special types
        // as the framework evolves. Potential approaches:
        // 1. Annotation-based: Mark special types in framework with #[special_parsing]
        // 2. Plugin-based: Allow framework to register custom parsers
        // 3. ABI extension: Add parsing hints to module ABI
        match qualified_name.as_str() {
            STRING_TYPE_STR => {
                // String: parse as JSON string and BCS encode it
                let s = value.as_str().ok_or_else(|| {
                    CliError::UnableToParse("string", format!("expected string, got {}", value))
                })?;
                bcs::to_bytes(s).map_err(|e| CliError::BCS("string", e))
            },
            OBJECT_TYPE_STR => {
                // Object<T>: parse as address
                let addr_str = value.as_str().ok_or_else(|| {
                    CliError::UnableToParse(
                        "object",
                        format!("expected address string, got {}", value),
                    )
                })?;
                let addr = load_account_arg(addr_str)
                    .map_err(|e| CliError::UnableToParse("object address", e.to_string()))?;
                bcs::to_bytes(&addr).map_err(|e| CliError::BCS("object", e))
            },
            FIXED_POINT32_TYPE_STR => {
                // FixedPoint32: parse as u64
                let v = parse_number::<u64>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("fixed_point32", e))
            },
            FIXED_POINT64_TYPE_STR => {
                // FixedPoint64: parse as u128
                let v = parse_number::<u128>(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("fixed_point64", e))
            },
            _ => {
                // Could be a regular struct or a nested enum (e.g., a struct field, vector
                // element, or Option<T> inner type that is itself an enum). Check on-chain.
                let is_enum = self.is_enum_type(&tag).await?;

                if is_enum {
                    // Enum: expect {"VariantName": {fields...}}
                    let obj = value.as_object().ok_or_else(|| {
                        CliError::UnableToParse(
                            "enum",
                            format!(
                                "expected {{\"VariantName\": {{fields}}}} for enum {}, got {}",
                                tag.name, value
                            ),
                        )
                    })?;
                    if obj.len() == 1 {
                        let (variant_name, variant_fields_value) = obj.iter().next().unwrap();
                        if let Some(fields_obj) = variant_fields_value.as_object() {
                            return self
                                .construct_enum_argument(&tag, variant_name, fields_obj, depth + 1)
                                .await;
                        }
                    }
                    Err(CliError::CommandArgumentError(format!(
                        "Invalid enum value for type {}. Expected {{\"VariantName\": {{fields}}}}, got {}",
                        tag.name, value
                    )))
                } else {
                    // Regular struct: parse as JSON object with named fields
                    let obj = value.as_object().ok_or_else(|| {
                        CliError::UnableToParse("struct", format!("expected object, got {}", value))
                    })?;
                    self.construct_struct_argument(&tag, obj, depth + 1).await
                }
            },
        }
    }

    /// Parse a value based on its Move type and encode to BCS.
    ///
    /// This is the core parsing logic that handles all Move types recursively.
    /// It dispatches to specialized handlers for different type categories.
    ///
    /// Uses `#[async_recursion]` to enable simple async fn syntax while supporting
    /// recursive calls for nested types (vectors, structs, enums).
    #[async_recursion]
    pub async fn parse_value_by_type(
        &self,
        move_type: &MoveType,
        value: &JsonValue,
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        // Check nesting depth limit
        if depth > MAX_NESTING_DEPTH {
            return Err(CliError::CommandArgumentError(format!(
                "Nesting depth {} exceeds maximum allowed depth of {}. \
                 This limit applies to nested structs, enums, and vectors.",
                depth, MAX_NESTING_DEPTH
            )));
        }

        match move_type {
            MoveType::Bool => {
                let v = value.as_bool().ok_or_else(|| {
                    CliError::UnableToParse("bool", format!("expected boolean, got {}", value))
                })?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("bool", e))
            },
            MoveType::U8 => self.parse_primitive_number("u8", value),
            MoveType::U16 => self.parse_primitive_number("u16", value),
            MoveType::U32 => self.parse_primitive_number("u32", value),
            MoveType::U64 => self.parse_primitive_number("u64", value),
            MoveType::U128 => self.parse_primitive_number("u128", value),
            MoveType::U256 => self.parse_primitive_number("u256", value),
            MoveType::I8 => self.parse_primitive_number("i8", value),
            MoveType::I16 => self.parse_primitive_number("i16", value),
            MoveType::I32 => self.parse_primitive_number("i32", value),
            MoveType::I64 => self.parse_primitive_number("i64", value),
            MoveType::I128 => self.parse_primitive_number("i128", value),
            MoveType::I256 => {
                let v = parse_i256(value)?;
                bcs::to_bytes(&v).map_err(|e| CliError::BCS("i256", e))
            },
            MoveType::Address => self.parse_address(value),
            MoveType::Signer => Err(CliError::CommandArgumentError(
                "Signer type not allowed in transaction arguments".to_string(),
            )),
            MoveType::Vector { items } => self.parse_vector(items, value, depth).await,
            MoveType::Struct(struct_tag) => self.parse_struct(struct_tag, value, depth).await,
            MoveType::GenericTypeParam { index } => Err(CliError::CommandArgumentError(format!(
                "Unresolved generic type parameter T{}",
                index
            ))),
            MoveType::Reference { .. } => Err(CliError::CommandArgumentError(
                "Reference types not allowed in transaction arguments".to_string(),
            )),
            _ => Err(CliError::CommandArgumentError(format!(
                "Unsupported type: {:?}",
                move_type
            ))),
        }
    }
}

/// Substitute generic type parameters in a field type.
fn substitute_type_params(
    field_type: &MoveType,
    struct_tag: &StructTag,
) -> CliTypedResult<MoveType> {
    match field_type {
        MoveType::GenericTypeParam { index } => {
            if (*index as usize) < struct_tag.type_args.len() {
                let type_arg = &struct_tag.type_args[*index as usize];
                Ok(MoveType::from(type_arg))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Type parameter index {} out of bounds",
                    index
                )))
            }
        },
        MoveType::Vector { items } => {
            let substituted = substitute_type_params(items, struct_tag)?;
            Ok(MoveType::Vector {
                items: Box::new(substituted),
            })
        },
        MoveType::Struct(s) => {
            // Recursively substitute type parameters in nested struct
            let mut new_generic_type_params = Vec::new();
            for arg in &s.generic_type_params {
                let substituted = substitute_type_params(arg, struct_tag)?;
                new_generic_type_params.push(substituted);
            }
            Ok(MoveType::Struct(MoveStructTag {
                address: s.address,
                module: s.module.clone(),
                name: s.name.clone(),
                generic_type_params: new_generic_type_params,
            }))
        },
        _ => Ok(field_type.clone()),
    }
}

/// Encode a u64 value as ULEB128 (Variable-length encoding).
/// This is a thin wrapper around the shared write_u64_as_uleb128 function.
fn encode_uleb128(value: u64, output: &mut Vec<u8>) {
    super::write_u64_as_uleb128(output, value as usize);
}

/// Parse a JSON value as a number type.
///
/// Handles both string and number JSON types for maximum flexibility.
/// Note: String and number cases are intentionally handled differently:
/// - String case: uses as_str() with error checking for consistency with other string parsing
/// - Number case: uses to_string() because JSON numbers need conversion to string for FromStr
fn parse_number<T: FromStr>(value: &JsonValue) -> CliTypedResult<T>
where
    <T as FromStr>::Err: std::fmt::Display,
{
    let temp_string;
    let s = if value.is_string() {
        value.as_str().ok_or_else(|| {
            CliError::UnableToParse(
                std::any::type_name::<T>(),
                format!("failed to extract string from JSON value: {}", value),
            )
        })?
    } else if value.is_number() {
        // to_string() is necessary here: JSON number values must be converted to string
        // Store in temp_string to extend the temporary's lifetime
        temp_string = value.to_string();
        &temp_string
    } else {
        return Err(CliError::UnableToParse(
            std::any::type_name::<T>(),
            format!("expected number or string, got {}", value),
        ));
    };

    T::from_str(s).map_err(|e| CliError::UnableToParse(std::any::type_name::<T>(), e.to_string()))
}

/// Parse a U256 from JSON.
fn parse_u256(value: &JsonValue) -> CliTypedResult<U256> {
    let s = value.as_str().ok_or_else(|| {
        CliError::UnableToParse("u256", format!("expected string, got {}", value))
    })?;
    U256::from_str(s).map_err(|e| CliError::UnableToParse("u256", e.to_string()))
}

/// Parse an I256 from JSON.
fn parse_i256(value: &JsonValue) -> CliTypedResult<I256> {
    let s = value.as_str().ok_or_else(|| {
        CliError::UnableToParse("i256", format!("expected string, got {}", value))
    })?;
    I256::from_str(s).map_err(|e| CliError::UnableToParse("i256", e.to_string()))
}

/// Convert FieldDefinition to MoveStructField using CompiledModule.
fn convert_field_to_move_struct_field(
    module: &CompiledModule,
    field_def: &move_binary_format::file_format::FieldDefinition,
) -> MoveStructField {
    MoveStructField {
        name: ModuleAccess::identifier_at(module, field_def.name)
            .to_owned()
            .into(),
        typ: convert_signature_token_to_move_type(module, &field_def.signature.0),
    }
}

/// Helper to create MoveStructTag from struct handle index and optional type arguments.
fn create_move_struct_tag(
    module: &CompiledModule,
    idx: move_binary_format::file_format::StructHandleIndex,
    type_args: &[move_binary_format::file_format::SignatureToken],
) -> MoveStructTag {
    let handle = ModuleAccess::struct_handle_at(module, idx);
    let module_handle = ModuleAccess::module_handle_at(module, handle.module);
    MoveStructTag {
        address: (*ModuleAccess::address_identifier_at(module, module_handle.address)).into(),
        module: ModuleAccess::identifier_at(module, module_handle.name)
            .to_owned()
            .into(),
        name: ModuleAccess::identifier_at(module, handle.name)
            .to_owned()
            .into(),
        generic_type_params: type_args
            .iter()
            .map(|t| convert_signature_token_to_move_type(module, t))
            .collect(),
    }
}

/// Convert SignatureToken to MoveType using CompiledModule for lookups.
fn convert_signature_token_to_move_type(
    module: &CompiledModule,
    token: &move_binary_format::file_format::SignatureToken,
) -> MoveType {
    use move_binary_format::file_format::SignatureToken;

    match token {
        SignatureToken::Bool => MoveType::Bool,
        SignatureToken::U8 => MoveType::U8,
        SignatureToken::U16 => MoveType::U16,
        SignatureToken::U32 => MoveType::U32,
        SignatureToken::U64 => MoveType::U64,
        SignatureToken::U128 => MoveType::U128,
        SignatureToken::U256 => MoveType::U256,
        SignatureToken::I8 => MoveType::I8,
        SignatureToken::I16 => MoveType::I16,
        SignatureToken::I32 => MoveType::I32,
        SignatureToken::I64 => MoveType::I64,
        SignatureToken::I128 => MoveType::I128,
        SignatureToken::I256 => MoveType::I256,
        SignatureToken::Address => MoveType::Address,
        SignatureToken::Signer => MoveType::Signer,
        SignatureToken::Vector(inner) => MoveType::Vector {
            items: Box::new(convert_signature_token_to_move_type(module, inner)),
        },
        SignatureToken::Struct(idx) => MoveType::Struct(create_move_struct_tag(module, *idx, &[])),
        SignatureToken::StructInstantiation(idx, type_args) => {
            MoveType::Struct(create_move_struct_tag(module, *idx, type_args))
        },
        SignatureToken::TypeParameter(idx) => MoveType::GenericTypeParam { index: *idx },
        SignatureToken::Reference(inner) => MoveType::Reference {
            mutable: false,
            to: Box::new(convert_signature_token_to_move_type(module, inner)),
        },
        SignatureToken::MutableReference(inner) => MoveType::Reference {
            mutable: true,
            to: Box::new(convert_signature_token_to_move_type(module, inner)),
        },
        SignatureToken::Function(args, results, abilities) => MoveType::Function {
            args: args
                .iter()
                .map(|t| convert_signature_token_to_move_type(module, t))
                .collect(),
            results: results
                .iter()
                .map(|t| convert_signature_token_to_move_type(module, t))
                .collect(),
            abilities: *abilities,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::language_storage::{OPTION_MODULE_NAME_STR, OPTION_STRUCT_NAME_STR};

    #[test]
    fn test_parse_type_string() {
        // Test simple struct
        let result = StructTag::from_str("0x1::option::Option")
            .map_err(|e| CliError::CommandArgumentError(format!("Invalid type string: {}", e)));
        assert!(result.is_ok());

        // Test generic struct
        let result = StructTag::from_str("0x1::option::Option<u64>")
            .map_err(|e| CliError::CommandArgumentError(format!("Invalid type string: {}", e)));
        assert!(result.is_ok());
        let tag = result.unwrap();
        assert_eq!(tag.module.as_str(), OPTION_MODULE_NAME_STR);
        assert_eq!(tag.name.as_str(), OPTION_STRUCT_NAME_STR);
        assert_eq!(tag.type_args.len(), 1);
    }

    #[test]
    fn test_encode_uleb128() {
        let mut output = Vec::new();
        encode_uleb128(0, &mut output);
        assert_eq!(output, vec![0]);

        let mut output = Vec::new();
        encode_uleb128(127, &mut output);
        assert_eq!(output, vec![127]);

        let mut output = Vec::new();
        encode_uleb128(128, &mut output);
        assert_eq!(output, vec![0x80, 0x01]);
    }

    #[test]
    fn test_parse_number() {
        let value = serde_json::json!("123");
        let result = parse_number::<u64>(&value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);

        let value = serde_json::json!(123);
        let result = parse_number::<u64>(&value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[test]
    fn test_type_args_preserved_with_struct_args() {
        use crate::move_types::EntryFunctionArgumentsJSON;

        // Test JSON with type_args and struct arguments
        let json_str = r#"{
            "function_id": "0x1::test::test_generic",
            "type_args": ["u64", "address"],
            "args": [
                {
                    "type": "u64",
                    "value": "100"
                }
            ]
        }"#;

        let parsed: EntryFunctionArgumentsJSON = serde_json::from_str(json_str).unwrap();

        // Verify type_args are parsed correctly
        assert_eq!(parsed.type_args.len(), 2);
        assert_eq!(parsed.type_args[0], "u64");
        assert_eq!(parsed.type_args[1], "address");

        // Verify function_id is parsed
        assert_eq!(parsed.function_id, "0x1::test::test_generic");
    }

    #[test]
    fn test_option_variant_format() {
        use crate::move_types::EntryFunctionArgumentsJSON;

        // Test Option::Some with enum format (single key with variant name)
        let json_some = r#"{
            "function_id": "0x1::test::test_option",
            "type_args": [],
            "args": [
                {
                    "type": "0x1::option::Option<u64>",
                    "value": {
                        "Some": {"0": "100"}
                    }
                }
            ]
        }"#;

        let parsed: EntryFunctionArgumentsJSON = serde_json::from_str(json_some).unwrap();
        assert_eq!(parsed.args.len(), 1);
        assert_eq!(parsed.args[0].arg_type, "0x1::option::Option<u64>");
        // Verify the value is an object with a single key "Some"
        let value_obj = parsed.args[0].value.as_object().unwrap();
        assert_eq!(value_obj.len(), 1);
        assert!(value_obj.contains_key("Some"));

        // Test Option::None with enum format
        let json_none = r#"{
            "function_id": "0x1::test::test_option",
            "type_args": [],
            "args": [
                {
                    "type": "0x1::option::Option<u64>",
                    "value": {
                        "None": {}
                    }
                }
            ]
        }"#;

        let parsed: EntryFunctionArgumentsJSON = serde_json::from_str(json_none).unwrap();
        assert_eq!(parsed.args.len(), 1);
        assert_eq!(parsed.args[0].arg_type, "0x1::option::Option<u64>");
        // Verify the value is an object with a single key "None"
        let value_obj = parsed.args[0].value.as_object().unwrap();
        assert_eq!(value_obj.len(), 1);
        assert!(value_obj.contains_key("None"));
    }

    #[test]
    fn test_option_vector_format() {
        use crate::move_types::EntryFunctionArgumentsJSON;

        // Test Option with vector format: [value] for Some
        let json_some = r#"{
            "function_id": "0x1::test::test_option",
            "type_args": [],
            "args": [
                {
                    "type": "0x1::option::Option<u64>",
                    "value": ["100"]
                }
            ]
        }"#;

        let parsed: EntryFunctionArgumentsJSON = serde_json::from_str(json_some).unwrap();
        assert_eq!(parsed.args.len(), 1);
        assert!(parsed.args[0].value.is_array());
        assert_eq!(parsed.args[0].value.as_array().unwrap().len(), 1);

        // Test Option with vector format: [] for None
        let json_none = r#"{
            "function_id": "0x1::test::test_option",
            "type_args": [],
            "args": [
                {
                    "type": "0x1::option::Option<u64>",
                    "value": []
                }
            ]
        }"#;

        let parsed: EntryFunctionArgumentsJSON = serde_json::from_str(json_none).unwrap();
        assert_eq!(parsed.args.len(), 1);
        assert!(parsed.args[0].value.is_array());
        assert_eq!(parsed.args[0].value.as_array().unwrap().len(), 0);
    }
}
