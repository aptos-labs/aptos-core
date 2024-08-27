module 0x42::test {
    struct S2<A, B, C> {
        x: A,
        y: B,
        z: C,
    }

    fun drop_S2<A, B, C>(s: S2<A, B, C>) {
        S2 { .. } = s;
    }

    fun proj_0_S2<A, B, C>(s: &S2<A, B, C>): &A {
        let a;
        S2 { x: a, .. } = s;
        a
    }

    fun proj_1_S2<A, B, C>(s: &S2<A, B, C>): &B {
        let b;
        S2 { y: b, .. } = s;
        b
    }

    fun proj_2_S2<A, B, C>(s: &S2<A, B, C>): &C {
        let c;
        S2 { z: c, .. } = s;
        c
    }
}
