// `from_bcs::from_bytes` rejects a bool byte other than 0 or 1 wherever the bool sits: after
// a u8, next to another bool, after a u64, in a vector, an option, an enum payload, and
// nested aggregates. `valid` decodes the same shapes with valid bools.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x1::main {
    use std::option::{Self, Option};
    use std::vector;

    struct U8Bool has copy, drop { a: u8, b: bool }
    struct BoolPair has copy, drop { x: bool, y: bool }
    struct Padded has copy, drop { a: u64, b: bool }
    struct Outer has copy, drop { p: U8Bool, v: vector<BoolPair>, o: Option<bool> }
    enum E has copy, drop { A { n: u16 }, B { f: bool, g: u8 } }

    public fun valid(): u64 {
        let byte_bool = 0x1::from_bcs::from_bytes<U8Bool>(x"0701");
        let pair = 0x1::from_bcs::from_bytes<BoolPair>(x"0100");
        let padded_bool = 0x1::from_bcs::from_bytes<Padded>(x"070000000000000001");
        let byte_bools = 0x1::from_bcs::from_bytes<vector<U8Bool>>(x"0205000601");
        let outer = 0x1::from_bcs::from_bytes<Outer>(x"090102010000010101");
        let tagged = 0x1::from_bcs::from_bytes<E>(x"010109");
        let bits = 0;
        if (byte_bool.a == 7 && byte_bool.b) bits = bits | 1;
        if (pair.x && !pair.y) bits = bits | 2;
        if (padded_bool.a == 7 && padded_bool.b) bits = bits | 4;
        if (vector::length(&byte_bools) == 2 && !vector::borrow(&byte_bools, 0).b && vector::borrow(&byte_bools, 1).b) bits = bits | 8;
        if (outer.p.b && vector::length(&outer.v) == 2 && *option::borrow(&outer.o)) bits = bits | 16;
        if (tagged == E::B { f: true, g: 9 }) bits = bits | 32;
        bits
    }

    public fun u8_bool(): u8 { 0x1::from_bcs::from_bytes<U8Bool>(x"0703").a }
    public fun bool_pair(): bool { 0x1::from_bcs::from_bytes<BoolPair>(x"0102").x }
    public fun padded(): u64 { 0x1::from_bcs::from_bytes<Padded>(x"0700000000000000ff").a }
    public fun vec_bool(): u64 { vector::length(&0x1::from_bcs::from_bytes<vector<bool>>(x"03010002")) }
    public fun vec_u8_bool(): u64 { vector::length(&0x1::from_bcs::from_bytes<vector<U8Bool>>(x"020500060a")) }
    public fun option_bool(): bool { option::is_some(&0x1::from_bcs::from_bytes<Option<bool>>(x"0180")) }
    public fun enum_payload(): bool { 0x1::from_bcs::from_bytes<E>(x"010209") == E::A { n: 0 } }
    public fun nested_last(): bool { 0x1::from_bcs::from_bytes<Outer>(x"090102010000010102").p.b }
}

// RUN: execute 0x1::main::valid
// CHECK: results: 63

// RUN: execute 0x1::main::u8_bool
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::bool_pair
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::padded
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::vec_bool
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::vec_u8_bool
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::option_bool
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::enum_payload
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::nested_last
// CHECK: aborted: code 65537 in 0x1::from_bcs
