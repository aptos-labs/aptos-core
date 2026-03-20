module 0xc0ffee::m {
    enum Data has drop {
        V1(u8),
        V2(u8)
    }

    fun make_pair(x: u8): (Data, u8) {
        (Data::V1(x), x)
    }

    // Function-returned mixed tuple
    fun test_fn_call(x: u8): u8 {
        match (make_pair(x)) {
            (Data::V1(a), 1) => a + 10,
            (Data::V2(a), 2) => a + 20,
            _ => 99,
        }
    }

    // Block expression returning mixed tuple
    fun test_block_expr(x: u8): u8 {
        match ({let y = x + 1; (Data::V1(y), y)}) {
            (Data::V1(a), 5) => a + 50,
            (Data::V2(a), _) => a,
            _ => 0,
        }
    }

    // Nested mixed-tuple matches: inner match in arm body of outer match.
    // Both produce _$disc_0/_$disc_1/_$np_0/_$prim_0 bindings; inner must
    // shadow outer correctly without binding conflicts.
    fun test_nested(x: u8): u8 {
        match (make_pair(x)) {
            (Data::V1(a), 1) => match (make_pair(a)) {
                (Data::V1(b), 1) => b + 10,
                (Data::V2(b), _) => b + 20,
                _ => 30,
            },
            (Data::V2(a), _) => a,
            _ => 99,
        }
    }
}
