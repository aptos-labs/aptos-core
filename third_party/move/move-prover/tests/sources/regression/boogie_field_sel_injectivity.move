// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A field selector must identify one Move field.
//
// `boogie_field_sel` appends the variant to the field name so that same-named
// fields of different variants stay apart. Joined with a bare `_`, that was not
// injective: field `a` of variant `B_C` and field `a_B` of variant `C` both
// rendered `$a_B_C`, as did a ghost field literally named `a_B_C`, which carries
// no variant at all.
//
// Boogie shares identically-named fields across a datatype's constructors, so the
// collision was silent whenever the two fields had the same type -- two distinct
// Move fields simply became one selector. With different types it surfaced as
// `type mismatch between field $a_B_C and identically-named field in constructor`,
// and against a ghost field as `more than one declaration of variable name`.
//
// `G` and `H` must be reachable from one verification target each; a datatype is
// only emitted in shards that reach it.

module 0x42::boogie_field_sel_injectivity {

    /// Differently-typed collision: `$a_B_C` from two directions.
    enum G has copy, drop {
        B_C { a: u64 },
        C { a_B: bool },
    }

    /// Must verify. Both selectors have to survive as distinct fields.
    public fun distinct_selectors(p: u64, q: bool): bool {
        let l = G::B_C { a: p };
        let r = G::C { a_B: q };
        l.a == p && r.a_B == q
    }

    spec distinct_selectors {
        aborts_if false;
        ensures result == true;
    }

    /// Ghost-field collision: a ghost carries no variant, so its bare name could
    /// equal a variant field's joined name.
    enum H has copy, drop {
        B_C { a: u64 },
        D { z: bool },
    }

    spec H {
        ghost a_B_C: bool = false;
    }

    /// Must verify.
    public fun ghost_does_not_shadow_field(p: u64): H {
        H::B_C { a: p }
    }

    spec ghost_does_not_shadow_field {
        aborts_if false;
        ensures result.a == p;
    }
}
