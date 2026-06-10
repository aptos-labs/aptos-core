// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Provides pragmas and properties of the specification language.

use crate::{
    ast::{ConditionKind, PropertyBag, PropertyValue},
    builder::module_builder::SpecBlockContext,
    symbol::SymbolPool,
};
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

/// Pragma indicating whether verification should be performed for a function.
pub const VERIFY_PRAGMA: &str = "verify";

/// Pragma defining a timeout.
pub const TIMEOUT_PRAGMA: &str = "timeout";

/// Pragma defining a random seed.
pub const SEED_PRAGMA: &str = "seed";

/// Pragma indicating an estimate how long verification takes. Verification
/// is skipped if the timeout is smaller than this.
pub const VERIFY_DURATION_ESTIMATE_PRAGMA: &str = "verify_duration_estimate";

/// Pragma indicating whether implementation of function should be ignored and
/// instead treated to be like a native function.
pub const INTRINSIC_PRAGMA: &str = "intrinsic";

/// Pragma indicating whether implementation of function should be ignored and
/// instead interpreted by its pre and post conditions only.
pub const OPAQUE_PRAGMA: &str = "opaque";

/// Pragma indicating whether emits specification should be considered partial.
pub const EMITS_IS_PARTIAL_PRAGMA: &str = "emits_is_partial";

/// Pragma indicating whether no emits specification should mean that no events are to be emitted.
pub const EMITS_IS_STRICT_PRAGMA: &str = "emits_is_strict";

/// Pragma indicating whether aborts_if specification should be considered partial.
pub const ABORTS_IF_IS_PARTIAL_PRAGMA: &str = "aborts_if_is_partial";

/// Pragma indicating whether no explicit aborts_if specification should be treated
/// like `aborts_if` false.
pub const ABORTS_IF_IS_STRICT_PRAGMA: &str = "aborts_if_is_strict";

/// Pragma indicating that requires are also enforced if the aborts condition is true.
pub const REQUIRES_IF_ABORTS_PRAGMA: &str = "requires_if_aborts";

/// Pragma indicating that the function will run smoke tests
pub const ALWAYS_ABORTS_TEST_PRAGMA: &str = "always_aborts_test";

/// Pragma indicating that adding u64 or u128 values should not be checked
/// for overflow.
pub const ADDITION_OVERFLOW_UNCHECKED_PRAGMA: &str = "addition_overflow_unchecked";

/// Pragma indicating that aborts from this function shall be ignored.
pub const ASSUME_NO_ABORT_FROM_HERE_PRAGMA: &str = "assume_no_abort_from_here";

/// Pragma which indicates that the function's abort and ensure conditions shall be exported
/// to the verification context even if the implementation of the function is inlined.
pub const EXPORT_ENSURES_PRAGMA: &str = "export_ensures";

/// Pragma indicating that the function can only be called from certain caller.
/// Unlike other pragmas, this pragma expects a function name like `0x1::M::f` instead
/// of a boolean or a number.
pub const FRIEND_PRAGMA: &str = "friend";

/// Pragma indicating that invariants are not to be checked between entry and exit
/// to this function
pub const DISABLE_INVARIANTS_IN_BODY_PRAGMA: &str = "disable_invariants_in_body";

/// Pragma indicating that invariants are not to be checked between entry and exit
/// to this function
pub const DELEGATE_INVARIANTS_TO_CALLER_PRAGMA: &str = "delegate_invariants_to_caller";

/// Pragma indicating that all loops within the scope of this pragma should be unrolled
/// to a certain depth *when there are no invariants specified*
pub const UNROLL_PRAGMA: &str = "unroll";

/// Pragma controlling which spec conditions are inferred.
/// Values: `none` (skip inference), `only_ensures` (skip aborts_if), `only_aborts` (skip ensures).
/// Default (unset): infer both ensures and aborts_if.
pub const INFERENCE_PRAGMA: &str = "inference";

/// # Pragmas for intrinsic table declaration

/// The intrinsic type for `Map<K, V>`
pub const INTRINSIC_TYPE_MAP: &str = "map";

/// Create a new table with an empty content
/// `[move] fun map_new<K, V>(): Map<K, V>`
pub const INTRINSIC_FUN_MAP_NEW: &str = "map_new";

/// Create a new table with an empty content (the spec version)
/// `[spec] fun map_new<K, V>(): Map<K, V>`
pub const INTRINSIC_FUN_MAP_SPEC_NEW: &str = "map_spec_new";

/// Get the value associated with key `k`.
/// The behavior is undefined if `k` does not exist in the map
/// `[spec] fun map_get<K, V>(m: Map<K, V>, k: K): V`
pub const INTRINSIC_FUN_MAP_SPEC_GET: &str = "map_spec_get";

