//# publish
module 0x42::test {
	public struct S<A, B, C>(A, B, C) has drop;

	public struct T<A, B> has drop {
		x: A,
		y: B
	}
}

//# publish
module 0x42::test_projection {
	use 0x42::test::S;
	use 0x42::test::T;


	fun proj_0<A, B: drop, C: drop>(s: S<A, B, C>): A {
		let S(x, ..) = s;
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

	fun get_x<A, B: drop>(s: T<A, B>): A {
		let T { x, .. } = s;
		x
	}

	fun get_x_ref<A, B>(s: &T<A, B>): &A {
		let T { x, .. } = s;
		x
	}

	fun test_proj_0(): u8 {
		let x = S(42, @0x1, true);
		proj_0(x)
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
		*get_x_ref(&x)
	}
}

//# run --verbose -- 0x42::test_projection::test_proj_0

//# run --verbose -- 0x42::test_projection::test_proj_0_ref

//# run --verbose -- 0x42::test_projection::test_proj_2

//# run --verbose -- 0x42::test_projection::test_proj_2_ref

//# run --verbose -- 0x42::test_projection::test_get_x

//# run --verbose -- 0x42::test_projection::test_get_x_ref
