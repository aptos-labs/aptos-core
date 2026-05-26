// Regression test for bug: rename_quant_vars_in_exp picked the same nice name
// for both an inner and outer quantifier binder when a loop modifies both a
// struct and a u64 counter.
//
// When a loop havocs multiple variables (here `w: &mut Wrapper` and `i: u64`),
// the WP engine generates nested forall quantifiers:
//   forall $qi: u64: (forall $qw: Wrapper: body)
//
// rename_quant_vars_in_exp renames $-prefixed bound variables to short nice
// names.  It processes the expression bottom-up: the inner `$qw` is renamed to
// `x` first.  When the outer `$qi` is processed, `x` is *bound* (not free) in
// the body, so the free-variable scan does not add `x` to used_names.  Without
// the fix the outer `$qi` is also renamed to `x`, and then substituting
// `$qi → x` inside the body makes `$qi >= 0` become `x >= 0` where `x` is
// already bound as `Wrapper` by the inner quantifier → type error.
//
// The fix collects inner quantifier binder names into used_names so the outer
// rename picks a distinct name (`y`).
//
// flag: -T=20
module 0x42::nested_quant_rename {

    struct Wrapper has copy, drop {
        val: u64,
    }

    spec Wrapper {
        // A struct invariant causes DataInvariantInstrumentationProcessor to
        // inject a Prop before SpecInferenceProcessor runs, so the WP sees a
        // struct-valued forall wrapper for the Wrapper havoc.
        invariant val >= 0;
    }

    // Loop modifying both `w` (Wrapper) and `i` (u64).
    // WP produces nested foralls: forall $qi: u64: (forall $qw: Wrapper: P)
    // Without the fix: both get renamed to `x` → `forall x: Wrapper: x >= 0`
    //   type error (x is Wrapper, not u64).
    // With the fix: inner gets `x`, outer gets `y` → correct types throughout.
    fun inc_loop(w: &mut Wrapper, n: u64) {
        let i = 0;
        while ({
            spec {
                invariant [inferred] i <= n;
                invariant [inferred] w.val == old(w).val + i;
            };
            i < n
        }) {
            w.val += 1;
            i += 1;
        };
    }
    spec inc_loop(w: &mut Wrapper, n: u64) {
        pragma opaque = true;
        ensures [inferred] w.val == old(w).val ==> w == Wrapper{val: old(w).val + n};
        aborts_if [inferred] w.val + n > MAX_U64;
    }

}
/*
Verification: Succeeded.
*/
