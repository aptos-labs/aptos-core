module 0x42::test {
	struct T { }

	struct S<T: drop> {
		x: T
	}

	struct U<T> {
		x: T
	}

	fun test(): U<S<T>> {
		U<S<T>> { x: S { x: T { } } }
	}
}
