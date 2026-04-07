// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Parser for struct and enum transaction arguments.
//!
//! This module enables the Aptos CLI to accept public copy structs and enums as transaction
//! arguments in JSON format, automatically encoding them to BCS without requiring manual encoding.
//!
//! ABI is parsed from the module bytecode returned by the REST API. Since the `abi` field
//! in `MoveModuleBytecode` uses `#[serde(skip_deserializing)]`, it is always `None` after
//! REST deserialization; instead, `try_parse_abi()` is called locally to derive the ABI
//! from the `bytecode` field.

use crate::commands::{FunctionArgType, MAX_VECTOR_DEPTH};
use aptos_api_types::{MoveModule, MoveModuleBytecode, MoveStructTag, MoveType};
use aptos_cli_common::{load_account_arg, CliError, CliTypedResult};
use aptos_rest_client::Client;
use async_recursion::async_recursion;
use move_core_types::language_storage::{ModuleId, StructTag, CORE_CODE_ADDRESS};
use serde_json::Value as JsonValue;
use std::sync::RwLock;

/// Parser for struct and enum arguments that queries on-chain module metadata
/// and encodes arguments to BCS format.
///
/// Includes a module ABI cache to avoid repeated REST API fetches for the same module.
pub struct StructArgParser {
    rest_client: Client,
    /// Cache of module ABIs keyed by ModuleId.
    /// Uses RwLock for thread-safe access (required since parser is shared across async tasks).
    module_cache: RwLock<ahash::AHashMap<ModuleId, MoveModule>>,
}

impl StructArgParser {
    /// Create a new parser with the given REST client.
    pub fn new(rest_client: Client) -> Self {
        Self {
            rest_client,
            module_cache: RwLock::new(ahash::AHashMap::new()),
        }
    }

    /// Convert Option<T> legacy array format to enum variant and fields.
    ///
    /// This is a helper to avoid duplicating the conversion logic across multiple locations.
    /// The legacy format uses:
    /// - [] for None
    /// - [value] for Some(value)
    ///
    /// Returns (variant_name, fields_map) where fields_map has "e" key for Some variant.
    pub fn convert_option_array_to_enum_format(
        array: &[JsonValue],
    ) -> CliTypedResult<(&'static str, serde_json::Map<String, JsonValue>)> {
        if array.is_empty() {
            Ok(("None", serde_json::Map::new()))
        } else if array.len() == 1 {
            let mut map = serde_json::Map::new();
            map.insert("e".to_string(), array[0].clone());
            Ok(("Some", map))
        } else {
            Err(CliError::CommandArgumentError(format!(
                "Option<T> as vector must have 0 or 1 elements, got {}",
                array.len()
            )))
        }
    }

    /// Verify that a struct exists on-chain via ABI and cache the module ABI.
    ///
    /// Requires the module to expose ABI (i.e. published with metadata). If ABI is absent,
    /// returns an error rather than falling back to bytecode deserialization.
    ///
    /// Uses a cache to avoid repeated REST API fetches of the same module.
    pub async fn verify_struct_exists(&self, struct_tag: &StructTag) -> CliTypedResult<()> {
        let module_id = ModuleId::new(struct_tag.address, struct_tag.module.clone());

        // Check cache first.
        {
            let cache_read = self.module_cache.read().map_err(|e| {
                CliError::CommandArgumentError(format!("Failed to acquire cache read lock: {}", e))
            })?;
            if let Some(abi) = cache_read.get(&module_id) {
                return if abi
                    .structs
                    .iter()
                    .any(|s| s.name.as_str() == struct_tag.name.as_str())
                {
                    Ok(())
                } else {
                    Err(CliError::CommandArgumentError(format!(
                        "Struct `{}` not found in module {}::{}",
                        struct_tag.name, struct_tag.address, struct_tag.module
                    )))
                };
            }
        }

        // Module not cached — fetch from REST API.
        // Note: `MoveModuleBytecode.abi` uses `#[serde(skip_deserializing)]` so it is always
        // `None` after REST deserialization. We call `try_parse_abi()` locally to derive the
        // ABI from the bytecode field instead.
        let module: MoveModuleBytecode = self
            .rest_client
            .get_account_module(struct_tag.address, struct_tag.module.as_str())
            .await
            .map_err(|e| {
                CliError::CommandArgumentError(format!(
                    "Failed to fetch module {}::{}: {}",
                    struct_tag.address, struct_tag.module, e
                ))
            })?
            .into_inner()
            .try_parse_abi()
            .map_err(|e| {
                CliError::CommandArgumentError(format!(
                    "Failed to parse ABI for module {}::{}: {}",
                    struct_tag.address, struct_tag.module, e
                ))
            })?;

        let abi = module.abi.ok_or_else(|| {
            CliError::CommandArgumentError(format!(
                "Module {}::{} does not have valid ABI.",
                struct_tag.address, struct_tag.module
            ))
        })?;

        // Verify the struct exists in the ABI.
        if !abi
            .structs
            .iter()
            .any(|s| s.name.as_str() == struct_tag.name.as_str())
        {
            return Err(CliError::CommandArgumentError(format!(
                "Struct `{}` not found in module {}::{}",
                struct_tag.name, struct_tag.address, struct_tag.module
            )));
        }

        // Cache the ABI.
        self.module_cache
            .write()
            .map_err(|e| {
                CliError::CommandArgumentError(format!("Failed to acquire cache write lock: {}", e))
            })?
            .insert(module_id, abi);

        Ok(())
    }