/// Set the value to `v` with the key associated with `k`
/// `[spec] fun map_set<K, V>(m: Map<K, V>, k: K, v: V): Map<K, V>`
pub const INTRINSIC_FUN_MAP_SPEC_SET: &str = "map_spec_set";

/// Remove the map entry associated with key `k`
/// The behavior is undefined if `k` does not exist in the map
/// `[spec] fun map_del<K, V>(m: Map<K, V>, k: K): Map<K, V>`
pub const INTRINSIC_FUN_MAP_SPEC_DEL: &str = "map_spec_del";

/// Get the number of entries in the map (the spec version)
/// `[spec] fun map_len<K, V>(m: Map<K, V>): num`
pub const INTRINSIC_FUN_MAP_SPEC_LEN: &str = "map_spec_len";

/// Check whether the map is empty (the spec version)
/// `[move] fun map_is_empty<K, V>(m: Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_SPEC_IS_EMPTY: &str = "map_spec_is_empty";

/// Get the number of entries in the map
/// `[move] fun map_len<K, V>(m: &Map<K, V>): u64`
pub const INTRINSIC_FUN_MAP_LEN: &str = "map_len";

/// Check whether the map is empty
/// `[move] fun map_is_empty<K, V>(m: &Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_IS_EMPTY: &str = "map_is_empty";

/// Check if the map has an entry associated with key `k` (the spec version)
/// `[spec] fun map_has_key<K, V>(m: Map<K, V>, k: K): bool`
pub const INTRINSIC_FUN_MAP_SPEC_HAS_KEY: &str = "map_spec_has_key";

/// Check if the map has an entry associated with key `k`
/// `[move] fun map_has_key<K, V>(m: &Map<K, V>, k: K): bool`
pub const INTRINSIC_FUN_MAP_HAS_KEY: &str = "map_has_key";

/// Destroys the map, aborts if the length is not zero.
/// `[move] fun map_destroy_empty<K, V>(m: Map<K, V>)`
pub const INTRINSIC_FUN_MAP_DESTROY_EMPTY: &str = "map_destroy_empty";

/// Add a new entry to the map, aborts if the key already exists
/// `[move] fun map_add_no_override<K, V>(m: &mut Map<K, V>, k: K, v: V)`
pub const INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE: &str = "map_add_no_override";

/// Add a new entry to the map, override if the key already exists
/// `[move] fun map_add_override_if_exists<K, V>(m: &mut Map<K, V>, k: K, v: V)`
pub const INTRINSIC_FUN_MAP_ADD_OVERRIDE_IF_EXISTS: &str = "map_add_override_if_exists";

/// Remove an entry from the map, aborts if the key does not exists
/// `[move] fun map_del_must_exist<K, V>(m: &mut Map<K, V>, k: K): V`
pub const INTRINSIC_FUN_MAP_DEL_MUST_EXIST: &str = "map_del_must_exist";

/// Remove an entry from the map, aborts if the key does not exists
/// `[move] fun map_del_return_key<K, V>(m: &mut Map<K, V>, k: K): (K, V)`
pub const INTRINSIC_FUN_MAP_DEL_RETURN_KEY: &str = "map_del_return_key";

/// Immutable borrow of a value from the map, aborts if the key does not exist
/// `[move] fun map_borrow<K, V>(m: &Map<K, V>, k: K): &V`
pub const INTRINSIC_FUN_MAP_BORROW: &str = "map_borrow";

/// Mutable borrow of a value from the map, aborts if the key does not exist
/// `[move] fun map_borrow_mut<K, V>(m: &mut Map<K, V>, k: K): &mut V`
pub const INTRINSIC_FUN_MAP_BORROW_MUT: &str = "map_borrow_mut";

/// Mutable borrow of a value from the map, add the entry (k,default) if the key does not exist
/// `[move] fun map_borrow_mut<K, V>(m: &mut Map<K, V>, k: K, default: V): &mut V`
pub const INTRINSIC_FUN_MAP_BORROW_MUT_WITH_DEFAULT: &str = "map_borrow_mut_with_default";

/// Mutable borrow of a value from the map, return deafult if the key does not exist
/// `[move] fun map_borrow_with_default<K, V>(m: &Map<K, V>, k: K, default: V): &V`
pub const INTRINSIC_FUN_MAP_BORROW_WITH_DEFAULT: &str = "map_borrow_with_default";

/// Optional lookup: returns `Some(v)` when the key is in the map, `None` otherwise.
/// Never aborts. Requires `V: copy`.
/// `[move] fun map_get<K, V>(m: &Map<K, V>, k: K): Option<V>`
pub const INTRINSIC_FUN_MAP_GET: &str = "map_get";

