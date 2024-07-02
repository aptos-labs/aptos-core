module A::A {
	use B::B;
	use B::C;

	fun foo() {
		B::foo();
		C::baz();
	}
}
