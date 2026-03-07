//# publish
module 0xc0ffee::corner_cases {
    enum E has drop {
        A(u8),
        B(u8),
    }

    fun compute_mixed(e1: E, e2: E, p: u8, q: u8): u8 {
        match ((e1, e2, p, q)) {
            (E::A(x), E::B(y), 5, 10) if x > 0 => x + y + 5,
            (E::B(_), E::A(_), 0, w) => w,
            (_, _, _, _) => 1,
        }
    }

    public fun test_mixed(): u8 {
        let r1 = compute_mixed(E::A(1), E::B(2), 5, 10);
        let r2 = compute_mixed(E::B(3), E::A(4), 0, 7);
        let r3 = compute_mixed(E::B(9), E::B(1), 9, 10);
        r1 + r2 + r3
    }

    fun test_primitive_tuple(x: u8, y: u8): u8 {
        match ((x, y)) {
            (1, 9) => 9,
            (_, _) => 0,
        }
    }

    public fun test_tuple_match_true(): u8 {
        test_primitive_tuple(1, 9)
    }

    public fun test_tuple_match_false(): u8 {
        test_primitive_tuple(2, 9)
    }
}

//# run 0xc0ffee::corner_cases::test_mixed

//# run 0xc0ffee::corner_cases::test_tuple_match_true

//# run 0xc0ffee::corner_cases::test_tuple_match_false
