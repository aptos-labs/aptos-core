module 0x42::test {
	struct S1<A, B, C>(A, B, C);

	struct S2<A, B, C> {
		x: A,
		y: B,
		z: C,
	}

	fun drop_S1<A, B, C>(x: S1) {
		S1(..) = x;
	}

	fun proj_0_S1<A, B>(x: &S1<A, B>): &A {
		let a;
		S1(a, ..) = x;
		a
	}

	fun proj_1_S1<A, B>(x: &S1<A, B>): &B {
		let b;
		S1(_, b, ..) = x;
		S1(.., b, _) = x;
		b
	}

	fun proj_2_S1<A, B>(x: &S1<A, B>): &C {
		let c;
		S1(.., c) = x;
		c
	}
}
