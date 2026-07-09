// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x42::generic_struct_pack_unpack {
    struct Box<T> has drop {
        value: T,
    }

    struct Pair<A, B> has drop {
        first: A,
        second: B,
    }

    fun box_u8(v: u8): u8 {
        let b = Box<u8> { value: v };
        let Box<u8> { value } = b;
        value
    }

    fun box_u64(v: u64): u64 {
        let b = Box<u64> { value: v };
        let Box<u64> { value } = b;
        value
    }

    fun box_u128(v: u128): u128 {
        let b = Box<u128> { value: v };
        let Box<u128> { value } = b;
        value
    }

    fun box_address(a: address): u64 {
        let b = Box<address> { value: a };
        let Box<address> { value } = b;
        if (value == a) { 1 } else { 0 }
    }

    fun pair_swapped(x: u8, y: u64): u64 {
        let p = Pair<u8, u64> { first: x, second: y };
        let q = Pair<u64, u8> { first: y, second: x };
        let Pair { first: a, second: b } = p;
        let Pair { first: c, second: d } = q;
        (a as u64) + b + c + (d as u64)
    }

    fun pair_wide(x: u64, y: u64): u64 {
        let p = Pair<u128, u64> { first: (x as u128), second: y };
        let Pair { first: a, second: b } = p;
        (a as u64) + b
    }

    fun box_nested(v: u64): u64 {
        let b = Box<Box<u64>> { value: Box<u64> { value: v } };
        let Box<Box<u64>> { value: inner } = b;
        let Box<u64> { value } = inner;
        value
    }
}

// RUN: execute 0x42::generic_struct_pack_unpack::box_u8 --args 200
// CHECK: results: 200

// RUN: execute 0x42::generic_struct_pack_unpack::box_u64 --args 1234567890123
// CHECK: results: 1234567890123

// RUN: execute 0x42::generic_struct_pack_unpack::box_u128 --args 340282366920938463463374607431768211455
// CHECK: results: 340282366920938463463374607431768211455

// RUN: execute 0x42::generic_struct_pack_unpack::box_address --args 0xcafe
// CHECK: results: 1

// RUN: execute 0x42::generic_struct_pack_unpack::pair_swapped --args 7, 1000
// CHECK: results: 2014

// RUN: execute 0x42::generic_struct_pack_unpack::pair_wide --args 11, 2000
// CHECK: results: 2011

// RUN: execute 0x42::generic_struct_pack_unpack::box_nested --args 99
// CHECK: results: 99
