// Regression test for https://github.com/aptos-labs/aptos-core/issues/19877
//
// The Move runtime truncates signed integer division toward zero
// (Rust / C semantics). The Move Prover used to model `$Div` and `$Mod`
// with Boogie's built-in `div` / `mod`, which are Euclidean (floor toward
// `-inf`, non-negative remainder). The two semantics diverge whenever
// the dividend is negative with a non-zero remainder.
//
// This test pins down truncate-toward-zero semantics for both `/` and `%`
// across all sign combinations and the `MIN_I / -1` overflow case.

module 0x42::signed_div_mod {

    // --- Division ----------------------------------------------------------

    public fun div_i64(x: i64, y: i64): i64 {
        x / y
    }

    spec div_i64 {
        aborts_if y == 0;
        aborts_if x == MIN_I64 && y == -1;
        // Truncate toward zero: same sign as mathematical quotient,
        // magnitude = floor(|x| / |y|).
        ensures x == -1 && y == 2 ==> result == 0;
        ensures x == -7 && y == 3 ==> result == -2;
        ensures x == -7 && y == -3 ==> result == 2;
        ensures x == 7 && y == -3 ==> result == -2;
        ensures x == 7 && y == 3 ==> result == 2;
        ensures x == -10 && y == 3 ==> result == -3;
    }

    // Cast pattern from the issue: (signed as i128) / (unsigned as i128).
    public fun cast_div(x: i64, y: u64): i64 {
        ((x as i128) / (y as i128)) as i64
    }

    spec cast_div {
        aborts_if y == 0;
        ensures x == -1 && y == 2 ==> result == 0;
    }

    // --- Modulus -----------------------------------------------------------

    // Under truncate-toward-zero, `x % y` has the same sign as `x`,
    // and `|x % y| < |y|`. (Boogie's Euclidean `mod` keeps the remainder
    // non-negative, which diverges whenever the dividend is negative.)
    public fun mod_i64(x: i64, y: i64): i64 {
        x % y
    }

    spec mod_i64 {
        aborts_if y == 0;
        ensures x == 7 && y == 3 ==> result == 1;
        ensures x == -7 && y == 3 ==> result == -1;
        ensures x == 7 && y == -3 ==> result == 1;
        ensures x == -7 && y == -3 ==> result == -1;
    }

    // --- Overflow ----------------------------------------------------------

    // MIN_I64 / -1 is mathematically MAX_I64 + 1, which doesn't fit in i64.
    // The runtime aborts; the prover must too.
    public fun div_min_by_neg_one_aborts(): i64 {
        MIN_I64 / -1
    }

    spec div_min_by_neg_one_aborts {
        aborts_if true;
    }

    // --- Unsigned (regression: must not change behavior) -------------------

    public fun div_u64(x: u64, y: u64): u64 {
        x / y
    }

    spec div_u64 {
        aborts_if y == 0;
        ensures result == x / y;
        ensures x == 7 && y == 3 ==> result == 2;
        ensures x == 7 && y == 2 ==> result == 3;
    }

    public fun mod_u64(x: u64, y: u64): u64 {
        x % y
    }

    spec mod_u64 {
        aborts_if y == 0;
        ensures result == x % y;
        ensures x == 7 && y == 3 ==> result == 1;
    }
}