    /// Retrieve the cached ABI for the module containing `struct_tag`.
    ///
    /// # Precondition
    /// `verify_struct_exists` must have been called first to populate the cache.
    fn get_cached_abi(&self, struct_tag: &StructTag) -> CliTypedResult<MoveModule> {
        let module_id = ModuleId::new(struct_tag.address, struct_tag.module.clone());
        self.module_cache
            .read()
            .map_err(|e| {
                CliError::CommandArgumentError(format!("Failed to acquire cache read lock: {}", e))
            })?
            .get(&module_id)
            .cloned()
            .ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Module {}::{} not found in cache. verify_struct_exists must be called first.",
                    struct_tag.address, struct_tag.module
                ))
            })
    }

    /// Check nesting depth limit to prevent stack overflow.
    fn check_depth(depth: u8, type_name: &str) -> CliTypedResult<()> {
        if depth > MAX_VECTOR_DEPTH {
            return Err(CliError::CommandArgumentError(format!(
                "`{}` nesting depth {} exceeds maximum allowed depth of {}",
                type_name, depth, MAX_VECTOR_DEPTH
            )));
        }
        Ok(())
    }

    /// Check if a StructTag represents an enum type (has variants).
    ///
    /// Uses the ABI `is_enum` flag directly — no bytecode deserialization required.
    ///
    /// # Precondition
    /// `verify_struct_exists` must have been called first to ensure the module is cached.
    ///
    /// # Returns
    /// - `Ok(true)` if the type is an enum
    /// - `Ok(false)` if the type is a struct
    pub async fn is_enum_type(&self, struct_tag: &StructTag) -> CliTypedResult<bool> {
        self.verify_struct_exists(struct_tag).await?;
        let abi = self.get_cached_abi(struct_tag)?;
        let struct_def = abi
            .structs
            .iter()
            .find(|s| s.name.as_str() == struct_tag.name.as_str())
            .ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Type `{}` not found in ABI of module {}::{}",
                    struct_tag.name, struct_tag.address, struct_tag.module
                ))
            })?;
        Ok(struct_def.is_enum)
    }

    /// Parse Option<T> value which can be in two formats:
    /// 1. Legacy array format: [] for None, [value] for Some(value)
    /// 2. New enum format: {"None": {}} or {"Some": {"e": value}}
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
                    "Expected array for `Option` type, got: {}",
                    value
                ))
            })?;
            let (variant, fields_map) = Self::convert_option_array_to_enum_format(array)?;
            self.construct_enum_argument(struct_tag, variant, &fields_map, depth)
                .await
        } else if value.is_object() {
            // New enum format: {"None": {}} or {"Some": {"e": value}}
            let obj = value.as_object().ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Expected object for `Option` enum format, got: {}",
                    value
                ))
            })?;
            if obj.len() == 1 {
                let (variant_name, variant_fields) = obj.iter().next().ok_or_else(|| {
                    CliError::CommandArgumentError(
                        "Unexpected empty object for `Option`".to_string(),
                    )
                })?;
                if let Some(fields_obj) = variant_fields.as_object() {
                    return self
                        .construct_enum_argument(struct_tag, variant_name, fields_obj, depth)
                        .await;
                }
            }
            Err(CliError::CommandArgumentError(format!(
                "Invalid `Option` format. Expected {{\"None\": {{}}}} or {{\"Some\": {{\"e\": value}}}}, got `{}`",
                value
            )))
        } else {
            Err(CliError::CommandArgumentError(format!(
                "Invalid `Option` value. Expected array or object, got {}",
                value
            )))
        }
    }

    /// Construct a struct argument by parsing fields and encoding to BCS.
    ///
    /// Uses the module ABI for field names and types. Only modules with ABI are supported.
    pub async fn construct_struct_argument(
        &self,
        struct_tag: &StructTag,
        field_values: &serde_json::Map<String, JsonValue>,
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        // Check nesting depth limit
        Self::check_depth(depth, "Struct")?;

        // Verify struct exists and populate cache
        self.verify_struct_exists(struct_tag).await?;

        let abi = self.get_cached_abi(struct_tag)?;
        let struct_def = abi
            .structs
            .iter()
            .find(|s| s.name.as_str() == struct_tag.name.as_str())
            .ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Struct `{}` not found in module {}::{}",
                    struct_tag.name, struct_tag.address, struct_tag.module
                ))
            })?;

        // Reject enums passed to the struct path.
        if struct_def.is_enum {
            return Err(CliError::CommandArgumentError(format!(
                "Type `{}` is an enum.",
                struct_tag.name
            )));
        }

        let fields = &struct_def.fields;

        // Validate that all provided fields exist in the struct definition.
        let expected_field_names: std::collections::BTreeSet<&str> =
            fields.iter().map(|f| f.name.as_str()).collect();

        for provided_field_name in field_values.keys() {
            if !expected_field_names.contains(provided_field_name.as_str()) {
                // List valid fields in ABI-defined order for a consistent error message.
                let valid_fields: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
                return Err(CliError::CommandArgumentError(format!(
                    "Unknown field `{}` for struct `{}`. Valid fields are: {}",
                    provided_field_name,
                    struct_tag.name,
                    valid_fields.join(", ")
                )));
            }
        }

        // Parse and encode each field in ABI-defined order.
        // Order matters for BCS encoding: fields must appear in the same order as
        // the struct definition. We iterate over `fields` from the ABI (not over the
        // user-supplied JSON keys) to preserve the correct canonical order.
        let mut encoded_fields = Vec::new();

        for field in fields {
            let field_name = field.name.as_str();
            let field_value = field_values.get(field_name).ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Missing field `{}` for struct `{}`",
                    field_name, struct_tag.name
                ))
            })?;

            let field_type = substitute_type_params(&field.typ, struct_tag, 0)?;
            let encoded_value = self
                .parse_value_by_type(&field_type, field_value, depth + 1)
                .await?;
            encoded_fields.extend(encoded_value);
        }

        Ok(encoded_fields)
    }

    /// Construct an enum argument by encoding variant index and fields.
    ///
    /// Uses the module ABI for variant names and field types. Only modules with ABI are supported.
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

        // Special handling for Option<T> for backward compatibility (uses vector encoding).
        // Check full module path to ensure it's std::option::Option, not a custom enum named "Option".
        if struct_tag.is_option() {
            // Convert field_values map to array for Option.
            // Branch on variant first so "Some" with 0 fields gets a clear error immediately
            // rather than being silently passed downstream as an empty array.
            let fields_array = if variant == "None" {
                if !field_values.is_empty() {
                    return Err(CliError::CommandArgumentError(
                        "Option::None should not have any fields".to_string(),
                    ));
                }
                vec![]
            } else if variant == "Some" {
                // For Option::Some, expect a single field named "e"
                if field_values.len() != 1 {
                    return Err(CliError::CommandArgumentError(format!(
                        "Option::Some requires exactly 1 field named \"e\", got {}",
                        field_values.len()
                    )));
                }

                let field_value = field_values.get("e").ok_or_else(|| {
                    let actual_field = field_values
                        .keys()
                        .next()
                        .expect("field_values has exactly one key (checked above)");
                    CliError::CommandArgumentError(format!(
                        "Option::Some field must be named \"e\", got \"{}\"",
                        actual_field
                    ))
                })?;

                vec![field_value.clone()]
            } else {
                return Err(CliError::CommandArgumentError(format!(
                    "Unknown Option variant `{}`. Expected `None` or `Some`.",
                    variant
                )));
            };
            return self
                .construct_option_argument_from_array(struct_tag, variant, &fields_array, depth)
                .await;
        }

        // Verify enum exists and populate cache.
        self.verify_struct_exists(struct_tag).await?;

        let abi = self.get_cached_abi(struct_tag)?;
        let struct_def = abi
            .structs
            .iter()
            .find(|s| s.name.as_str() == struct_tag.name.as_str())
            .ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Type `{}` not found in ABI of module {}::{}",
                    struct_tag.name, struct_tag.address, struct_tag.module
                ))
            })?;

        if !struct_def.is_enum {
            return Err(CliError::StructNotEnumError(struct_tag.name.to_string()));
        }

        if struct_def.variants.is_empty() {
            return Err(CliError::CommandArgumentError(format!(
                "Enum `{}` has no variants in ABI. The node may not support enum ABI.",
                struct_tag.name
            )));
        }

        let variants = &struct_def.variants;

        // Find the variant by name and get its index.
        let (variant_index, variant_def) = variants
            .iter()
            .enumerate()
            .find(|(_, v)| v.name.as_str() == variant)
            .ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Variant `{}` not found in enum `{}::{}::{}`. Available variants: {}",
                    variant,
                    struct_tag.address,
                    struct_tag.module,
                    struct_tag.name,
                    variants
                        .iter()
                        .map(|v| v.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;

        // Start encoding: variant index (ULEB128) + fields.
        let mut encoded = Vec::new();
        encode_uleb128(variant_index as u64, &mut encoded);

        // Validate that all provided fields exist in the variant definition.
        let expected_field_names: std::collections::BTreeSet<&str> =
            variant_def.fields.iter().map(|f| f.name.as_str()).collect();

        for provided_field_name in field_values.keys() {
            if !expected_field_names.contains(provided_field_name.as_str()) {
                // List valid fields in ABI-defined order for a consistent error message.
                let valid_fields: Vec<&str> =
                    variant_def.fields.iter().map(|f| f.name.as_str()).collect();
                return Err(CliError::CommandArgumentError(format!(
                    "Unknown field `{}` for variant `{}::{}`. Valid fields are: {}",
                    provided_field_name,
                    struct_tag.name,
                    variant,
                    valid_fields.join(", ")
                )));
            }
        }

        // Parse and encode each field.
        // Use depth + 1 so that the variant's fields are one level deeper than the enum
        // container itself, consistent with construct_option_argument_from_array which
        // similarly parses the inner value at depth + 1.
        for field in &variant_def.fields {
            let field_value = field_values.get(field.name.as_str()).ok_or_else(|| {
                CliError::CommandArgumentError(format!(
                    "Missing field `{}` for variant `{}::{}`",
                    field.name, struct_tag.name, variant
                ))
            })?;
            let field_type = substitute_type_params(&field.typ, struct_tag, 0)?;
            let encoded_value = self
                .parse_value_by_type(&field_type, field_value, depth + 1)
                .await?;
            encoded.extend(encoded_value);
        }

        Ok(encoded)
    }

    /// Construct an Option<T> argument using vector encoding for backward compatibility.
    ///
    /// Depth tracking: `depth` is the depth of the Option container itself. The inner
    /// value is parsed at `depth + 1`, consistent with how `parse_vector` increments depth
    /// for vector elements. This mirrors the pattern:
    ///   - `parse_vector(depth)` → `parse_value_by_type(depth + 1)` for each element
    ///   - `construct_option_argument_from_array(depth)` → `parse_value_by_type(depth + 1)` for Some value
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
                    .parse_value_by_type(&inner_type, &field_values[0], depth + 1)
                    .await?;
                result.extend(encoded_value);
                Ok(result)
            },
            _ => Err(CliError::CommandArgumentError(format!(
                "Unknown Option variant `{}`. Expected `None` or `Some`.",
                variant
            ))),
        }
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
    /// - Regular structs: Field-by-field parsing via ABI
    /// - Enums: Variant-index + field encoding via ABI
    #[async_recursion]
    async fn parse_struct(
        &self,
        struct_tag: &MoveStructTag,
        value: &JsonValue,
        depth: u8,
    ) -> CliTypedResult<Vec<u8>> {
        // Convert MoveStructTag to StructTag for further processing
        let tag: StructTag = struct_tag.try_into()?;

        // Special handling for Option<T> - can appear as nested field type
        if tag.is_option() {
            return self.parse_option_value(&tag, value, depth).await;
        }

        // Use component-wise comparison for robustness — avoids relying on the
        // Address Display implementation producing a specific short form (e.g. "0x1").
        let is_0x1 = tag.address == CORE_CODE_ADDRESS;
        let module = tag.module.as_str();
        let name = tag.name.as_str();

        // Special handling for well-known framework types.
        //
        // These types from std/aptos_std require special parsing logic that differs
        // from generic struct handling:
        // - String (0x1::string::String): UTF-8 encoded string, not a generic struct
        // - Object (0x1::object::Object<T>): Address wrapper with phantom type parameter
        if is_0x1 && module == "string" && name == "String" {
            // String: parse as JSON string and BCS encode it
            let s = value.as_str().ok_or_else(|| {
                CliError::UnableToParse("string", format!("expected string, got {}", value))
            })?;
            bcs::to_bytes(s).map_err(|e| CliError::BCS("string", e))
        } else if is_0x1 && module == "object" && name == "Object" {
            // Object<T>: parse as address
            let addr_str = value.as_str().ok_or_else(|| {
                CliError::UnableToParse("object", format!("expected address string, got {}", value))
            })?;
            let addr = load_account_arg(addr_str)
                .map_err(|e| CliError::UnableToParse("object address", e.to_string()))?;
            bcs::to_bytes(&addr).map_err(|e| CliError::BCS("object", e))
        } else if is_0x1 && module == "fixed_point32" && name == "FixedPoint32" {
            // FixedPoint32: parse as u64
            FunctionArgType::U64.parse_arg_json(value).map(|a| a.arg)
        } else if is_0x1 && module == "fixed_point64" && name == "FixedPoint64" {
            // FixedPoint64: parse as u128
            FunctionArgType::U128.parse_arg_json(value).map(|a| a.arg)
        } else {
            // Could be a regular struct or a nested enum (e.g., a struct field, vector
            // element, or Option<T> inner type that is itself an enum). Check on-chain.
            let is_enum = self.is_enum_type(&tag).await?;

            if is_enum {
                // Enum: expect {"VariantName": {fields...}}
                // For unit variants (no fields) use {"VariantName": {}}.
                let obj = value.as_object().ok_or_else(|| {
                    CliError::UnableToParse(
                        "enum",
                        format!(
                            "expected {{\"VariantName\": {{fields}}}} for enum `{}`, got {}",
                            tag.name, value
                        ),
                    )
                })?;
                if obj.len() != 1 {
                    return Err(CliError::CommandArgumentError(format!(
                        "Enum `{}` must have exactly one key (the variant name), got {}",
                        tag.name, value
                    )));
                }
                let (variant_name, variant_fields_value) = obj
                    .iter()
                    .next()
                    .expect("obj has exactly one entry (checked above)");
                let fields_obj = variant_fields_value.as_object().ok_or_else(|| {
                    CliError::CommandArgumentError(format!(
                        "Enum variant value must be a JSON object — \
                         use {{\"{}\":{{}}}} for a unit variant or \
                         {{\"{}\":{{\"field\":value}}}} for a variant with fields, got {}",
                        variant_name, variant_name, variant_fields_value
                    ))
                })?;
                return self
                    .construct_enum_argument(&tag, variant_name, fields_obj, depth)
                    .await;
            } else {
                // Regular struct: parse as JSON object with named fields
                let obj = value.as_object().ok_or_else(|| {
                    CliError::UnableToParse("struct", format!("expected object, got {}", value))
                })?;
                self.construct_struct_argument(&tag, obj, depth).await
            }
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
        if depth > MAX_VECTOR_DEPTH {
            return Err(CliError::CommandArgumentError(format!(
                "Nesting depth {} exceeds maximum allowed depth of {}. \
                 This limit applies to nested structs, enums, and vectors.",
                depth, MAX_VECTOR_DEPTH
            )));
        }

        match move_type {
            MoveType::Bool => FunctionArgType::Bool.parse_arg_json(value).map(|a| a.arg),
            MoveType::U8 => FunctionArgType::U8.parse_arg_json(value).map(|a| a.arg),
            MoveType::U16 => FunctionArgType::U16.parse_arg_json(value).map(|a| a.arg),
            MoveType::U32 => FunctionArgType::U32.parse_arg_json(value).map(|a| a.arg),
            MoveType::U64 => FunctionArgType::U64.parse_arg_json(value).map(|a| a.arg),
            MoveType::U128 => FunctionArgType::U128.parse_arg_json(value).map(|a| a.arg),
            MoveType::U256 => FunctionArgType::U256.parse_arg_json(value).map(|a| a.arg),
            MoveType::I8 => FunctionArgType::I8.parse_arg_json(value).map(|a| a.arg),
            MoveType::I16 => FunctionArgType::I16.parse_arg_json(value).map(|a| a.arg),
            MoveType::I32 => FunctionArgType::I32.parse_arg_json(value).map(|a| a.arg),
            MoveType::I64 => FunctionArgType::I64.parse_arg_json(value).map(|a| a.arg),
            MoveType::I128 => FunctionArgType::I128.parse_arg_json(value).map(|a| a.arg),
            MoveType::I256 => FunctionArgType::I256.parse_arg_json(value).map(|a| a.arg),
            MoveType::Address => FunctionArgType::Address
                .parse_arg_json(value)
                .map(|a| a.arg),
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
                "Unsupported type in transaction arguments: {:?}",
                move_type
            ))),
        }
    }
}

