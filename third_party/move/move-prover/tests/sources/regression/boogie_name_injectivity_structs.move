// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Struct datatype names, and instantiation suffixes built from them, are injective:
// `0x42::Y_A::B` and `0x42::Y::A_B` are distinct datatypes. Both are reachable from one
// verification target, since only reachable types are emitted per shard.

module 0x42::Y_A {
    struct B has copy, drop, store { v: u64 }

    public fun mk(v: u64): B { B { v } }

    public fun get(b: &B): u64 { b.v }
}

module 0x42::Y {
    struct A_B has copy, drop, store { w: bool }

    public fun mk(w: bool): A_B { A_B { w } }

    public fun get(b: &A_B): bool { b.w }
}

module 0x42::struct_names {
    use 0x42::Y_A;
    use 0x42::Y;

    struct S<T: copy + drop + store, phantom U> has copy, drop, store { t: T }

    /// Must verify. Reaches both colliding datatypes, so one datatype serving
    /// both leaves one of the two field selectors undeclared.
    public fun both_structs(v: u64, w: bool): bool {
        Y_A::get(&Y_A::mk(v)) == v && Y::get(&Y::mk(w)) == w
    }

    spec both_structs {
        aborts_if false;
        ensures result == true;
    }

    /// Must verify. Reaches both colliding instantiations of `S`.
    public fun both_instantiations(v: u64, w: bool): bool {
        let l = S<Y_A::B, u8> { t: Y_A::mk(v) };
        let r = S<Y::A_B, u8> { t: Y::mk(w) };
        Y_A::get(&l.t) == v && Y::get(&r.t) == w
    }

    spec both_instantiations {
        aborts_if false;
        ensures result == true;
    }
}
