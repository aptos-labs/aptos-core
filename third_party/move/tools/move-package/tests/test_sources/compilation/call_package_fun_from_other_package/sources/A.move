module A::A {
	use B::B;

	fun foo() {
		B::foo();
	}
}
