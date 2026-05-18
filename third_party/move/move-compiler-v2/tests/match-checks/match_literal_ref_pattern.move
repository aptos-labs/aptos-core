module 0xc0ffee::match_literal_ref_pattern {
    enum E has copy, drop {
        V1(u64),
        V2,
    }

    fun match_ref_enum_field_with_ref_bool(e: &E): u64 {
        match (e) {
            E::V1(&true) => 1,
            E::V1(_) => 0,
            E::V2 => 2,
        }
    }
}
