// A value-capturing lambda whose captured local is also mutated through a
// sibling `&mut` argument of the same retained inline-opaque call is rejected.
// The closure model snapshots the captured local at call entry, while inline
// expansion reads it at the callback use site after the `&mut` mutation, so the
// two disagree on the result; the conflict check forbids the shape. (This is
// not a borrow conflict — the `&mut` borrow ends before the read — so regular
// compilation accepts it; the check is what keeps the prover model faithful.)
module 0x42::opaque_inline_value_capture_conflict {
    inline fun run(f: |u64| u64, r: &mut u64, x: u64): u64 {
        *r = 1;
        f(x)
    }
    spec run {
        pragma opaque;
        requires !aborts_of<f>(x);
        aborts_if false;
        ensures r == 1;
        ensures ensures_of<f>(x, result);
    }

    fun caller(): u64 {
        let c = 0;
        // error: `c` is mutated through one argument and captured through another
        run(|y| y + c, &mut c, 0)
    }
    spec caller {
        aborts_if false;
        ensures result == 1;
    }
}
