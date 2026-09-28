// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Enum variant constructor names are injective: variant `A_B` of `E` and variant `B` of `E_A`
// are distinct constructors. Both enums are reachable from one verification target, since
// only reachable types are emitted per shard.

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