/// Build a map from two parallel vectors of keys and values. Aborts when the lengths differ
/// or when the keys contain a duplicate.
/// `[move] fun map_new_from<K, V>(keys: vector<K>, values: vector<V>): Map<K, V>`
pub const INTRINSIC_FUN_MAP_NEW_FROM: &str = "map_new_from";

/// Decompose the map into two parallel vectors holding its keys and values. The order of
/// the elements in the returned vectors is unspecified, but `keys[i]` maps to `values[i]`.
/// `[move] fun map_to_vec_pair<K, V>(m: Map<K, V>): (vector<K>, vector<V>)`
pub const INTRINSIC_FUN_MAP_TO_VEC_PAIR: &str = "map_to_vec_pair";

/// Insert or update; returns the displaced value as `Option<V>`. Never aborts.
/// `[move] fun map_upsert<K, V>(m: &mut Map<K, V>, k: K, v: V): Option<V>`
pub const INTRINSIC_FUN_MAP_UPSERT: &str = "map_upsert";

/// Insert or update; returns the displaced key and value as `(Option<K>, Option<V>)`. Never
/// aborts. The returned key equals the input key under structural equality (the SMT model
/// encodes keys through `$EncodeKey`, which is an injection over `$IsEqual`).
/// `[move] fun map_upsert_kv<K, V>(m: &mut Map<K, V>, k: K, v: V): (Option<K>, Option<V>)`
pub const INTRINSIC_FUN_MAP_UPSERT_KV: &str = "map_upsert_kv";

/// Project the map's keys into a vector. Order is unspecified.
/// `[move] fun map_keys<K, V>(m: &Map<K, V>): vector<K>`
pub const INTRINSIC_FUN_MAP_KEYS: &str = "map_keys";

/// Project the map's values into a vector. Order is unspecified. Values may repeat if
/// distinct keys map to equal values.
/// `[move] fun map_values<K, V>(m: &Map<K, V>): vector<V>`
pub const INTRINSIC_FUN_MAP_VALUES: &str = "map_values";

/// Return the smallest key under cmp::compare ordering. Aborts when the map is empty.
/// `[move] fun map_front_key<K, V>(m: &Map<K, V>): K`
pub const INTRINSIC_FUN_MAP_FRONT_KEY: &str = "map_front_key";

/// Return the largest key under cmp::compare ordering. Aborts when the map is empty.
/// `[move] fun map_back_key<K, V>(m: &Map<K, V>): K`
pub const INTRINSIC_FUN_MAP_BACK_KEY: &str = "map_back_key";

/// Return the smallest key under cmp::compare ordering together with a reference to its
/// value. Aborts when the map is empty.
/// `[move] fun map_borrow_front<K, V>(m: &Map<K, V>): (K, &V)`
pub const INTRINSIC_FUN_MAP_BORROW_FRONT: &str = "map_borrow_front";

/// Return the largest key under cmp::compare ordering together with a reference to its
/// value. Aborts when the map is empty.
/// `[move] fun map_borrow_back<K, V>(m: &Map<K, V>): (K, &V)`
pub const INTRINSIC_FUN_MAP_BORROW_BACK: &str = "map_borrow_back";

/// Pop the smallest entry (key, value) under cmp::compare. Aborts when the map is empty.
/// `[move] fun map_pop_front<K, V>(m: &mut Map<K, V>): (K, V)`
pub const INTRINSIC_FUN_MAP_POP_FRONT: &str = "map_pop_front";

/// Pop the largest entry (key, value) under cmp::compare. Aborts when the map is empty.
/// `[move] fun map_pop_back<K, V>(m: &mut Map<K, V>): (K, V)`
pub const INTRINSIC_FUN_MAP_POP_BACK: &str = "map_pop_back";

/// Largest key strictly less than the given key under cmp::compare, or None.
/// `[move] fun map_prev_key<K, V>(m: &Map<K, V>, k: &K): Option<K>`
pub const INTRINSIC_FUN_MAP_PREV_KEY: &str = "map_prev_key";

/// Smallest key strictly greater than the given key under cmp::compare, or None.
/// `[move] fun map_next_key<K, V>(m: &Map<K, V>, k: &K): Option<K>`
pub const INTRINSIC_FUN_MAP_NEXT_KEY: &str = "map_next_key";

/// Remove the entry at the given key if present, returning Some(V) or None.
/// `[move] fun map_remove_or_none<K, V>(m: &mut Map<K, V>, k: &K): Option<V>`
pub const INTRINSIC_FUN_MAP_REMOVE_OR_NONE: &str = "map_remove_or_none";

// === Iterator API roles. ===
// These intrinsics operate on the map's IteratorPtr type — looked up by name in the
// map struct's own module. The role bindings are sound only for maps whose
// IteratorPtr is shaped like BigOrderedMap's: an enum with an `End` variant and a
// `Some { ..., key: K }` variant (additional fields are not read in spec, only the
// `key` field is). OrderedMap's IteratorPtr does not match this shape.

