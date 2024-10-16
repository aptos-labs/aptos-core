//# publish
module 0x42::pack_unpack_ref {
    struct S has drop {
        f: u64,
        g: u64
    }

    struct G has drop {
        x1: u64,
        x2: u64,
        s: S,
    }

    fun unpack_ref_G() {
        let s = S {f: 0, g: 1};
        let g = G {x1: 2, x2: 3, s};
        let G{ x1, x2, s  } = &mut g;
        let S {f, g} = s;
        assert!(*f == 0, 0);
        assert!(*g == 1, 1);
        assert!(*x1 == 2, 2);
        assert!(*x2 == 3, 3);
        *x1 = *x1 + 1;
        *x2 = *x2 + 1;
        *f = *f + 1;
        *g = *g + 1;
        assert!(*f == 1, 0);
        assert!(*g == 2, 1);
        assert!(*x1 == 3, 2);
        assert!(*x2 == 4, 3);
    }
}

//# run  0x42::pack_unpack_ref::unpack_ref_G
