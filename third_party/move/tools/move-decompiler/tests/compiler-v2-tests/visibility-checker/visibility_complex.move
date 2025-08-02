module 0x42::A {
	friend 0x42::C;
	public(friend) fun foo() {}
}

module 0x42::B {
	public(package) fun foo() {}
}

module 0x42::C {
	friend 0x42::D;
	public(friend) fun foo() {
		0x42::A::foo();
		0x42::B::foo();
	}
}

module 0x42::D {
	public(package) fun bar() {
		0x42::B::foo();
		0x42::C::foo();
	}
}
