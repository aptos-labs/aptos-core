// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{identifier::IdentStr, language_storage::ModuleId, language_storage::CORE_CODE_ADDRESS};

/// Describes the reference-related side effects of a native function.
///
/// This is intentionally conservative for now: we model reference return derivations
/// and the read dependencies that drive native read-poisoning. Future work can
/// populate write effects once the runtime enforces them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeRefEffects {
    pub return_effects: &'static [NativeReturnEffect],
    pub read_param_indexes: &'static [usize],
    #[allow(dead_code)]
    pub write_param_indexes: &'static [usize],
}

impl NativeRefEffects {
    pub const fn new(
        return_effects: &'static [NativeReturnEffect],
        read_param_indexes: &'static [usize],
        write_param_indexes: &'static [usize],
    ) -> Self {
        Self {
            return_effects,
            read_param_indexes,
            write_param_indexes,
        }
    }
}

/// How a returned reference should be derived with respect to the callee's frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeReturnEffect {
    pub ret_idx: usize,
    pub path: NativeReturnPath,
}

impl NativeReturnEffect {
    pub const fn new(ret_idx: usize, path: NativeReturnPath) -> Self {
        Self { ret_idx, path }
    }
}

/// Path specification for transforming a reference returned from a native function.
#[derive(Debug, Clone, Copy)]
pub(crate) enum NativeReturnPath {
    /// Use `ret_idx` as the first hop in the access path tree.
    #[allow(dead_code)]
    RetIndex,
    /// Use a statically defined path (relative to the source node).
    Const(&'static [usize]),
}

/// Lookup reference-effects metadata for the native identified by `(module_id, function_name)`.
pub(crate) fn lookup_native_ref_effects(
    module_id: &ModuleId,
    function_name: &IdentStr,
) -> Option<&'static NativeRefEffects> {
    if module_id.address() != &CORE_CODE_ADDRESS {
        return None;
    }

