module 0x42::m1 {

    public struct S<G: store + drop> has drop {
        f: u64,
        g: T<G>
    }

    public struct T<G: store + drop> has store, copy, drop {
        h: G
    }

    public struct V<G: store + drop> has store, copy, drop {
        items: vector<G>
    }

    public struct Nested<G: store + copy +drop> has store, copy, drop {
        inner: T<V<G>>
    }

    public struct P<phantom T> has drop {
        p: u64
    }

    struct Inner<T: store + drop> has drop {
        t: T
    }

    public struct NoFields has drop {}

}

module 0x42::m3 {
    use 0x42::m1::T;

    public struct S<G: store + drop> has drop {
        f: u64,
        g: T<G>
    }

}

module 0x42::m2 {

    use 0x42::m1::{S, T, V, Nested, P, Inner, NoFields};
    use 0x42::m3::S as S3;

    public fun try_pack_unpack_no_fields() {
        let no_fields = NoFields {};
        let NoFields {} = no_fields;
    }

    public fun try_pack_unpack_phantom() {
        let p = P<Inner<u64>> { p: 42 };
        assert!(p.p == 42, 1);
        let P { p } = p;
        assert!(p == 42, 4);
    }

    public fun try_pack() {
        let t = T { h: 42 };
        assert!(t.h == 42, 1);
        let s = S { f: 43, g: t };
        assert!(s.f == 43, 2);
    }

    public fun try_pack_vector() {
        let t = T { h: vector[42] };
        assert!(t.h[0] == 42, 3);
    }

    public fun try_unpack() {
        let s = S { f: 43, g: T { h: 42 } };
        let S { f, g: T { h } } = s;
        assert!(f == 43, 4);
        assert!(h == 42, 5);
    }

    public fun try_unpack_with_let_ignore() {
        let s = S { f: 100, g: T { h: 7 } };
        let S { f, g: _ } = s;
        assert!(f == 100, 6);
    }

    public fun try_nested_pack() {
        let v = V { items: vector[1, 2, 3] };
        let nested = Nested { inner: T { h: v } };
        assert!(nested.inner.h.items[2] == 3, 7);
    }

    public fun try_unpack_nested() {
        let s = Nested {
            inner: T { h: V { items: vector[10, 20] } }
        };
        let Nested { inner: T { h: V { items } } } = s;
        assert!(items[0] == 10, 8);
        assert!(items[1] == 20, 9);
    }

    public fun return_struct(): S<u64> {
        S { f: 77, g: T { h: 88 } }
    }

    public fun try_return_struct() {
        let s = return_struct();
        let S { f, g: T { h } } = s;
        assert!(f == 77, 10);
        assert!(h == 88, 11);
    }

    public fun take_struct(s: S<vector<u8>>) {
        assert!(s.g.h[0] == 255, 12);
    }

    public fun try_pass_struct_as_arg() {
        let s = S { f: 1, g: T { h: vector[255] } };
        take_struct(s);
    }

    public fun try_pack_S3() {
        let t = T { h: 42 };
        assert!(t.h == 42, 1);
        let s = S3 { f: 43, g: t };
        assert!(s.f == 43, 13);
        assert!(s.g.h == 42, 14);
    }
}
