// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module implements validation for struct API attributes.
//! It ensures that functions with struct API attributes (Pack, PackVariant, Unpack, UnpackVariant,
//! TestVariant, BorrowFieldImmutable, BorrowFieldMutable) are correctly named and typed.
//!
//! ## Validation Rules
//!
//! ### Phase 1: Name/Attribute Correspondence
//! 1. Function name matches struct API pattern ↔ has corresponding struct API attribute (bidirectional)
//! 2. Attribute type must match name pattern (e.g., pack$S requires Pack attribute)
//! 3. Only one struct API attribute allowed per function
//!
//! ### Phase 2: Implementation Validation
//!
//! **Name Parsing:**
//! - Struct name must exist locally in the module
//! - Variant name must exist in the enum (for variant operations)
//! - Field offset must be within valid bounds
//! - Type order must be valid (for variant field operations)
//!
//! **Signature Validation:**
//! - Pack: parameters match struct field types in order, returns struct
//! - PackVariant: parameters match variant field types in order, returns struct
//! - Unpack: parameter is struct, returns field types in order
//! - UnpackVariant: parameter is struct, returns variant field types in order
//! - TestVariant: parameter is &struct (no need to check return type)
//! - BorrowField: parameter is &/&mut struct, returns &/&mut FieldType with matching mutability
//!
//! **Index/Offset Validation:**
//! - Borrow field: name offset must equal attribute offset
//! - Variant operations: name variant index must equal attribute variant index
//!
//! **Bytecode Pattern Validation:**
//! - Pack: MoveLoc* + Pack + Ret
//! - PackVariant: MoveLoc* + PackVariant + Ret (bytecode variant index must equal attribute)
//! - Unpack: MoveLoc + Unpack + Ret
//! - UnpackVariant: MoveLoc + UnpackVariant + Ret (bytecode variant index must equal attribute)
//! - TestVariant: MoveLoc + TestVariant + Ret (bytecode variant index must equal attribute)
//! - BorrowField: MoveLoc + ImmBorrowField/MutBorrowField + Ret

use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CodeUnit, CompiledModule, FunctionAttribute, FunctionDefinition, FunctionHandle,
        MemberCount, SignatureToken, StructFieldInformation, StructHandleIndex, VariantDefinition,
    },
};
use move_core_types::{
    language_storage::{
        BORROW, BORROW_MUT, PACK, PACK_VARIANT, PUBLIC_STRUCT_DELIMITER, TEST_VARIANT, UNPACK,
        UNPACK_VARIANT,
    },
    vm_status::StatusCode,
};
use std::collections::{BTreeMap, HashMap};

/// Pre-computed metadata for efficient struct API validation.
/// This context is computed once per module and reused for all function validations,
pub struct StructApiContext {
    /// Map from struct name to struct handle index
    struct_name_to_handle: HashMap<String, StructHandleIndex>,

    /// Map from struct name to struct definition index
    struct_name_to_def: HashMap<String, usize>,

    /// Pre-computed type order maps for each enum, keyed by struct name.
    /// Maps (offset, type) to type_order.
    /// Only populated for enums (structs with DeclaredVariants).
    enum_type_order_maps: HashMap<String, BTreeMap<(u16, SignatureToken), u16>>,

    /// Pre-computed variant indices for each enum, keyed by struct name.
    /// Maps (offset, type) to list of variant indices that have that field type at that offset.
    /// Only populated for enums (structs with DeclaredVariants).
    enum_variant_indices_maps: HashMap<String, BTreeMap<(u16, SignatureToken), Vec<u16>>>,
}

impl StructApiContext {
    /// Build the context once for the entire module.
    pub fn new(module: &CompiledModule) -> Self {
        let mut struct_name_to_handle = HashMap::new();
        let mut struct_name_to_def = HashMap::new();
        let mut enum_type_order_maps = HashMap::new();
        let mut enum_variant_indices_maps = HashMap::new();

        // Build struct definition map and enum type order maps.
        // Only iterate through struct definitions (locally defined structs) to avoid
        // name collisions with imported structs. Struct API functions can only be
        // defined for locally defined structs.
        for (idx, def) in module.struct_defs().iter().enumerate() {
            let handle = module.struct_handle_at(def.struct_handle);
            let name = module.identifier_at(handle.name).as_str().to_string();

            // Store both the struct handle index and definition index for this struct.
            // Using def.struct_handle ensures the handle and def always refer to the same struct.
            struct_name_to_handle.insert(name.clone(), def.struct_handle);
            struct_name_to_def.insert(name.clone(), idx);

            // If this is an enum, pre-compute its type order map and variant indices map
            if let StructFieldInformation::DeclaredVariants(variants) = &def.field_information {
                let (type_order_map, variant_indices_map) =
                    build_variant_type_order_and_indices_map(variants);
                enum_type_order_maps.insert(name.clone(), type_order_map);
                enum_variant_indices_maps.insert(name, variant_indices_map);
            }
        }

        Self {
            struct_name_to_handle,
            struct_name_to_def,
            enum_type_order_maps,
            enum_variant_indices_maps,
        }
    }

    /// Get struct handle by name
    pub fn get_struct_handle(&self, name: &str) -> Option<StructHandleIndex> {
        self.struct_name_to_handle.get(name).copied()
    }

    /// Get struct definition index by name
    pub fn get_struct_def_index(&self, name: &str) -> Option<usize> {
        self.struct_name_to_def.get(name).copied()
    }

    /// Get pre-computed type order map for an enum
    pub fn get_type_order_map(
        &self,
        enum_name: &str,
    ) -> Option<&BTreeMap<(u16, SignatureToken), u16>> {
        self.enum_type_order_maps.get(enum_name)
    }

    /// Get pre-computed variant indices map for an enum
    pub fn get_variant_indices_map(
        &self,
        enum_name: &str,
    ) -> Option<&BTreeMap<(u16, SignatureToken), Vec<u16>>> {
        self.enum_variant_indices_maps.get(enum_name)
    }
}

