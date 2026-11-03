// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// In a unit verified through its model, `aborts_of` and `ensures_of` of a literal of
/// a known target are what its contract states at the frame its type arguments induce,
/// whether a call of it goes through that contract or its body is inlined.
module 0x1::generic_function_literals_model {
    struct Pending has key { n: u64 }
    struct Version has drop, store { major: u64 }

    /// A ghost field: the unit is verified through its model.
    struct Marked has drop { v: u64 }
    spec Marked { ghost tag: num; }

    fun check(s: u64) {
        assert!(s == 1, 1);
    }
    spec check {
        pragma opaque;
        aborts_if s != 1;
    }

    fun put<T: drop + store>(x: T) {
        let p = &mut Pending[@0x1];
        p.n = 1;
        let _y = x;
    }
    spec put {
        pragma opaque;
        aborts_if !exists<Pending>(@0x1);
        modifies global<Pending>(@0x1);
        ensures global<Pending>(@0x1).n == 1;
    }

    fun via(s: u64, x: u64) {
        check(s);
        put<Version>(Version { major: x })
    }
    spec via {
        pragma opaque;
        modifies global<Pending>(@0x1);
        aborts_if s != 1;
        aborts_if aborts_of<put<Version>>(Version { major: x });
        ensures ensures_of<put<Version>>(Version { major: x });
    }

    /// A literal without type arguments.
    fun guarded(s: u64, x: u64) {
        check(s);
        put<Version>(Version { major: x })
    }
    spec guarded {
        pragma opaque;
        modifies global<Pending>(@0x1);
        aborts_if aborts_of<check>(s);
        aborts_if aborts_of<put<Version>>(Version { major: x });
        ensures ensures_of<put<Version>>(Version { major: x });
    }

    /// Targets a call inlines, their aborts stated exactly.
    fun positive<T: drop>(v: u64, x: T): T {
        assert!(v > 0, 1);
        x
    }
    spec positive {
        aborts_if v == 0;
        ensures result == x;
    }

    fun inlined_generic(v: u64, x: u64): u64 {
        let Version { major } = positive<Version>(v, Version { major: x });
        major
    }
    spec inlined_generic {
        aborts_if aborts_of<positive<Version>>(v, Version { major: x });
        ensures result == x;
    }

    fun nonzero(v: u64): u64 {
        assert!(v > 0, 1);
        v
    }
    spec nonzero {
        aborts_if v == 0;
        ensures result == v;
    }

    fun inlined_literal(v: u64): u64 {
        nonzero(v)
    }
    spec inlined_literal {
        aborts_if aborts_of<nonzero>(v);
        ensures ensures_of<nonzero>(v, result);
    }

    /// Through the contract of `via`, the target's contract.
    fun caller(s: u64, x: u64) {
        via(s, x)
    }

    fun caller_guarded(s: u64, x: u64) {
        guarded(s, x)
    }
    spec caller_guarded {
        modifies global<Pending>(@0x1);
        aborts_if s != 1;
        aborts_if !exists<Pending>(@0x1);
        ensures global<Pending>(@0x1).n == 1;
    }
    spec caller {
        modifies global<Pending>(@0x1);
        aborts_if s != 1;
        aborts_if !exists<Pending>(@0x1);
        ensures global<Pending>(@0x1).n == 1;
    }

    fun other_argument(x: u64) {
        put<Version>(Version { major: x })
    }
    spec other_argument {
        pragma opaque;
        modifies global<Pending>(@0x1);
        aborts_if aborts_of<put<Version>>(Version { major: x });
        ensures global<Pending>(@0x1).n == 0; // error: the target's contract states 1
    }
}
