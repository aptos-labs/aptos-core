// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::defaulted_num {
    fun mask(x: u8): u8 { x | 3 }
    spec mask {
        // Both literals below have type u256 before representation analysis.
        // Only the first may adopt the other operand's narrower width.
        ensures result == (x | 3);
        ensures result == (x | 3u256);
        ensures result == (x | 3u8);
    }

    spec schema Masked {
        x: u8;
        ensures (x | 3) >= x;
    }
    fun caller(x: u8): u8 { mask(x) }
    spec caller {
        // The marker must survive schema expansion as well.
        include Masked;
    }
}
