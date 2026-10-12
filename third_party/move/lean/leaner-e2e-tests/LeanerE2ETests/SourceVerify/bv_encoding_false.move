// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Claims that hold only with the Prover's eight-bit wrapping of encoded
// specification arithmetic (G14 in prover-test-problems.md). Leaner's
// arithmetic is mathematical, so it rejects each one; the Prover proves them.
module 0x42::bv_encoding_false {
    spec fun add_one(x: u8): u8 { x + 1 }

    fun sum(x: u8): u8 { x }
    spec sum {
        requires x == 255;
        ensures int2bv(x) + int2bv(1u8) == 0u8; // error: the sum is 256
    }

    fun spec_call(x: u8): u8 { x }
    spec spec_call {
        requires x == 255;
        ensures add_one(int2bv(x)) == 0u8; // error: the sum is 256
    }

    fun narrowing_bound(x: u16): u16 { x | 0 }
    spec narrowing_bound {
        pragma bv = b"0";
        ensures (x as u8) <= 255u8; // error: the cast keeps x
    }
}
