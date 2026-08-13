// exclude_for: cvc5
// Unsuffixed integer literals in spec mode default to u256 when no context
// type determines them; when such a literal meets a Bitwise-classified
// unsigned operand, the analysis used to report "integer type mismatch
// (u8 vs u256)". Defaulted literals now adopt the sibling operand's type.
// Explicitly suffixed literals must keep the mismatch diagnostic.
module 0x42::bv_spec_literal_default {

    public fun or3(x: u8): u8 {
        x | 3
    }

    // `3` here is builder-defaulted to u256 and must adopt u8.
    spec or3 {
        aborts_if false;
        ensures result == (x | 3);
    }

    public fun or_suffixed(x: u8): u8 {
        x | 3
    }

    // `3u256` is explicitly suffixed: the width mismatch must still be
    // reported, not silently adopted.
    spec or_suffixed {
        aborts_if false;
        ensures result == ((x as u256) & (x | 3u256) as u8);
    }
}

// The defaulted-literal marker is keyed by source location, which survives
// the expression cloning done when a callee's spec is injected into a
// caller's verification — the adoption must also apply to the clone.
module 0x42::bv_spec_literal_default_caller {
    use 0x42::bv_spec_literal_default;

    public fun call_or3(x: u8): u8 {
        bv_spec_literal_default::or3(x)
    }

    spec call_or3 {
        aborts_if false;
        ensures result == (x | 3);
    }
}
