// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `aborts_of` and `ensures_of` of a known target with type arguments read the call
/// a body's `f<T>(x)` denotes, at closed type arguments and at the verified function's
/// own type parameter, by their target's body and through its caller's contract.
module 0x1::generic_function_literals {
    struct Pending has key { n: u64 }
    struct Version has drop, store { major: u64 }

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

    /// Wherever its type arguments are written, the reading is one.
    fun same(x: u64): u64 { x }
    spec same {
        aborts_if false;
        ensures aborts_of<put<Version>>(Version { major: x }) ==
            aborts_of<put<Version>>(Version { major: x });
    }

    fun via(x: u64) {
        put<Version>(Version { major: x })
    }
    spec via {
        pragma opaque;
        modifies global<Pending>(@0x1);
        aborts_if aborts_of<put<Version>>(Version { major: x });
        ensures ensures_of<put<Version>>(Version { major: x });
    }

    fun check(s: u64) {
        assert!(s == 1, 1);
    }
    spec check {
        pragma opaque;
        aborts_if s != 1;
    }

    /// After a call leaving the memory as it was, the target reads from the entry.
    fun after_check(s: u64, x: u64) {
        check(s);
        put<Version>(Version { major: x })
    }
    spec after_check {
        pragma opaque;
        modifies global<Pending>(@0x1);
        aborts_if aborts_of<check>(s);
        aborts_if aborts_of<put<Version>>(Version { major: x });
        ensures ensures_of<put<Version>>(Version { major: x });
    }

    /// Through the contract of `via`, the target's contract.
    fun caller(x: u64) {
        via(x)
    }
    spec caller {
        modifies global<Pending>(@0x1);
        aborts_if !exists<Pending>(@0x1);
        ensures global<Pending>(@0x1).n == 1;
    }

    fun via_generic<T: drop + store>(x: T) {
        put<T>(x)
    }
    spec via_generic {
        pragma opaque;
        modifies global<Pending>(@0x1);
        aborts_if aborts_of<put<T>>(x);
        ensures ensures_of<put<T>>(x);
    }

    fun caller_generic(x: u64) {
        via_generic<Version>(Version { major: x })
    }
    spec caller_generic {
        modifies global<Pending>(@0x1);
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
        ensures ensures_of<put<Version>>(Version { major: 0 }); // error: another argument than the call's
    }

    fun no_call(x: u64) {
        let Version { major: _ } = Version { major: x };
    }
    spec no_call {
        aborts_if false;
        ensures ensures_of<put<Version>>(Version { major: x }); // error: nothing runs the target
    }
}
