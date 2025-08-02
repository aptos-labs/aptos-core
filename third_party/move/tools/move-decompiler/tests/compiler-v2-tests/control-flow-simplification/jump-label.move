module 0x42::test {
	use 0x1::string::{String, Self};

	fun foo<T>(): String {
		abort 0
	}

	fun bar<T>(_x: bool): T {
		abort 0
	}

	fun baz<T>(_x: vector<u8>): T {
		abort 0
	}

	fun test<T>(x: vector<u8>): T {
		let y = foo<T>();
		if (y == string::utf8(b"bool")) {
			let z = baz(x);
			return bar<T>(z)
		} else if (y == string::utf8(b"u8")) {
			let z = baz(x);
			return bar<T>(z)
		} else if (y == string::utf8(b"u64")) {
			let z = baz(x);
			return bar<T>(z)
		}else {
			abort 0
		}
	}
}
