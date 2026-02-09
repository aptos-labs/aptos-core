/// Test that demonstrates discriminator re-evaluation bug.
/// The discriminator expression should be evaluated only once,
/// but the current transformation evaluates it multiple times.
module 0xc0ffee::m {
    fun increment(counter: &mut u64): bool {
        *counter = *counter + 1;
        *counter % 2 == 0
    }

    /// If the transformation is correct, `counter` should be 1 after the match.
    /// If the discriminator is re-evaluated, `counter` will be > 1.
    public fun test_side_effect(counter: &mut u64, q: bool): u8 {
        match ((increment(counter), q)) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 3,
        }
    }

    /// Same test but with a non-exhaustive match (has wildcard).
    public fun test_side_effect_wildcard(counter: &mut u64, q: bool): u8 {
        match ((increment(counter), q)) {
            (true, true) => 0,
            (true, false) => 1,
            _ => 2,
        }
    }
}
