module 0x42::test {
	struct S1<A, B, C>(A, B, C);

	struct S2<A, B, C> {
		x: A,
		y: B,
		z: C,
	}

	fun proj_0_S1<A, B>(x: &S1<A, B>): &A {
		let a;
		S1(a, b) = x;
		a
	}

	fun proj_0_S2<A, B, C>(s: &S2<A, B, C>): &A {
		let a;
		let y;
		let z;
		S2 { x: a, y, z} = s;
		a
	}
}
