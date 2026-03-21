module 0xc0ffee::enum_variant_literal_coverage {
    enum E has drop {
        V1(u64),
        V2,
    }

    // Non-exhaustive: missing non-1 values of V1
    public fun non_exhaustive(e: E): u64 {
        match (e) {
            E::V1(1) => 1,
            E::V2 => 2,
        }
    }

    // Unreachable: E::V1(1) is subsumed by E::V1(_)
    public fun unreachable(e: E): u64 {
        match (e) {
            E::V1(_) => 1,
            E::V1(1) => 2,
            E::V2 => 3,
        }
    }

    // Exhaustive with literals (should pass with no errors)
    public fun exhaustive(e: E): u64 {
        match (e) {
            E::V1(1) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }
}
