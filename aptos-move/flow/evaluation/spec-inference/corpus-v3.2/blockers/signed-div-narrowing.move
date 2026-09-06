module 0x42::sdiv {
    // The `calculate_pnl` shape, with abort behaviour stated completely. This
    // does not verify: the range `aborts_if` is reported as not aborting, and
    // the `ensures` does not hold.
    //
    // Two obvious explanations were probed and both are wrong, so do not spend
    // the time again:
    //
    //   * signed division does NOT disagree on rounding. Specification `/`
    //     truncates toward zero exactly as the code does -- `-7 / 2 == -3`
    //     holds and `== -4` fails.
    //   * the narrowing cast is NOT mis-modelled. `x as i64` aborts exactly
    //     outside `[MIN_I64, MAX_I64]`, and `MIN_I64 as i128` round-trips.
    //
    // Each piece is right on its own, so the disagreement is in the
    // composition -- the most likely remaining candidate is how a `u128` to
    // `i128` cast inside a *specification* expression relates to the same cast
    // in code, since a specification cast is total over `num` while the code's
    // aborts.
    fun scaled_delta(hi: u128, lo: u128, m: u64): i64 {
        let d = (hi as i128) - (lo as i128);
        (d / (m as i128)) as i64
    }
    spec scaled_delta {
        aborts_if hi > MAX_I128;
        aborts_if lo > MAX_I128;
        aborts_if m == 0;
        aborts_if ((hi as i128) - (lo as i128)) / (m as i128) < MIN_I64
               || ((hi as i128) - (lo as i128)) / (m as i128) > MAX_I64;
        ensures result == ((((hi as i128) - (lo as i128)) / (m as i128)) as i64);
    }
}
