// A function parameter applied more than once has no state anchor, but
// behavioral predicates over a PURE lambda do not need one: the derived
// conditions depend only on the predicate's arguments, so they resolve the
// same at any state. (State-reading abort conditions in this situation are
// rejected instead; see `aborts_of_state_multi_apply` in
// `bp_inline_errors.move`.)
module 0x42::aborts_of_pure_multi_apply {

    inline fun apply_twice(f: |u64| u64, x: u64): (u64, u64) {
        let a = f(x);
        let b = f(x);
        spec {
            assert !aborts_of<f>(x);
            assert ensures_of<f>(x, a);
            assert ensures_of<f>(x, b);
        };
        (a, b)
    }

    fun pure_twice(x: u64): (u64, u64) {
        apply_twice(|y| y + 1, x)
    }
    spec pure_twice {
        requires x < MAX_U64;
        ensures result_1 == x + 1;
        ensures result_2 == x + 1;
    }
}
