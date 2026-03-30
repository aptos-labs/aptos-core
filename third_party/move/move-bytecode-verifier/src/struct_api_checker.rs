// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
//! 1. Function name matches struct API pattern iff it has the corresponding struct API attribute
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
//! - BorrowField: MoveLoc + ImmBorrowField/MutBorrowField/ImmBorrowVariantField/MutBorrowVariantField (or Generic variants) + Ret

use move_binary_format::{
    access::ModuleAccess,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CodeUnit, CompiledModule, FieldDefinition, FieldHandleIndex,
        FieldInstantiationIndex, FunctionAttribute, FunctionDefinition, FunctionHandle,
        MemberCount, Signature, SignatureIndex, SignatureToken, StructDefInstantiationIndex,
        StructDefinitionIndex, StructFieldInformation, StructHandleIndex, StructVariantHandleIndex,
        StructVariantInstantiationIndex, VariantDefinition, VariantFieldHandleIndex,
        VariantFieldInstantiationIndex, VariantIndex,
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

fn struct_api_err(msg: impl Into<String>) -> PartialVMError {
    PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE).with_message(msg.into())
}

// ── Pre-computed module context ───────────────────────────────────────────────

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

        // Enforce uniqueness of struct API attributes on all function handles,
        // including imported ones that are never visited by check_function.
        // Note: for locally-defined functions this check also fires here before
        // check_function can annotate the error with a FunctionDefinition index,
        // so duplicate-attribute errors on local functions will lack that annotation.
        // This is an acceptable diagnostic trade-off for uniform coverage.
        for handle in module.function_handles() {
            try_get_struct_api_attr(&handle.attributes)?;
        }

        Ok(Self {
            struct_name_to_handle,
            struct_name_to_def,
            enum_type_order_maps,
            enum_variant_indices_maps,
        })
    }

    fn get_struct_handle(&self, name: &str) -> Option<StructHandleIndex> {
        self.struct_name_to_handle.get(name).copied()
    }

    fn get_struct_def_index(&self, name: &str) -> Option<StructDefinitionIndex> {
        self.struct_name_to_def.get(name).copied()
    }

    fn get_type_order_map(&self, enum_name: &str) -> Option<&BTreeMap<(u16, SignatureToken), u16>> {
        self.enum_type_order_maps.get(enum_name)
    }

    fn get_variant_indices_map(
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
    let mut found = None;
    for attr in attrs {
        let is_struct_api = match attr {
            Pack
            | PackVariant(_)
            | Unpack
            | UnpackVariant(_)
            | TestVariant(_)
            | BorrowFieldImmutable(_)
            | BorrowFieldMutable(_) => true,
            Persistent | ModuleLock => false,
        };
        if is_struct_api {
            if found.is_some() {
                return Err(struct_api_err(
                    "function has multiple struct API attributes; at most one is allowed",
                ));
            }
            found = Some(attr.clone());
        }
    }
    Ok(found)
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
            struct_api_err("enum has too many variants; variant index overflows u16")
        })?;

        for (field_offset, field) in variant.fields.iter().enumerate() {
            let field_type = field.signature.0.clone();
            // Convert field offset to u16 with checked conversion
            let field_offset = u16::try_from(field_offset).map_err(|_| {
                struct_api_err("enum variant has too many fields; field offset overflows u16")
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
                    .ok_or_else(|| struct_api_err("type order counter overflows u16 (too many distinct field types across variants)"))?;
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

// ── Canonical-type-parameter helpers ─────────────────────────────────────────

/// Errors with `INVALID_STRUCT_API_CODE` if `type_args` is not the canonical identity
/// instantiation `[TypeParameter(0), ..., TypeParameter(n-1)]`.
fn ensure_canonical_type_params(type_args: &[SignatureToken], n: usize) -> PartialVMResult<()> {
    let is_canonical = type_args.len() == n
        && type_args
            .iter()
            .enumerate()
            .all(|(i, t)| matches!(t, SignatureToken::TypeParameter(j) if *j as usize == i));
    if is_canonical {
        Ok(())
    } else {
        Err(struct_api_err(
            "non-canonical type parameters in generic struct API wrapper",
        ))
    }
}

/// Recursively checks a single signature token: for every `StructInstantiation` whose handle
/// equals `struct_handle_idx`, the type arguments must be canonical `[T0, ..., Tn-1]`.
fn check_token_canonical_type_params(
    token: &SignatureToken,
    struct_handle_idx: StructHandleIndex,
    n: usize,
) -> PartialVMResult<()> {
    match token {
        SignatureToken::StructInstantiation(idx, type_args) => {
            if *idx == struct_handle_idx {
                ensure_canonical_type_params(type_args, n)?;
            }
            for arg in type_args {
                check_token_canonical_type_params(arg, struct_handle_idx, n)?;
            }
            Ok(())
        },
        SignatureToken::Reference(inner)
        | SignatureToken::MutableReference(inner)
        | SignatureToken::Vector(inner) => {
            check_token_canonical_type_params(inner, struct_handle_idx, n)
        },
        SignatureToken::Function(params, returns, _) => {
            for t in params.iter().chain(returns.iter()) {
                check_token_canonical_type_params(t, struct_handle_idx, n)?;
            }
            Ok(())
        },
        SignatureToken::Bool
        | SignatureToken::U8
        | SignatureToken::U16
        | SignatureToken::U32
        | SignatureToken::U64
        | SignatureToken::U128
        | SignatureToken::U256
        | SignatureToken::I8
        | SignatureToken::I16
        | SignatureToken::I32
        | SignatureToken::I64
        | SignatureToken::I128
        | SignatureToken::I256
        | SignatureToken::Address
        | SignatureToken::Signer
        | SignatureToken::Struct(_)
        | SignatureToken::TypeParameter(_) => Ok(()),
    }
}

// ── Parsed name ───────────────────────────────────────────────────────────────

/// The operation kind inferred from a function name's prefix.
///
/// `Pack` and `Unpack` each cover both the plain and variant forms — the distinction
/// comes from `ParsedApiData::variant_name` being `None` (plain) or `Some` (variant).
/// `Borrow` covers both immutable and mutable — the distinction comes from
/// `ParsedApiData::is_mutable`.
#[derive(PartialEq)]
enum NamePrefix {
    Pack,
    Unpack,
    TestVariant,
    Borrow,
}

/// Data parsed and validated from a struct API function name.
/// This is a flat data bag — the operation kind is recorded in `prefix`/`is_mutable`
/// and used only to verify consistency with the `FunctionAttribute`. All subsequent
/// dispatch is on the `FunctionAttribute` directly.
struct ParsedApiData {
    prefix: NamePrefix,
    /// `true` iff the prefix string was `"borrow_mut"`.
    is_mutable: bool,
    struct_def_idx: StructDefinitionIndex,
    struct_handle_idx: StructHandleIndex,
    /// Needed for enum type-order map lookups during borrow validation.
    struct_name: String,
    /// `Some` for variant operations (PackVariant, UnpackVariant, TestVariant).
    variant_name: Option<String>,
    /// `Some` for borrow operations.
    offset: Option<u16>,
    /// `Some` for borrow operations on enums.
    type_order: Option<u16>,
}

impl ParsedApiData {
    /// Returns `true` if this parsed name is compatible with the given `FunctionAttribute`.
    /// Used to verify that the name prefix agrees with the attribute kind before dispatching
    /// on the attribute in Phase 2.
    fn matches_attr(&self, attr: &FunctionAttribute) -> bool {
        match attr {
            FunctionAttribute::Pack => {
                self.prefix == NamePrefix::Pack && self.variant_name.is_none()
            },
            FunctionAttribute::PackVariant(_) => {
                self.prefix == NamePrefix::Pack && self.variant_name.is_some()
            },
            FunctionAttribute::Unpack => {
                self.prefix == NamePrefix::Unpack && self.variant_name.is_none()
            },
            FunctionAttribute::UnpackVariant(_) => {
                self.prefix == NamePrefix::Unpack && self.variant_name.is_some()
            },
            FunctionAttribute::TestVariant(_) => self.prefix == NamePrefix::TestVariant,
            FunctionAttribute::BorrowFieldImmutable(_) => {
                self.prefix == NamePrefix::Borrow && !self.is_mutable
            },
            FunctionAttribute::BorrowFieldMutable(_) => {
                self.prefix == NamePrefix::Borrow && self.is_mutable
            },
            // Non-struct-API attributes: these are filtered out by try_get_struct_api_attr
            // before matches_attr is ever called, so they can never be a match.
            FunctionAttribute::Persistent | FunctionAttribute::ModuleLock => false,
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
///   Example: `pack$A$B` with enum `A` variant `B`: shortest match gives struct `A`, variant `B`///
/// - `expect_variant_in_name = false` (Pack/Unpack/Borrow* attributes, or no attribute):
///   use **longest match** so that struct names containing `$` are found correctly.
///   Example: `pack$A$B` with struct `A$B`: longest match gives struct `A$B`, no remaining parts///
/// If no locally defined struct matches any prefix of the name parts, the function is
/// treated as a regular (non-API) function and Ok(None) is returned. Failures in parsing
/// the remaining parts (variant name, offset, type_order) also return Ok(None) rather
/// than an error, so that functions with unusual names in older modules are never
/// incorrectly rejected.
///
/// Returns `Some(data)` if the name unambiguously matches a struct API pattern,
/// or `None` if the name doesn't match any struct API pattern.
fn parse_struct_api_name(
    function_name: &str,
    ctx: &StructApiContext,
    expect_variant_in_name: bool,
) -> Option<ParsedApiData> {
    let parts: Vec<&str> = function_name.split(PUBLIC_STRUCT_DELIMITER).collect();

    // Need at least 2 parts for any struct API function (e.g., "pack$S")
    if parts.len() < 2 {
        return None;
    }

    let prefix_str = parts[0];

    let name_prefix = match prefix_str {
        PACK => NamePrefix::Pack,
        UNPACK => NamePrefix::Unpack,
        TEST_VARIANT => NamePrefix::TestVariant,
        BORROW | BORROW_MUT => NamePrefix::Borrow,
        _ => return None,
    };
    let is_mutable = prefix_str == BORROW_MUT;

    // Find the struct name by trying increasingly longer combinations of '$'-separated parts.
    // - Shortest match (expect_variant_in_name=true): stop at first matching struct so the
    //   remaining parts form the variant name. Correct when the attribute expects a variant.
    // - Longest match (expect_variant_in_name=false): keep going to find the longest struct
    //   name. Correct for Pack/Unpack/Borrow where no variant name follows the struct name.
    let mut candidate = String::new();
    let mut struct_name = String::new();
    let mut struct_end = 0usize;
    let mut struct_handle_idx = None;
    for (i, part) in parts.iter().enumerate().skip(1) {
        if i > 1 {
            candidate.push_str(PUBLIC_STRUCT_DELIMITER);
        }
        candidate.push_str(part);
        if let Some(h) = ctx.get_struct_handle(&candidate) {
            struct_end = i + 1;
            struct_name = candidate.clone();
            struct_handle_idx = Some(h);
            if expect_variant_in_name {
                break; // shortest match: stop here so remaining parts form the variant name
            }
        }
    }

    if struct_end == 0 {
        // No locally defined struct matches — not a struct API function.
        return None;
    }

    // struct_handle_idx is always set when struct_end != 0 (they are assigned together in the
    // loop above), so ? here only propagates None if the invariant is somehow violated.
    let struct_handle_idx = struct_handle_idx?;
    let struct_def_idx = ctx.get_struct_def_index(&struct_name)?;

    // Remaining parts after the struct name (variant, offset, type_order, etc.)
    let remaining = &parts[struct_end..];

    let (variant_name, offset, type_order) = match name_prefix {
        NamePrefix::Pack | NamePrefix::Unpack => {
            if remaining.is_empty() {
                // pack$S / unpack$S
                (None, None, None)
            } else {
                // pack$S$Variant / unpack$S$Variant (variant name may itself contain '$')
                (Some(remaining.join(PUBLIC_STRUCT_DELIMITER)), None, None)
            }
        },
        NamePrefix::TestVariant => {
            if remaining.is_empty() {
                // test_variant requires a variant name — not a struct API function
                return None;
            }
            (Some(remaining.join(PUBLIC_STRUCT_DELIMITER)), None, None)
        },
        NamePrefix::Borrow => {
            // Offsets and type_orders are numeric and cannot contain '$', so
            // remaining[0] is always the offset and remaining[1] (if present) is type_order.
            let (offset, type_order) = match remaining {
                [o] => (o.parse::<u16>().ok()?, None),
                [o, t] => (o.parse::<u16>().ok()?, Some(t.parse::<u16>().ok()?)),
                _ => return None,
            };
            (None, Some(offset), type_order)
        },
    };

    Some(ParsedApiData {
        prefix: name_prefix,
        is_mutable,
        struct_def_idx,
        struct_handle_idx,
        struct_name,
        variant_name,
        offset,
        type_order,
    })
}

// ── Shared field-level helpers ────────────────────────────────────────────────

/// Validates that `token` is `Struct(expected)` or `StructInstantiation(expected, _)`.
/// Canonical type parameter order is guaranteed by the upfront
/// `check_signature_canonical_type_params` call in `check_struct_api_impl`.
fn check_struct_token(token: &SignatureToken, expected: StructHandleIndex) -> PartialVMResult<()> {
    match token {
        SignatureToken::Struct(idx) | SignatureToken::StructInstantiation(idx, _)
            if *idx == expected =>
        {
            Ok(())
        },
        _ => Err(struct_api_err("type does not match expected struct")),
    }
}

/// Helper to validate that a signature's types match a list of field definitions.
/// Checks both count and type equality in order.
fn validate_signature_against_fields(
    sig: &Signature,
    fields: &[FieldDefinition],
) -> PartialVMResult<()> {
    if sig.0.len() != fields.len() {
        return Err(struct_api_err("struct API function parameter or return count does not match the number of struct fields"));
    }
    for (i, field) in fields.iter().enumerate() {
        if sig.0[i] != field.signature.0 {
            return Err(struct_api_err(format!("struct API function parameter or return type at position {} does not match the field type", i)));
        }
    }
    Ok(())
}

// ── Bytecode-level field-handle resolvers ─────────────────────────────────────
//
// Each resolver checks that the field/variant handle belongs to `expected` struct,
// and (for generic instructions) that the type-parameter instantiation is canonical.

fn resolve_field(
    module: &CompiledModule,
    idx: FieldHandleIndex,
    expected: StructDefinitionIndex,
) -> PartialVMResult<MemberCount> {
    let fh = module.field_handle_at(idx);
    if fh.owner != expected {
        return Err(struct_api_err(
            "borrow field instruction references a field belonging to a different struct",
        ));
    }
    Ok(fh.field)
}

fn resolve_field_generic(
    module: &CompiledModule,
    idx: FieldInstantiationIndex,
    expected: StructDefinitionIndex,
    n: usize,
) -> PartialVMResult<MemberCount> {
    let inst = module.field_instantiation_at(idx);
    let fh = module.field_handle_at(inst.handle);
    if fh.owner != expected {
        return Err(struct_api_err(
            "borrow field instruction references a field belonging to a different struct",
        ));
    }
    ensure_canonical_type_params(&module.signature_at(inst.type_parameters).0, n)?;
    Ok(fh.field)
}

fn resolve_variant_field(
    module: &CompiledModule,
    idx: VariantFieldHandleIndex,
    expected: StructDefinitionIndex,
) -> PartialVMResult<MemberCount> {
    let vfh = module.variant_field_handle_at(idx);
    if vfh.struct_index != expected {
        return Err(struct_api_err(
            "borrow variant field instruction references a field belonging to a different struct",
        ));
    }
    Ok(vfh.field)
}

fn resolve_variant_field_generic(
    module: &CompiledModule,
    idx: VariantFieldInstantiationIndex,
    expected: StructDefinitionIndex,
    n: usize,
) -> PartialVMResult<MemberCount> {
    let inst = module.variant_field_instantiation_at(idx);
    let vfh = module.variant_field_handle_at(inst.handle);
    if vfh.struct_index != expected {
        return Err(struct_api_err(
            "borrow variant field instruction references a field belonging to a different struct",
        ));
    }
    ensure_canonical_type_params(&module.signature_at(inst.type_parameters).0, n)?;
    Ok(vfh.field)
}

// ── Bytecode pattern checker ──────────────────────────────────────────────────

/// Verifies that a struct API wrapper's code unit matches the canonical pattern:
/// MoveLoc(0), MoveLoc(1), ..., MoveLoc(n-1)
/// <expected_bytecode>
/// Ret
///
/// Requires exactly `num_move_locs` MoveLoc instructions before the operation, using
/// consecutive local indices starting from 0, matching the order in which the compiler
/// emits struct API wrappers. This prevents hand-crafted bytecode from reordering arguments
/// and still passing verification.
fn check_struct_api_bytecode_pattern<F>(
    api_name: &str,
    code: &CodeUnit,
    num_move_locs: usize,
    is_expected_bytecode: F,
) -> PartialVMResult<()>
where
    F: Fn(&Bytecode) -> bool,
{
    if code.code.len() != num_move_locs + 2 {
        return Err(struct_api_err(format!(
            "{} function must have exactly {} bytecode instructions (MoveLoc*, <op>, Ret)",
            api_name,
            num_move_locs + 2
        )));
    }

    if !matches!(code.code[code.code.len() - 1], Bytecode::Ret) {
        return Err(struct_api_err(format!(
            "{} function bytecode must end with Ret",
            api_name
        )));
    }

    if !is_expected_bytecode(&code.code[code.code.len() - 2]) {
        return Err(struct_api_err(format!(
            "{} function bytecode must have the expected operation second-to-last",
            api_name
        )));
    }

    // Check all preceding bytecodes are MoveLoc with indices 0, 1, 2, ... in order.
    let num_actual_move_locs = code.code.len() - 2;
    for (i, bc) in code.code[..num_actual_move_locs].iter().enumerate() {
        match bc {
            Bytecode::MoveLoc(idx) if *idx as usize == i => {},
            _ => {
                return Err(struct_api_err(format!(
                    "{} function must load arguments in sequential order starting from local 0",
                    api_name
                )))
            },
        }
    }

    Ok(())
}

/// Enum to represent either a direct variant handle index or a generic variant instantiation index
enum VariantIndexRef {
    Direct(StructVariantHandleIndex),
    Generic(StructVariantInstantiationIndex),
}

// ── Per-function validator ────────────────────────────────────────────────────

/// Holds all the per-function context needed for Phase 2 validation, so it does not need to
/// be threaded as individual parameters through every helper.
struct StructApiWrapperValidator<'m> {
    module: &'m CompiledModule,
    ctx: &'m StructApiContext,
    handle: &'m FunctionHandle,
    code: &'m CodeUnit,
    struct_def_idx: StructDefinitionIndex,
    struct_handle_idx: StructHandleIndex,
    num_type_params: usize,
}

impl<'m> StructApiWrapperValidator<'m> {
    // ── Signature-level checks ────────────────────────────────────────────────

    /// Verifies that every `StructInstantiation(struct_handle_idx, ...)` appearing in the
    /// function's declared parameter and return signatures uses canonical type parameters.
    ///
    /// This covers signature-level instantiations. Bytecode-level instantiations (in the
    /// struct-def, field, and variant instantiation pools) are checked separately at their
    /// respective call sites via `ensure_canonical_type_params`.
    fn check_signature_canonical_type_params(&self) -> PartialVMResult<()> {
        for sig_idx in [self.handle.parameters, self.handle.return_] {
            for token in &self.module.signature_at(sig_idx).0 {
                check_token_canonical_type_params(
                    token,
                    self.struct_handle_idx,
                    self.num_type_params,
                )?;
            }
        }
        Ok(())
    }

    /// Validates that signature types (parameters or returns) match the struct or variant field
    /// types in order. `sig_idx` is `handle.parameters` for pack and `handle.return_` for unpack.
    ///
    /// - For regular structs: validates against Declared fields
    /// - For enums with variant_name: validates against specific variant's fields
    /// - Rejects Native structs and mismatched struct/enum usage
    fn validate_signature_matches_fields(
        &self,
        sig_idx: SignatureIndex,
        variant_name: Option<&str>,
    ) -> PartialVMResult<()> {
        let sig = self.module.signature_at(sig_idx);
        let struct_def = &self.module.struct_defs()[self.struct_def_idx.0 as usize];

        match &struct_def.field_information {
            StructFieldInformation::Native => {
                Err(struct_api_err("cannot pack/unpack a native struct"))
            },
            StructFieldInformation::Declared(fields) => {
                if variant_name.is_some() {
                    return Err(struct_api_err(
                        "pack_variant/unpack_variant cannot be used on a regular (non-enum) struct",
                    ));
                }
                validate_signature_against_fields(sig, fields)
            },
            StructFieldInformation::DeclaredVariants(variants) => {
                let variant_name = variant_name.ok_or_else(|| {
                    struct_api_err(
                        "pack/unpack cannot be used on an enum; use pack_variant/unpack_variant instead",
                    )
                })?;
                let variant = variants
                    .iter()
                    .find(|v| self.module.identifier_at(v.name).as_str() == variant_name)
                    .ok_or_else(|| {
                        struct_api_err("variant name in function name not found in the enum")
                    })?;
                validate_signature_against_fields(sig, &variant.fields)
            },
        }
    }

    /// Checks that the return signature is exactly one value of the expected struct type.
    /// Used for pack and pack_variant functions.
    fn check_pack_return_type(&self) -> PartialVMResult<()> {
        let ret = self.module.signature_at(self.handle.return_);
        if ret.0.len() != 1 {
            return Err(struct_api_err(
                "pack function must return exactly one value",
            ));
        }
        check_struct_token(&ret.0[0], self.struct_handle_idx)
    }

    /// Checks that the parameter signature is exactly one value of the expected struct type
    /// (by value for unpack/unpack_variant, by immutable reference for test_variant).
    fn check_struct_param(&self, as_ref: bool) -> PartialVMResult<()> {
        let param_sig = self.module.signature_at(self.handle.parameters);
        if param_sig.0.len() != 1 {
            return Err(struct_api_err("function must have exactly one parameter"));
        }
        let param_type = &param_sig.0[0];
        if as_ref {
            match param_type {
                SignatureToken::Reference(inner) => {
                    check_struct_token(inner, self.struct_handle_idx)
                },
                _ => Err(struct_api_err(
                    "test_variant function parameter must be an immutable reference to the struct",
                )),
            }
        } else {
            check_struct_token(param_type, self.struct_handle_idx)
        }
    }

    /// Checks that the parameter signature is exactly one reference to the expected struct type,
    /// with the given mutability.
    fn check_borrow_param(&self, is_mutable: bool) -> PartialVMResult<()> {
        let param_sig = self.module.signature_at(self.handle.parameters);
        if param_sig.0.len() != 1 {
            return Err(struct_api_err(
                "borrow field function must have exactly one parameter",
            ));
        }
        let param_type = &param_sig.0[0];
        let (idx, param_is_mutable) = match param_type {
            SignatureToken::Reference(inner) | SignatureToken::MutableReference(inner) => {
                let param_is_mutable = matches!(param_type, SignatureToken::MutableReference(_));
                match inner.as_ref() {
                    SignatureToken::Struct(idx) | SignatureToken::StructInstantiation(idx, _) => {
                        (*idx, param_is_mutable)
                    },
                    _ => {
                        return Err(struct_api_err(
                            "borrow field function parameter must be a reference type",
                        ))
                    },
                }
            },
            _ => {
                return Err(struct_api_err(
                    "borrow field function parameter must be a reference type",
                ))
            },
        };
        if idx != self.struct_handle_idx {
            return Err(struct_api_err(
                "borrow field function parameter does not reference the correct struct",
            ));
        }
        if is_mutable != param_is_mutable {
            return Err(struct_api_err("borrow field function parameter mutability does not match the attribute (borrow vs borrow_mut)"));
        }
        Ok(())
    }

    /// Validates that the variant index stored in a PackVariant/UnpackVariant/TestVariant
    /// attribute matches the variant named in the function name. Returns the validated index.
    fn validate_variant_index(
        &self,
        variant_name: &str,
        attr_variant_idx: VariantIndex,
    ) -> PartialVMResult<VariantIndex> {
        let struct_def = &self.module.struct_defs()[self.struct_def_idx.0 as usize];
        let variants = match &struct_def.field_information {
            StructFieldInformation::DeclaredVariants(variants) => variants,
            StructFieldInformation::Declared(_) | StructFieldInformation::Native => {
                return Err(struct_api_err("pack_variant/unpack_variant/test_variant attribute used on a struct without variants"));
            },
        };
        let found = variants
            .iter()
            .enumerate()
            .find(|(_, v)| self.module.identifier_at(v.name).as_str() == variant_name)
            .and_then(|(idx, _)| u16::try_from(idx).ok());
        match found {
            Some(idx) if idx == attr_variant_idx => Ok(idx),
            _ => Err(struct_api_err(
                "variant name in function name does not match the variant index in the attribute",
            )),
        }
    }

    /// Validates the return type of a borrow field function.
    ///
    /// Checks that the return type is a reference with the correct mutability, and that
    /// the referenced type matches the field type at `(offset, type_order)` in the struct.
    /// Returns the validated field type (cloned) so the bytecode checker can use it as a
    /// trusted anchor for the completeness check.
    fn validate_borrow_return_type(
        &self,
        struct_name: &str,
        offset: u16,
        type_order: Option<u16>,
        is_mutable: bool,
    ) -> PartialVMResult<SignatureToken> {
        let ret = self.module.signature_at(self.handle.return_);
        if ret.0.len() != 1 {
            return Err(struct_api_err(
                "borrow field function must return exactly one value",
            ));
        }
        let actual_inner = match &ret.0[0] {
            SignatureToken::Reference(inner) if !is_mutable => inner.as_ref(),
            SignatureToken::MutableReference(inner) if is_mutable => inner.as_ref(),
            _ => {
                return Err(struct_api_err(
                    "borrow field function return type mutability does not match the attribute",
                ))
            },
        };

        let struct_def = &self.module.struct_defs()[self.struct_def_idx.0 as usize];
        let expected: &SignatureToken = match &struct_def.field_information {
            StructFieldInformation::Native => {
                return Err(struct_api_err(
                    "borrow field function cannot be used on a native struct",
                ))
            },
            StructFieldInformation::Declared(fields) => {
                if type_order.is_some() {
                    return Err(struct_api_err("borrow field function on a regular struct must not include a type_order component"));
                }
                if (offset as usize) >= fields.len() {
                    return Err(struct_api_err(
                        "borrow field offset is out of bounds for the struct",
                    ));
                }
                &fields[offset as usize].signature.0
            },
            StructFieldInformation::DeclaredVariants(_) => {
                let type_order = type_order.ok_or_else(|| {
                    struct_api_err("borrow field function on an enum must include a type_order component in its name")
                })?;
                let order_map = self.ctx.get_type_order_map(struct_name).ok_or_else(|| {
                    struct_api_err("internal error: no type order map found for enum")
                })?;
                order_map
                    .iter()
                    .find(|((fo, _), ord)| *fo == offset && **ord == type_order)
                    .map(|((_, ft), _)| ft)
                    .ok_or_else(|| {
                        struct_api_err(
                            "borrow field (offset, type_order) combination not found in the enum",
                        )
                    })?
            },
        };

        if actual_inner != expected {
            return Err(struct_api_err(
                "borrow field function return type does not match the field type at the given offset",
            ));
        }
        Ok(expected.clone())
    }

    // ── Bytecode-level checks ─────────────────────────────────────────────────

    /// Validates that the `StructDefInstantiation` at `sdi_idx` references this struct's
    /// definition index and uses canonical type parameters `[TypeParam(0), ..., TypeParam(n-1)]`.
    fn check_struct_def_instantiation(
        &self,
        sdi_idx: StructDefInstantiationIndex,
    ) -> PartialVMResult<()> {
        let sdi = self.module.struct_instantiation_at(sdi_idx);
        if sdi.def != self.struct_def_idx {
            return Err(struct_api_err(
                "generic struct instruction references wrong struct definition",
            ));
        }
        ensure_canonical_type_params(
            &self.module.signature_at(sdi.type_parameters).0,
            self.num_type_params,
        )
    }

    /// Check the bytecode pattern for pack:
    /// MoveLoc(0), ..., MoveLoc(n-1), Pack/PackGeneric, Ret
    fn check_pack_pattern(&self) -> PartialVMResult<()> {
        let struct_def = &self.module.struct_defs()[self.struct_def_idx.0 as usize];
        let num_fields = match &struct_def.field_information {
            StructFieldInformation::Declared(fields) => fields.len(),
            StructFieldInformation::Native => {
                return Err(struct_api_err(
                    "internal error: pack pattern check reached for native struct",
                ))
            },
            StructFieldInformation::DeclaredVariants(_) => {
                return Err(struct_api_err(
                    "internal error: pack pattern check reached for enum struct",
                ))
            },
        };
        // Pre-validate the PackGeneric instantiation outside the closure.
        let generic_ok = match self.code.code.get(self.code.code.len().wrapping_sub(2)) {
            Some(Bytecode::PackGeneric(sdi_idx)) => {
                self.check_struct_def_instantiation(*sdi_idx)?;
                true
            },
            _ => false,
        };
        check_struct_api_bytecode_pattern(PACK, self.code, num_fields, |bc| match bc {
            Bytecode::Pack(def_idx) => *def_idx == self.struct_def_idx,
            Bytecode::PackGeneric(_) => generic_ok,
            _ => false,
        })
    }

    /// Check the bytecode pattern for unpack:
    /// MoveLoc(0), Unpack/UnpackGeneric, Ret
    fn check_unpack_pattern(&self) -> PartialVMResult<()> {
        // Pre-validate the UnpackGeneric instantiation outside the closure.
        let generic_ok = match self.code.code.get(self.code.code.len().wrapping_sub(2)) {
            Some(Bytecode::UnpackGeneric(sdi_idx)) => {
                self.check_struct_def_instantiation(*sdi_idx)?;
                true
            },
            _ => false,
        };
        check_struct_api_bytecode_pattern(UNPACK, self.code, 1, |bc| match bc {
            Bytecode::Unpack(def_idx) => *def_idx == self.struct_def_idx,
            Bytecode::UnpackGeneric(_) => generic_ok,
            _ => false,
        })
    }

    /// Shared pattern check for variant operations (pack_variant, unpack_variant, test_variant).
    ///
    /// Runs the MoveLoc*–<op>–Ret shape check, then scans the code for the variant instruction,
    /// verifies that the handle belongs to `self.struct_def_idx`, and checks that the variant
    /// index matches `expected_variant_idx`.
    fn check_variant_op_pattern(
        &self,
        api_name: &str,
        num_move_locs: usize,
        expected_variant_idx: VariantIndex,
        to_variant_ref: impl Fn(&Bytecode) -> Option<VariantIndexRef>,
    ) -> PartialVMResult<()> {
        check_struct_api_bytecode_pattern(api_name, self.code, num_move_locs, |bc| {
            to_variant_ref(bc).is_some()
        })?;

        // Scan for the variant instruction and validate struct ownership + variant index.
        let mut found_idx = None;
        for bc in &self.code.code {
            if let Some(idx_ref) = to_variant_ref(bc) {
                let (variant_idx, struct_idx) = match idx_ref {
                    VariantIndexRef::Direct(idx) => {
                        let handle = self.module.struct_variant_handle_at(idx);
                        (handle.variant, handle.struct_index)
                    },
                    VariantIndexRef::Generic(idx) => {
                        let inst = self.module.struct_variant_instantiation_at(idx);
                        let handle = self.module.struct_variant_handle_at(inst.handle);
                        ensure_canonical_type_params(
                            &self.module.signature_at(inst.type_parameters).0,
                            self.num_type_params,
                        )?;
                        (handle.variant, handle.struct_index)
                    },
                };
                if struct_idx != self.struct_def_idx {
                    return Err(struct_api_err(
                        "variant instruction references a variant belonging to a different struct",
                    ));
                }
                found_idx = Some(variant_idx);
                break;
            }
        }

        let found_idx = found_idx.ok_or_else(|| {
            struct_api_err("could not find variant instruction in struct API function bytecode")
        })?;
        if found_idx != expected_variant_idx {
            return Err(struct_api_err(
                "variant index in bytecode does not match the variant index in the attribute",
            ));
        }
        Ok(())
    }

    /// Check the bytecode pattern for pack_variant:
    /// MoveLoc(0), ..., MoveLoc(n-1), PackVariant/PackVariantGeneric, Ret
    fn check_pack_variant_pattern(
        &self,
        expected_variant_idx: VariantIndex,
    ) -> PartialVMResult<()> {
        let struct_def = &self.module.struct_defs()[self.struct_def_idx.0 as usize];
        let num_fields = match &struct_def.field_information {
            StructFieldInformation::DeclaredVariants(variants) => variants
                .get(expected_variant_idx as usize)
                .map(|v| v.fields.len())
                .ok_or_else(|| {
                    struct_api_err("internal error: could not resolve variant field count")
                })?,
            StructFieldInformation::Declared(_) | StructFieldInformation::Native => {
                return Err(struct_api_err(
                    "internal error: pack_variant pattern check reached for non-enum struct",
                ))
            },
        };
        self.check_variant_op_pattern(
            PACK_VARIANT,
            num_fields,
            expected_variant_idx,
            |bc| match bc {
                Bytecode::PackVariant(idx) => Some(VariantIndexRef::Direct(*idx)),
                Bytecode::PackVariantGeneric(idx) => Some(VariantIndexRef::Generic(*idx)),
                _ => None,
            },
        )
    }

    /// Check the bytecode pattern for unpack_variant:
    /// MoveLoc(0), UnpackVariant/UnpackVariantGeneric, Ret
    fn check_unpack_variant_pattern(
        &self,
        expected_variant_idx: VariantIndex,
    ) -> PartialVMResult<()> {
        self.check_variant_op_pattern(UNPACK_VARIANT, 1, expected_variant_idx, |bc| match bc {
            Bytecode::UnpackVariant(idx) => Some(VariantIndexRef::Direct(*idx)),
            Bytecode::UnpackVariantGeneric(idx) => Some(VariantIndexRef::Generic(*idx)),
            _ => None,
        })
    }

    /// Check the bytecode pattern for test_variant:
    /// MoveLoc(0), TestVariant/TestVariantGeneric, Ret
    fn check_test_variant_pattern(
        &self,
        expected_variant_idx: VariantIndex,
    ) -> PartialVMResult<()> {
        self.check_variant_op_pattern(TEST_VARIANT, 1, expected_variant_idx, |bc| match bc {
            Bytecode::TestVariant(idx) => Some(VariantIndexRef::Direct(*idx)),
            Bytecode::TestVariantGeneric(idx) => Some(VariantIndexRef::Generic(*idx)),
            _ => None,
        })
    }

    /// Extracts the field offset from a borrow-field bytecode instruction, checking struct
    /// ownership, mutability, and (for generic instructions) canonical type parameters.
    fn get_borrow_field_offset(
        &self,
        bytecode: &Bytecode,
        expected_is_mutable: bool,
    ) -> PartialVMResult<MemberCount> {
        let (is_mutable, offset) = match bytecode {
            Bytecode::ImmBorrowField(h) => (false, resolve_field(self.module, *h, self.struct_def_idx)?),
            Bytecode::MutBorrowField(h) => (true, resolve_field(self.module, *h, self.struct_def_idx)?),
            Bytecode::ImmBorrowFieldGeneric(h) => (false, resolve_field_generic(self.module, *h, self.struct_def_idx, self.num_type_params)?),
            Bytecode::MutBorrowFieldGeneric(h) => (true, resolve_field_generic(self.module, *h, self.struct_def_idx, self.num_type_params)?),
            Bytecode::ImmBorrowVariantField(h) => (false, resolve_variant_field(self.module, *h, self.struct_def_idx)?),
            Bytecode::MutBorrowVariantField(h) => (true, resolve_variant_field(self.module, *h, self.struct_def_idx)?),
            Bytecode::ImmBorrowVariantFieldGeneric(h) => (false, resolve_variant_field_generic(self.module, *h, self.struct_def_idx, self.num_type_params)?),
            Bytecode::MutBorrowVariantFieldGeneric(h) => (true, resolve_variant_field_generic(self.module, *h, self.struct_def_idx, self.num_type_params)?),
            _ => return Err(struct_api_err("struct API function contains an unexpected instruction (expected a borrow field operation)")),
        };
        if is_mutable != expected_is_mutable {
            return Err(struct_api_err("borrow field instruction mutability does not match the attribute (borrow vs borrow_mut)"));
        }
        Ok(offset)
    }

    /// Validates that a BorrowVariantField instruction covers exactly the complete and ordered
    /// set of variants for the given offset and type.
    ///
    /// `expected_field_type` is the type already validated by `validate_borrow_return_type`.
    /// Using this trusted value prevents a hand-crafted variant field handle whose first variant
    /// has a different type from redirecting the lookup to a different completeness set.
    fn check_borrow_variant_completeness(
        &self,
        bytecode: &Bytecode,
        struct_name: &str,
        expected_field_type: &SignatureToken,
    ) -> PartialVMResult<()> {
        let variant_field_handle = match bytecode {
            Bytecode::ImmBorrowVariantField(vfh) | Bytecode::MutBorrowVariantField(vfh) => {
                self.module.variant_field_handle_at(*vfh)
            },
            Bytecode::ImmBorrowVariantFieldGeneric(vfi)
            | Bytecode::MutBorrowVariantFieldGeneric(vfi) => {
                let inst = self.module.variant_field_instantiation_at(*vfi);
                self.module.variant_field_handle_at(inst.handle)
            },
            _ => return Ok(()), // Not a variant field borrow, nothing to check
        };

        let struct_def = &self.module.struct_defs()[self.struct_def_idx.0 as usize];
        let enum_variants = match &struct_def.field_information {
            StructFieldInformation::DeclaredVariants(variants) => variants,
            StructFieldInformation::Declared(_) | StructFieldInformation::Native => {
                return Err(struct_api_err(
                    "borrow variant field instruction used on a non-enum struct",
                ))
            },
        };

        let variant_indices_map =
            self.ctx
                .get_variant_indices_map(struct_name)
                .ok_or_else(|| {
                    struct_api_err("internal error: no variant indices map found for enum")
                })?;

        let offset = variant_field_handle.field;
        let actual_variants = &variant_field_handle.variants;

        if actual_variants.is_empty() {
            return Err(struct_api_err(
                "borrow variant field instruction must specify at least one variant",
            ));
        }

        // Safety: the bounds checker (which runs before this verifier phase) already validates
        // that all variant indices in VariantFieldHandle.variants are within bounds.
        let first_variant_idx = actual_variants[0];
        if (first_variant_idx as usize) >= enum_variants.len() {
            return Err(struct_api_err(
                "borrow variant field instruction variant index is out of bounds",
            ));
        }

        let first_variant_fields = &enum_variants[first_variant_idx as usize].fields;
        if (offset as usize) >= first_variant_fields.len() {
            return Err(struct_api_err(
                "borrow variant field instruction field offset is out of bounds",
            ));
        }

        // Assert that the type derived from the bytecode's first variant matches the
        // validated return type, binding the completeness check to the same type group.
        let derived_type = &first_variant_fields[offset as usize].signature.0;
        if derived_type != expected_field_type {
            return Err(struct_api_err(
                "borrow variant field instruction type group does not match the validated return type",
            ));
        }

        // Look up expected variants using the trusted type.
        // expected_variants is guaranteed to be in ascending order by construction.
        let expected_variants = variant_indices_map
            .get(&(offset, expected_field_type.clone()))
            .ok_or_else(|| {
                struct_api_err(
                    "internal error: (offset, type) pair not found in variant indices map",
                )
            })?;

        if actual_variants != expected_variants.as_slice() {
            return Err(struct_api_err("borrow variant field instruction does not cover exactly the required set of variants (must include all variants with this field type at this offset, in ascending order)"));
        }

        Ok(())
    }

    /// Check the bytecode pattern for borrow/borrow_mut:
    /// MoveLoc(0), <BorrowField op>, Ret
    fn check_borrow_pattern(
        &self,
        is_mutable: bool,
        struct_name: &str,
        expected_offset: u16,
        expected_field_type: &SignatureToken,
    ) -> PartialVMResult<()> {
        let api_name = if is_mutable { BORROW_MUT } else { BORROW };
        check_struct_api_bytecode_pattern(api_name, self.code, 1, |bc| match bc {
            Bytecode::ImmBorrowField(_)
            | Bytecode::ImmBorrowFieldGeneric(_)
            | Bytecode::ImmBorrowVariantField(_)
            | Bytecode::ImmBorrowVariantFieldGeneric(_) => !is_mutable,
            Bytecode::MutBorrowField(_)
            | Bytecode::MutBorrowFieldGeneric(_)
            | Bytecode::MutBorrowVariantField(_)
            | Bytecode::MutBorrowVariantFieldGeneric(_) => is_mutable,
            _ => false,
        })?;

        let actual_offset = self.get_borrow_field_offset(&self.code.code[1], is_mutable)?;
        if actual_offset != expected_offset as MemberCount {
            return Err(struct_api_err(
                "borrow field instruction field offset does not match the offset in the function name",
            ));
        }

        // For variant field borrows, validate completeness and ordering of variants.
        self.check_borrow_variant_completeness(&self.code.code[1], struct_name, expected_field_type)
    }

    // ── Top-level dispatch ────────────────────────────────────────────────────

    /// Dispatch to the appropriate validation based on the `FunctionAttribute`.
    /// `data` carries fields parsed from the function name; `attr` is the source of truth
    /// for which operation this is and carries index payloads (variant index, borrow offset).
    fn validate(&self, data: ParsedApiData, attr: FunctionAttribute) -> PartialVMResult<()> {
        match attr {
            FunctionAttribute::Pack => {
                self.validate_signature_matches_fields(self.handle.parameters, None)?;
                self.check_pack_return_type()?;
                self.check_pack_pattern()
            },

            FunctionAttribute::PackVariant(attr_variant_idx) => {
                let variant_name = data.variant_name.as_deref().ok_or_else(|| {
                    struct_api_err("internal error: variant_name is None for PackVariant")
                })?;
                self.validate_signature_matches_fields(self.handle.parameters, Some(variant_name))?;
                self.check_pack_return_type()?;
                let idx = self.validate_variant_index(variant_name, attr_variant_idx)?;
                self.check_pack_variant_pattern(idx)
            },

            FunctionAttribute::Unpack => {
                self.check_struct_param(false)?;
                self.validate_signature_matches_fields(self.handle.return_, None)?;
                self.check_unpack_pattern()
            },

            FunctionAttribute::UnpackVariant(attr_variant_idx) => {
                let variant_name = data.variant_name.as_deref().ok_or_else(|| {
                    struct_api_err("internal error: variant_name is None for UnpackVariant")
                })?;
                self.check_struct_param(false)?;
                self.validate_signature_matches_fields(self.handle.return_, Some(variant_name))?;
                let idx = self.validate_variant_index(variant_name, attr_variant_idx)?;
                self.check_unpack_variant_pattern(idx)
            },

            FunctionAttribute::TestVariant(attr_variant_idx) => {
                let variant_name = data.variant_name.as_deref().ok_or_else(|| {
                    struct_api_err("internal error: variant_name is None for TestVariant")
                })?;
                self.check_struct_param(true)?;
                let idx = self.validate_variant_index(variant_name, attr_variant_idx)?;
                self.check_test_variant_pattern(idx)
            },

            FunctionAttribute::BorrowFieldImmutable(attr_offset)
            | FunctionAttribute::BorrowFieldMutable(attr_offset) => {
                let is_mutable = data.is_mutable;
                let name_offset = data.offset.ok_or_else(|| {
                    struct_api_err("internal error: offset is None for BorrowField attribute")
                })?;
                if attr_offset != name_offset {
                    return Err(struct_api_err(
                        "borrow field attribute offset does not match the offset in the function name",
                    ));
                }
                self.check_borrow_param(is_mutable)?;
                let ft = self.validate_borrow_return_type(
                    &data.struct_name,
                    attr_offset,
                    data.type_order,
                    is_mutable,
                )?;
                self.check_borrow_pattern(is_mutable, &data.struct_name, attr_offset, &ft)
            },

            // Persistent and ModuleLock are not struct API attributes and are filtered out
            // by try_get_struct_api_attr before validate() is called. Unreachable in practice,
            // but listed exhaustively so adding future FunctionAttribute variants is a compile error.
            FunctionAttribute::Persistent | FunctionAttribute::ModuleLock => Err(struct_api_err(
                "internal error: non-struct-API attribute reached validation dispatch",
            )),
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

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
pub(crate) fn check_function(
    module: &CompiledModule,
    function_definition: &FunctionDefinition,
    ctx: &StructApiContext,
) -> PartialVMResult<()> {
    let handle = module.function_handle_at(function_definition.function);
    let function_name = module.identifier_at(handle.name).as_str();

    // ── Phase 1: name / attribute correspondence ──────────────────────────────
    // Read the attribute first to guide name parsing (variant attrs use shortest-match
    // to leave room for the variant name; everything else uses longest-match).
    let struct_api_attr = try_get_struct_api_attr(&handle.attributes)?;
    let expect_variant_in_name = matches!(
        struct_api_attr,
        Some(
            FunctionAttribute::PackVariant(_)
                | FunctionAttribute::UnpackVariant(_)
                | FunctionAttribute::TestVariant(_)
        )
    );
    let name_data = parse_struct_api_name(function_name, ctx, expect_variant_in_name);

    let (data, attr) = match (name_data, struct_api_attr) {
        (None, None) => return Ok(()),
        (Some(_), None) => return Err(struct_api_err("function name matches a struct API pattern but is missing the corresponding struct API attribute")),
        (None, Some(_)) => return Err(struct_api_err("function has a struct API attribute but its name does not match the expected struct API pattern")),
        (Some(data), Some(attr)) => {
            if !data.matches_attr(&attr) {
                return Err(struct_api_err(
                    "struct API attribute type does not match the function name pattern",
                ));
            }
            (data, attr)
        },
    };

    // ── Phase 2: implementation validation ───────────────────────────────────
    let struct_handle = module.struct_handle_at(data.struct_handle_idx);

    // Structs with the key ability cannot have struct APIs.
    if struct_handle.abilities.has_key() {
        return Err(struct_api_err(
            "struct with key ability cannot have struct APIs",
        ));
    }

    let num_type_params = struct_handle.type_parameters.len();

    // The wrapper function must have exactly as many type parameters as the struct it wraps.
    // Without this check a wrapper with extra type params (e.g. pack$Box<T,U> for Box<T>) would
    // pass: the canonical-type-params check only validates the struct's own N params, so the
    // extra U is invisible to it.
    // Note: ability-constraint mismatches on individual type params are caught by the general
    // bytecode type checker (CONSTRAINT_NOT_SATISFIED) before this point, so we need only
    // verify the count here.
    if handle.type_parameters.len() != num_type_params {
        return Err(struct_api_err(
            "struct API wrapper function has a different number of type parameters than the struct",
        ));
    }

    let Some(code) = function_definition.code.as_ref() else {
        return Err(struct_api_err(
            "struct API function must have a code body (cannot be native)",
        ));
    };

    let v = StructApiWrapperValidator {
        module,
        ctx,
        handle,
        code,
        struct_def_idx: data.struct_def_idx,
        struct_handle_idx: data.struct_handle_idx,
        num_type_params,
    };

    // Verify canonical type params in the function's declared parameter and return signatures.
    // Bytecode-level instantiation pool entries are checked separately at their use sites.
    v.check_signature_canonical_type_params()?;

    v.validate(data, attr)
}
