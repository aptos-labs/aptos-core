//# publish
module 0x42::Test {

    public fun quux(f:|u64, u64|, _z: u64) {
        let x = 3;
	let q = 5;
        f(x, q);
    }

    public fun foo(f:|u64, u64|, z: u64) {
        quux(|a, b| f(a, b), z);
    }

    public fun test_shadowing() {
        let _x = 1;
	let z = 4;
        foo(|y, _q| {
            _x = y  // We expect this to assign 3 via foo if renaming works correctly. If not it would
                    // have the value 1.
        }, z);
        assert!(_x == 3, 0)
    }

    public fun test_shadowing2() {
        let _x = 1;
	let z = 4;
        quux(|y, _q| {
            _x = y  // We expect this to assign 3 via foo if renaming works correctly. If not it would
                    // have the value 1.
        }, z);
        assert!(_x == 3, 0)
    }
}

//# run 0x42::Test::test_shadowing
