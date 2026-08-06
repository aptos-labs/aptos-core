// Tests ghost fields (`ghost f: T;` in struct spec blocks): model-only fields
// carried by each value. Ghosts without an initializer are fresh at pack
// (arbitrary within the declared type's value domain), preserved by every
// copy/move/borrow, excluded from equality, updatable from spec blocks, and
// readable in specification expressions. Writes (pack initializers and
// updates) are asserted to stay in the declared type's domain; use `num`
// for an unbounded ghost integer.
//
// Equality semantics are tested in `ghost_field_equality.move`, initializers
// and data invariants in `ghost_field_invariants.move`, and behavioral
// predicate (stored function) invariants in `ghost_field_bp.move`.
module 0x42::ghost_field {
    struct S has copy, drop { x: u64 }
    spec S {
        ghost g: u64;
    }

    // The ghost travels with the value: exact propagation, no extra rules.
    fun pass_through(s: S): S { s }
    spec pass_through {
        ensures result == s;
        ensures result.g == s.g;
    }

    // Equality ignores ghosts: two separately packed values with equal
    // runtime fields are equal, although their ghosts are unrelated.
    fun make_two(): (S, S) { (S { x: 1 }, S { x: 1 }) }
    spec make_two {
        ensures result_1 == result_2;
    }

    // Ghosts are fresh (unconstrained) at pack: this must FAIL.
    fun fresh_unconstrained(): S { S { x: 1 } }
    spec fresh_unconstrained {
        ensures result.g == 0;
    }

    // Update through a mutable reference.
    fun set_via_ref(s: &mut S) {
        spec {
            update s.g = 7;
        };
    }
    spec set_via_ref {
        ensures s.g == 7;
    }

    // Update of an owned local; the ghost flows to the returned value.
    fun set_local(): S {
        let s = S { x: 1 };
        spec {
            update s.g = 5;
        };
        s
    }
    spec set_local {
        ensures result.g == 5;
        ensures result.x == 1;
    }

    // Reading a ghost in an inline spec block.
    fun read_inline(s: &mut S) {
        spec {
            update s.g = 3;
        };
        spec {
            assert s.g == 3;
        };
    }

    // Ghost fields on enums are variant-agnostic: declared once, present in
    // every variant, selectable and updatable regardless of the variant.
    enum E has copy, drop { A { y: u64 }, B }
    spec E {
        ghost h: u64;
    }

    fun enum_pass(e: E): E { e }
    spec enum_pass {
        ensures result == e;
        ensures result.h == e.h;
    }

    fun enum_update(e: &mut E) {
        spec {
            update e.h = 9;
        };
    }
    spec enum_update {
        ensures e.h == 9;
    }

    // Ghost field with a type from the struct's type parameters.
    struct P<T: copy + drop> has copy, drop { v: T }
    spec P {
        ghost t: T;
    }

    fun generic_pass<T: copy + drop>(p: P<T>): P<T> { p }
    spec generic_pass {
        ensures result.t == p.t;
    }

    // ---- Ghost updates through a nested mutable borrow of a local ----
    // clean_and_optimize would previously drop the write-back chain for `r`
    // (spec_instrumentation's WriteRef is emitted after the optimizer), so
    // the ghost update on `r` didn't propagate back to `s`. Fix keeps the
    // Prop marker's base local marked as written.
    struct NB has copy, drop { x: u64 }
    spec NB { ghost g: u64; }

    fun nb_via_local_borrow(): NB {
        let s = NB { x: 0 };
        let r = &mut s;
        spec { update r.g = 7; };
        s
    }
    spec nb_via_local_borrow { ensures result.g == 7; }

    // Two updates through the same borrow — the later value wins.
    fun nb_two_updates(): NB {
        let s = NB { x: 0 };
        let r = &mut s;
        spec { update r.g = 3; };
        spec { update r.g = 5; };
        s
    }
    spec nb_two_updates { ensures result.g == 5; }

    // ---- Same-module ghost field type ----
    // At declaration analysis, the enclosing module is still being built —
    // types referenced by a ghost field may not be visible in `GlobalEnv`
    // yet. The recursive-type check must fall back to the builder's tables
    // for same-module references. Verifies without panic.
    struct SmA has copy, drop { x: u64 }
    struct SmB has copy, drop { y: u64 }
    spec SmA { ghost b: SmB; }

    fun sm_pack(): SmA { SmA { x: 1 } }

    // ---- Runtime-empty struct with a ghost field ----
    // The ghost is selectable and updatable even when the runtime layout is
    // empty, and its declared type's range holds from the start.
    struct EmptyG has copy, drop {}
    spec EmptyG { ghost g: u64; }

    fun empty_g_roundtrip(): EmptyG {
        let e = EmptyG {};
        spec { update e.g = 5; };
        e
    }
    spec empty_g_roundtrip { ensures result.g == 5; }

    fun empty_g_pack(): EmptyG { EmptyG {} }
    spec empty_g_pack { ensures result.g <= 18446744073709551615; }

    // ---- Ghost update RHS references a temp that copy propagation and
    // liveness would eliminate ----
    // `let z = y; spec { update s.g = z; };` used to leave the fun-spec's
    // stale temp for `z` in the emitted update, even though the code's
    // maintained Prop had been renumbered to `y`. `align_temps` now walks
    // both sides in parallel to build the stale→fresh temp map.
    struct DceS has copy, drop { x: u64 }
    spec DceS { ghost g: u64; }

    fun dce_copy(s: &mut DceS, y: u64) {
        let z = y;
        spec { update s.g = z; };
    }
    spec dce_copy { ensures s.g == y; }

    // ---- Range check on copies across declared domains ----
    // The domain authority for the skip decision is the SOURCE's declaration,
    // not the expression's (coerced) node type: copying a `num`-declared
    // ghost into a bounded ghost keeps the range assert.
    struct Wide has copy, drop { x: u64 }
    spec Wide { ghost w: num; }
    struct Narrow has copy, drop { x: u64 }
    spec Narrow { ghost n: u8; }

    // Unbounded num into u8: the range assert must FAIL.
    fun copy_num_to_u8_unbounded(a: Wide, b: &mut Narrow) {
        spec { update b.n = a.w; };
    }

    // Same copy with the domain established: verifies exactly.
    fun copy_num_to_u8_bounded(a: Wide, b: &mut Narrow) {
        spec { update b.n = a.w; };
    }
    spec copy_num_to_u8_bounded {
        // `num` is a mathematical integer: bound both sides.
        requires a.w >= 0 && a.w < 256;
        ensures b.n == a.w;
    }
}
