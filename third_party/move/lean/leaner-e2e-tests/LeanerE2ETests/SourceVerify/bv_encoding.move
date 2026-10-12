// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The Move Prover's bit-vector representation is a backend encoding. Leaner
// keeps specification arithmetic mathematical wherever `pragma bv`, bitwise
// operations or `bv_internal` select that encoding (G14 in
// prover-test-problems.md). The `*_sum` claims hold for unbounded integers;
// the Prover, wrapping at eight bits, rejects them.
module 0x42::bv_encoding {
    fun pragma_sum(x: u8): u8 { x }
    spec pragma_sum {
        pragma bv = b"0";
        requires x == 255;
        ensures x + 1 > 255;
    }

    fun bitwise_sum(x: u8): u8 { x }
    spec bitwise_sum {
        requires x == 255;
        ensures ((x & (255 as u8)) + 1) > 255;
    }

    fun body_sum(x: u8): u8 { x & 255 }
    spec body_sum {
        requires x == 255;
        ensures result + 1 > 255;
    }

    fun internal_sum(x: u8): u8 { x | 0 }
    spec internal_sum {
        pragma opaque;
        pragma bv_internal;
        requires x == 255;
        ensures [concrete] result + 1 > 255;
    }

    fun converted_sum(x: u8): u8 { x }
    spec converted_sum {
        requires x == 255;
        ensures int2bv(x) + int2bv((1 as u8)) > 255;
    }

    // The Prover renders signed values as integers, so its signed `int2bv`
    // does not wrap; Leaner's yields a value of the type.
    fun signed_wrap(x: i8): i8 { x }
    spec signed_wrap {
        requires x == 127;
        ensures int2bv(((x as num) + 1) as i8) == -128;
    }

    // Both verifiers agree on the remaining functions.
    fun identity(x: u8): u8 { x | 0 }
    spec identity {
        pragma opaque;
        pragma bv_internal;
        pragma bv = b"0";
        pragma bv_ret = b"0";
        aborts_if false;
        ensures result == x;
    }

    fun caller(x: u8): u64 { (identity(x) as u64) + 1 }
    spec caller {
        aborts_if false;
        ensures result == (x as num) + 1;
    }

    fun clear(x: &mut u8) { *x = *x & 0; }
    spec clear {
        pragma opaque;
        pragma bv_internal;
        aborts_if false;
        ensures (x as num) == 0;
    }

    fun clear_caller(): u8 {
        let x = 255;
        clear(&mut x);
        x
    }
    spec clear_caller {
        aborts_if false;
        ensures result == 0;
    }

    // Executable arithmetic stays checked.
    fun checked_product(x: u8): u8 { (x | 0u8) * 129u8 }
    spec checked_product {
        requires x == 2;
        aborts_if true;
    }
}
