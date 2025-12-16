module 0xc0ffee::m_friend {
    friend 0xc0ffee::n_friend;

    friend struct S<T: copy> {
        f: T,
    }

}

module 0xc0ffee::n_friend {
    use 0xc0ffee::m_friend::S;

    fun test_pack_friend_struct(): S<u64> {
        S { f: 22 }
    }

    fun test_unpack_friend_struct(s: S<u64>): u64 {
        let S { f } = s;
        f
    }

    fun test_borrow_friend_struct(s: &S<u64>): &u64 {
        &s.f
    }

    fun test_mut_borrow_friend_struct(s: &mut S<u64>): &mut u64 {
        &mut s.f
    }

}