/// Iterator at the smallest key, or end-sentinel if the map is empty. Never aborts.
/// `[move] fun map_iter_new_begin<K, V>(m: &Map<K, V>): IteratorPtr<K>`
pub const INTRINSIC_FUN_MAP_ITER_NEW_BEGIN: &str = "map_iter_new_begin";

/// End sentinel iterator. Never aborts.
/// `[move] fun map_iter_new_end<K, V>(m: &Map<K, V>): IteratorPtr<K>`
pub const INTRINSIC_FUN_MAP_ITER_NEW_END: &str = "map_iter_new_end";

/// True iff the iterator is the end sentinel. Never aborts.
/// `[move] fun map_iter_is_end<K, V>(it: &IteratorPtr<K>, m: &Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_ITER_IS_END: &str = "map_iter_is_end";

/// True iff the iterator is at the begin position. An End iterator on an empty map
/// counts as begin. Never aborts.
/// `[move] fun map_iter_is_begin<K, V>(it: &IteratorPtr<K>, m: &Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_ITER_IS_BEGIN: &str = "map_iter_is_begin";

/// Read the key the iterator points at. Aborts if at end.
/// `[move] fun map_iter_borrow_key<K>(it: &IteratorPtr<K>): &K`
pub const INTRINSIC_FUN_MAP_ITER_BORROW_KEY: &str = "map_iter_borrow_key";

/// Read the value the iterator points at. Aborts if at end.
/// `[move] fun map_iter_borrow<K, V>(it: IteratorPtr<K>, m: &Map<K, V>): &V`
pub const INTRINSIC_FUN_MAP_ITER_BORROW: &str = "map_iter_borrow";

/// Advance the iterator to the next-larger key, or end if none. Aborts if currently
/// at end.
/// `[move] fun map_iter_next<K, V>(it: IteratorPtr<K>, m: &Map<K, V>): IteratorPtr<K>`
pub const INTRINSIC_FUN_MAP_ITER_NEXT: &str = "map_iter_next";

/// Step the iterator to the prev-smaller key. Aborts if currently at the begin
/// (smallest key or empty-map end).
/// `[move] fun map_iter_prev<K, V>(it: IteratorPtr<K>, m: &Map<K, V>): IteratorPtr<K>`
pub const INTRINSIC_FUN_MAP_ITER_PREV: &str = "map_iter_prev";

/// Iterator at the given key if present, end-sentinel otherwise. Never aborts.
/// `[move] fun map_internal_find<K, V>(m: &Map<K, V>, k: &K): IteratorPtr<K>`
pub const INTRINSIC_FUN_MAP_INTERNAL_FIND: &str = "map_internal_find";

/// Iterator at the smallest key K >= input under cmp::compare ordering, or
/// end-sentinel if no such key exists. Never aborts.
/// `[move] fun map_internal_lower_bound<K, V>(m: &Map<K, V>, k: &K): IteratorPtr<K>`
pub const INTRINSIC_FUN_MAP_INTERNAL_LOWER_BOUND: &str = "map_internal_lower_bound";

// === IteratorPtrWithPath family ===
// These roles operate on a companion-of-companion type `IteratorPtrWithPath<K>` that
// wraps an `IteratorPtr<K>` with an implementation-only path. Like the iter roles,
// they require the map type to declare both `IteratorPtr` and `IteratorPtrWithPath`
// structs in its home module.

/// Iterator-with-path at the given key if present, end-sentinel iterator-with-path
/// otherwise. Never aborts.
/// `[move] fun map_internal_find_with_path<K, V>(m: &Map<K, V>, k: &K): IteratorPtrWithPath<K>`
pub const INTRINSIC_FUN_MAP_INTERNAL_FIND_WITH_PATH: &str = "map_internal_find_with_path";

/// Project the wrapped `IteratorPtr<K>` from an `IteratorPtrWithPath<K>`. Never aborts.
/// `[move] fun map_iter_with_path_get_iter<K>(it: &IteratorPtrWithPath<K>): IteratorPtr<K>`
pub const INTRINSIC_FUN_MAP_ITER_WITH_PATH_GET_ITER: &str = "map_iter_with_path_get_iter";

/// Remove the entry at the iterator-with-path's key, returning its value. Aborts when
/// the iterator points to end.
/// `[move] fun map_iter_remove<K, V>(it: IteratorPtrWithPath<K>, m: &mut Map<K, V>): V`
pub const INTRINSIC_FUN_MAP_ITER_REMOVE: &str = "map_iter_remove";

// === Configured constructors ===
// Variants of map_new that take additional configuration parameters. The Boogie body
// returns `EmptyTable()` with abort conditions on the config parameters when applicable.

