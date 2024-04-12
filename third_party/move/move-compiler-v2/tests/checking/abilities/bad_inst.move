module 0x42::test {
	struct T { }

	struct S<Y: drop> {
		x: Y
	}

	struct U<X> {
		x: X
	}

	fun test(): U<S<T>> {
		U<S<T>> { x: S { x: T { } } }
	}
}
