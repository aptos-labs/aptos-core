module 0x42::test {
	struct S(u8);

	fun foo(): S {
		S(0)
	}
}
