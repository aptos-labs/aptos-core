// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A module axiom over values is assumed in every function, whatever memory it
// reaches. A bitwise operation's operands are taken in either order, an
// `int2bv` of a literal is that literal wrapped, and a specification's
// remainder of a shift is the runtime's.
module 0x42::module_axioms {
    spec module {
        axiom forall a: u16, b: u16 where a != b: spec_swap(a) != spec_swap(b);
    }

    spec fun spec_swap(x: u16): u16;

    fun swap(x: u16): u16 { (x << 8) | (x >> 8) }
    spec swap {
        pragma opaque;
        ensures [abstract] result == spec_swap(x);
    }

    fun distinct(a: u16, b: u16): bool { swap(a) != swap(b) }
    spec distinct {
        ensures a != b ==> result;
    }

    spec fun spec_bit(byte: u8, bit: u64): u8 {
        int2bv(((1 as u8) << (bit as u64)) as u8) & byte
    }

    fun has_bit(byte: u8, bit: u64): bool { byte & (1 << (bit as u8)) != 0 }
    spec has_bit {
        requires bit < 8;
        ensures result == (spec_bit(byte, bit) > 0);
    }

    spec fun count_down(e: u64): u64 {
        if (e == 0) { 0 } else { count_down(e - int2bv((1 as u64))) }
    }

    fun zero(): u64 { 0 }
    spec zero {
        ensures result == count_down(0);
    }
}
