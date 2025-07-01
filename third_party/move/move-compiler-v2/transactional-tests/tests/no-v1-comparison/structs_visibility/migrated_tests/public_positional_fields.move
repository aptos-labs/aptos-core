//# publish
module 0x42::test {
	public struct Tup<T, U>(T, U);
}

//# publish
module 0x42::test_positional_fields {
	use 0x42::test::Tup;

	fun baz(x: u64, y: u64): u64 {
		let Tup(y, x) = Tup(x, y);
		y - x
	}

	fun test_ok() {
		assert!(baz(2, 1) == 1, 42);
	}
}

//# run --verbose -- 0x42::test_positional_fields::test_ok
