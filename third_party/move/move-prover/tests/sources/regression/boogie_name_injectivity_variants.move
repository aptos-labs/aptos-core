// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Generated Boogie names must be injective: enum variant constructors.
//
// `boogie_struct_variant_name` appends the variant to the already-rendered
// struct name, so enum `E` with variant `A_B` and enum `E_A` with variant `B`
// both rendered the constructor `$42_variant_names_E_A_B`. Both datatypes are
// emitted -- `E` and `E_A` are distinct entities, so the emission dedup does not
// drop either -- but the two datatypes then declare one constructor name, which
// Boogie rejects. Nothing in this file verifies before the fix.
//
// Both enums must be reachable from ONE verification target: verification is
// sharded per target and only reachable types are emitted per shard, so two
// colliding constructors reached from two different functions never meet in one
// Boogie file and the collision stays hidden.

module 0x42::variant_names {
    enum E has copy, drop {
        A_B { x: u64 }
    }

    enum E_A has copy, drop {
        B { y: bool }
    }

    /// Must verify. Reaches both colliding constructors, and their payload
    /// types differ, so one constructor cannot represent both.
    public fun both_variants(x: u64, y: bool): bool {
        let l = E::A_B { x };
        let r = E_A::B { y };
        l.x == x && r.y == y
    }

    spec both_variants {
        aborts_if false;
        ensures result == true;
    }
}
