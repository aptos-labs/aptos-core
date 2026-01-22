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
//! The VM and SDK use struct API *function names* (e.g. `pack$S`) to discover struct API
//! wrappers at runtime. The struct API *attributes* (e.g. `#[pack]`) tell the verifier what
//! invariants to enforce on those wrappers. Requiring both to agree prevents two classes of
//! attack:
//! - A function with a struct API name but no attribute would bypass verifier checks entirely,
//!   letting hand-crafted bytecode masquerade as a legitimate wrapper.
//! - A function with an attribute but a non-API name would never be discovered by callers,
//!   making the attribute meaningless and potentially confusing future tooling.
//!
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
        MemberCount, SignatureToken, StructDefinitionIndex, StructFieldInformation,
        StructHandleIndex, VariantDefinition, VariantIndex,
    },
};
use move_core_types::{
    language_storage::{
        BORROW, BORROW_MUT, PACK, PACK_VARIANT, PUBLIC_STRUCT_DELIMITER, TEST_VARIANT, UNPACK,
        UNPACK_VARIANT,
    },
    vm_status::StatusCode,
};
use std::collections::BTreeMap;

/// Pre-computed metadata for efficient struct API validation.
/// This context is computed once per module and reused for all function validations.
pub struct StructApiContext {
    /// Map from struct name to struct handle index
    /// TODO: direclty using string is not very efficient, consider using `IdentifierIndex` instead
    struct_name_to_handle: BTreeMap<String, StructHandleIndex>,

    /// Map from struct name to struct definition index
    struct_name_to_def: BTreeMap<String, StructDefinitionIndex>,

    /// Pre-computed type order maps for each enum, keyed by struct name.
    /// Maps (offset, type) to type_order.
    /// Only populated for enums (structs with DeclaredVariants).
    enum_type_order_maps: BTreeMap<String, BTreeMap<(u16, SignatureToken), u16>>,

    /// Pre-computed variant indices for each enum, keyed by struct name.
    /// Maps (offset, type) to list of variant indices that have that field type at that offset.
    /// Only populated for enums (structs with DeclaredVariants).
    enum_variant_indices_maps: BTreeMap<String, BTreeMap<(u16, SignatureToken), Vec<u16>>>,
}

impl StructApiContext {
    /// Build the context once for the entire module.
    pub fn new(module: &CompiledModule) -> PartialVMResult<Self> {
        let mut struct_name_to_handle = BTreeMap::new();
        let mut struct_name_to_def = BTreeMap::new();
        let mut enum_type_order_maps = BTreeMap::new();
        let mut enum_variant_indices_maps = BTreeMap::new();

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
            struct_name_to_def.insert(name.clone(), StructDefinitionIndex(idx as u16));

            // If this is an enum, pre-compute its type order map and variant indices map
            if let StructFieldInformation::DeclaredVariants(variants) = &def.field_information {
                let (type_order_map, variant_indices_map) =
                    build_variant_type_order_and_indices_map(variants)?;
                enum_type_order_maps.insert(name.clone(), type_order_map);
                enum_variant_indices_maps.insert(name, variant_indices_map);
            }
        }

