module 0xc0ffee::match_literal_ref_type_error {
    enum E has copy, drop {
        V1(u64),
        V2,
    }

    fun match_ref_u64_with_bool(x: &u64): u64 {
        match (x) {
            true => 1,
            _ => 0,
        }
    }

    fun match_ref_bool_with_bytes(x: &bool): u64 {
        match (x) {
            b"hi" => 1,
            _ => 0,
        }
    }

    fun match_ref_enum_field_with_bool(e: &E): u64 {
        match (e) {
            E::V1(true) => 1,
            E::V1(_) => 0,
            E::V2 => 2,
        }
    }
}