/// Create an empty map with explicit degree/reuse-slots configuration. Aborts when
/// either `inner_max_degree` or `leaf_max_degree` is non-zero but out of range.
/// `[move] fun map_new_with_config<K, V>(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool): Map<K, V>`
pub const INTRINSIC_FUN_MAP_NEW_WITH_CONFIG: &str = "map_new_with_config";

/// Create an empty map with the reusable-slots policy enabled. Conservatively reports
/// `aborts_if false` (matches the existing trusted spec; the Move source's BCS-size
/// check on K/V is not expressible at this layer).
/// `[move] fun map_new_with_reusable<K, V>(): Map<K, V>`
pub const INTRINSIC_FUN_MAP_NEW_WITH_REUSABLE: &str = "map_new_with_reusable";

/// Create an empty map configured against the provided key/value size hints.
/// Conservatively reports `aborts_if false` (matches the existing trusted spec).
/// `[move] fun map_new_with_type_size_hints<K, V>(avg_key_bytes: u64, max_key_bytes: u64, avg_value_bytes: u64, max_value_bytes: u64): Map<K, V>`
pub const INTRINSIC_FUN_MAP_NEW_WITH_TYPE_SIZE_HINTS: &str = "map_new_with_type_size_hints";

/// Abort condition for map_destroy_empty: true when the map is non-empty
/// `[spec] fun map_spec_aborts_destroy_empty<K, V>(m: Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY: &str = "map_spec_aborts_destroy_empty";

/// Abort condition for map_add_no_override: true when the key already exists
/// `[spec] fun map_spec_aborts_add<K, V>(m: Map<K, V>, k: K, v: V): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD: &str = "map_spec_aborts_add";

/// Abort condition for map_del_must_exist / map_del_return_key: true when key not found
/// `[spec] fun map_spec_aborts_del<K, V>(m: Map<K, V>, k: K): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL: &str = "map_spec_aborts_del";

/// Abort condition for map_borrow / map_borrow_mut: true when key not found
/// `[spec] fun map_spec_aborts_borrow<K, V>(m: Map<K, V>, k: K): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW: &str = "map_spec_aborts_borrow";

/// Abort condition for map_new_from: true when len(keys) != len(values) or keys contains
/// duplicates (under $EncodeKey).
/// `[spec] fun map_spec_aborts_new_from<K, V>(keys: vector<K>, values: vector<V>): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_FROM: &str = "map_spec_aborts_new_from";

/// Abort condition for map_new_with_config: true when a configured degree is non-zero and
/// outside the supported range.
/// `[spec] fun map_spec_aborts_new_with_config<K, V>(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_WITH_CONFIG: &str = "map_spec_aborts_new_with_config";

/// Abort condition for the order-key family (front_key / back_key / borrow_front /
/// borrow_back / pop_front / pop_back): true when the map is empty.
/// `[spec] fun map_spec_aborts_empty_map<K, V>(m: Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY_MAP: &str = "map_spec_aborts_empty_map";

/// Abort condition for map_iter_borrow_key: true when the iterator is the End sentinel.
/// `[spec] fun map_spec_aborts_iter_borrow_key<K>(it: IteratorPtr<K>): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_KEY: &str = "map_spec_aborts_iter_borrow_key";

/// Abort condition for map_iter_borrow / map_iter_next: true when the iterator is End or
/// its cached key is no longer in the map.
/// `[spec] fun map_spec_aborts_iter_oob<K, V>(it: IteratorPtr<K>, m: Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_OOB: &str = "map_spec_aborts_iter_oob";

/// Abort condition for map_iter_prev: true when the iterator is at begin (smallest key,
/// or End-on-empty-map) or its cached key is no longer in the map.
/// `[spec] fun map_spec_aborts_iter_prev<K, V>(it: IteratorPtr<K>, m: Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_PREV: &str = "map_spec_aborts_iter_prev";

/// Abort condition for map_iter_remove: true when the wrapped iterator is End or its
/// cached key is no longer in the map.
/// `[spec] fun map_spec_aborts_iter_remove<K, V>(self: IteratorPtrWithPath<K>, m: Map<K, V>): bool`
pub const INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_REMOVE: &str = "map_spec_aborts_iter_remove";

/// Definition of an intrinsic function associated with an intrinsic type.
///
/// For Move functions, `spec_fun` and `abort_spec_fun` encode the counterpart spec function names
/// directly, eliminating the need for separate static lookup tables.
pub struct IntrinsicFunDef {
    /// Whether this is a Move-level function (`true`) or a spec-only function (`false`).
    pub is_move_fun: bool,
    /// For Move functions only: the name of the spec counterpart used for pure spec calls.
    pub spec_fun: Option<&'static str>,
    /// For Move functions only: the name of the abort-condition spec function.
    pub abort_spec_fun: Option<&'static str>,
}

