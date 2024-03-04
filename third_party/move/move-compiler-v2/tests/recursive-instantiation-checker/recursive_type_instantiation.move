module 0x42::m0 {
	struct S<T> {
		f: T
	}

	fun test0<T>() {
		test0<S<T>>()
	}

	fun test1<T>() {
		test2<T>()
	}

	fun test2<T>() {
		test1<S<T>>()
	}

	fun test3<T>() {
		test4<T>()
	}

	fun test4<T>() {
		test5<T>()
	}

	fun test5<T>() {
		test3<S<T>>()
	}
}
