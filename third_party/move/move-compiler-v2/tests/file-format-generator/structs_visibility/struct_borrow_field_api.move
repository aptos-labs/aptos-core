module 0x42::m1 {

    public struct S<G: store + drop> has drop {
        f: u64,
        g: T<G>
    }

    public struct T<G: store + drop> has store, copy, drop {
        h: G
    }
}

module 0x42::m2 {

    use 0x42::m1::{S, T};

    public fun try_immut_borrow_fields() {
        let t = T { h: 100 };
        let s = S { f: 200, g: t };
        let f = &s.f;
        let h = &s.g.h;
        assert!(*f == 200, 3);
        assert!(*h == 100, 4);
    }

    public fun try_immut_from_mut() {
        let t = T { h: 5 };
        let s = S { f: 6, g: t };
        let r = &mut s;
        let f = &r.f;
        assert!(*f == 6, 5);
    }

    public fun try_mut_borrow_seq() {
        let t = T { h: 1 };
        let s = S { f: 2, g: t };
        let f = &mut s.f;
        *f = *f + 10;
        let h = &mut s.g.h;
        *h = *h + 20;
        assert!(s.f == 12, 7);
        assert!(s.g.h == 21, 8);
    }

    public fun try_immut_and_mut_diff_fields() {
        let t = T { h: 30 };
        let s = S { f: 40, g: t };
        let f = &s.f;
        assert!(*f == 40, 9);
        let h = &mut s.g.h;
        *h = *h + 1;
        assert!(*h == 31, 10);
    }

    public fun try_reborrow_same_field_seq() {
        let t = T { h: 9 };
        let s = S { f: 8, g: t };
        let f = &mut s.f;
        *f = 88;
        let f2 = &mut s.f;
        *f2 = *f2 + 1;
        assert!(s.f == 89, 11);
    }

    public fun try_unpack_ref() {
        let t = T { h: 9 };
        let s = S { f: 8, g: t };
        let S { f, g } = &s;
        assert!(*f == 8, 11);
        assert!(g.h == 9, 12);
        let S { f: _, g } = &s;
        assert!(g.h == 9, 14);
        let S { f: _, g: T { h } } = &s;
        assert!(*h == 9, 15);
        let S { f, g: T { h: _ } } = &s;
        assert!(*f == 8, 15);
    }
}