impl IntrinsicFunDef {
    /// Construct a definition for a Move-level intrinsic function.
    pub fn move_fun(spec: Option<&'static str>, abort: Option<&'static str>) -> Self {
        Self {
            is_move_fun: true,
            spec_fun: spec,
            abort_spec_fun: abort,
        }
    }

    /// Construct a definition for a spec-only intrinsic function.
    pub fn spec_fun() -> Self {
        Self {
            is_move_fun: false,
            spec_fun: None,
            abort_spec_fun: None,
        }
    }
}

/// All intrinsic functions associated with the map type.
///
/// Each Move function entry encodes its spec counterpart (`spec_fun`) and abort-condition
/// spec counterpart (`abort_spec_fun`) directly, replacing the former separate static tables
/// `INTRINSIC_TYPE_MAP_MOVE_TO_SPEC_FUN` and `INTRINSIC_TYPE_MAP_MOVE_TO_ABORT_SPEC_FUN`.
///
/// Notes on spec_fun pairings (read-only functions only):
/// - Mutating functions (e.g. `map_add_no_override`) are excluded from spec_fun because their
///   `&mut` params already cause `try_as_pure_spec_call` to return `None` at check 1.
pub static INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS: Lazy<BTreeMap<&'static str, IntrinsicFunDef>> =
    Lazy::new(|| {
        BTreeMap::from([
            (
                INTRINSIC_FUN_MAP_NEW,
                IntrinsicFunDef::move_fun(Some(INTRINSIC_FUN_MAP_SPEC_NEW), None),
            ),
            (INTRINSIC_FUN_MAP_SPEC_NEW, IntrinsicFunDef::spec_fun()),
            (INTRINSIC_FUN_MAP_SPEC_GET, IntrinsicFunDef::spec_fun()),
            (INTRINSIC_FUN_MAP_SPEC_SET, IntrinsicFunDef::spec_fun()),
            (INTRINSIC_FUN_MAP_SPEC_DEL, IntrinsicFunDef::spec_fun()),
            (INTRINSIC_FUN_MAP_SPEC_LEN, IntrinsicFunDef::spec_fun()),
            (INTRINSIC_FUN_MAP_SPEC_IS_EMPTY, IntrinsicFunDef::spec_fun()),
            (INTRINSIC_FUN_MAP_SPEC_HAS_KEY, IntrinsicFunDef::spec_fun()),
            (
                INTRINSIC_FUN_MAP_LEN,
                IntrinsicFunDef::move_fun(Some(INTRINSIC_FUN_MAP_SPEC_LEN), None),
            ),
            (
                INTRINSIC_FUN_MAP_IS_EMPTY,
                IntrinsicFunDef::move_fun(Some(INTRINSIC_FUN_MAP_SPEC_IS_EMPTY), None),
            ),
            (
                INTRINSIC_FUN_MAP_HAS_KEY,
                IntrinsicFunDef::move_fun(Some(INTRINSIC_FUN_MAP_SPEC_HAS_KEY), None),
            ),
            (
                INTRINSIC_FUN_MAP_DESTROY_EMPTY,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY)),
            ),
            (
                INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD)),
            ),
            (
                INTRINSIC_FUN_MAP_ADD_OVERRIDE_IF_EXISTS,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_DEL_MUST_EXIST,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL)),
            ),
            (
                INTRINSIC_FUN_MAP_DEL_RETURN_KEY,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL)),
            ),
            (
                INTRINSIC_FUN_MAP_BORROW,
                IntrinsicFunDef::move_fun(
                    Some(INTRINSIC_FUN_MAP_SPEC_GET),
                    Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW),
                ),
            ),
            (
                INTRINSIC_FUN_MAP_BORROW_MUT,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW)),
            ),
            (
                INTRINSIC_FUN_MAP_BORROW_MUT_WITH_DEFAULT,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_BORROW_WITH_DEFAULT,
                IntrinsicFunDef::move_fun(Some(INTRINSIC_FUN_MAP_SPEC_GET), None),
            ),
            (INTRINSIC_FUN_MAP_GET, IntrinsicFunDef::move_fun(None, None)),
            (
                INTRINSIC_FUN_MAP_NEW_FROM,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_FROM)),
            ),
            (
                INTRINSIC_FUN_MAP_TO_VEC_PAIR,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_UPSERT,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_UPSERT_KV,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_KEYS,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_VALUES,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_FRONT_KEY,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY_MAP)),
            ),
            (
                INTRINSIC_FUN_MAP_BACK_KEY,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY_MAP)),
            ),
            (
                INTRINSIC_FUN_MAP_BORROW_FRONT,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY_MAP)),
            ),
            (
                INTRINSIC_FUN_MAP_BORROW_BACK,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY_MAP)),
            ),
            (
                INTRINSIC_FUN_MAP_POP_FRONT,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY_MAP)),
            ),
            (
                INTRINSIC_FUN_MAP_POP_BACK,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY_MAP)),
            ),
            (
                INTRINSIC_FUN_MAP_PREV_KEY,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_NEXT_KEY,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_REMOVE_OR_NONE,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_NEW_BEGIN,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_NEW_END,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_IS_END,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_IS_BEGIN,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_BORROW_KEY,
                IntrinsicFunDef::move_fun(
                    None,
                    Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_KEY),
                ),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_BORROW,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_OOB)),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_NEXT,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_OOB)),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_PREV,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_PREV)),
            ),
            (
                INTRINSIC_FUN_MAP_INTERNAL_FIND,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_INTERNAL_LOWER_BOUND,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_INTERNAL_FIND_WITH_PATH,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_WITH_PATH_GET_ITER,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_ITER_REMOVE,
                IntrinsicFunDef::move_fun(None, Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_REMOVE)),
            ),
            (
                INTRINSIC_FUN_MAP_NEW_WITH_CONFIG,
                IntrinsicFunDef::move_fun(
                    None,
                    Some(INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_WITH_CONFIG),
                ),
            ),
            (
                INTRINSIC_FUN_MAP_NEW_WITH_REUSABLE,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_NEW_WITH_TYPE_SIZE_HINTS,
                IntrinsicFunDef::move_fun(None, None),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_FROM,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_WITH_CONFIG,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY_MAP,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_KEY,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_OOB,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_PREV,
                IntrinsicFunDef::spec_fun(),
            ),
            (
                INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_REMOVE,
                IntrinsicFunDef::spec_fun(),
            ),
        ])
    });

