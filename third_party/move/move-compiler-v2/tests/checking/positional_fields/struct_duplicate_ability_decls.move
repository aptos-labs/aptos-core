module 0x42::test {
	struct S has copy {} has copy;

	struct T<T> has copy { x: T } has drop;
}
