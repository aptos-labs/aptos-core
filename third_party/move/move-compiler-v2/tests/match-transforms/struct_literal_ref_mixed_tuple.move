module 0xc0ffee::m {
    enum E has drop, copy {
        V1(u64),
        V2,
    }

    // Mixed tuple: (&E, u64) — enum part by ref, primitive part by value
    fun ref_enum_mixed_tuple(e: &E, n: u64): u64 {
        match ((e, n)) {
            (E::V1(1), 2) => 10,
            (E::V1(x), _) => *x,
            (E::V2, _) => 99,
        }
    }

    // Mixed tuple: (&u64, E) — primitive ref + enum by value
    fun ref_prim_mixed_tuple(x: &u64, e: E): u64 {
        match ((x, e)) {
            (1, E::V1(2)) => 10,
            (_, E::V1(y)) => y,
            (_, E::V2) => 99,
        }
    }
}
