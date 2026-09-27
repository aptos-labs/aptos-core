// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Updating a field on an enum receiver whose variant does not declare that field
// must not be provably a no-op.
//
// `update_field(s, f, v)` is a user-facing spec builtin that accepts enum receivers,
// and for an enum field the backend emits a merged `$Update` wrapper that dispatches
// on the receiver's constructor. That dispatch chain used to close with `else s`, so
// a receiver outside the wrapper's variant set was returned unchanged. The encoding
// therefore claimed that updating a field the dispatched variant does not carry is
// the identity, which let the prover discharge `result == update_field(e, f, v)`
// against a body that never wrote anything.
//
// The chain now closes with an uninterpreted value, so such a receiver is
// unspecified rather than unchanged.

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
