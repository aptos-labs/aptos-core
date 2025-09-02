module 0x8675309::M {
    struct S has copy, drop { f: u64, g: u64 }
    fun id<T>(r: &T): &T {
        r
    }
    fun id_mut<T>(r: &mut T): &mut T {
        r
    }

    fun t0(cond: bool, s: &mut S, other: &S) {
        let f;
        if (cond) f = &s.f else f = &other.f;
        freeze(s); // error in v2 even though s is not read
        *f;
    }

    fun t1(cond: bool, s: &mut S) {
        let f;
        if (cond) f = &s.f else f = &s.g;
        freeze(s); // error in v2 even though s is not read
        *f;
    }

    fun t2(cond: bool, s: &mut S, other: &S) {
        let x;
        if (cond) x = freeze(s) else x = other;
        freeze(s); // error in v2 even though s is not read
        *x;
    }

}
