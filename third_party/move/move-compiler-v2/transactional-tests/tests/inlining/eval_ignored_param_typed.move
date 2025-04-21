//# publish
module 0x42::Test {
    inline fun foo(f:|u64, u64, u64| u64, g: |u64, u64, u64| u64, x: u64, _: u64, y: u64, z: u64): u64 {
	let r1 = f({x = x + 1; x}, {y = y + 1; y}, {z = z + 1; z});
	let r2 = g({x = x + 1; x}, {y = y + 1; y}, {z  = z + 1 ; z});
	r1 + r2 + 3*x + 5*y + 7*z
    }

    public fun test() {
	let r = foo(|x: u64, _: u64, z: u64| x*z, |_: u64, y: u64, _: u64| y, 1, 10, 100, 1000);
        assert!(r == 9637, r);
    }
}

//# run 0x42::Test::test
