//# publish
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

	fun proj_0_ref<A, B, C>(x: &S<A, B, C>): &A {
		let S(x, ..) = x;
		x
	}

	fun proj_2<A: drop, B: drop, C>(x: S<A, B, C>): C {
		let S(.., z) = x;
		z
	}

	fun proj_2_ref<A, B, C>(x: &S<A, B, C>): &C {
		let S(.., z) = x;
		z
	}

	fun get_x<A, B: drop>(self: T<A, B>): A {
		let T { x, .. } = self;
		x
	}

	fun get_x_ref<A, B>(self: &T<A, B>): &A {
		let T { x, .. } = self;
		x
	}

	fun test_proj_0(): u8 {
		let x = S(42, @0x1, true);
		x.proj_0()
	}

	fun test_proj_0_ref(): u8 {
		let x = S(42, @0x1, true);
		*proj_0_ref(&x)
	}

	fun test_proj_2(): bool {
		let x = S(42, @0x1, true);
		proj_2(x)
	}

	fun test_proj_2_ref(): bool {
		let x = S(42, @0x1, true);
		*proj_2_ref(&x)
	}

	fun test_get_x(): u8 {
		let x = T{ x: 42, y: true };
		get_x(x)
	}

	fun test_get_x_ref(): u8 {
		let x = T{ x: 42, y: true };
		*x.get_x_ref()
	}
}

//# run --verbose -- 0x42::test::test_proj_0

//# run --verbose -- 0x42::test::test_proj_0_ref

//# run --verbose -- 0x42::test::test_proj_2

//# run --verbose -- 0x42::test::test_proj_2_ref

//# run --verbose -- 0x42::test::test_get_x

//# run --verbose -- 0x42::test::test_get_x_ref
