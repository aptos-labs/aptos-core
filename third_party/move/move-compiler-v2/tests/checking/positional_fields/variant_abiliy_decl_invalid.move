module 0x42::test {
	enum Foo<T> has drop {
		A(T),
		B(u8, bool),
	} has drop, copy;
}
