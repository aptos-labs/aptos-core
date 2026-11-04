// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::cast_comparison {
    public fun direct(a: u64, b: u8): bool {
        (a as u8) < b
    }

    public fun nested(a: u8, b: u64, c: u8): bool {
        a + (b as u8) < c
    }

    public fun at_most(a: u64, b: u8): bool {
        (a as u8) <= b
    }
}
