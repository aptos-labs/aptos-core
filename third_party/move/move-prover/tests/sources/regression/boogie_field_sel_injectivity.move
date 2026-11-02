// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A field selector identifies one Move field: same-named fields of different variants, and a
// ghost field spelled like their join, stay distinct. `G` and `H` are each reachable from one
// verification target, since a datatype is only emitted in shards that reach it.

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