    match module_id.name().as_str() {
        "signer" => match function_name.as_str() {
            "borrow_address" => Some(&SIGNER_BORROW_ADDRESS_EFFECTS),
            _ => None,
        },
        "table" => match function_name.as_str() {
            "borrow_box" => Some(&TABLE_BORROW_BOX_EFFECTS),
            "borrow_box_mut" => Some(&TABLE_BORROW_BOX_MUT_EFFECTS),
            "add_box" => Some(&TABLE_ADD_BOX_EFFECTS),
            "contains_box" => Some(&TABLE_CONTAINS_BOX_EFFECTS),
            "remove_box" => Some(&TABLE_REMOVE_BOX_EFFECTS),
            "destroy_empty_box" => Some(&TABLE_DESTROY_EMPTY_BOX_EFFECTS),
            _ => None,
        },
        "string" => match function_name.as_str() {
            "internal_check_utf8" => Some(&STRING_INTERNAL_CHECK_UTF8_EFFECTS),
            "internal_is_char_boundary" => Some(&STRING_INTERNAL_IS_CHAR_BOUNDARY_EFFECTS),
            "internal_sub_string" => Some(&STRING_INTERNAL_SUB_STRING_EFFECTS),
            "internal_index_of" => Some(&STRING_INTERNAL_INDEX_OF_EFFECTS),
            _ => None,
        },
        "bcs" => match function_name.as_str() {
            "to_bytes" => Some(&BCS_TO_BYTES_EFFECTS),
            _ => None,
        },
        "string_utils" => match function_name.as_str() {
            "native_format" => Some(&STRING_UTILS_NATIVE_FORMAT_EFFECTS),
            "native_format_list" => Some(&STRING_UTILS_NATIVE_FORMAT_LIST_EFFECTS),
            _ => None,
        },
        "crypto_algebra" => match function_name.as_str() {
            "deserialize_internal" => Some(&CRYPTO_ALGEBRA_DESERIALIZE_EFFECTS),
            "hash_to_internal" => Some(&CRYPTO_ALGEBRA_HASH_TO_EFFECTS),
            _ => None,
        },
        "ristretto255" => match function_name.as_str() {
            "point_compress_internal" => Some(&RISTRETTO_POINT_COMPRESS_EFFECTS),
            "point_mul_internal" => Some(&RISTRETTO_POINT_MUL_EFFECTS),
            "basepoint_double_mul_internal" => Some(&RISTRETTO_BASEPOINT_DOUBLE_MUL_EFFECTS),
            "point_add_internal" => Some(&RISTRETTO_POINT_ADD_EFFECTS),
            "point_sub_internal" => Some(&RISTRETTO_POINT_SUB_EFFECTS),
            "point_neg_internal" => Some(&RISTRETTO_POINT_NEG_EFFECTS),
            "point_equals" => Some(&RISTRETTO_POINT_EQUALS_EFFECTS),
            "multi_scalar_mul_internal" => Some(&RISTRETTO_MULTI_SCALAR_MUL_EFFECTS),
            _ => None,
        },
        "ristretto255_bulletproofs" => match function_name.as_str() {
            "verify_range_proof_internal" => Some(&RISTRETTO_BP_VERIFY_RANGE_PROOF_EFFECTS),
            "verify_batch_range_proof_internal" => {
                Some(&RISTRETTO_BP_VERIFY_BATCH_RANGE_PROOF_EFFECTS)
            },
            "prove_range_internal" => Some(&RISTRETTO_BP_PROVE_RANGE_EFFECTS),
            "prove_batch_range_internal" => Some(&RISTRETTO_BP_PROVE_BATCH_RANGE_EFFECTS),
            _ => None,
        },
        "aggregator" => match function_name.as_str() {
            "add" => Some(&AGGREGATOR_ADD_EFFECTS),
            "sub" => Some(&AGGREGATOR_SUB_EFFECTS),
            "read" => Some(&AGGREGATOR_READ_EFFECTS),
            _ => None,
        },
        "aggregator_factory" => match function_name.as_str() {
            "new_aggregator" => Some(&AGGREGATOR_FACTORY_NEW_EFFECTS),
            _ => None,
        },
        "aggregator_v2" => match function_name.as_str() {
            "try_add" => Some(&AGGREGATOR_V2_TRY_ADD_EFFECTS),
            "try_sub" => Some(&AGGREGATOR_V2_TRY_SUB_EFFECTS),
            "is_at_least_impl" => Some(&AGGREGATOR_V2_IS_AT_LEAST_EFFECTS),
            "read" => Some(&AGGREGATOR_V2_READ_EFFECTS),
            "snapshot" => Some(&AGGREGATOR_V2_SNAPSHOT_EFFECTS),
            "read_snapshot" => Some(&AGGREGATOR_V2_READ_SNAPSHOT_EFFECTS),
            "read_derived_string" => Some(&AGGREGATOR_V2_READ_DERIVED_STRING_EFFECTS),
            "derive_string_concat" => Some(&AGGREGATOR_V2_DERIVE_STRING_CONCAT_EFFECTS),
            "copy_snapshot" => Some(&AGGREGATOR_V2_COPY_SNAPSHOT_EFFECTS),
            "string_concat" => Some(&AGGREGATOR_V2_STRING_CONCAT_EFFECTS),
            _ => None,
        },
        "function_info" => match function_name.as_str() {
            "check_dispatch_type_compatibility_impl" => {
                Some(&FUNCTION_INFO_CHECK_DISPATCH_EFFECTS)
            },
            "is_identifier" => Some(&FUNCTION_INFO_IS_IDENTIFIER_EFFECTS),
            "load_function_impl" => Some(&FUNCTION_INFO_LOAD_FUNCTION_EFFECTS),
            _ => None,
        },
        "event" => match function_name.as_str() {
            "emitted_events_by_handle" => Some(&EVENT_EMITTED_EVENTS_BY_HANDLE_EFFECTS),
            _ => None,
        },
        "permissioned_signer" => match function_name.as_str() {
            "is_permissioned_signer_impl" => Some(&PERMISSIONED_SIGNER_IS_PERMISSIONED_EFFECTS),
            "permission_address" => Some(&PERMISSIONED_SIGNER_PERMISSION_ADDRESS_EFFECTS),
            _ => None,
        },
        _ => None,
    }
}

const SIGNER_RETURN_PATH: &[usize] = &[1];
const TABLE_RETURN_PATH: &[usize] = &[8];
const ZERO_INDEX: &[usize] = &[0];
const ONE_INDEX: &[usize] = &[1];
const ZERO_ONE_INDEXES: &[usize] = &[0, 1];
const ONE_TWO_INDEXES: &[usize] = &[1, 2];
const FOUR_FIVE_INDEXES: &[usize] = &[4, 5];
const EMPTY_INDEXES: &[usize] = &[];
const EMPTY_RETURNS: &[NativeReturnEffect] = &[];

const SIGNER_BORROW_ADDRESS_RETURNS: &[NativeReturnEffect] =
    &[NativeReturnEffect::new(0, NativeReturnPath::Const(SIGNER_RETURN_PATH))];

