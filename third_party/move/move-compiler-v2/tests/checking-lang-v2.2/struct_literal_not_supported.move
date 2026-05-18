module 0xc0ffee::struct_literal_not_supported {
    enum E has drop {
        V1(u64),
        V2,
    }

    struct S has drop { x: u64 }

    public fun test_enum(e: E): u64 {
        match (e) {
            E::V1(1) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }

    public fun test_struct(s: S): u64 {
        match (s) {
            S { x: 42 } => 1,
            S { x: _ } => 2,
        }
    }
}