/// Checks whether a pragma is valid in a specific spec block.
pub fn is_pragma_valid_for_block(
    symbols: &SymbolPool,
    bag: &PropertyBag,
    target: &SpecBlockContext,
    pragma: &str,
) -> bool {
    use crate::builder::module_builder::SpecBlockContext::*;
    match target {
        Module => matches!(
            pragma,
            VERIFY_PRAGMA
                | EMITS_IS_STRICT_PRAGMA
                | EMITS_IS_PARTIAL_PRAGMA
                | ABORTS_IF_IS_STRICT_PRAGMA
                | ABORTS_IF_IS_PARTIAL_PRAGMA
                | INTRINSIC_PRAGMA
                | UNROLL_PRAGMA
                | INFERENCE_PRAGMA
        ),
        Function(..) | FunctionCodeV2(.., Some(..)) => matches!(
            pragma,
            VERIFY_PRAGMA
                | TIMEOUT_PRAGMA
                | SEED_PRAGMA
                | VERIFY_DURATION_ESTIMATE_PRAGMA
                | INTRINSIC_PRAGMA
                | OPAQUE_PRAGMA
                | EMITS_IS_STRICT_PRAGMA
                | EMITS_IS_PARTIAL_PRAGMA
                | ABORTS_IF_IS_PARTIAL_PRAGMA
                | ABORTS_IF_IS_STRICT_PRAGMA
                | REQUIRES_IF_ABORTS_PRAGMA
                | ALWAYS_ABORTS_TEST_PRAGMA
                | ADDITION_OVERFLOW_UNCHECKED_PRAGMA
                | ASSUME_NO_ABORT_FROM_HERE_PRAGMA
                | EXPORT_ENSURES_PRAGMA
                | FRIEND_PRAGMA
                | DISABLE_INVARIANTS_IN_BODY_PRAGMA
                | DELEGATE_INVARIANTS_TO_CALLER_PRAGMA
                | BV_PARAM_PROP
                | BV_RET_PROP
                | UNROLL_PRAGMA
                | INFERENCE_PRAGMA
        ),
        Struct(..) => match pragma {
            INTRINSIC_PRAGMA | BV_PARAM_PROP => true,
            _ if INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS.contains_key(pragma) => bag
                .get(&symbols.make(INTRINSIC_PRAGMA))
                .map(|v| match v {
                    PropertyValue::Symbol(s) => symbols.string(*s).as_str() == INTRINSIC_TYPE_MAP,
                    _ => false,
                })
                .unwrap_or(false),
            // all other cases
            _ => false,
        },
        _ => false,
    }
}

/// Internal property attached to conditions if they are injected via an apply or a module
/// invariant.
pub const CONDITION_INJECTED_PROP: &str = "$injected";

/// Property which can be attached to conditions to make them exported into the VC context
/// even if they are injected.
pub const CONDITION_EXPORT_PROP: &str = "export";

/// Property which can be attached to a module invariant to make it global.
pub const CONDITION_GLOBAL_PROP: &str = "global";

