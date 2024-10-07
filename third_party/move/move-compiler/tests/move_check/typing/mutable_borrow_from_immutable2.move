module 0x8675309::M {
    struct X { f: u64 }
    struct S { v: u64, x: X }
    fun t() {
        let s = S { v: 0, x: X { f: 0 }};
        &mut (&s).v;
    }
    fun t1a() {
        let s = S { v: 0, x: X { f: 0 }};
        &mut (&s.x).f;
    }
    fun t1b() {
        let s = S { v: 0, x: X { f: 0 }};
        let sref = &s;
        &mut sref.v;
    }
    fun t1c() {
        let s = S { v: 0, x: X { f: 0 }};
        let xref = &s.x;
        &mut xref.f;
    }

    fun t2(s: &S, x: &X) {
        x.f = x.f + 1;
        s.x.f = s.x.f + 1
    }
}
