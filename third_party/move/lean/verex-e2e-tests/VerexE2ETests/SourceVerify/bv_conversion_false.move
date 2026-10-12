// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::bv_conversion_false {
    fun unsigned_bound(x: u8): u8 { x }
    spec unsigned_bound {
        requires x == 255;
        // The conversion yields zero; erasing it would prove 256 > 255.
        ensures int2bv(((x as num) + 1) as u8) > 255; // error: wraps to zero
    }

    fun signed_bound(x: i8): i8 { x }
    spec signed_bound {
        requires x == 127;
        ensures (int2bv(((x as num) + 129) as i8) as u8) > 255; // error: wraps to zero
    }

    fun cast_not_modular(x: u16): u16 { x }
    spec cast_not_modular {
        requires x == 256;
        // Only `int2bv` wraps.
        ensures (x as u8) == 0; // error: the cast keeps 256
    }
}
