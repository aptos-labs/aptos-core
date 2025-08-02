module 0x42::pack_unpack_ref {
    struct S {
        f: u64,
        g: u64
    }

    struct G {
        x1: u64,
        x2: u64,
        s: S,
    }

    fun unpack_ref(s: &S): (u64, u64) {
        let S{f, g} = s;
        (*f, *g)
    }

    fun unpack_ref_G(g: &G): (u64, u64, u64, u64) {
        let G{ x1, x2, s  } = g;
        let S {f, g} = s;
        (*x1, *x2, *f, *g)
    }

    fun unpack_mut_ref(s: &mut S): (u64, u64) {
        let S{f, g} = s;
        (*f, *g)
    }

}
