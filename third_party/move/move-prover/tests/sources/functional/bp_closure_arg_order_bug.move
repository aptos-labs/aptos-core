// exclude_for: cvc5
// Regression: when a lambda captures a value whose callee parameter comes
// AFTER a `&mut` parameter (`|s| bump(s, d)` for `bump(e: &mut E, d: u64)`),
// the apply-procedure dispatch used to emit the call with captured arguments
// blindly prepended — (int, $Mutation E) where ($Mutation E, int) is expected —
// instead of interleaving them per the closure mask.
module 0x42::bp_closure_arg_order_bug {

    enum E has drop { V { x: u64 } }

    fun bump(e: &mut E, d: u64) {
        e.x += d;
    }

    spec bump {
        aborts_if e.x + d > MAX_U64;
        ensures e.x == old(e).x + d;
    }

    fun apply(f: |&mut E| has drop, e: &mut E) {
        f(e)
    }

    public fun caller(e: &mut E, d: u64) {
        apply(|s| bump(s, d), e)
    }
}
