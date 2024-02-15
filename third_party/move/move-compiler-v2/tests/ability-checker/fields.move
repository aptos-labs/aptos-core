module 0x42::fields {

    struct S {
        f: u64,
        g: T
    }

    struct T {
        h: u64
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
}
