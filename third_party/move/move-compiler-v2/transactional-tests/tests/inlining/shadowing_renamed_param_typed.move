//# publish
module 0x42::Test {

    public inline fun foo(f:|u64|, x:u64) {
        f(x);
    }

    public inline fun foo2(f:|u64|, x:u64) {
        let x = x;
        f(x);
    }

    public fun test_shadowing(x: u64) {
        foo(|y: u64| {
            x = y  // We expect this to assign 3 via foo if renaming works correctly. If not it would
                    // have the value 1.
        }, 3);
        assert!(x == 3, 0);

        foo2(|y: u64| {
            x = y  // We expect this to assign 3 via foo if renaming works correctly. If not it would
                    // have the value 1.
        }, 5);
        assert!(x == 5, 0)
    }

    public fun test_shadowing2(q: u64) {
        let x = q;
        foo(|y: u64| {
            x = y  // We expect this to assign 3 via foo if renaming works correctly. If not it would
                    // have the value 1.
        }, 3);
        assert!(x == 3, 0);

        foo2(|y: u64| {
            x = y  // We expect this to assign 3 via foo if renaming works correctly. If not it would
                    // have the value 1.
        }, 5);
        assert!(x == 5, 0)
    }

    fun test_shadowing_entry() {
        test_shadowing(1);
        test_shadowing2(1)
    }
}

//# run 0x42::Test::test_shadowing_entry
