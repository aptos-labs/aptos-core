module 0x42::explicate_drop {
	fun unused_var() {
		let _x = 42;
	}

	fun unused_arg<T: drop>(x: T) {
	}

	fun id<T>(x: T): T {
		x
	}

	fun unused_call_assign() {
		let _x = id(42);
	}
}
