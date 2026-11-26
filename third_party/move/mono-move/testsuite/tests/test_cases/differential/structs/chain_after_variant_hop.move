// Reading and writing a field two structs deep inside an enum variant (`variant.p.q.r`),
// and reading a field chain through a frozen mutable reference. The golden pins how the
// chains lower.

// RUN: publish --print(stackless,micro-ops)
module 0x1::chain_after_variant_hop {
    struct R has copy, drop { k: u8, r: u64 }
    struct Q has copy, drop { pad: u64, q: R }
    enum E has drop {
        A { p: Q },
        B { n: u64, p: Q },
    }
    enum F has drop {
        A { p: Q },
        B { p: Q, n: u64 },
    }
    struct S has drop { a: Q, z: u64 }

    fun read_r(variant: &E): u64 { variant.p.q.r }

    fun write_r(variant: &mut E, value: u64) { variant.p.q.r = value }

    fun make(sel: u64): E {
        let payload = Q { pad: 1, q: R { k: 2, r: 3 } };
        if (sel == 0) E::A { p: payload } else E::B { n: 9, p: payload }
    }

    public fun hop(sel: u64): u64 {
        let variant = make(sel);
        let value = read_r(&variant) + 40;
        write_r(&mut variant, value);
        read_r(&variant)
    }

    // `p` comes first in both variants of `F`; in `E` its position differs by variant.
    fun read_f(variant: &F): u64 { variant.p.q.r }

    public fun hop_shared(sel: u64): u64 {
        let payload = Q { pad: 1, q: R { k: 2, r: 3 } };
        let variant = if (sel == 0) F::A { p: payload } else F::B { p: payload, n: 9 };
        read_f(&variant)
    }

    public fun frozen_chain(): u64 {
        let holder = S { a: Q { pad: 5, q: R { k: 6, r: 7 } }, z: 8 };
        let mut_ref = &mut holder.a;
        let frozen = freeze(mut_ref);
        frozen.q.r * 10 + (frozen.q.k as u64)
    }
}

// RUN: execute 0x1::chain_after_variant_hop::hop --args 0
// CHECK: results: 43

// RUN: execute 0x1::chain_after_variant_hop::hop --args 1
// CHECK: results: 43

// RUN: execute 0x1::chain_after_variant_hop::hop_shared --args 0
// CHECK: results: 3

// RUN: execute 0x1::chain_after_variant_hop::hop_shared --args 1
// CHECK: results: 3

// RUN: execute 0x1::chain_after_variant_hop::frozen_chain
// CHECK: results: 76
