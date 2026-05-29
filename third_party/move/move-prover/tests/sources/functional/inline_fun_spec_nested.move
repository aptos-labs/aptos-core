// Regression: a non-opaque inline function with a function-level spec that
// itself calls an opaque inline function with a function-level spec.
//
// `outer_inline` (non-opaque) calls `inner_opaque` (opaque). When the prover
// verifies `outer_inline`'s body against its spec, the call to `inner_opaque`
// must be treated opaquely — the prover uses inner_opaque's spec at the call
// site, not the body. inner_opaque's spec lies (says `result == x + 2`) so the
// only way `outer_inline`'s spec `result == x + 2` can verify is if the
// substitution happens at the call site.
//
// `top` is a non-inline caller of `inner_opaque` whose only purpose is to
// trigger the finalizing inliner pass — without it, the snapshot is empty and
// the pass is skipped, hiding the regression.
module 0x42::TestInlineFunSpecNested {

    spec module {
        pragma verify = true;
    }

    public inline fun inner_opaque(x: u64): u64 {
        x + 1
    }
    spec inner_opaque {
        pragma opaque = true;
        pragma verify = false;
        aborts_if x == 0xFFFFFFFFFFFFFFFF || x == 0xFFFFFFFFFFFFFFFE;
        ensures result == x + 2;
    }

    public inline fun outer_inline(x: u64): u64 {
        inner_opaque(x)
    }
    spec outer_inline {
        aborts_if x == 0xFFFFFFFFFFFFFFFF || x == 0xFFFFFFFFFFFFFFFE;
        ensures result == x + 2;
    }

    // Forces the finalizing inliner pass to run by ensuring at least one
    // non-inline caller has a preserved opaque-inline-spec call.
    public fun top(): u64 {
        inner_opaque(41)
    }
    spec top {
        ensures result == 43;
    }
}