const TABLE_BORROW_BOX_RETURNS: &[NativeReturnEffect] =
    &[NativeReturnEffect::new(0, NativeReturnPath::Const(TABLE_RETURN_PATH))];

const SIGNER_BORROW_ADDRESS_EFFECTS: NativeRefEffects = NativeRefEffects::new(
    SIGNER_BORROW_ADDRESS_RETURNS,
    ZERO_INDEX,
    EMPTY_INDEXES,
);

const TABLE_BORROW_BOX_EFFECTS: NativeRefEffects = NativeRefEffects::new(
    TABLE_BORROW_BOX_RETURNS,
    ZERO_INDEX,
    EMPTY_INDEXES,
);

const TABLE_BORROW_BOX_MUT_EFFECTS: NativeRefEffects = NativeRefEffects::new(
    TABLE_BORROW_BOX_RETURNS,
    EMPTY_INDEXES,
    ZERO_INDEX,
);

const TABLE_ADD_BOX_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const TABLE_CONTAINS_BOX_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const TABLE_REMOVE_BOX_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const TABLE_DESTROY_EMPTY_BOX_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);

const STRING_INTERNAL_CHECK_UTF8_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const STRING_INTERNAL_IS_CHAR_BOUNDARY_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const STRING_INTERNAL_SUB_STRING_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const STRING_INTERNAL_INDEX_OF_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_ONE_INDEXES, EMPTY_INDEXES);

const BCS_TO_BYTES_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);

const STRING_UTILS_NATIVE_FORMAT_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const STRING_UTILS_NATIVE_FORMAT_LIST_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_ONE_INDEXES, EMPTY_INDEXES);

const CRYPTO_ALGEBRA_DESERIALIZE_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const CRYPTO_ALGEBRA_HASH_TO_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_ONE_INDEXES, EMPTY_INDEXES);

// These Ristretto natives can mutate their backing handles when `in_place` is set,
// but the runtime currently models them conservatively as read effects only.
const RISTRETTO_POINT_COMPRESS_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const RISTRETTO_POINT_MUL_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const RISTRETTO_BASEPOINT_DOUBLE_MUL_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ONE_INDEX, EMPTY_INDEXES);
const RISTRETTO_POINT_ADD_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_ONE_INDEXES, EMPTY_INDEXES);
const RISTRETTO_POINT_SUB_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_ONE_INDEXES, EMPTY_INDEXES);
const RISTRETTO_POINT_NEG_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const RISTRETTO_POINT_EQUALS_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_ONE_INDEXES, EMPTY_INDEXES);
const RISTRETTO_MULTI_SCALAR_MUL_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_ONE_INDEXES, EMPTY_INDEXES);

const RISTRETTO_BP_VERIFY_RANGE_PROOF_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ONE_TWO_INDEXES, EMPTY_INDEXES);
const RISTRETTO_BP_VERIFY_BATCH_RANGE_PROOF_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ONE_TWO_INDEXES, EMPTY_INDEXES);
const RISTRETTO_BP_PROVE_RANGE_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, FOUR_FIVE_INDEXES, EMPTY_INDEXES);
const RISTRETTO_BP_PROVE_BATCH_RANGE_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, FOUR_FIVE_INDEXES, EMPTY_INDEXES);

const AGGREGATOR_ADD_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_SUB_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_READ_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);

const AGGREGATOR_FACTORY_NEW_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);

const AGGREGATOR_V2_TRY_ADD_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_TRY_SUB_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_IS_AT_LEAST_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_READ_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_SNAPSHOT_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_READ_SNAPSHOT_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_READ_DERIVED_STRING_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_DERIVE_STRING_CONCAT_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ONE_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_COPY_SNAPSHOT_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const AGGREGATOR_V2_STRING_CONCAT_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ONE_INDEX, EMPTY_INDEXES);

const FUNCTION_INFO_CHECK_DISPATCH_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_ONE_INDEXES, EMPTY_INDEXES);
const FUNCTION_INFO_IS_IDENTIFIER_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const FUNCTION_INFO_LOAD_FUNCTION_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);

const EVENT_EMITTED_EVENTS_BY_HANDLE_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);

const PERMISSIONED_SIGNER_IS_PERMISSIONED_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
const PERMISSIONED_SIGNER_PERMISSION_ADDRESS_EFFECTS: NativeRefEffects =
    NativeRefEffects::new(EMPTY_RETURNS, ZERO_INDEX, EMPTY_INDEXES);
