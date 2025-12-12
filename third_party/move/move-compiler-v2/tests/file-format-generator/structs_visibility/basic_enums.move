module 0xc0ffee::m {

    package enum Wrapper has drop {
        V1(u64), // same type at the same offset
        V2(u64),
    }

    public fun make(x: u64): Wrapper {
        Wrapper::V1(x)
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

    fun test_test_variant_mut_borrow(w: &mut Wrapper): bool {
        w is V1
    }

    fun test_test_variant_immutable_borrow(w: &Wrapper): bool {
        w is V1
    }

}

module 0xc0ffee::n7 {
    use 0xc0ffee::m::Wrapper;

    fun test_match(w: Wrapper): bool {
        match (w) {
            V1(_) => true,
            V2(_) => false,
        }
    }

    fun test_match_mut_borrow(w: &mut Wrapper): bool {
        match (w) {
            V1(_) => true,
            V2(_) => false,
        }
    }

}
