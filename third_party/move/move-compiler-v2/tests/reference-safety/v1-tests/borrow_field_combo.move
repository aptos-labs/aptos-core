module 0x8675309::M {
    struct Outer has copy, drop { s1: Inner, s2: Inner }
    struct Inner has copy, drop { f1: u64, f2: u64 }
    fun id<T>(r: &T): &T {
        r
    }
    fun id_mut<T>(r: &mut T): &mut T {
        r
    }

    fun t0(cond: bool, outer: &mut Outer, other: &mut Outer) {
        let inner = &outer.s1;
        let c; if (cond) c = copy inner else c = &outer.s1; // error in v2
        let f1 = &inner.f1;
        *c;
        *inner;
        *f1;
        *inner;
        *c;

        let inner = &outer.s1;
        let c; if (cond) c = id(copy inner) else c = id(&outer.s1);
        let f1 = &inner.f1;
        *c;
        *inner;
        *f1;
        *inner;
        *c;

        let inner = &mut outer.s1;
        let c; if (cond) c = freeze(copy inner) else c = &other.s1;
        let f1 = &inner.f1;
        *c;
        *inner;
        *f1;
        *inner;
        *c;

        let inner = &mut outer.s1;
        let c; if (cond) c = id(freeze(copy inner)) else c = &other.s1; // error in v2
        let f1 = &inner.f1;
        *c;
        *inner;
        *f1;
        *inner;
        *c;

        let inner = &mut outer.s1;
        let c; if (cond) c = copy inner else c = &mut outer.s2; // error in v2
        let f1 = &mut c.f1;
        *f1;
        *c;
        *inner;

        let inner = &mut outer.s1;
        let c; if (cond) c = id_mut(copy inner) else c = &mut outer.s2; // error in v2
        let f1 = &mut c.f1;
        *f1;
        *c;
        *inner;
    }

    fun t1(cond: bool, outer: &mut Outer, other: &mut Outer) {
        let inner = &outer.s1;
        let c; if (cond) c = &inner.f1 else c = &inner.f2;
        let f1 = &inner.f1;
        *c;
        *inner;
        *f1;
        *inner;
        *c;

        let inner = &outer.s1;
        let c; if (cond) c = id(&inner.f1) else c = &other.s1.f2;
        let f1 = &inner.f1;
        *c;
        *inner;
        *f1;
        *inner;
        *c;

        let inner = &mut outer.s1;
        let c; if (cond) c = &inner.f1 else c = &other.s1.f2;
        let f1 = &inner.f1;
        *c;
        *inner;
        *f1;
        *inner;
        *c;

        let inner = &mut outer.s1;
        let c; if (cond) c = id(&inner.f1) else c = &other.s1.f2;
        let f1 = &inner.f1;
        *c;
        *inner;
        *f1;
        *inner;
        *c;

        // The next case flags no error in the v1 compiler, though appears to be wrong,
        // since `c` can hold a reference to either `inner.f1` or `inner.f2`.
        let inner = &mut outer.s1;
        let c; if (cond) c = &mut inner.f1 else c = &mut inner.f2;
        let f1 = &mut inner.f1; // no error in v1, but should error.
        *c;
        *f1;
        *inner;

        // The next case flags no error in the v1 compiler, though appears to be wrong,
        // since `c` can hold a reference to either `inner.f1` or `inner.f2`.
        let inner = &mut outer.s1;
        let c; if (cond) c = id_mut(&mut inner.f1) else c = &mut inner.f2;
        let f1 = &mut inner.f1; // no error in v1, but should error
        *c;
        *f1;
        *inner;
    }

}
