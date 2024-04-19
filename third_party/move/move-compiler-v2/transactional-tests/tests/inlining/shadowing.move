//# publish
module 0x42::Test {

    public inline fun foo(f:|u64|) {
        let _x = 3;
        f(_x);
    }

    public fun test_shadowing() {
        let _x = 1;
        foo(|y| {
            _x = y  // We expect this to assign 3 via foo if renaming works correctly. If not it would
                    // have the value 1.
        });
        assert!(_x == 3, 0)
    }


}

//# run 0x42::Test::test_shadowing
