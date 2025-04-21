//# publish
module 0x42::Test {

    public inline fun foo(f:|u64|) {
        let x = 3;
        f(x);
    }

    public fun test_shadowing() {
        let x = 1;
        foo(|y: u64| {
            x = y  // We expect this to assign 3 via foo if renaming works correctly. If not it would
                    // have the value 1.
        });
        assert!(x == 3, 0)
    }


}

//# run 0x42::Test::test_shadowing