        Ok(Self {
            struct_name_to_handle,
            struct_name_to_def,
            enum_type_order_maps,
            enum_variant_indices_maps,
        })
    }

    /// Get struct handle by name
    pub fn get_struct_handle(&self, name: &str) -> Option<StructHandleIndex> {
        self.struct_name_to_handle.get(name).copied()
    }

    /// Get struct definition index by name
    pub fn get_struct_def_index(&self, name: &str) -> Option<StructDefinitionIndex> {
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
                return Err(
                    PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                        "function has multiple struct API attributes; at most one is allowed",
                    ),
                );
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
        struct_def_idx: StructDefinitionIndex,
    },
    PackVariant {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: StructDefinitionIndex,
        variant_name: String,
    },
    Unpack {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: StructDefinitionIndex,
    },
    UnpackVariant {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: StructDefinitionIndex,
        variant_name: String,
    },
    TestVariant {
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: StructDefinitionIndex,
        variant_name: String,
    },
    BorrowField {
        struct_name: String,
        struct_handle_idx: StructHandleIndex,
        struct_def_idx: StructDefinitionIndex,
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
/// Struct/enum names and variant names may contain '$' (for example, in hand-crafted
/// bytecode or in modules published before the struct API checker existed). This creates
/// an inherent ambiguity: `pack$A$B` could mean Pack for struct `A$B`, or PackVariant for
/// enum `A` variant `B`. Neither shortest-match nor longest-match alone is universally
/// correct. We resolve it with attribute-guided matching via `expect_variant_in_name`:
///
/// - `expect_variant_in_name = true` (PackVariant/UnpackVariant/TestVariant attributes):
///   use **shortest match** so that the remaining parts become the variant name.
///   Example: `pack$A$B` with enum `A` variant `B` → shortest match → struct `A`, variant `B` ✓
///
/// - `expect_variant_in_name = false` (Pack/Unpack/Borrow* attributes, or no attribute):
///   use **longest match** so that struct names containing `$` are found correctly.
///   Example: `pack$A$B` with struct `A$B` → longest match → struct `A$B`, no remaining ✓
///
/// If no locally defined struct matches any prefix of the name parts, the function is
/// treated as a regular (non-API) function and Ok(None) is returned. Failures in parsing
/// the remaining parts (variant name, offset, type_order) also return Ok(None) rather
/// than an error, so that functions with unusual names in older modules are never
/// incorrectly rejected.
///
/// Returns:
/// - Ok(Some(info)) if the name unambiguously matches a struct API pattern
/// - Ok(None) if the name doesn't match any struct API pattern
/// - Err only if a structural invariant is violated (e.g., overflow in index conversion)
fn parse_and_validate_struct_api_name(
    function_name: &str,
    ctx: &StructApiContext,
    expect_variant_in_name: bool,
) -> PartialVMResult<Option<StructApiNameInfo>> {
    let parts: Vec<&str> = function_name.split(PUBLIC_STRUCT_DELIMITER).collect();

    // Need at least 2 parts for any struct API function (e.g., "pack$S")
    if parts.len() < 2 {
        return Ok(None);
    }

    let prefix = parts[0];

    // Check if this looks like a struct API prefix
    if !matches!(prefix, PACK | UNPACK | TEST_VARIANT | BORROW | BORROW_MUT) {
        return Ok(None);
    }

    // Find the struct name by trying increasingly longer combinations of '$'-separated parts.
    // - Shortest match (expect_variant_in_name=true): stop at first matching struct so the
    //   remaining parts form the variant name. Correct when the attribute expects a variant.
    // - Longest match (expect_variant_in_name=false): keep going to find the longest struct
    //   name. Correct for Pack/Unpack/Borrow where no variant name follows the struct name.
    let mut candidate = String::new();
    let mut struct_name = String::new();
    let mut struct_end = 0usize; // index into parts[] of first remaining part after struct name
    for (i, part) in parts.iter().enumerate().skip(1) {
        if i > 1 {
            candidate.push_str(PUBLIC_STRUCT_DELIMITER);
        }
        candidate.push_str(part);
        if ctx.get_struct_handle(&candidate).is_some() {
            struct_end = i + 1;
            struct_name = candidate.clone();
            if expect_variant_in_name {
                break; // shortest match: stop here so remaining parts form the variant name
            }
        }
    }

    if struct_end == 0 {
        // No locally defined struct matches — not a struct API function.
        return Ok(None);
    }

    let struct_handle_idx = ctx.get_struct_handle(&struct_name).unwrap();
    let struct_def_idx = match ctx.get_struct_def_index(&struct_name) {
        Some(idx) => idx,
        None => return Ok(None),
    };

    // Remaining parts after the struct name (variant, offset, type_order, etc.)
    let remaining = &parts[struct_end..];

    match prefix {
        PACK => {
            if remaining.is_empty() {
                // pack$S
                Ok(Some(StructApiNameInfo::Pack {
                    struct_handle_idx,
                    struct_def_idx,
                }))
            } else {
                // pack$S$Variant  (variant name may itself contain '$')
                Ok(Some(StructApiNameInfo::PackVariant {
                    struct_handle_idx,
                    struct_def_idx,
                    variant_name: remaining.join(PUBLIC_STRUCT_DELIMITER),
                }))
            }
        },
        UNPACK => {
            if remaining.is_empty() {
                // unpack$S
                Ok(Some(StructApiNameInfo::Unpack {
                    struct_handle_idx,
                    struct_def_idx,
                }))
            } else {
                // unpack$S$Variant
                Ok(Some(StructApiNameInfo::UnpackVariant {
                    struct_handle_idx,
                    struct_def_idx,
                    variant_name: remaining.join(PUBLIC_STRUCT_DELIMITER),
                }))
            }
        },
        TEST_VARIANT => {
            if remaining.is_empty() {
                // test_variant requires a variant name — not a struct API
                return Ok(None);
            }
            Ok(Some(StructApiNameInfo::TestVariant {
                struct_handle_idx,
                struct_def_idx,
                variant_name: remaining.join(PUBLIC_STRUCT_DELIMITER),
            }))
        },
        BORROW | BORROW_MUT => {
            let is_mutable = prefix == BORROW_MUT;
            // Offsets and type_orders are numeric and cannot contain '$', so
            // remaining[0] is always the offset and remaining[1] (if present) is type_order.
            match remaining.len() {
                1 => {
                    let offset: u16 = match remaining[0].parse() {
                        Ok(o) => o,
                        Err(_) => return Ok(None),
                    };
                    Ok(Some(StructApiNameInfo::BorrowField {
                        struct_name,
                        struct_handle_idx,
                        struct_def_idx,
                        offset,
                        type_order: None,
                        is_mutable,
                    }))
                },
                2 => {
                    let offset: u16 = match remaining[0].parse() {
                        Ok(o) => o,
                        Err(_) => return Ok(None),
                    };
                    let type_order: u16 = match remaining[1].parse() {
                        Ok(t) => t,
                        Err(_) => return Ok(None),
                    };
                    Ok(Some(StructApiNameInfo::BorrowField {
                        struct_name,
                        struct_handle_idx,
                        struct_def_idx,
                        offset,
                        type_order: Some(type_order),
                        is_mutable,
                    }))
                },
                _ => Ok(None), // wrong number of remaining parts
            }
        },
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
        let api_name = match attr {
            FunctionAttribute::TestVariant(_) => "test_variant",
            FunctionAttribute::Unpack => "unpack",
            FunctionAttribute::UnpackVariant(_) => "unpack_variant",
            _ => "struct API",
        };
        return Err(
            PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(format!(
                "{} function must have exactly one parameter",
                api_name
            )),
        );
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
                    _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                        .with_message("test_variant function parameter inner type does not match the struct")),
                },
                _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("test_variant function parameter must be an immutable reference to the struct")),
            }
        },
        FunctionAttribute::Unpack | FunctionAttribute::UnpackVariant(_) => {
            // Unpack/UnpackVariant must take struct by value (not a reference)
            match param_type {
                SignatureToken::Struct(idx) if *idx == struct_handle_idx => Ok(()),
                SignatureToken::StructInstantiation(idx, _) if *idx == struct_handle_idx => Ok(()),
                _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("unpack function parameter type does not match the struct")),
            }
        },
        _ => {
            // Should not reach here - only Unpack/UnpackVariant/TestVariant call this function
            Err(
                PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                    "internal error: unexpected attribute in validate_struct_parameter_type",
                ),
            )
        },
    }
}

