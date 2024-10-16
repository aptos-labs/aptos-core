module A::A {
	use B::B;
	public fun foo() {
		B::foo();
	}
}
