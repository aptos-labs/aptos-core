module 0x42::test {
	#[test_only]
	use 0x42::foo;

	#[test]
	fun test() {
		assert!(foo::foo() == 42, 0);
	}
}
