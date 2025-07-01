module 0xc0ffee::m {

    package enum Wrapper has drop {
        V1(u64),
        V2(u64),
    }

    public fun make(x: u64): Wrapper {
        Wrapper::V1(x)
    }

    public(package) struct S {
        x: u64,
    }

    package struct S2 {
        x: u64,
        s: S,
    }

}

module 0xc0ffee::m_friend {
    friend 0xc0ffee::n_friend;

    friend struct T {
        x: u64,
    }

}

module 0xc0ffee::n {
    use 0xc0ffee::m;

    fun test() {
        let x = m::make(22);
        assert!(x.0 == 22, 1);
    }
}

module 0xc0ffee::n2 {
    use 0xc0ffee::m::Wrapper;

    fun test_pack() {
        let _x = Wrapper::V1(22);
    }
}

module 0xc0ffee::n3 {
    use 0xc0ffee::m::Wrapper;

    fun test_unpack(w: Wrapper) {
        let V1(_x) = w;
    }
}

module 0xc0ffee::n4 {
    use 0xc0ffee::m::Wrapper;

    fun test_select_variant(w: Wrapper): u64 {
        w.0
    }
}

module 0xc0ffee::n5 {
    use 0xc0ffee::m::Wrapper;

    fun test_test_variant(w: Wrapper): bool {
        w is V1
    }
}

module 0xc0ffee::n6 {
    use 0xc0ffee::m::Wrapper;

    fun test_match(w: Wrapper): bool {
        match (w) {
            V1(_) => true,
            V2(_) => false,
        }
    }
}

module 0xc0ffee::n7 {
    use 0xc0ffee::m::S;

    fun test_pack_struct(): S {
        S { x: 22 }
    }
}

module 0xc0ffee::n_friend {
    use 0xc0ffee::m_friend::T;

    fun test_pack_friend_struct(): T {
        T { x: 22 }
    }
}

module 0xc0ffee::n8 {
    use 0xc0ffee::m::S;

    inline fun test_pack_struct_inline(): S {
        S { x: 22 }
    }

    fun test_pack_struct(): S {
        test_pack_struct_inline()
    }
}

module 0xc0ffee::n9 {
    use 0xc0ffee::m::S;
    use 0xc0ffee::m::S2;

    inline fun test_inline(x: ||) {
        x()
    }

    fun test_pack_unpack_struct_in_lambda() {
        test_inline(|| {let x = S2 { x: 22, s: S { x: 33 } }; let S2 { x: _x, s: S { x: _y } } = x;} )
    }
}