/// Validate that the variant index in PackVariant/UnpackVariant/TestVariant attribute matches the variant name
/// parsed from the function name.
fn validate_variant_index(
    module: &CompiledModule,
    struct_def_idx: StructDefinitionIndex,
    variant_name: &str,
    variant_index: VariantIndex,
) -> PartialVMResult<()> {
    let struct_def = &module.struct_defs()[struct_def_idx.0 as usize];

    // Get the variants from the struct definition
    let variants = match &struct_def.field_information {
        StructFieldInformation::DeclaredVariants(variants) => variants,
        _ => {
            // Struct doesn't have variants, but PackVariant attribute is present
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("pack_variant/unpack_variant/test_variant attribute used on a struct without variants"));
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
        .and_then(|(idx, _)| u16::try_from(idx).ok());

    match found_variant_index {
        Some(idx) if idx == variant_index => Ok(()),
        _ => Err(
            PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                "variant name in function name does not match the variant index in the attribute",
            ),
        ),
    }
}

/// Build mappings for enum variants:
/// 1. (offset, type) -> type_order: assigns unique order to each distinct (offset, type) pair
/// 2. (offset, type) -> [variant_idx]: collects all variants that have this field type at this offset
///
/// ## Relationship to the compiler
///
/// This function mirrors `construct_map_for_borrow_field_api_with_type` in
/// `move-compiler-v2/src/file_format_generator/module_generator.rs`, which the compiler uses
/// when generating borrow field API wrapper functions. The verifier must assign the same
/// type_order values so that names like `borrow$S$0$1` decode to the same (offset, type_order)
/// pair in both places.
///
/// The two functions use different key types — `(u16, SignatureToken)` here vs
/// `(usize, Type)` in the compiler — but type_order assignment is driven purely by
/// **first-encounter order** as we iterate through variants and their fields in declaration
/// order. The BTreeMap is used only for O(log n) membership lookup (`contains_key`),
/// never iterated to determine assignment order. Therefore the `Ord` implementation of
/// `SignatureToken` vs `Type` is irrelevant to correctness: as long as both sides iterate
/// variants in declaration order (which they do), the type_order values will be identical.
///
/// MAINTENANCE NOTE: If the compiler's type_order assignment logic in
/// `construct_map_for_borrow_field_api_with_type` ever changes (e.g. different iteration
/// order, or skipping certain fields), this function must be updated in lock-step to avoid
/// divergence that would cause valid compiled modules to fail re-verification.
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
) -> PartialVMResult<(
    BTreeMap<(u16, SignatureToken), u16>,
    BTreeMap<(u16, SignatureToken), Vec<u16>>,
)> {
    let mut order_map: BTreeMap<(u16, SignatureToken), u16> = BTreeMap::new();
    let mut variant_indices_map: BTreeMap<(u16, SignatureToken), Vec<u16>> = BTreeMap::new();
    let mut next_order = 0u16;

    for (variant_idx, variant) in variants.iter().enumerate() {
        // Convert variant index to u16 with checked conversion
        let variant_idx_u16 = u16::try_from(variant_idx).map_err(|_| {
            PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("enum has too many variants; variant index overflows u16")
        })?;

        for (field_offset, field) in variant.fields.iter().enumerate() {
            let field_type = field.signature.0.clone();
            // Convert field offset to u16 with checked conversion
            let field_offset = u16::try_from(field_offset).map_err(|_| {
                PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("enum variant has too many fields; field offset overflows u16")
            })?;
            let key = (field_offset, field_type.clone());

            // Only assign type_order if this (offset, type) pair hasn't been seen before
            if !order_map.contains_key(&key) {
                order_map.insert(key.clone(), next_order);
                // Use checked_add to prevent overflow. This is safe because the number of
                // unique (offset, type) pairs is bounded by the file format limits, but
                // using checked arithmetic makes the code more robust to future changes.
                next_order = next_order
                    .checked_add(1)
                    .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                        .with_message("type order counter overflows u16 (too many distinct field types across variants)"))?;
            }

            // Add this variant index to the list for this (offset, type) combination
            variant_indices_map
                .entry(key)
                .or_default()
                .push(variant_idx_u16);
        }
    }

    Ok((order_map, variant_indices_map))
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
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow field function must have exactly one parameter"));
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
                return Err(
                    PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                        "borrow field function parameter does not reference the correct struct",
                    ),
                );
            }
            // Verify mutability matches: borrow requires &S, borrow_mut requires &mut S
            if is_mutable != param_is_mut {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("borrow field function parameter mutability does not match the attribute (borrow vs borrow_mut)"));
            }
            Ok(())
        },
        None => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow field function parameter must be a reference type")),
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
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow field function must return exactly one value"));
    }

    // Extract and validate the reference type
    match &return_sig[0] {
        SignatureToken::Reference(inner) if !is_mutable => Ok(&**inner),
        SignatureToken::MutableReference(inner) if is_mutable => Ok(&**inner),
        _ => Err(
            PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                "borrow field function return type mutability does not match the attribute",
            ),
        ),
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
    struct_def_idx: StructDefinitionIndex,
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

    let struct_def = &module.struct_defs()[struct_def_idx.0 as usize];

    // Validate field type based on whether this is a regular struct or variant
    match &struct_def.field_information {
        StructFieldInformation::Native => {
            // Native structs don't have accessible fields
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("borrow field function cannot be used on a native struct"))
        },

        StructFieldInformation::Declared(fields) => {
            // Regular struct: type_order must be None (3-part name: borrow$S$offset)
            if type_order.is_some() {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("borrow field function on a regular struct must not include a type_order component"));
            }

            // Validate offset is within bounds
            if (offset as usize) >= fields.len() {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("borrow field offset is out of bounds for the struct"));
            }

            // Validate field type matches return type
            let expected_field_type = &fields[offset as usize].signature.0;
            if actual_return_type != expected_field_type {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("borrow field function return type does not match the field type at the given offset"));
            }

            Ok(())
        },

        StructFieldInformation::DeclaredVariants(_) => {
            // Variant: require type_order in function name
            let type_order = type_order
                .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("borrow field function on an enum must include a type_order component in its name"))?;

            let order_map = ctx.get_type_order_map(struct_name).ok_or_else(|| {
                PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("internal error: no type order map found for enum")
            })?;

            // Find the expected field type for this (offset, type_order) combination
            let expected_field_type = order_map
                .iter()
                .find(|((field_offset, _), order)| *field_offset == offset && **order == type_order)
                .map(|((_, field_type), _)| field_type)
                .ok_or_else(|| {
                    PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                        "borrow field (offset, type_order) combination not found in the enum",
                    )
                })?;

            // Verify the return type matches the expected field type
            if actual_return_type != expected_field_type {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("borrow field function return type does not match the field type at the given offset and type_order"));
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
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("struct API function parameter or return count does not match the number of struct fields"));
    }

    // Each signature type must match the corresponding field type in order
    for (i, field) in fields.iter().enumerate() {
        if sig.0[i] != field.signature.0 {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message(format!("struct API function parameter or return type at position {} does not match the field type", i)));
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
    struct_def_idx: StructDefinitionIndex,
    variant_name: Option<&str>,
) -> PartialVMResult<()> {
    let sig = module.signature_at(sig_idx);
    let struct_def = &module.struct_defs()[struct_def_idx.0 as usize];

    match &struct_def.field_information {
        StructFieldInformation::Native => {
            // Native structs cannot be packed/unpacked
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("cannot pack/unpack a native struct"))
        },
        StructFieldInformation::Declared(fields) => {
            // Regular struct: variant operations not allowed
            if variant_name.is_some() {
                return Err(
                    PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                        "pack_variant/unpack_variant cannot be used on a regular (non-enum) struct",
                    ),
                );
            }

            validate_signature_against_fields(sig, fields)
        },
        StructFieldInformation::DeclaredVariants(variants) => {
            // Enum: variant operations required
            let variant_name = variant_name
                .ok_or_else(|| PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("pack/unpack cannot be used on an enum; use pack_variant/unpack_variant instead"))?;

            // Find the variant by name
            let variant = variants
                .iter()
                .find(|v| {
                    let name = module.identifier_at(v.name);
                    name.as_str() == variant_name
                })
                .ok_or_else(|| {
                    PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                        .with_message("variant name in function name not found in the enum")
                })?;

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
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("pack function must return exactly one value"));
    }

    // Check that the return type is the struct type
    let return_type = &return_sig.0[0];
    match return_type {
        SignatureToken::Struct(idx) => {
            if *idx != struct_handle_idx {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("pack function return type does not match the struct"));
            }
        },
        SignatureToken::StructInstantiation(idx, _) => {
            if *idx != struct_handle_idx {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("pack function return type does not match the struct"));
            }
        },
        _ => {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("pack function must return the struct type"));
        },
    }

    Ok(())
}

