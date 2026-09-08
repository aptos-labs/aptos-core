/// A compact Move fixture for the canonical LeanerLang round trip.
module 0x42::move_scalar {
    const ZERO: u64 = 0;

    struct Pair has copy, drop {
        left: u64,
        right: u64,
    }

    public fun invert(value: bool): bool {
        !value
    }
}
