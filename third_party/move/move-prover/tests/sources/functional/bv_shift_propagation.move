// exclude_for: cvc5
// Tests that Shl and Shr correctly propagate bv classification from a bv-annotated
// struct field without requiring an explicit pragma bv on the operation itself.
// Exercises the separate Shl|Shr match arm in NumberOperationAnalysis::execute.
module 0x42::BvShiftPropagation {

    struct Flags has copy, drop {
        bits: u64,
    }
    spec Flags {
        // bits is bv64; shift operations on bits should also be bv64 via propagation.
        pragma bv = b"0";
    }

    // Set a bit: the shift result inherits bv64 classification from bits.
    fun set_bit(f: Flags, pos: u8): Flags {
        Flags { bits: f.bits | ((1 as u64) << pos) }
    }
    spec set_bit {
        pragma bv = b"0,1";
        pragma bv_ret = b"0";
        aborts_if false;
        ensures result.bits == (f.bits | ((1 as u64) << pos));
    }

    // Test a bit: right-shift result inherits bv64 from bits.
    fun test_bit(f: Flags, pos: u8): bool {
        (f.bits >> pos) & 1 != 0
    }
    spec test_bit {
        pragma bv = b"0,1";
        aborts_if false;
        ensures result == ((f.bits >> pos) & (1 as u64) != (0 as u64));
    }

    // Shift left by a variable amount and OR: both Shl and BitOr on the same bv value.
    fun apply_mask(f: Flags, bit_pos: u8, width: u8): Flags {
        let mask = ((1 as u64) << width) - 1;
        Flags { bits: f.bits | (mask << bit_pos) }
    }
    spec apply_mask {
        pragma bv = b"0,1,2";
        pragma bv_ret = b"0";
        aborts_if false;
        ensures result.bits == (f.bits | ((((1 as u64) << width) - (1 as u64)) << bit_pos));
    }
}
