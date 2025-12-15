module 0xc0ffee::m {

    public struct S {
        x: u64,
    }

    package struct S2 {
        x: u64,
        s: S,
    }

    public(package) struct Empty {}

}

module 0xc0ffee::m_friend {
    friend 0xc0ffee::n_friend;

    friend struct T {
        x: u64,
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
