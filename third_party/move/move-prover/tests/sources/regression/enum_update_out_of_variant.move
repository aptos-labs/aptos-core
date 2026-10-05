// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Updating a field on an enum receiver whose variant lacks that field is not provably a
// no-op: the result is unspecified.

module 0x42::enum_update_out_of_variant {

    enum E has copy, drop { A { f: u64 }, C { g: bool } }

    /// `update_field(e, f, _v)` type-checks on a `C` receiver even though `C` carries
    /// no field `f`. Returning `e` unchanged must not satisfy it.
    public fun set_f_unchanged(e: E, _v: u64): E {
        e
    }
    spec set_f_unchanged {
        requires e is C;
        ensures result == update_field(e, f, _v); // error: unspecified for a C receiver
    }

    /// Positive control on a receiver the wrapper does cover: the dispatch must stay
    /// precise there, i.e. an `A` receiver must not be routed to the unspecified arm.
    /// This has to verify.
    public fun set_f_covered(e: E, v: u64): E {
        if (e is A) {
            e.f = v
        };
        e
    }
    spec set_f_covered {
        requires e is A;
        ensures result == update_field(e, f, v);
    }
}
