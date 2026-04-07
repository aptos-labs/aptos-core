//# publish
module 0xc0ffee::m {
    enum Data has drop {
        V1(u8),
        V2(u8)
    }

    fun make_pair(x: u8): (Data, u8) {
        (Data::V1(x), x)
    }

    fun make_triple(x: u8): (Data, u8, Data) {
        (Data::V1(x), x, Data::V2(x))
    }

    // Basic function-returned mixed tuple match
    fun test_basic(x: u8): u8 {
        match (make_pair(x)) {
            (Data::V1(a), 1) => a + 10,
            (Data::V2(a), 2) => a + 20,
            _ => 99,
        }
    }

    // Variable binding in primitive position
    fun test_var_binding(x: u8): u8 {
        match (make_pair(x)) {
            (Data::V1(a), y) => a + y,
            (Data::V2(a), y) => a + y + 100,
        }
    }

    // Guard on arm
    fun test_guard(x: u8): u8 {
        match (make_pair(x)) {
            (Data::V1(a), 5) if (a > 3) => a + 50,
            (Data::V1(a), 5) => a,
            _ => 99,
        }
    }

    // Wildcard-only arm
    fun test_wildcard_only(x: u8): u8 {
        match (make_pair(x)) {
            _ => 42,
        }
    }

    // Multiple enum + multiple primitive elements
    fun test_multi(x: u8): u8 {
        match (make_triple(x)) {
            (Data::V1(a), 1, Data::V2(b)) => a + b,
            (Data::V2(a), _, Data::V1(b)) => a + b + 100,
            _ => 77,
        }
    }

    // Block expression returning a mixed tuple
    fun test_block_expr(x: u8): u8 {
        match ({ let y = x + 1; (Data::V1(y), y) }) {
            (Data::V1(a), 5) => a + 50,
            (Data::V2(a), _) => a,
            _ => 0,
        }
    }

    // Nested mixed-tuple matches: inner match in arm body of outer match.
    // Both produce identically-named compiler temps; verifies inner bindings
    // shadow outer correctly without conflicts.
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

    public fun test() {
        assert!(test_basic(1) == 11);       // V1(1), 1 -> arm 1: 1 + 10
        assert!(test_basic(3) == 99);       // V1(3), 3 -> wildcard
        assert!(test_var_binding(7) == 14); // V1(7), 7 -> 7 + 7
        assert!(test_guard(5) == 55);       // V1(5), 5 -> a=5 > 3: 5 + 50
        assert!(test_guard(2) == 99);       // V1(2), 2 -> no match on 5: wildcard
        assert!(test_wildcard_only(0) == 42);
        assert!(test_multi(1) == 2);        // V1(1), 1, V2(1) -> 1 + 1
        assert!(test_multi(9) == 77);       // V1(9), 9, V2(9) -> wildcard
        assert!(test_block_expr(4) == 55);  // y=5, V1(5), 5 -> 5 + 50
        assert!(test_block_expr(1) == 0);   // y=2, V1(2), 2 -> wildcard
        assert!(test_nested(1) == 11);      // outer: V1(1),1 -> inner: V1(1),1 -> 1+10
        assert!(test_nested(2) == 99);      // outer: V1(2),2 -> wildcard
    }
}

//# run 0xc0ffee::m::test
