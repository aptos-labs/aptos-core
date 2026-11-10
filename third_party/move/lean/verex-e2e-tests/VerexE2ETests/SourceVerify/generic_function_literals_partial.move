// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// In a unit verified through its model, `aborts_of` of a literal of a known target
/// needs exact aborts: it is not read as the stated disjunction of a target declaring
/// them partial, or of one stating none. `ensures_of` needs no exactness, and either
/// needs a contract, which a native without a specification has not.
module 0x1::generic_function_literals_partial {
    struct Pending has key { n: u64 }
    struct Version has drop, store { major: u64 }

    /// A ghost field: the unit is verified through its model.
    struct Marked has drop { v: u64 }
    spec Marked { ghost tag: num; }

    fun partial<T: drop + store>(x: T) {
        let p = &mut Pending[@0x1];
        p.n = 1;
        let _y = x;
    }
    spec partial {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if !exists<Pending>(@0x1);
        modifies global<Pending>(@0x1);
        ensures global<Pending>(@0x1).n == 1;
    }

    fun unstated<T: drop + store>(x: T) {
        let p = &mut Pending[@0x1];
        p.n = 1;
        let _y = x;
    }
    spec unstated {
        pragma opaque;
        modifies global<Pending>(@0x1);
        ensures global<Pending>(@0x1).n == 1;
    }

    native fun untold<T: drop + store>(x: T);

    fun partial_literal(x: u64): u64 {
        x
    }
    spec partial_literal {
        pragma aborts_if_is_partial;
        ensures result == x;
    }

    fun over_partial(x: u64) {
        partial<Version>(Version { major: x })
    }
    spec over_partial {
        pragma aborts_if_is_partial;
        modifies global<Pending>(@0x1);
        ensures ensures_of<partial<Version>>(Version { major: x });
    }

    fun over_partial_wrong(x: u64) {
        partial<Version>(Version { major: x })
    }
    spec over_partial_wrong {
        pragma aborts_if_is_partial;
        modifies global<Pending>(@0x1);
        ensures ensures_of<partial<Version>>(Version { major: x });
        ensures global<Pending>(@0x1).n == 0; // error: the target's contract states 1
    }

    fun aborts_over_partial(x: u64) {
        partial<Version>(Version { major: x })
    }
    spec aborts_over_partial {
        pragma aborts_if_is_partial;
        ensures aborts_of<partial<Version>>(Version { major: x }) ==> x == 0; // error: `partial` states only a lower bound of its aborts
    }

    fun aborts_over_unstated(x: u64) {
        unstated<Version>(Version { major: x })
    }
    spec aborts_over_unstated {
        pragma aborts_if_is_partial;
        ensures aborts_of<unstated<Version>>(Version { major: x }) ==> x == 0; // error: `unstated` states no aborts
    }

    fun aborts_over_literal(x: u64): u64 {
        partial_literal(x)
    }
    spec aborts_over_literal {
        pragma aborts_if_is_partial;
        ensures aborts_of<partial_literal>(x) ==> x == 0; // error: `partial_literal` states only a lower bound of its aborts
    }

    fun aborts_over_native(x: u64) {
        untold<Version>(Version { major: x })
    }
    spec aborts_over_native {
        pragma aborts_if_is_partial;
        ensures aborts_of<untold<Version>>(Version { major: x }) ==> x == 0; // error: `untold` has no specification
    }

    fun ensures_over_native(x: u64) {
        untold<Version>(Version { major: x })
    }
    spec ensures_over_native {
        pragma aborts_if_is_partial;
        ensures ensures_of<untold<Version>>(Version { major: x }); // error: `untold` has no specification
    }
}
