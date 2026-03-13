module 0xc0ffee::enum_variant_literal_not_supported {
    enum E has drop {
        V1(u64),
        V2,
        V3 { x: u64 },
    }

    public fun test(e: E): u64 {
        match (e) {
            E::V1(1) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
            E::V3 { x: 42 } => 4,
            E::V3 { .. } => 5,
        }
    }
}
