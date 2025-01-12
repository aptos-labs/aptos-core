// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

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

// # Pragmas for intrinsic table declaration

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

/// All intrinsic functions associated with the map type
pub static INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS: Lazy<BTreeMap<&'static str, bool>> =
    Lazy::new(|| {
        BTreeMap::from([
            (INTRINSIC_FUN_MAP_NEW, true),
            (INTRINSIC_FUN_MAP_SPEC_NEW, false),
            (INTRINSIC_FUN_MAP_SPEC_GET, false),
            (INTRINSIC_FUN_MAP_SPEC_SET, false),
            (INTRINSIC_FUN_MAP_SPEC_DEL, false),
            (INTRINSIC_FUN_MAP_SPEC_LEN, false),
            (INTRINSIC_FUN_MAP_SPEC_IS_EMPTY, false),
            (INTRINSIC_FUN_MAP_SPEC_HAS_KEY, false),
            (INTRINSIC_FUN_MAP_LEN, true),
            (INTRINSIC_FUN_MAP_IS_EMPTY, true),
            (INTRINSIC_FUN_MAP_HAS_KEY, true),
            (INTRINSIC_FUN_MAP_DESTROY_EMPTY, true),
            (INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE, true),
            (INTRINSIC_FUN_MAP_ADD_OVERRIDE_IF_EXISTS, true),
            (INTRINSIC_FUN_MAP_DEL_MUST_EXIST, true),
            (INTRINSIC_FUN_MAP_DEL_RETURN_KEY, true),
            (INTRINSIC_FUN_MAP_BORROW, true),
            (INTRINSIC_FUN_MAP_BORROW_MUT, true),
            (INTRINSIC_FUN_MAP_BORROW_MUT_WITH_DEFAULT, true),
        ])
    });

/// Checks whether a pragma is valid in a specific spec block.
pub fn is_pragma_valid_for_block(
    symbols: &SymbolPool,
    bag: &PropertyBag,
    target: &SpecBlockContext<'_>,
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
        ),
        Function(..) => matches!(
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
