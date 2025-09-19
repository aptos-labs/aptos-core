module 0x42::test {
	struct S<A, B, C>(A, B, C) has drop;

	struct T<A, B> has drop {
		x: A,
		y: B
	}

	fun proj_0<A, B: drop, C: drop>(self: S<A, B, C>): A {
		let S(x, ..) = self;
		x
	}

	fun test_proj_0(): u8 {
		let x = S(42, @0x1, true);
		x.proj_0()
	}
}
