module 0x42::test {
	struct S0(u8);

	struct S1(bool, S0);

	enum E1 {
		V1(S0),
		V2(S1)
	}

	fun simple(x: S0) {
		let S0(_x) = x;
	}

	fun nested(x: S1) {
		let S1(_x, S0(_y)) = x;
	}

	fun match(x: E1) {
		match (x) {
			E1::V1(S0(_x)) => {},
			E1::V2(S1(_x, S0(_y))) => {}
		}
	}
}
