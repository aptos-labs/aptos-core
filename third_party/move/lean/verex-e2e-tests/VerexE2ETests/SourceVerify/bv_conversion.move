// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `int2bv` wraps into its fixed-width result type, two's complement for a
// signed one; `bv2int` reads the value back. A specification cast otherwise
// keeps its value.
module 0x42::bv_conversion {
    fun unsigned_wrap(x: u8): u8 { x }
    spec unsigned_wrap {
        requires x == 255;
        ensures int2bv(((x as num) + 1) as u8) == 0u8;
    }

    fun round_trip(x: u8): u8 { x }
    spec round_trip {
        ensures bv2int(int2bv(x)) == x;
    }

    fun narrowed(x: u16): u16 { x }
    spec narrowed {
        requires x == 256;
        ensures int2bv(((x as num) as u8)) == 0;
    }

    fun in_range_cast(x: u16): u16 { x }
    spec in_range_cast {
        requires x <= 255;
        ensures ((x as u8) as u16) == x;
    }

    spec fun add_num(x: num): num { x + 1 }

    fun num_argument(x: u8): u8 { x }
    spec num_argument {
        requires x == 255;
        ensures add_num(int2bv(x)) == 256;
    }
}
