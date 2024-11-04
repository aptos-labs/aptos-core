module 0x42::test {
	struct Foo has drop {
		x: u8,
		y: bool
	}

	fun test0(y: Foo): Foo {
		Foo { x: 42, ..y }
	}
}
