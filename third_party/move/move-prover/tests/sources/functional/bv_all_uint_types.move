// exclude_for: cvc5
// Tests all 6 unsigned integer types (u8–u256) in bitvector mode.
// Exercises uint_bv_type for all bit widths and boogie_type with bv_flag=true
// for each primitive numeric type.
module 0x42::BvAllUintTypes {

    struct All has copy, drop {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
        e: u128,
        f: u256,
    }
    spec All {
        pragma bv = b"0,1,2,3,4,5";
    }

    // XOR of a value with itself is always 0; trivially provable in bv mode.
    // Forces all 6 bv types to appear in the generated Boogie.
    fun bitops(x: All): All {
        All {
            a: x.a ^ x.a,
            b: x.b ^ x.b,
            c: x.c ^ x.c,
            d: x.d ^ x.d,
            e: x.e ^ x.e,
            f: x.f ^ x.f,
        }
    }
    spec bitops {
        pragma bv = b"0";
        pragma bv_ret = b"0";
        ensures result.a == (0 as u8);
        ensures result.b == (0 as u16);
        ensures result.c == (0 as u32);
        ensures result.d == (0 as u64);
        ensures result.e == (0 as u128);
        ensures result.f == (0 as u256);
    }

    // AND of max-value masks: each type AND-ed with its own all-ones value is itself.
    fun identity_and(x: All): All {
        All {
            a: x.a & 0xFF,
            b: x.b & 0xFFFF,
            c: x.c & 0xFFFFFFFF,
            d: x.d & 0xFFFFFFFFFFFFFFFF,
            e: x.e & 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF,
            f: x.f & 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF,
        }
    }
    spec identity_and {
        pragma bv = b"0";
        pragma bv_ret = b"0";
        ensures result.a == x.a;
        ensures result.b == x.b;
        ensures result.c == x.c;
        ensures result.d == x.d;
        ensures result.e == x.e;
        ensures result.f == x.f;
    }
}
