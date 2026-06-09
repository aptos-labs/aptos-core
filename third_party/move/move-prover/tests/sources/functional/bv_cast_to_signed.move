// exclude_for: cvc5
// Regression: casts from a bv-classified unsigned source to a signed-int /
// `Num` target must materialize a bv→int boundary in generated Boogie.
// Without this, the prover panicked with
// "signed integer cannot be turned into bit vector" because the analysis
// propagated `Bitwise` past the cast onto a signed return value.
module 0x42::bv_cast_to_signed {

    // u8 bitwise source, signed i64 target — minimal reproducer.
    public fun and_to_i64(a: u8, b: u8): i64 {
        (a & b) as i64
    }

    spec and_to_i64 {
        aborts_if false;
        ensures result == ((a & b) as i64);
    }

    // u32 bitwise source, wider signed i128 target.
    public fun or_to_i128(a: u32, b: u32): i128 {
        (a | b) as i128
    }

    spec or_to_i128 {
        aborts_if false;
        ensures result == ((a | b) as i128);
    }

    // Chained cast u8 → u16 (bv→bv upcast) → i32 (bv→int).
    // Verifies that an outer bv→int cast does not corrupt the inner bv→bv
    // upcast's classification.
    public fun xor_chained_to_i32(a: u8, b: u8): i32 {
        ((a ^ b) as u16) as i32
    }

    spec xor_chained_to_i32 {
        aborts_if false;
        ensures result == (((a ^ b) as u16) as i32);
    }

    // Ensure the existing bv→bv cast path still works alongside the new
    // bv→signed branch.
    public fun and_to_u16(a: u8, b: u8): u16 {
        (a & b) as u16
    }

    spec and_to_u16 {
        aborts_if false;
        ensures result == ((a & b) as u16);
    }

    // The bitwise context promotes the literal `7u8` to a bv-classified
    // Value, and `translate_cast` then runs on the surrounding cast.
    // translate_cast must not overwrite that literal's classification before
    // the bv→int branch, or translate_value would drop the bv suffix and feed
    // `int` into `$bv2int.N`.
    public fun literal_in_bitwise_to_i64(a: u8): i64 {
        (a & 7u8) as i64
    }

    spec literal_in_bitwise_to_i64 {
        aborts_if false;
        ensures result == ((a & 7u8) as i64);
    }
}
