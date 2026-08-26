// RUN: publish
module 0x1::boxes {
    struct Box<T> { value: T }

    struct Pair<A, B> { first: A, second: B }
}

module 0x1::main {
    use std::bcs;
    use std::option;
    use 0x1::boxes;

    struct Pair has drop {
        a: u64,
        b: bool,
    }

    struct Wrap has drop {
        v: vector<u8>,
    }

    public fun bytes_u64(): vector<u8> {
        bcs::to_bytes(&42u64)
    }

    public fun bytes_bool(): vector<u8> {
        bcs::to_bytes(&true)
    }

    public fun bytes_u16(): vector<u8> {
        bcs::to_bytes(&258u16)
    }

    public fun bytes_bytes(): vector<u8> {
        bcs::to_bytes(&b"abc")
    }

    public fun bytes_address(): vector<u8> {
        bcs::to_bytes(&@0x1)
    }

    public fun bytes_struct(): vector<u8> {
        bcs::to_bytes(&Pair { a: 1, b: true })
    }

    public fun bytes_nested_vec(): vector<u8> {
        bcs::to_bytes(&vector[vector[1u8, 2u8], vector[3u8, 4u8, 5u8]])
    }

    public fun bytes_wrap(): vector<u8> {
        bcs::to_bytes(&Wrap { v: b"hi" })
    }

    public fun size_u64(): u64 {
        bcs::serialized_size(&42u64)
    }

    public fun size_bool(): u64 {
        bcs::serialized_size(&true)
    }

    public fun size_u16(): u64 {
        bcs::serialized_size(&258u16)
    }

    public fun size_bytes(): u64 {
        bcs::serialized_size(&b"abc")
    }

    public fun size_address(): u64 {
        bcs::serialized_size(&@0x1)
    }

    public fun size_struct(): u64 {
        bcs::serialized_size(&Pair { a: 1, b: true })
    }

    public fun size_nested_vec(): u64 {
        bcs::serialized_size(&vector[vector[1u8, 2u8], vector[3u8, 4u8, 5u8]])
    }

    public fun size_wrap(): u64 {
        bcs::serialized_size(&Wrap { v: b"hi" })
    }

    public fun const_size_box_u64(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Box<u64>>())
    }

    public fun const_size_box_bool(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Box<bool>>())
    }

    public fun const_size_box_address(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Box<address>>())
    }

    public fun const_size_pair(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Pair<u64, bool>>())
    }

    public fun const_size_nested(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Box<boxes::Pair<u64, u128>>>())
    }

    public fun const_size_is_some_box_u64(): bool {
        option::is_some(&bcs::constant_serialized_size<boxes::Box<u64>>())
    }

    public fun const_size_is_some_box_vec(): bool {
        option::is_some(&bcs::constant_serialized_size<boxes::Box<vector<u8>>>())
    }
}

// RUN: execute 0x1::main::bytes_u64
// CHECK: results: 0x2a00000000000000

// RUN: execute 0x1::main::bytes_bool
// CHECK: results: 0x01

// RUN: execute 0x1::main::bytes_u16
// CHECK: results: 0x0201

// RUN: execute 0x1::main::bytes_bytes
// CHECK: results: 0x03616263

// RUN: execute 0x1::main::bytes_address
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000001

// RUN: execute 0x1::main::bytes_struct
// CHECK: results: 0x010000000000000001

// RUN: execute 0x1::main::bytes_nested_vec
// CHECK: results: 0x0202010203030405

// RUN: execute 0x1::main::bytes_wrap
// CHECK: results: 0x026869

// Each serialized size equals the byte length of the corresponding `to_bytes`.

// RUN: execute 0x1::main::size_u64
// CHECK: results: 8

// RUN: execute 0x1::main::size_bool
// CHECK: results: 1

// RUN: execute 0x1::main::size_u16
// CHECK: results: 2

// RUN: execute 0x1::main::size_bytes
// CHECK: results: 4

// RUN: execute 0x1::main::size_address
// CHECK: results: 32

// RUN: execute 0x1::main::size_struct
// CHECK: results: 9

// RUN: execute 0x1::main::size_nested_vec
// CHECK: results: 8

// RUN: execute 0x1::main::size_wrap
// CHECK: results: 3

// RUN: execute 0x1::main::const_size_box_u64
// CHECK: results: 8

// RUN: execute 0x1::main::const_size_box_bool
// CHECK: results: 1

// RUN: execute 0x1::main::const_size_box_address
// CHECK: results: 32

// RUN: execute 0x1::main::const_size_pair
// CHECK: results: 9

// RUN: execute 0x1::main::const_size_nested
// CHECK: results: 24

// RUN: execute 0x1::main::const_size_is_some_box_u64
// CHECK: results: true

// RUN: execute 0x1::main::const_size_is_some_box_vec
// CHECK: results: false
