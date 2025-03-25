module pyth::i64 {
    struct I64 has copy, drop, store {
        negative: bool,
        magnitude: u64
    }

    public fun get_is_negative(_i: &I64): bool {
        abort 0
    }

    public fun get_magnitude_if_positive(_i: &I64): u64 {
        abort 0
    }

    public fun get_magnitude_if_negative(_i: &I64): u64 {
        abort 0
    }
}