/// Extract struct API attribute from a function's attributes.
/// Returns an error if more than one struct API attribute is present.
/// Returns Ok(None) if no struct API attribute is present.
/// Returns Ok(Some(attr)) if exactly one struct API attribute is present.
fn try_get_struct_api_attr(
    attrs: &[FunctionAttribute],
) -> PartialVMResult<Option<FunctionAttribute>> {
    use FunctionAttribute::*;
    let mut count: usize = 0;
    let mut attr_attribute = None;
    for attr in attrs {
        let is_struct_api_attr = match attr {
            Pack
            | PackVariant(_)
            | Unpack
            | UnpackVariant(_)
            | TestVariant(_)
            | BorrowFieldImmutable(_)
            | BorrowFieldMutable(_) => {
                attr_attribute = Some(attr.clone());
                true
            },
            Persistent | ModuleLock => false,
        };
        if is_struct_api_attr {
            count += 1;
            if count > 1 {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
        }
    }
    Ok(attr_attribute)
}

/// Validated struct API name information.
/// Contains only the parsed and validated components that are actually used in Phase 2 validation.
#[derive(Debug, Clone)]
enum StructApiNameInfo {
    Pack {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: usize,
    },
    PackVariant {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: usize,
        variant_name: String,
    },
    Unpack {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: usize,
    },
    UnpackVariant {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: usize,
        variant_name: String,
    },
    TestVariant {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: usize,
        variant_name: String,
    },
    BorrowField {
        struct_name: String,
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: usize,
        offset: u16,
        type_order: Option<u16>,
        is_mutable: bool,
    },
}

impl StructApiNameInfo {
    /// Get the expected attribute type string for this struct API.
    fn expected_attr_type(&self) -> &str {
        match self {
            StructApiNameInfo::Pack { .. } => PACK,
            StructApiNameInfo::PackVariant { .. } => PACK_VARIANT,
            StructApiNameInfo::Unpack { .. } => UNPACK,
            StructApiNameInfo::UnpackVariant { .. } => UNPACK_VARIANT,
            StructApiNameInfo::TestVariant { .. } => TEST_VARIANT,
            StructApiNameInfo::BorrowField { is_mutable, .. } => {
                if *is_mutable {
                    BORROW_MUT
                } else {
                    BORROW
                }
            },
        }
    }
}

/// Parse and validate a struct API function name.
///
/// This function performs comprehensive validation:
/// 1. Parses the function name pattern
/// 2. Validates that the struct exists in the module
/// 3. Validates variant index/name (for variant-based APIs)
/// 4. Validates offset and type_order (for borrow field APIs)
///
/// Returns:
/// - Ok(Some(info)) if the name is a valid struct API function name
/// - Ok(None) if the name doesn't match any struct API pattern
/// - Err if the name looks like a struct API but validation fails
fn parse_and_validate_struct_api_name(
    function_name: &str,
    ctx: &StructApiContext,
) -> PartialVMResult<Option<StructApiNameInfo>> {
    let parts: Vec<&str> = function_name.split(PUBLIC_STRUCT_DELIMITER).collect();

    // Need at least 2 parts for any struct API function (e.g., "pack$S")
    if parts.len() < 2 {
        return Ok(None);
    }

    let prefix = parts[0];
    let struct_name = parts[1];

    // Check if this looks like a struct API prefix
    let is_struct_api_prefix = matches!(prefix, PACK | UNPACK | TEST_VARIANT | BORROW | BORROW_MUT);

    if !is_struct_api_prefix {
        return Ok(None);
    }

    // Check if the number of parts makes sense for this prefix
    // This validates the STRUCTURE before we validate struct existence
    let valid_parts_count = match prefix {
        PACK | UNPACK => parts.len() == 2 || parts.len() == 3,
        TEST_VARIANT => parts.len() == 3,
        BORROW | BORROW_MUT => parts.len() == 3 || parts.len() == 4,
        _ => false,
    };

    if !valid_parts_count {
        // Pattern doesn't match - this is just a regular function, not a struct API
        return Ok(None);
    }

    // Pattern matches - now validate that struct exists
    let Some(struct_handle_idx) = ctx.get_struct_handle(struct_name) else {
        // Looks like struct API but struct doesn't exist - this is an error
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    };

    let struct_def_idx = ctx
        .get_struct_def_index(struct_name)
        .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

    // Helper to parse offset and type_order for borrow field APIs
    // Only does syntactic validation (parsing), not semantic validation
    let parse_borrow_field_components =
        |offset_str: &str, type_order_str: Option<&str>| -> PartialVMResult<(u16, Option<u16>)> {
            let offset: u16 = offset_str
                .parse()
                .map_err(|_| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

            let type_order = if let Some(to_str) = type_order_str {
                Some(
                    to_str
                        .parse()
                        .map_err(|_| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?,
                )
            } else {
                None
            };

            // Note: We do NOT validate semantic correctness here (e.g., whether type_order
            // is appropriate for the struct type). That will be done in Phase 2.
            // This allows us to parse the name and detect that it's a struct API pattern,
            // even if the specific values are semantically incorrect.

            Ok((offset, type_order))
        };

    // Helper to parse borrow field API (both mutable and immutable)
    let parse_borrow_field_api = |is_mutable: bool| -> PartialVMResult<Option<StructApiNameInfo>> {
        if parts.len() == 3 {
            // borrow[_mut]$S$offset
            let (offset, type_order) = parse_borrow_field_components(parts[2], None)?;
            Ok(Some(StructApiNameInfo::BorrowField {
                struct_name: struct_name.to_string(),
                struct_handle_idx,
                struct_def_idx,
                offset,
                type_order,
                is_mutable,
            }))
        } else if parts.len() == 4 {
            // borrow[$mut]$S$offset$type_order
            let (offset, type_order) = parse_borrow_field_components(parts[2], Some(parts[3]))?;
            Ok(Some(StructApiNameInfo::BorrowField {
                struct_name: struct_name.to_string(),
                struct_handle_idx,
                struct_def_idx,
                offset,
                type_order,
                is_mutable,
            }))
        } else {
            // due to valid_parts_count, unreachable
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
        }
    };

    // Parse and validate based on prefix
    match prefix {
        PACK => {
            if parts.len() == 2 {
                // pack$S
                Ok(Some(StructApiNameInfo::Pack {
                    struct_handle_idx,
                    struct_def_idx,
                }))
            } else if parts.len() == 3 {
                // pack$S$Variant
                let variant_name = parts[2];
                Ok(Some(StructApiNameInfo::PackVariant {
                    struct_handle_idx,
                    struct_def_idx,
                    variant_name: variant_name.to_string(),
                }))
            } else {
                // due to valid_parts_count, unreachable
                Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
            }
        },
        UNPACK => {
            if parts.len() == 2 {
                // unpack$S
                Ok(Some(StructApiNameInfo::Unpack {
                    struct_handle_idx,
                    struct_def_idx,
                }))
            } else if parts.len() == 3 {
                // unpack$S$Variant
                let variant_name = parts[2];
                Ok(Some(StructApiNameInfo::UnpackVariant {
                    struct_handle_idx,
                    struct_def_idx,
                    variant_name: variant_name.to_string(),
                }))
            } else {
                // due to valid_parts_count, unreachable
                Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
            }
        },
        TEST_VARIANT => {
            if parts.len() == 3 {
                // test_variant$S$Variant
                let variant_name = parts[2];
                Ok(Some(StructApiNameInfo::TestVariant {
                    struct_handle_idx,
                    struct_def_idx,
                    variant_name: variant_name.to_string(),
                }))
            } else {
                // due to valid_parts_count, unreachable
                Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
            }
        },
        BORROW => parse_borrow_field_api(false),
        BORROW_MUT => parse_borrow_field_api(true),
        _ => Ok(None),
    }
}

/// Validate that the parameter type matches the StructName type.
/// Used for unpack$StructName and test_variant$StructName$VariantName functions.
/// - Unpack/UnpackVariant: parameter must be the struct by value (Struct or StructInstantiation)
/// - TestVariant: parameter must be an immutable reference (&Struct)
fn validate_struct_parameter_type(
    module: &CompiledModule,
    handle: &FunctionHandle,
    attr: &FunctionAttribute,
    struct_handle_idx: StructHandleIndex,
) -> PartialVMResult<()> {
    // Get the parameter signature
    let param_sig = module.signature_at(handle.parameters);

    // Must have exactly one parameter
    if param_sig.0.len() != 1 {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check parameter type based on attribute:
    // - Unpack/UnpackVariant: must be struct by value
    // - TestVariant: must be immutable reference to struct
    let param_type = &param_sig.0[0];

    match attr {
        FunctionAttribute::TestVariant(_) => {
            // TestVariant must take immutable reference (&Struct)
            match param_type {
                SignatureToken::Reference(inner) => match **inner {
                    SignatureToken::Struct(idx) if idx == struct_handle_idx => Ok(()),
                    SignatureToken::StructInstantiation(idx, _) if idx == struct_handle_idx => {
                        Ok(())
                    },
                    _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
                },
                _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
            }
        },
        FunctionAttribute::Unpack | FunctionAttribute::UnpackVariant(_) => {
            // Unpack/UnpackVariant must take struct by value (not a reference)
            match param_type {
                SignatureToken::Struct(idx) if *idx == struct_handle_idx => Ok(()),
                SignatureToken::StructInstantiation(idx, _) if *idx == struct_handle_idx => Ok(()),
                _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
            }
        },
        _ => {
            // Should not reach here - only Unpack/UnpackVariant/TestVariant call this function
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
        },
    }
}

/// Validate that the variant index in PackVariant/UnpackVariant/TestVariant attribute matches the variant name
/// parsed from the function name.
fn validate_variant_index(
    module: &CompiledModule,
    struct_def_idx: usize,
    variant_name: &str,
    variant_index: u16,
) -> PartialVMResult<()> {
    let struct_def = &module.struct_defs()[struct_def_idx];

    // Get the variants from the struct definition
    let variants = match &struct_def.field_information {
        StructFieldInformation::DeclaredVariants(variants) => variants,
        _ => {
            // Struct doesn't have variants, but PackVariant attribute is present
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        },
    };

    // Find the variant by name and check its index
    let found_variant_index = variants
        .iter()
        .enumerate()
        .find(|(_, variant)| {
            let name = module.identifier_at(variant.name).as_str();
            name == variant_name
        })
        .map(|(idx, _)| idx as u16);

    match found_variant_index {
        Some(idx) if idx == variant_index => Ok(()),
        _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
    }
}

/// Build mappings for enum variants:
/// 1. (offset, type) -> type_order: assigns unique order to each distinct (offset, type) pair
/// 2. (offset, type) -> [variant_idx]: collects all variants that have this field type at this offset
///
/// This mirrors the logic in construct_map_for_borrow_field_api_with_type from module_generator.
///
/// For enums with multiple variants, fields at the same offset may have different types.
/// We assign a unique type_order to each distinct (offset, type) pair encountered.
/// The ordering follows the order in which variants and their fields are declared.
///
/// IMPORTANT: The variant indices in the returned map are guaranteed to be in ascending order
/// because we iterate through variants sequentially and push indices in order.
///
/// Example:
/// ```move
/// enum Color {
///   Red { r: u8 },      // offset 0, type u8 -> type_order 0, variants [0]
///   Green { g: u16 },   // offset 0, type u16 -> type_order 1, variants [1]
///   Blue { b: u8 },     // offset 0, type u8 -> already type_order 0, variants [0, 2]
/// }
/// ```
/// Note: variants [0, 2] are in ascending order
fn build_variant_type_order_and_indices_map(
    variants: &[VariantDefinition],
) -> (
    std::collections::BTreeMap<(u16, SignatureToken), u16>,
    std::collections::BTreeMap<(u16, SignatureToken), Vec<u16>>,
) {
    use std::collections::BTreeMap;

    let mut order_map: BTreeMap<(u16, SignatureToken), u16> = BTreeMap::new();
    let mut variant_indices_map: BTreeMap<(u16, SignatureToken), Vec<u16>> = BTreeMap::new();
    let mut next_order = 0u16;

    for (variant_idx, variant) in variants.iter().enumerate() {
        for (field_offset, field) in variant.fields.iter().enumerate() {
            let field_type = field.signature.0.clone();
            let field_offset = field_offset as u16;
            let key = (field_offset, field_type.clone());

            // Only assign type_order if this (offset, type) pair hasn't been seen before
            if !order_map.contains_key(&key) {
                order_map.insert(key.clone(), next_order);
                next_order += 1;
            }

            // Add this variant index to the list for this (offset, type) combination
            variant_indices_map
                .entry(key)
                .or_default()
                .push(variant_idx as u16);
        }
    }

    (order_map, variant_indices_map)
}

/// Validate the parameter of a borrow field function.
/// The parameter must be a reference to the correct struct type with matching mutability.
fn validate_borrow_param(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_handle_idx: StructHandleIndex,
    is_mutable: bool,
) -> PartialVMResult<()> {
    let param_sig = module.signature_at(handle.parameters);

    // Must have exactly one parameter
    if param_sig.0.len() != 1 {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Extract struct index and mutability from the reference
    let param_type = &param_sig.0[0];
    let param_info = match param_type {
        SignatureToken::Reference(inner) => match **inner {
            SignatureToken::Struct(idx) => Some((idx, false)),
            SignatureToken::StructInstantiation(idx, _) => Some((idx, false)),
            _ => None,
        },
        SignatureToken::MutableReference(inner) => match **inner {
            SignatureToken::Struct(idx) => Some((idx, true)),
            SignatureToken::StructInstantiation(idx, _) => Some((idx, true)),
            _ => None,
        },
        _ => None,
    };

    match param_info {
        Some((idx, param_is_mut)) => {
            // Verify the reference points to the correct struct
            if idx != struct_handle_idx {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            // Verify mutability matches: borrow requires &S, borrow_mut requires &mut S
            if is_mutable != param_is_mut {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            Ok(())
        },
        None => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
    }
}

/// Extract the inner type from a reference return type.
/// Returns the type that the reference points to, checking that mutability matches expectations.
fn extract_return_inner_type(
    return_sig: &[SignatureToken],
    is_mutable: bool,
) -> PartialVMResult<&SignatureToken> {
    // Must return exactly one value
    if return_sig.len() != 1 {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Extract and validate the reference type
    match &return_sig[0] {
        SignatureToken::Reference(inner) if !is_mutable => Ok(&**inner),
        SignatureToken::MutableReference(inner) if is_mutable => Ok(&**inner),
        _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
    }
}

/// Validate that borrow field function has correct parameter and return types.
///
/// Checks:
/// 1. Parameter is a reference to the struct (&S or &mut S) with correct mutability
/// 2. Return type is a reference (&FieldType or &mut FieldType) with correct mutability
/// 3. Return type matches the specific field type at the given offset
/// 4. For variants: type_order determines which field type to validate against
/// 5. Offset is within valid bounds for the struct/variant
fn validate_borrow_field_types(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_name: &str,
    struct_handle_idx: StructHandleIndex,
    struct_def_idx: usize,
    offset: u16,
    type_order: Option<u16>,
    is_mutable: bool,
    ctx: &StructApiContext,
) -> PartialVMResult<()> {
    // Validate parameter: must be a reference to the struct with correct mutability
    validate_borrow_param(module, handle, struct_handle_idx, is_mutable)?;

    // Validate return type: must be a reference with correct mutability
    let return_sig = module.signature_at(handle.return_);
    let actual_return_type = extract_return_inner_type(&return_sig.0, is_mutable)?;

    let struct_def = &module.struct_defs()[struct_def_idx];

    // Validate field type based on whether this is a regular struct or variant
    match &struct_def.field_information {
        StructFieldInformation::Native => {
            // Native structs don't have accessible fields
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
        },

        StructFieldInformation::Declared(fields) => {
            // Regular struct: validate offset and check field type directly
            if (offset as usize) >= fields.len() {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }

            let expected_field_type = &fields[offset as usize].signature.0;
            if actual_return_type != expected_field_type {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }

            Ok(())
        },

        StructFieldInformation::DeclaredVariants(_) => {
            // Variant: require type_order in function name
            let type_order = type_order
                .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

            let order_map = ctx
                .get_type_order_map(struct_name)
                .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

            // Find the expected field type for this (offset, type_order) combination
            let expected_field_type = order_map
                .iter()
                .find(|((field_offset, _), order)| *field_offset == offset && **order == type_order)
                .map(|((_, field_type), _)| field_type)
                .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

            // Verify the return type matches the expected field type
            if actual_return_type != expected_field_type {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }

            Ok(())
        },
    }
}

/// Helper to validate that a signature's types match a list of field definitions.
/// Checks both count and type equality in order.
fn validate_signature_against_fields(
    sig: &move_binary_format::file_format::Signature,
    fields: &[move_binary_format::file_format::FieldDefinition],
) -> PartialVMResult<()> {
    // Signature count must match field count
    if sig.0.len() != fields.len() {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Each signature type must match the corresponding field type in order
    for (i, field) in fields.iter().enumerate() {
        if sig.0[i] != field.signature.0 {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        }
    }

    Ok(())
}

/// Helper to validate that signature types match struct or variant field types in order.
/// Used by both pack (validates parameters) and unpack (validates return types).
///
/// - For regular structs: validates against Declared fields
/// - For enums with variant_name: validates against specific variant's fields
/// - Rejects Native structs and mismatched struct/enum usage
fn validate_signature_matches_fields(
    module: &CompiledModule,
    sig_idx: move_binary_format::file_format::SignatureIndex,
    struct_def_idx: usize,
    variant_name: Option<&str>,
) -> PartialVMResult<()> {
    let sig = module.signature_at(sig_idx);
    let struct_def = &module.struct_defs()[struct_def_idx];

    match &struct_def.field_information {
        StructFieldInformation::Native => {
            // Native structs cannot be packed/unpacked
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
        },
        StructFieldInformation::Declared(fields) => {
            // Regular struct: variant operations not allowed
            if variant_name.is_some() {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }

            validate_signature_against_fields(sig, fields)
        },
        StructFieldInformation::DeclaredVariants(variants) => {
            // Enum: variant operations required
            let variant_name = variant_name
                .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

            // Find the variant by name
            let variant = variants
                .iter()
                .find(|v| {
                    let name = module.identifier_at(v.name);
                    name.as_str() == variant_name
                })
                .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

            validate_signature_against_fields(sig, &variant.fields)
        },
    }
}

/// Validate that the return type of pack$StructName matches the StructName type.
/// The function should return exactly one value of the struct type.
fn validate_pack_return_type(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_handle_idx: StructHandleIndex,
) -> PartialVMResult<()> {
    // Get the return signature
    let return_sig = module.signature_at(handle.return_);

    // Pack functions should return exactly one value
    if return_sig.0.len() != 1 {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check that the return type is the struct type
    let return_type = &return_sig.0[0];
    match return_type {
        SignatureToken::Struct(idx) => {
            if *idx != struct_handle_idx {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
        },
        SignatureToken::StructInstantiation(idx, _) => {
            if *idx != struct_handle_idx {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
        },
        _ => {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        },
    }

    Ok(())
}

/// Validate that the parameters of pack$StructName match the struct field types in order.
/// The function should take N parameters matching the N field types of the struct.
fn validate_pack_parameters(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_def_idx: usize,
) -> PartialVMResult<()> {
    validate_signature_matches_fields(module, handle.parameters, struct_def_idx, None)
}

/// Validate that the parameters of pack$StructName$VariantName match the variant field types in order.
/// The function should take N parameters matching the N field types of the specific variant.
fn validate_pack_variant_parameters(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_def_idx: usize,
    variant_name: &str,
) -> PartialVMResult<()> {
    validate_signature_matches_fields(
        module,
        handle.parameters,
        struct_def_idx,
        Some(variant_name),
    )
}

/// Validate that the return types of unpack$StructName match the struct field types in order.
/// The function should return a tuple of N values matching the N field types of the struct.
fn validate_unpack_return_types(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_def_idx: usize,
) -> PartialVMResult<()> {
    validate_signature_matches_fields(module, handle.return_, struct_def_idx, None)
}

/// Validate that the return types of unpack$StructName$VariantName match the variant field types in order.
/// The function should return a tuple of N values matching the N field types of the specific variant.
fn validate_unpack_variant_return_types(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_def_idx: usize,
    variant_name: &str,
) -> PartialVMResult<()> {
    validate_signature_matches_fields(module, handle.return_, struct_def_idx, Some(variant_name))
}

/// Helper function to check pack-like bytecode patterns.
/// Pattern:
/// MoveLoc(...)
/// <expected_bytecode>
/// Ret
fn pattern_check_for_pack_like<F>(code: &CodeUnit, is_expected_bytecode: F) -> PartialVMResult<()>
where
    F: Fn(&Bytecode) -> bool,
{
    if code.code.len() < 2 {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check last bytecode is Ret
    if !matches!(code.code[code.code.len() - 1], Bytecode::Ret) {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check second-to-last bytecode is the expected pack bytecode
    if !is_expected_bytecode(&code.code[code.code.len() - 2]) {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check all preceding bytecodes are MoveLoc
    for i in 0..code.code.len() - 2 {
        if !matches!(code.code[i], Bytecode::MoveLoc(_)) {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        }
    }

    Ok(())
}

/// Check the pattern of the pack API.
/// Pattern:
/// MoveLoc(...)
/// Pack(...) | PackGeneric(...)
/// Ret
fn pattern_check_for_pack(code: &CodeUnit) -> PartialVMResult<()> {
    pattern_check_for_pack_like(code, |bc| {
        // don't need to check the struct definition index because the return type is already checked.
        matches!(bc, Bytecode::Pack(_) | Bytecode::PackGeneric(_))
    })
}

/// Enum to represent either a direct variant handle index or a generic variant instantiation index
enum VariantIndexRef {
    Direct(move_binary_format::file_format::StructVariantHandleIndex),
    Generic(move_binary_format::file_format::StructVariantInstantiationIndex),
}

/// Helper to extract and validate variant index from bytecode.
/// Takes a matcher closure that identifies the relevant variant bytecode instructions
/// and returns the appropriate index type (direct or generic).
fn extract_and_validate_variant_index<F>(
    resolver: &BinaryIndexedView,
    expected_variant_idx: u16,
    code: &CodeUnit,
    matcher: F,
) -> PartialVMResult<()>
where
    F: Fn(&Bytecode) -> Option<VariantIndexRef>,
{
    // Find the variant instruction and extract its index
    let mut bytecode_variant_idx = None;
    for bc in &code.code {
        if let Some(idx_ref) = matcher(bc) {
            bytecode_variant_idx = Some(match idx_ref {
                VariantIndexRef::Direct(idx) => {
                    // For non-generic variants, resolve directly
                    resolver.struct_variant_handle_at(idx)?.variant
                },
                VariantIndexRef::Generic(idx) => {
                    // For generic variants, resolve through instantiation
                    let inst = resolver.struct_variant_instantiation_at(idx)?;
                    resolver.struct_variant_handle_at(inst.handle)?.variant
                },
            });
            break;
        }
    }

    // Ensure we found a variant instruction
    let bytecode_variant_idx = bytecode_variant_idx
        .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

    // Validate the extracted index matches the expected index
    if bytecode_variant_idx != expected_variant_idx {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    Ok(())
}

/// Check the pattern of the pack variant API.
/// Pattern:
/// MoveLoc(...)
/// PackVariant(...) | PackVariantGeneric(...)
/// Ret
/// Also validates that the variant index in the bytecode matches the expected variant index.
fn pattern_check_for_pack_variant(
    resolver: &BinaryIndexedView,
    expected_variant_idx: u16,
    code: &CodeUnit,
) -> PartialVMResult<()> {
    // Check the basic pattern
    pattern_check_for_pack_like(code, |bc| {
        matches!(
            bc,
            Bytecode::PackVariant(_) | Bytecode::PackVariantGeneric(_)
        )
    })?;

    // Extract and validate variant index
    extract_and_validate_variant_index(resolver, expected_variant_idx, code, |bc| match bc {
        Bytecode::PackVariant(idx) => Some(VariantIndexRef::Direct(*idx)),
        Bytecode::PackVariantGeneric(idx) => Some(VariantIndexRef::Generic(*idx)),
        _ => None,
    })
}

/// Helper function to check simple 3-bytecode operation patterns.
/// Pattern:
/// MoveLoc(...)
/// <expected_operation>
/// Ret
fn pattern_check_for_simple_operation<F>(
    code: &CodeUnit,
    is_expected_operation: F,
) -> PartialVMResult<()>
where
    F: Fn(&Bytecode) -> bool,
{
    if code.code.len() != 3 {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check first bytecode is MoveLoc
    if !matches!(code.code[0], Bytecode::MoveLoc(_)) {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check second bytecode is the expected operation
    if !is_expected_operation(&code.code[1]) {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check third bytecode is Ret
    if !matches!(code.code[2], Bytecode::Ret) {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    Ok(())
}

/// Check the pattern of the unpack API.
/// Pattern:
/// MoveLoc(...)
/// Unpack(...) | UnpackGeneric(...)
/// Ret
fn pattern_check_for_unpack(code: &CodeUnit) -> PartialVMResult<()> {
    pattern_check_for_simple_operation(code, |bc| {
        // don't need to check the struct definition index because the parameter type is already checked.
        matches!(bc, Bytecode::Unpack(_) | Bytecode::UnpackGeneric(_))
    })
}

/// Check the pattern of the unpack variant API.
/// Pattern:
/// MoveLoc(...)
/// UnpackVariant(...) | UnpackVariantGeneric(...)
/// Ret
/// Also validates that the variant index in the bytecode matches the expected variant index.
fn pattern_check_for_unpack_variant(
    resolver: &BinaryIndexedView,
    expected_variant_idx: u16,
    code: &CodeUnit,
) -> PartialVMResult<()> {
    // Check the basic pattern
    pattern_check_for_simple_operation(code, |bc| {
        matches!(
            bc,
            Bytecode::UnpackVariant(_) | Bytecode::UnpackVariantGeneric(_)
        )
    })?;

    // Extract and validate variant index
    extract_and_validate_variant_index(resolver, expected_variant_idx, code, |bc| match bc {
        Bytecode::UnpackVariant(idx) => Some(VariantIndexRef::Direct(*idx)),
        Bytecode::UnpackVariantGeneric(idx) => Some(VariantIndexRef::Generic(*idx)),
        _ => None,
    })
}

/// Check the pattern of the test variant API.
/// Pattern:
/// MoveLoc(...)
/// TestVariant(...) | TestVariantGeneric(...)
/// Ret
/// Also validates that the variant index in the bytecode matches the expected variant index.
fn pattern_check_for_test_variant(
    resolver: &BinaryIndexedView,
    expected_variant_idx: u16,
    code: &CodeUnit,
) -> PartialVMResult<()> {
    // Check the basic pattern
    pattern_check_for_simple_operation(code, |bc| {
        matches!(
            bc,
            Bytecode::TestVariant(_) | Bytecode::TestVariantGeneric(_)
        )
    })?;

    // Extract and validate variant index
    extract_and_validate_variant_index(resolver, expected_variant_idx, code, |bc| match bc {
        Bytecode::TestVariant(idx) => Some(VariantIndexRef::Direct(*idx)),
        Bytecode::TestVariantGeneric(idx) => Some(VariantIndexRef::Generic(*idx)),
        _ => None,
    })
}

/// Extract field offset from a borrow field bytecode and check mutability.
/// Returns the field offset if the bytecode is a valid borrow field operation
/// with the expected mutability, or an error otherwise.
fn get_borrow_field_offset(
    resolver: &BinaryIndexedView,
    bytecode: &Bytecode,
    expected_is_mutable: bool,
) -> PartialVMResult<MemberCount> {
    match bytecode {
        Bytecode::ImmBorrowField(fh) => {
            if expected_is_mutable {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            Ok(resolver.field_handle_at(*fh)?.field)
        },
        Bytecode::MutBorrowField(fh) => {
            if !expected_is_mutable {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            Ok(resolver.field_handle_at(*fh)?.field)
        },
        Bytecode::ImmBorrowFieldGeneric(fi) => {
            if expected_is_mutable {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            let inst = resolver.field_instantiation_at(*fi)?;
            Ok(resolver.field_handle_at(inst.handle)?.field)
        },
        Bytecode::MutBorrowFieldGeneric(fi) => {
            if !expected_is_mutable {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            let inst = resolver.field_instantiation_at(*fi)?;
            Ok(resolver.field_handle_at(inst.handle)?.field)
        },
        Bytecode::ImmBorrowVariantField(vfh) => {
            if expected_is_mutable {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            Ok(resolver.variant_field_handle_at(*vfh)?.field)
        },
        Bytecode::MutBorrowVariantField(vfh) => {
            if !expected_is_mutable {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            Ok(resolver.variant_field_handle_at(*vfh)?.field)
        },
        Bytecode::ImmBorrowVariantFieldGeneric(vfi) => {
            if expected_is_mutable {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            let inst = resolver.variant_field_instantiation_at(*vfi)?;
            Ok(resolver.variant_field_handle_at(inst.handle)?.field)
        },
        Bytecode::MutBorrowVariantFieldGeneric(vfi) => {
            if !expected_is_mutable {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
            let inst = resolver.variant_field_instantiation_at(*vfi)?;
            Ok(resolver.variant_field_handle_at(inst.handle)?.field)
        },
        _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
    }
}

/// Validates that a BorrowVariantField instruction contains the complete and ordered
/// set of variants for the given offset and type.
///
/// For BorrowVariantField instructions, the VariantFieldHandle.variants must contain:
/// 1. ALL variants that have a field at the given offset with the same type (completeness)
/// 2. The variants must be in ascending order by their variant index (ordering)
///
/// This function uses the pre-computed variant_indices_map from StructApiContext to efficiently
/// validate variant completeness without recomputing the variant list.
fn validate_variant_field_completeness(
    resolver: &BinaryIndexedView,
    module: &CompiledModule,
    bytecode: &Bytecode,
    ctx: &StructApiContext,
) -> PartialVMResult<()> {
    // Extract variant field handle based on bytecode type
    let variant_field_handle = match bytecode {
        Bytecode::ImmBorrowVariantField(vfh) | Bytecode::MutBorrowVariantField(vfh) => {
            resolver.variant_field_handle_at(*vfh)?
        },
        Bytecode::ImmBorrowVariantFieldGeneric(vfi)
        | Bytecode::MutBorrowVariantFieldGeneric(vfi) => {
            let inst = resolver.variant_field_instantiation_at(*vfi)?;
            resolver.variant_field_handle_at(inst.handle)?
        },
        _ => return Ok(()), // Not a variant field borrow, nothing to check
    };

    let struct_def = resolver.struct_def_at(variant_field_handle.struct_index)?;

    // Get the enum variants
    let enum_variants = match &struct_def.field_information {
        StructFieldInformation::DeclaredVariants(variants) => variants,
        _ => return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)), // Should be enum
    };

    // Get struct name to look up the pre-computed variant indices map
    let struct_handle = module.struct_handle_at(struct_def.struct_handle);
    let struct_name = module.identifier_at(struct_handle.name).as_str();

    let variant_indices_map = ctx
        .get_variant_indices_map(struct_name)
        .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

    let offset = variant_field_handle.field;
    let actual_variants = &variant_field_handle.variants;

    if actual_variants.is_empty() {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Get the type for the first variant in actual_variants to determine which type group
    let first_variant_idx = actual_variants[0];
    let first_variant_def = &enum_variants[first_variant_idx as usize];
    let first_variant_fields = &first_variant_def.fields;

    if (offset as usize) >= first_variant_fields.len() {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    let expected_type = &first_variant_fields[offset as usize].signature.0;

    // Look up the expected variants from the pre-computed map
    // Note: expected_variants is guaranteed to be in ascending order by construction
    // (see build_variant_type_order_and_indices_map)
    let expected_variants = variant_indices_map
        .get(&(offset, expected_type.clone()))
        .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))?;

    // Check both completeness and ordering in one comparison:
    // Since expected_variants is already sorted in ascending order, we just need to
    // verify that actual_variants exactly matches expected_variants.
    if actual_variants != expected_variants.as_slice() {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    Ok(())
}

/// Check the pattern of the borrow field API.
/// Pattern:
/// MoveLoc(...)
/// if immut is true:
///     ImmBorrowField(...) | ImmBorrowFieldGeneric(...) | ImmBorrowVariantField(...) | ImmBorrowVariantFieldGeneric(...)
/// else:
///     MutBorrowField(...) | MutBorrowFieldGeneric(...) | MutBorrowVariantField(...) | MutBorrowVariantFieldGeneric(...)
/// Ret
fn pattern_check_for_borrow_field(
    immut: bool,
    resolver: &BinaryIndexedView,
    module: &CompiledModule,
    offset: &MemberCount,
    code: &CodeUnit,
    ctx: &StructApiContext,
) -> PartialVMResult<()> {
    if code.code.len() != 3 {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Check pattern: MoveLoc, BorrowField*, Ret
    if !matches!(code.code[0], Bytecode::MoveLoc(_)) {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }
    if !matches!(code.code[2], Bytecode::Ret) {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // Validate borrow field bytecode and check offset
    let actual_offset = get_borrow_field_offset(resolver, &code.code[1], !immut)?;
    if actual_offset != *offset {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    }

    // For variant field borrows, validate completeness and ordering of variants
    validate_variant_field_completeness(resolver, module, &code.code[1], ctx)?;

    Ok(())
}

/// Check well-formedness of struct API attributes.
///
/// Validation is performed in two phases:
///
/// Phase 1: Check name/attribute correspondence
/// - If function name looks like a struct API function:
///   - Must have the corresponding struct API attribute
///   - Attribute type must match the name pattern
/// - If function name does NOT look like a struct API function:
///   - Must NOT have any struct API attribute
///
/// Phase 2: Validate struct API implementation
/// - Validate that the function name matches the expected pattern for the attribute
/// - Validate parameter types, return types, variant indices
/// - Validate bytecode pattern implementation
///
/// Note: Struct operations can only happen in the module where the struct is defined.
pub fn check_struct_api_impl(
    resolver: &BinaryIndexedView,
    module: &CompiledModule,
    function_definition: &FunctionDefinition,
    ctx: &StructApiContext,
) -> PartialVMResult<()> {
    let handle = module.function_handle_at(function_definition.function);
    let function_name = module.identifier_at(handle.name).as_str();

    // ========================================================================
    // Phase 1: Parse and validate function name, check attribute correspondence
    // ========================================================================

    // Parse and comprehensively validate the function name
    // This validates: struct exists, variant index valid, offset valid, type_order valid
    let name_info = parse_and_validate_struct_api_name(function_name, ctx)?;

    // Get the actual attribute (if any)
    let struct_api_attr = try_get_struct_api_attr(&handle.attributes)?;

    // Enforce 1-to-1 relationship: both directions must match
    let info = match (name_info, struct_api_attr.as_ref()) {
        (Some(info), Some(attr)) => {
            // Both name and attribute present - validate they match

            // Get the attribute type string
            let actual_type = match attr {
                FunctionAttribute::Pack => PACK,
                FunctionAttribute::PackVariant(_) => PACK_VARIANT,
                FunctionAttribute::Unpack => UNPACK,
                FunctionAttribute::UnpackVariant(_) => UNPACK_VARIANT,
                FunctionAttribute::TestVariant(_) => TEST_VARIANT,
                FunctionAttribute::BorrowFieldImmutable(_) => BORROW,
                FunctionAttribute::BorrowFieldMutable(_) => BORROW_MUT,
                _ => return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
            };

            // Verify attribute type matches the name pattern
            if info.expected_attr_type() != actual_type {
                // ERROR: Attribute type doesn't match name pattern
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }

            // Both match - proceed to Phase 2 validation
            info
        },
        (Some(_), None) => {
            // ERROR: Name matches struct API pattern but no attribute present
            // This enforces: Name → Attribute
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        },
        (None, Some(_)) => {
            // ERROR: Has struct API attribute but name doesn't match pattern
            // This enforces: Attribute → Name
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        },
        (None, None) => {
            // Neither name nor attribute matches struct API - regular function, OK
            return Ok(());
        },
    };

    // ========================================================================
    // Phase 2: Validate struct API implementation
    // ========================================================================
    // At this point, we know:
    // - Function name has been fully parsed and validated (Phase 1)
    // - Struct exists, variant exists (if applicable), offset/type_order parsed
    // - Corresponding attribute is present and matches the name pattern
    // Now validate the implementation details: signature types and bytecode patterns.

    let attr = struct_api_attr.unwrap(); // Safe: we checked above

    // Validate signature types based on the parsed name information
    match &info {
        StructApiNameInfo::Pack {
            struct_handle_idx,
            struct_def_idx,
        } => {
            // Validate both parameters and return type for pack
            validate_pack_parameters(module, handle, *struct_def_idx)?;
            validate_pack_return_type(module, handle, *struct_handle_idx)?;
        },
        StructApiNameInfo::PackVariant {
            struct_handle_idx,
            struct_def_idx,
            variant_name,
        } => {
            // Validate both parameters and return type for pack_variant
            validate_pack_variant_parameters(module, handle, *struct_def_idx, variant_name)?;
            validate_pack_return_type(module, handle, *struct_handle_idx)?;
        },
        StructApiNameInfo::Unpack {
            struct_handle_idx,
            struct_def_idx,
        } => {
            // Validate both parameter and return types for unpack
            validate_struct_parameter_type(module, handle, &attr, *struct_handle_idx)?;
            validate_unpack_return_types(module, handle, *struct_def_idx)?;
        },
        StructApiNameInfo::UnpackVariant {
            struct_handle_idx,
            struct_def_idx,
            variant_name,
        } => {
            // Validate both parameter and return types for unpack_variant
            validate_struct_parameter_type(module, handle, &attr, *struct_handle_idx)?;
            validate_unpack_variant_return_types(module, handle, *struct_def_idx, variant_name)?;
        },
        StructApiNameInfo::TestVariant {
            struct_handle_idx, ..
        } => {
            // TestVariant only needs parameter validation (returns bool)
            validate_struct_parameter_type(module, handle, &attr, *struct_handle_idx)?;
        },
        StructApiNameInfo::BorrowField {
            struct_name,
            struct_handle_idx,
            struct_def_idx,
            offset,
            type_order,
            is_mutable,
        } => {
            validate_borrow_field_types(
                module,
                handle,
                struct_name,
                *struct_handle_idx,
                *struct_def_idx,
                *offset,
                *type_order,
                *is_mutable,
                ctx,
            )?;
        },
    }

    // Validate variant index for variant-based attributes
    match &info {
        StructApiNameInfo::PackVariant {
            struct_def_idx,
            variant_name,
            ..
        }
        | StructApiNameInfo::UnpackVariant {
            struct_def_idx,
            variant_name,
            ..
        }
        | StructApiNameInfo::TestVariant {
            struct_def_idx,
            variant_name,
            ..
        } => {
            // Get the attribute's variant_index for validation
            let attr_variant_index = match attr {
                FunctionAttribute::PackVariant(idx)
                | FunctionAttribute::UnpackVariant(idx)
                | FunctionAttribute::TestVariant(idx) => idx,
                _ => unreachable!(), // Already validated in Phase 1
            };
            validate_variant_index(module, *struct_def_idx, variant_name, attr_variant_index)?;
        },
        _ => {},
    }

    // Validate offset for borrow field attributes
    // The offset parsed from the function name must match the offset in the attribute
    if let StructApiNameInfo::BorrowField {
        offset: name_offset,
        ..
    } = &info
    {
        let attr_offset = match attr {
            FunctionAttribute::BorrowFieldImmutable(offset)
            | FunctionAttribute::BorrowFieldMutable(offset) => offset,
            _ => unreachable!(), // Already validated in Phase 1
        };
        if *name_offset != attr_offset {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        }
    }

    // Validate bytecode pattern implementation
    let Some(code) = function_definition.code.as_ref() else {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
    };

    match attr {
        FunctionAttribute::Pack => pattern_check_for_pack(code),
        FunctionAttribute::PackVariant(variant_idx) => {
            pattern_check_for_pack_variant(resolver, variant_idx, code)
        },
        FunctionAttribute::Unpack => pattern_check_for_unpack(code),
        FunctionAttribute::UnpackVariant(variant_idx) => {
            pattern_check_for_unpack_variant(resolver, variant_idx, code)
        },
        FunctionAttribute::TestVariant(variant_idx) => {
            pattern_check_for_test_variant(resolver, variant_idx, code)
        },
        FunctionAttribute::BorrowFieldImmutable(offset) => {
            pattern_check_for_borrow_field(true, resolver, module, &offset, code, ctx)
        },
        FunctionAttribute::BorrowFieldMutable(offset) => {
            pattern_check_for_borrow_field(false, resolver, module, &offset, code, ctx)
        },
        _ => Ok(()),
    }
}
