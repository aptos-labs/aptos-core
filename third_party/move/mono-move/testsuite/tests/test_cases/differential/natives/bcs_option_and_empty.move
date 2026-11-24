// `bcs::to_bytes` of top-level empty vectors and of `Option` values, and
// `bcs::constant_serialized_size` of an enum (`Option<u64>`), top-level vectors, primitives
// and a function type. Each size step returns (is_some, size or 0); the struct step is the
// control for the `None` cases. Expected bytes follow BCS: a vector is a ULEB128 length then
// its elements, an Option is tag 0 (None) or tag 1 then the value.

// RUN: publish
module 0x42::main {
    use std::bcs;
    use std::option::{Self, Option};

    struct Item has drop { tag: u8, data: vector<u8> }

    struct Wrap has drop { value: u64 }

    public fun bytes_empty_u64s(): vector<u8> {
        bcs::to_bytes(&vector<u64>[])
    }

    public fun bytes_empty_bytes(): vector<u8> {
        bcs::to_bytes(&vector<u8>[])
    }

    public fun bytes_empty_items(): vector<u8> {
        bcs::to_bytes(&vector<Item>[])
    }

    public fun bytes_none(): vector<u8> {
        bcs::to_bytes(&option::none<u64>())
    }

    public fun bytes_some(): vector<u8> {
        bcs::to_bytes(&option::some(5u64))
    }

    public fun bytes_some_empty_vector(): vector<u8> {
        bcs::to_bytes(&option::some(vector<u64>[]))
    }

    public fun bytes_some_item(): vector<u8> {
        bcs::to_bytes(&option::some(Item { tag: 7, data: b"hi" }))
    }

    public fun bytes_some_none(): vector<u8> {
        bcs::to_bytes(&option::some(option::none<u8>()))
    }

    fun describe(size: Option<u64>): (bool, u64) {
        (size.is_some(), size.destroy_with_default(0))
    }

    public fun const_size_struct(): (bool, u64) {
        describe(bcs::constant_serialized_size<Wrap>())
    }

    public fun const_size_option(): (bool, u64) {
        describe(bcs::constant_serialized_size<Option<u64>>())
    }

    public fun const_size_vector_u8(): (bool, u64) {
        describe(bcs::constant_serialized_size<vector<u8>>())
    }

    public fun const_size_vector_u64(): (bool, u64) {
        describe(bcs::constant_serialized_size<vector<u64>>())
    }

    public fun const_size_function(): (bool, u64) {
        describe(bcs::constant_serialized_size<|u64|u64>())
    }

    public fun const_size_bool(): (bool, u64) {
        describe(bcs::constant_serialized_size<bool>())
    }

    public fun const_size_u64(): (bool, u64) {
        describe(bcs::constant_serialized_size<u64>())
    }

    public fun const_size_address(): (bool, u64) {
        describe(bcs::constant_serialized_size<address>())
    }
}

// RUN: execute 0x42::main::bytes_empty_u64s
// CHECK: results: 0x00

// RUN: execute 0x42::main::bytes_empty_bytes
// CHECK: results: 0x00

// RUN: execute 0x42::main::bytes_empty_items
// CHECK: results: 0x00

// RUN: execute 0x42::main::bytes_none
// CHECK: results: 0x00

// RUN: execute 0x42::main::bytes_some
// CHECK: results: 0x010500000000000000

// RUN: execute 0x42::main::bytes_some_empty_vector
// CHECK: results: 0x0100

// RUN: execute 0x42::main::bytes_some_item
// CHECK: results: 0x0107026869

// RUN: execute 0x42::main::bytes_some_none
// CHECK: results: 0x0100

// RUN: execute 0x42::main::const_size_struct
// CHECK: results: true, 8

// RUN: execute 0x42::main::const_size_option
// CHECK: results: false, 0

// RUN: execute 0x42::main::const_size_vector_u8
// CHECK: results: false, 0

// RUN: execute 0x42::main::const_size_vector_u64
// CHECK: results: false, 0

// RUN: execute 0x42::main::const_size_function
// CHECK: results: false, 0

// RUN: execute 0x42::main::const_size_bool
// CHECK: results: true, 1

// RUN: execute 0x42::main::const_size_u64
// CHECK: results: true, 8

// RUN: execute 0x42::main::const_size_address
// CHECK: results: true, 32
