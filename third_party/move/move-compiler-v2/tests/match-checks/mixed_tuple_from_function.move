module 0xc0ffee::m {
    enum Data has drop {
        V1(u8),
        V2(u8)
    }

    fun make_pair(x: u8): (Data, u8) {
        (Data::V1(x), x)
    }

    fun test_mixed_tuple_from_fn(x: u8): u8 {
        match (make_pair(x)) {
            (Data::V1(a), 1) => a + 10,
            (Data::V2(a), 2) => a + 20,
            _ => 99,
        }
    }
}
