// exclude_for: cvc5
// Tests vector::length() when the result destination is in Bitwise (BV) mode.
//
module 0x42::BvVectorLengthBvDest {
    use std::vector;

    struct ByteStream has copy, drop {
        data: vector<u8>, // field 0 — marked BV by the struct spec below
        cur: u64,         // field 1
    }
    spec ByteStream {
        // Mark the data field (index 0) as BV, producing Vec(bv8) in Boogie.
        pragma bv = b"0";
    }

    /// Returns true if there are bytes remaining in the stream.
    ///
    /// With `pragma bv = b"0"` on the spec, `s` is in BV mode, so `s.cur` is
    /// BV-typed. The `<` comparison propagates BV to the length result, making
    /// the dest-temp for the length call Bitwise-typed.
    fun has_remaining(s: &ByteStream): bool {
        s.cur < vector::length(&s.data)
    }
    spec has_remaining {
        pragma bv = b"0";
        aborts_if false;
        ensures result == (s.cur < len(s.data));
    }

    /// Same scenario via a local variable that holds the length.
    ///
    /// The assignment `let n = data.length()` produces a dest-temp for length;
    /// the subsequent `cur < n` comparison drives n's temp to Bitwise mode.
    fun remaining_count(s: &ByteStream): u64 {
        let n = vector::length(&s.data);
        if (s.cur < n) { n - s.cur } else { 0 }
    }
    spec remaining_count {
        pragma bv = b"0";
        aborts_if false;
        ensures result == (if (s.cur < len(s.data)) { len(s.data) - s.cur } else { 0 });
    }
}
