module 0x8675309::M {
    struct S has copy, drop { f: u64, g: u64 }
    fun id<T>(r: &T): &T {
        r
    }
    fun id_mut<T>(r: &mut T): &mut T {
        r
    }

    fun t0(cond: bool, s: &mut S, other: &S,) {
        let f;
        if (cond) f = &s.f else f = &other.f;
        *s;
        *f;
        *s;
    }

    fun t1(cond: bool, s: &mut S) {
        let f;
        if (cond) f = &s.f else f = &s.g;
        *s;
        *f;
        *s;
    }

    fun t2(cond: bool, s: &mut S, other: &S) {
        let x: &S;
        if (cond) x = copy s else x = other; // error in v2 because of copy of mut ref
        *s;
        *x;
    }

}
