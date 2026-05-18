module 0xc0ffee::m {
    enum E has drop {
        V1(u64),
        V2,
    }

    struct S has drop { x: u64 }

    // Struct literal extraction + mixed-tuple lowering (enum)
    fun enum_mixed_tuple(e: E, n: u64): u64 {
        match ((e, n)) {
            (E::V1(1), 2) => 10,
            (E::V1(x), _) => x,
            (E::V2, _) => 99,
        }
    }

    // Struct literal extraction + mixed-tuple lowering (plain struct)
    fun struct_mixed_tuple(s: S, n: u64): u64 {
        match ((s, n)) {
            (S { x: 1 }, 2) => 10,
            (S { x }, _) => x,
        }
    }
}