/// Property which can be attached to a global invariant to mark it as not to be used as
/// an assumption in other verification steps. This can be used for invariants which are
/// nonoperational constraints on system behavior, i.e. the systems "works" whether the
/// invariant holds or not. Invariant marked as such are not assumed when
/// memory is accessed, but only in the pre-state of a memory update.
pub const CONDITION_ISOLATED_PROP: &str = "isolated";

/// Abstract property which can be used together with an opaque specification. An abstract
/// property is not verified against the implementation, but will be used for the
/// function's behavior in the application context. This allows to "override" the specification
/// with a more abstract version. In general we would need to prove the abstraction is
/// subsumed by the implementation, but this is currently not done.
pub const CONDITION_ABSTRACT_PROP: &str = "abstract";

/// Opposite to the abstract property.
pub const CONDITION_CONCRETE_PROP: &str = "concrete";

/// Property which indicates that an aborts_if should be assumed.
/// For callers of a function with such an aborts_if, the negation of the condition becomes
/// an assumption.
pub const CONDITION_ABORT_ASSUME_PROP: &str = "assume";

/// Property which indicates that an aborts_if should be asserted.
/// For callers of a function with such an aborts_if, the negation of the condition becomes
/// an assertion.
pub const CONDITION_ABORT_ASSERT_PROP: &str = "assert";

/// A property which can be attached to any condition to exclude it from verification. The
/// condition will still be type checked.
pub const CONDITION_DEACTIVATED_PROP: &str = "deactivated";

/// A property which marks a condition as inferred by the spec inference engine.
/// Values: `Bool(true)` for normal, `Symbol("vacuous")` for vacuously strong,
/// `Symbol("sathard")` for hard-to-solve quantifier patterns.
pub const CONDITION_INFERRED_PROP: &str = "inferred";

/// Symbol value for `inferred` property indicating vacuously strong conditions
/// (unconstrained quantifier variables).
pub const CONDITION_INFERRED_VACUOUS: &str = "vacuous";

/// Symbol value for `inferred` property indicating conditions with quantifiers
/// that are hard for SAT/SMT solvers (exists in aborts_if, forall in ensures).
pub const CONDITION_INFERRED_SATHARD: &str = "sathard";

/// Symbol value for `inferred` property indicating conditions suggested by an
/// AI agent (e.g. loop invariants that the WP engine cannot derive).
pub const CONDITION_INFERRED_AGENT: &str = "agent";

/// A property which can be attached to an aborts_with to indicate that it should act as check
/// whether the function produces exactly the provided number of error codes.
pub const CONDITION_CHECK_ABORT_CODES_PROP: &str = "check";

/// A property that can be attached to a global invariant to indicate that it should be
/// enabled disabled by the disable_invariant_in_body pragma
pub const CONDITION_SUSPENDABLE_PROP: &str = "suspendable";

/// A property that can be attached to a loop invariant to indicate that the loop needs to
/// be unrolled to a certain depth, a typical relaxation in bounded model checking.
pub const CONDITION_UNROLL_PROP: &str = "unroll";

/// A pragama defined in the spec block of a function or a struct
/// to explicitly specify which argument or field will be translated into a bv type in the boogie file
/// example: bv=b"0,1"
pub const BV_PARAM_PROP: &str = "bv";

/// A pragama defined in the spec block of a function
/// to explicitly specify which return value will be translated into a bv type in the boogie file
/// example: bv_ret=b"0,1"
pub const BV_RET_PROP: &str = "bv_ret";

/// A function which determines whether a property is valid for a given condition kind.
pub fn is_property_valid_for_condition(kind: &ConditionKind, prop: &str) -> bool {
    if matches!(
        prop,
        CONDITION_INJECTED_PROP
            | CONDITION_EXPORT_PROP
            | CONDITION_ABSTRACT_PROP
            | CONDITION_CONCRETE_PROP
            | CONDITION_DEACTIVATED_PROP
            | CONDITION_INFERRED_PROP
    ) {
        // Applicable everywhere.
        return true;
    }
    use crate::ast::ConditionKind::*;
    match kind {
        LoopInvariant => {
            matches!(prop, CONDITION_UNROLL_PROP)
        },
        GlobalInvariant(..) | GlobalInvariantUpdate(..) => {
            matches!(
                prop,
                CONDITION_GLOBAL_PROP | CONDITION_ISOLATED_PROP | CONDITION_SUSPENDABLE_PROP
            )
        },
        SucceedsIf | AbortsIf => matches!(
            prop,
            CONDITION_ABORT_ASSERT_PROP | CONDITION_ABORT_ASSUME_PROP
        ),
        AbortsWith => matches!(prop, CONDITION_CHECK_ABORT_CODES_PROP),
        _ => {
            // every other condition can only take general properties
            false
        },
    }
}
