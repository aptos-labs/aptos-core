// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A function type's Boogie name must record where its parameters end and its
// results begin.
//
// `fun_type` rendered `$fun_<params>_<results>`, where each side is a flat
// `_`-joined list of element suffixes. The split was therefore unrecoverable, and
// every arity split of one token sequence produced one name:
//
//   |u64, u8| u8    ->  $fun_u64_u8_u8
//   |u64| (u8, u8)  ->  $fun_u64_u8_u8
//
// These are distinct `mono_info.fun_infos` keys, so both were emitted. Boogie
// rejected the duplicate datatype, `$IsValid`, `$IsEqual` and `$apply`
// declarations and reported `wrong number of arguments in call to $apply`, so the
// whole file failed -- a correct program could not be verified.
//
// The arities are now part of the name. `translate_fun_type` also gained the
// entity-keyed emission guard the other three emission loops already had, so any
// residual collision is reported against the Move types that fused rather than
// reaching Boogie as a duplicate declaration.
//
// Both function types must be reachable from ONE verification target: function
// types are emitted per shard, so two reached from two different functions never
// meet in one Boogie file.

module 0x42::fun_type_arity_injectivity {

    fun two_in(_x: u64, y: u8): u8 { y }

    fun two_out(_x: u64): (u8, u8) { (1, 2) }

    /// Must verify. Reaches both arity splits of `u64, u8, u8`.
    public fun both_arities(): bool {
        let a: |u64, u8|u8 has copy + drop = two_in;
        let b: |u64|(u8, u8) has copy + drop = two_out;
        let r = a(1, 7);
        let (p, q) = b(3);
        r == 7 && p == 1 && q == 2
    }

    spec both_arities {
        ensures result == true;
    }
}
