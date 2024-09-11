//# publish
module 0x42::test {
	struct Tup<T, U>(T, U);

	fun baz(x: u64, y: u64): u64 {
		let Tup(y, x) = Tup(x, y);
		y - x
	}

	fun test_ok() {
		assert!(baz(2, 1) == 1, 42);
	}
}

//# run --verbose -- 0x42::test::test_ok
