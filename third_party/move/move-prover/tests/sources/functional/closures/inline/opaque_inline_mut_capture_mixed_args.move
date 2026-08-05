// Test combining a mut-capturing closure with an independent plain `&mut`
// argument of the retained call (distinct roots, both mutated by the callee).
module 0x42::opaque_inline_mut_capture_mixed_args {
    inline fun modify_both(f: |u64|, r: &mut u64) {
        f(1);
        *r = *r + 100;
    }
    spec modify_both {
        pragma opaque;
        requires !aborts_of<f>(1);
        requires r < 1000000;
        aborts_if false;
        ensures ensures_of<f>(1);
        ensures r == old(r) + 100;
    }

    /// Test: the capture and the plain `&mut` argument target distinct roots.
    fun test_mixed_args(): (u64, u64) {
        let x = 10;
        let z = 20;
        let rx = &mut x;
        modify_both(|y| *rx = *rx + y spec {
            aborts_if rx + y > MAX_U64;
            ensures rx == old(rx) + y;
        }, &mut z);
        (x, z)
    }
    spec test_mixed_args {
        ensures result_1 == 11;
        ensures result_2 == 120;
    }
}
