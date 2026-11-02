// Differential coverage for byte-string and vector module constants: every
// constant load goes through mono's constant-pool heap materialization, so
// container constants pin the decode path.

// RUN: publish
module 0x1::consts {
    use std::vector;

    const BYTES: vector<u8> = b"constants";
    const WIDE_VEC: vector<u128> = vector[1, 340282366920938463463374607431768211455];
    const NEG_VEC: vector<i16> = vector[-32768, 0, 32767];
    const EMPTY: vector<u8> = b"";

    public fun get_bytes(): vector<u8> { BYTES }
    public fun get_wide_vec_len_sum(): u64 {
        let v = WIDE_VEC;
        vector::length(&v)
    }
    public fun get_neg_vec_last(): i16 {
        *vector::borrow(&NEG_VEC, 2)
    }

    // A constant used twice in one function must not alias mutable state.
    public fun bytes_twice(): u64 {
        let a = BYTES;
        let b = BYTES;
        vector::push_back(&mut a, 0);
        vector::length(&a) * 100 + vector::length(&b)
    }

    // Empty-vector constant: mono represents empty vectors as null, so
    // length / equality / growth pin the empty<=>null invariant.
    public fun empty_len(): u64 { vector::length(&EMPTY) }

    public fun empty_eq(): bool { EMPTY == b"" }

    public fun empty_push(): u64 {
        let v = EMPTY;
        vector::push_back(&mut v, 9);
        vector::length(&v) * 10 + (*vector::borrow(&v, 0) as u64)
    }
}

// RUN: execute 0x1::consts::get_bytes
// CHECK: results: 0x636f6e7374616e7473

// RUN: execute 0x1::consts::get_wide_vec_len_sum
// CHECK: results: 2

// RUN: execute 0x1::consts::get_neg_vec_last
// CHECK: results: 32767

// a grows to 10, b stays 9.
// RUN: execute 0x1::consts::bytes_twice
// CHECK: results: 1009

// RUN: execute 0x1::consts::empty_len
// CHECK: results: 0

// RUN: execute 0x1::consts::empty_eq
// CHECK: results: true

// RUN: execute 0x1::consts::empty_push
// CHECK: results: 19
