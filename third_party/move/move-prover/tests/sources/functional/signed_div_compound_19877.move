// Signed `/` and `%` must truncate toward zero on the SPEC side even when the
// dividend is an arithmetic expression. Spec arithmetic widens to `num`, which
// erases the signed integer type, so a dispatch keyed only on that type falls
// back to Boogie's Euclidean `div` and disagrees with the truncating body.
// The original #19877 regression covers plain variables and single casts, both
// of which keep their type; these shapes do not.
module 0x42::signed_div_compound_19877 {

    // Product then divide, through casts: the `i64_math::mul_div` shape.
    public fun mul_div(a: i64, b: u64, c: u64): i64 {
        (((a as i128) * (b as i128) / (c as i128)) as i64)
    }

    spec mul_div {
        aborts_if c == 0;
        aborts_if (a as i128) * (b as i128) / (c as i128) > (MAX_I64 as i128);
        aborts_if (a as i128) * (b as i128) / (c as i128) < (MIN_I64 as i128);
        ensures (result as i128) == (a as i128) * (b as i128) / (c as i128);
    }

    // Compound dividend with no casts: the widening, not the cast, is what
    // erases the type.
    public fun div_compound(x: i64, y: i64, z: i64): i64 {
        (x * y) / z
    }

    spec div_compound {
        pragma aborts_if_is_partial;
        ensures result == (x * y) / z;
    }

    // The same for `%`.
    public fun mod_compound(x: i64, y: i64, z: i64): i64 {
        (x * y) % z
    }

    spec mod_compound {
        pragma aborts_if_is_partial;
        ensures result == (x * y) % z;
    }

    // Unsigned compound division keeps the plain operator: truncation and
    // Euclidean division agree on non-negative values, so this is only here to
    // pin that the encoding is not disturbed.
    public fun div_compound_unsigned(x: u64, y: u64, z: u64): u64 {
        (x * y) / z
    }

    spec div_compound_unsigned {
        pragma aborts_if_is_partial;
        ensures result == (x * y) / z;
    }
}
