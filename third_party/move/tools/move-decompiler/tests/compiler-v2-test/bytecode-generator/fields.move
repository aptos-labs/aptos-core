module 0x42::fields {

    struct S has drop {
        f: u64,
        g: T
    }

    struct T has drop {
        h: u64
    }

    struct G<X> has drop {
        f: X
    }

    fun read_val(x: S): u64 {
        x.g.h
    }

    fun read_ref(x: &S): u64 {
        x.g.h
    }

    fun write_val(x: S): S {
        x.g.h = 42;
        x
    }

    fun write_param(x: &mut S) {
        x.g.h = 42;
    }

    fun write_local_via_ref(): S {
        let x = S { f: 0, g: T { h: 0 } };
        let r = &mut x;
        r.g.h = 42;
        x
    }

    fun write_local_direct(): S {
        let x = S { f: 0, g: T { h: 0 } };
        x.g.h = 42;
        x
    }

    fun read_generic_val(x: G<u64>): u64 {
        x.f
    }

    fun write_generic_val(x: &mut G<u64>, v: u64) {
        x.f = v
    }

    fun write_local_via_ref_2(): S {
        let x = S { f: 0, g: T { h: 0 } };
        let r = &mut x.g.h;
        *r = 42;
        x
    }
}