/// Substitute generic type parameters in a field type.
///
/// `depth` tracks the structural depth of the *type expression* (not the value-parsing depth).
/// ABI types come from an external REST API and cannot be fully trusted to be finitely nested,
/// so this function bounds recursion independently using `MAX_VECTOR_DEPTH`. Callers always
/// start at 0; recursive calls increment by 1 for each nested `vector` or type-parameter
/// position inside a struct.
fn substitute_type_params(
    field_type: &MoveType,
    struct_tag: &StructTag,
    depth: u8,
) -> CliTypedResult<MoveType> {
    if depth > MAX_VECTOR_DEPTH {
        return Err(CliError::CommandArgumentError(format!(
            "Type nesting depth {} exceeds maximum allowed depth of {}",
            depth, MAX_VECTOR_DEPTH
        )));
    }
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
            let substituted = substitute_type_params(items, struct_tag, depth + 1)?;
            Ok(MoveType::Vector {
                items: Box::new(substituted),
            })
        },
        MoveType::Struct(s) => {
            let mut new_generic_type_params = Vec::new();
            for arg in &s.generic_type_params {
                let substituted = substitute_type_params(arg, struct_tag, depth + 1)?;
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
fn encode_uleb128(value: u64, output: &mut Vec<u8>) {
    let mut buf = move_binary_format::file_format_common::BinaryData::new();
    move_binary_format::file_format_common::write_u64_as_uleb128(&mut buf, value)
        .expect("ULEB128 encoding should not fail for valid u64");
    output.extend_from_slice(buf.as_inner());
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::language_storage::{OPTION_MODULE_NAME_STR, OPTION_STRUCT_NAME_STR};
    use std::str::FromStr;

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
                        "Some": {"e": "100"}
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
