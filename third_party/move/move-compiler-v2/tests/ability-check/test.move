module 0x42::test {
	struct S<T> has key {
		f: T
	}

	fun test(a: address) acquires S {
		borrow_global<S<bool>>(a);
	}
}
