module 0x8675309::M {
    enum X has copy, drop {
        A,
        B
    }

    // Exhaustive set with wildcard should error on unreachable arm
    fun test_exhaustive_then_wildcard(x: X): u64 {
        match ((x, x)) {
            (X::A, X::A) => 1,
            (X::A, X::B) => 2,
            (X::B, X::A) => 3,
            (X::B, X::B) => 4,
            _ => 5,
        }
    }

    // Wildcard first, then specific pattern should error on unreachable arm
    fun test_wildcard_first(x: X): u64 {
        match ((x, x)) {
            _ => 1,
            (X::A, X::A) => 2,
        }
    }
}
