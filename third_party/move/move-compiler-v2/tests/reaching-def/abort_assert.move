module 0x42::abort_assert {

    // Test simple abort - code after abort is unreachable
    fun simple_abort(): u64 {
        let x = 1;
        abort 0;
        x  // unreachable, but x still has def from above
    }

    // Test conditional abort
    fun conditional_abort(cond: bool): u64 {
        let x = 1;
        if (cond) {
            abort 0;
        };
        x  // x has def from init (abort path doesn't reach here)
    }

    // Test assignment before abort in branch
    fun assign_before_abort(cond: bool): u64 {
        let x;
        if (cond) {
            x = 1;
            abort 0;
        } else {
            x = 2;
        };
        x  // x only has def from else branch (true branch aborts)
    }

    // Test assert (which may abort)
    fun with_assert(n: u64): u64 {
        let x = 1;
        assert!(n > 0, 100);
        x  // x has def from init (assert may or may not abort)
    }

    // Test multiple asserts
    fun multiple_asserts(a: u64, b: u64): u64 {
        let x = 1;
        assert!(a > 0, 100);
        let y = 2;
        assert!(b > 0, 101);
        x + y
    }

    // Test abort in nested conditional
    fun nested_abort(a: bool, b: bool): u64 {
        let x = 0;
        if (a) {
            if (b) {
                x = 1;
                abort 0;
            } else {
                x = 2;
            };
        } else {
            x = 3;
        };
        x  // x has defs from: else-inner (2) and else-outer (3)
    }

    // Test abort after assignment in same block
    fun abort_after_assign(): u64 {
        let x = 1;
        let y = x + 1;
        abort y;
        x  // unreachable
    }

    // Test conditional with both branches potentially aborting
    fun both_may_abort(cond: bool, n: u64): u64 {
        let x;
        if (cond) {
            assert!(n > 0, 100);
            x = 1;
        } else {
            assert!(n > 10, 101);
            x = 2;
        };
        x  // x has defs from both branches (asserts don't guarantee abort)
    }
}