/// Validate that the parameters of pack$StructName match the struct field types in order.
/// The function should take N parameters matching the N field types of the struct.
fn validate_pack_parameters(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_def_idx: StructDefinitionIndex,
) -> PartialVMResult<()> {
    validate_signature_matches_fields(module, handle.parameters, struct_def_idx, None)
}

/// Validate that the parameters of pack$StructName$VariantName match the variant field types in order.
/// The function should take N parameters matching the N field types of the specific variant.
fn validate_pack_variant_parameters(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_def_idx: StructDefinitionIndex,
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
    struct_def_idx: StructDefinitionIndex,
) -> PartialVMResult<()> {
    validate_signature_matches_fields(module, handle.return_, struct_def_idx, None)
}

/// Validate that the return types of unpack$StructName$VariantName match the variant field types in order.
/// The function should return a tuple of N values matching the N field types of the specific variant.
fn validate_unpack_variant_return_types(
    module: &CompiledModule,
    handle: &FunctionHandle,
    struct_def_idx: StructDefinitionIndex,
    variant_name: &str,
) -> PartialVMResult<()> {
    validate_signature_matches_fields(module, handle.return_, struct_def_idx, Some(variant_name))
}

/// Helper function to check bytecode patterns of the form:
/// MoveLoc(0), MoveLoc(1), ..., MoveLoc(n-1)
/// <expected_bytecode>
/// Ret
///
/// `num_move_locs`: if `Some(n)`, exactly `n` MoveLoc instructions are required before the
/// operation; if `None`, any number of MoveLoc instructions is accepted (pack-like, where n
/// equals the number of fields). In both cases the MoveLoc instructions must use consecutive
/// local indices starting from 0, matching the order in which the compiler emits struct API
/// wrappers. This prevents hand-crafted bytecode from reordering arguments and still passing
/// verification.
fn pattern_check_for_pack_like<F>(
    api_name: &str,
    code: &CodeUnit,
    num_move_locs: Option<usize>,
    is_expected_bytecode: F,
) -> PartialVMResult<()>
where
    F: Fn(&Bytecode) -> bool,
{
    if code.code.len() < 2 {
        return Err(
            PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(format!(
                "{} function bytecode must have at least 2 instructions",
                api_name
            )),
        );
    }

    if let Some(n) = num_move_locs {
        if code.code.len() != n + 2 {
            return Err(
                PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(format!(
                    "{} function must have exactly {} bytecode instructions (MoveLoc, <op>, Ret)",
                    api_name,
                    n + 2
                )),
            );
        }
    }

    // Check last bytecode is Ret
    if !matches!(code.code[code.code.len() - 1], Bytecode::Ret) {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message(format!("{} function bytecode must end with Ret", api_name)));
    }

    // Check second-to-last bytecode is the expected operation
    if !is_expected_bytecode(&code.code[code.code.len() - 2]) {
        return Err(
            PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(format!(
                "{} function bytecode must have the expected operation second-to-last",
                api_name
            )),
        );
    }

    // Check all preceding bytecodes are MoveLoc with indices 0, 1, 2, ... in order.
    // The number of MoveLoc instructions equals the number of fields/parameters.
    let num_actual_move_locs = code.code.len() - 2;
    for (i, bc) in code.code[..num_actual_move_locs].iter().enumerate() {
        match bc {
            Bytecode::MoveLoc(idx) if *idx as usize == i => {},
            _ => {
                return Err(
                    PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(format!(
                        "{} function must load arguments in sequential order starting from local 0",
                        api_name
                    )),
                )
            },
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
    pattern_check_for_pack_like("pack", code, None, |bc| {
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
    let bytecode_variant_idx = bytecode_variant_idx.ok_or_else(|| {
        PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("could not find variant instruction in struct API function bytecode")
    })?;

    // Validate the extracted index matches the expected index
    if bytecode_variant_idx != expected_variant_idx {
        return Err(
            PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                "variant index in bytecode does not match the variant index in the attribute",
            ),
        );
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
    pattern_check_for_pack_like("pack_variant", code, None, |bc| {
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

/// Check the pattern of the unpack API.
/// Pattern:
/// MoveLoc(...)
/// Unpack(...) | UnpackGeneric(...)
/// Ret
fn pattern_check_for_unpack(code: &CodeUnit) -> PartialVMResult<()> {
    pattern_check_for_pack_like("unpack", code, Some(1), |bc| {
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
    pattern_check_for_pack_like("unpack_variant", code, Some(1), |bc| {
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
    pattern_check_for_pack_like("test_variant", code, Some(1), |bc| {
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
    // Extract (is_mutable, offset) pair based on bytecode type
    let (is_mutable, offset) = match bytecode {
        Bytecode::ImmBorrowField(fh) => (false, resolver.field_handle_at(*fh)?.field),
        Bytecode::MutBorrowField(fh) => (true, resolver.field_handle_at(*fh)?.field),
        Bytecode::ImmBorrowFieldGeneric(fi) => {
            let inst = resolver.field_instantiation_at(*fi)?;
            (false, resolver.field_handle_at(inst.handle)?.field)
        },
        Bytecode::MutBorrowFieldGeneric(fi) => {
            let inst = resolver.field_instantiation_at(*fi)?;
            (true, resolver.field_handle_at(inst.handle)?.field)
        },
        Bytecode::ImmBorrowVariantField(vfh) => {
            (false, resolver.variant_field_handle_at(*vfh)?.field)
        },
        Bytecode::MutBorrowVariantField(vfh) => {
            (true, resolver.variant_field_handle_at(*vfh)?.field)
        },
        Bytecode::ImmBorrowVariantFieldGeneric(vfi) => {
            let inst = resolver.variant_field_instantiation_at(*vfi)?;
            (false, resolver.variant_field_handle_at(inst.handle)?.field)
        },
        Bytecode::MutBorrowVariantFieldGeneric(vfi) => {
            let inst = resolver.variant_field_instantiation_at(*vfi)?;
            (true, resolver.variant_field_handle_at(inst.handle)?.field)
        },
        _ => return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("struct API function contains an unexpected instruction (expected a borrow field operation)")),
    };

    // Validate mutability matches expected
    if is_mutable != expected_is_mutable {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow field instruction mutability does not match the attribute (borrow vs borrow_mut)"));
    }

    Ok(offset)
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
        _ => {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("borrow variant field instruction used on a non-enum struct"))
        }, // Should be enum
    };

    // Get struct name to look up the pre-computed variant indices map
    let struct_handle = module.struct_handle_at(struct_def.struct_handle);
    let struct_name = module.identifier_at(struct_handle.name).as_str();

    let variant_indices_map = ctx.get_variant_indices_map(struct_name).ok_or_else(|| {
        PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("internal error: no variant indices map found for enum")
    })?;

    let offset = variant_field_handle.field;
    let actual_variants = &variant_field_handle.variants;

    if actual_variants.is_empty() {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow variant field instruction must specify at least one variant"));
    }

    // Get the type for the first variant in actual_variants to determine which type group
    let first_variant_idx = actual_variants[0];

    // Safety: the bounds checker (which runs before this verifier phase) already validates
    // that all variant indices in VariantFieldHandle.variants are within bounds, so
    // first_variant_idx is guaranteed to be a valid index into enum_variants. The check
    // below is an extra defensive layer.
    if (first_variant_idx as usize) >= enum_variants.len() {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow variant field instruction variant index is out of bounds"));
    }

    let first_variant_def = &enum_variants[first_variant_idx as usize];
    let first_variant_fields = &first_variant_def.fields;

    if (offset as usize) >= first_variant_fields.len() {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow variant field instruction field offset is out of bounds"));
    }

    let expected_type = &first_variant_fields[offset as usize].signature.0;

    // Look up the expected variants from the pre-computed map
    // Note: expected_variants is guaranteed to be in ascending order by construction
    // (see build_variant_type_order_and_indices_map)
    let expected_variants = variant_indices_map
        .get(&(offset, expected_type.clone()))
        .ok_or_else(|| {
            PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                "internal error: (offset, type) pair not found in variant indices map",
            )
        })?;

    // Check both completeness and ordering in one comparison:
    // Since expected_variants is already sorted in ascending order, we just need to
    // verify that actual_variants exactly matches expected_variants.
    if actual_variants != expected_variants.as_slice() {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow variant field instruction does not cover exactly the required set of variants (must include all variants with this field type at this offset, in ascending order)"));
    }

    Ok(())
}

/// Check the pattern of the borrow field API.
/// Pattern:
/// MoveLoc(...)
/// if is_mutable is false:
///     ImmBorrowField(...) | ImmBorrowFieldGeneric(...) | ImmBorrowVariantField(...) | ImmBorrowVariantFieldGeneric(...)
/// else:
///     MutBorrowField(...) | MutBorrowFieldGeneric(...) | MutBorrowVariantField(...) | MutBorrowVariantFieldGeneric(...)
/// Ret
fn pattern_check_for_borrow_field(
    is_mutable: bool,
    resolver: &BinaryIndexedView,
    module: &CompiledModule,
    offset: &MemberCount,
    code: &CodeUnit,
    ctx: &StructApiContext,
) -> PartialVMResult<()> {
    // Check the basic pattern using the shared helper
    let api_name = if is_mutable { "borrow_mut" } else { "borrow" };
    pattern_check_for_pack_like(api_name, code, Some(1), |bc| {
        // Check if it's a borrow field operation with the correct mutability
        match bc {
            Bytecode::ImmBorrowField(_)
            | Bytecode::ImmBorrowFieldGeneric(_)
            | Bytecode::ImmBorrowVariantField(_)
            | Bytecode::ImmBorrowVariantFieldGeneric(_) => !is_mutable,
            Bytecode::MutBorrowField(_)
            | Bytecode::MutBorrowFieldGeneric(_)
            | Bytecode::MutBorrowVariantField(_)
            | Bytecode::MutBorrowVariantFieldGeneric(_) => is_mutable,
            _ => false,
        }
    })?;

    // Validate borrow field bytecode and check offset
    let actual_offset = get_borrow_field_offset(resolver, &code.code[1], is_mutable)?;
    if actual_offset != *offset {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("borrow field instruction field offset does not match the offset in the function name"));
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

    // Read the attribute first so we can guide name parsing when struct names contain '$'.
    let struct_api_attr = try_get_struct_api_attr(&handle.attributes)?;

    // Attribute-guided name parsing: variant-type attributes (PackVariant/UnpackVariant/
    // TestVariant) expect a variant name after the struct name, so use shortest-match to
    // leave room for the remaining parts. All others use longest-match to handle struct
    // names that contain '$'. When there is no attribute we fall back to longest-match
    // (the function will be rejected anyway if it has a struct API name but no attribute).
    let expect_variant_in_name = matches!(
        struct_api_attr,
        Some(
            FunctionAttribute::PackVariant(_)
                | FunctionAttribute::UnpackVariant(_)
                | FunctionAttribute::TestVariant(_)
        )
    );
    // Parse and comprehensively validate the function name
    // This validates: struct exists, variant index valid, offset valid, type_order valid
    let name_info = parse_and_validate_struct_api_name(function_name, ctx, expect_variant_in_name)?;

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
                _ => return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                    .with_message("internal error: unexpected non-struct-API attribute in struct API type resolution")),
            };

            // Verify attribute type matches the name pattern
            if info.expected_attr_type() != actual_type {
                // ERROR: Attribute type doesn't match name pattern
                return Err(
                    PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(
                        "struct API attribute type does not match the function name pattern",
                    ),
                );
            }

            // Both match - proceed to Phase 2 validation
            info
        },
        (Some(_), None) => {
            // ERROR: Name matches struct API pattern but no attribute present
            // This enforces: Name → Attribute
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("function name matches a struct API pattern but is missing the corresponding struct API attribute"));
        },
        (None, Some(_)) => {
            // ERROR: Has struct API attribute but name doesn't match pattern
            // This enforces: Attribute → Name
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("function has a struct API attribute but its name does not match the expected struct API pattern"));
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
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("borrow field offset in function name does not match the offset in the attribute"));
        }
    }

    // Validate bytecode pattern implementation
    let Some(code) = function_definition.code.as_ref() else {
        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
            .with_message("struct API function must have a code body (cannot be native)"));
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
            pattern_check_for_borrow_field(false, resolver, module, &offset, code, ctx)
        },
        FunctionAttribute::BorrowFieldMutable(offset) => {
            pattern_check_for_borrow_field(true, resolver, module, &offset, code, ctx)
        },
        FunctionAttribute::Persistent | FunctionAttribute::ModuleLock => {
            // These should never reach here - try_get_struct_api_attr only returns struct API attributes.
            // If we reach this, Phase 1 validation failed to filter properly.
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)
                .with_message("internal error: non-struct-API attribute reached the bytecode validation phase"))
        },
    }
}
