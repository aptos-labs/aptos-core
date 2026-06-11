// Differential test for the `bcs` natives `to_bytes` and `serialized_size`,
// covering primitives, vectors, and (nested) structs.
//
// `to_bytes` is called through the real `std::bcs`. The plain move-stdlib `bcs`
// module declares only `to_bytes`, so `serialized_size` is reached through the
// `test_natives` module (the V1 side re-registers the legacy impl there, the V2
// side registers the mono-move native).
//
// `constant_serialized_size` is intentionally not covered: its `Option<u64>`
// (enum) return is not yet implemented in mono-move.

// RUN: publish
module 0x1::test_natives {
    native public fun serialized_size<T>(v: &T): u64;
}
module 0x42::bcs_test {
    use std::bcs;
    use 0x1::test_natives;

    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    struct Nested has copy, drop {
        p: Point,
        tag: u8,
    }

    fun point(): Point { Point { x: 1, y: 2 } }

    fun nested(): Nested { Nested { p: Point { x: 1, y: 2 }, tag: 9 } }

    // --- to_bytes ---
    public fun bytes_u8(): vector<u8> { bcs::to_bytes(&7u8) }
    public fun bytes_u64(): vector<u8> { bcs::to_bytes(&42u64) }
    public fun bytes_u128(): vector<u8> { bcs::to_bytes(&42u128) }
    public fun bytes_bool(): vector<u8> { bcs::to_bytes(&true) }
    public fun bytes_addr(): vector<u8> { bcs::to_bytes(&@0x1) }
    public fun bytes_vec_u8(): vector<u8> { bcs::to_bytes(&vector<u8>[1, 2, 3]) }
    public fun bytes_vec_u64(): vector<u8> { bcs::to_bytes(&vector<u64>[1, 2]) }
    public fun bytes_struct(): vector<u8> { bcs::to_bytes(&point()) }
    public fun bytes_nested(): vector<u8> { bcs::to_bytes(&nested()) }

    // --- serialized_size ---
    public fun size_u8(): u64 { test_natives::serialized_size(&7u8) }
    public fun size_u64(): u64 { test_natives::serialized_size(&42u64) }
    public fun size_u128(): u64 { test_natives::serialized_size(&42u128) }
    public fun size_bool(): u64 { test_natives::serialized_size(&true) }
    public fun size_addr(): u64 { test_natives::serialized_size(&@0x1) }
    public fun size_vec_u8(): u64 { test_natives::serialized_size(&vector<u8>[1, 2, 3]) }
    public fun size_vec_u64(): u64 { test_natives::serialized_size(&vector<u64>[1, 2]) }
    public fun size_struct(): u64 { test_natives::serialized_size(&point()) }
    public fun size_nested(): u64 { test_natives::serialized_size(&nested()) }
}

// --- to_bytes ---
// RUN: execute 0x42::bcs_test::bytes_u8
// CHECK: results: 0x07
// RUN: execute 0x42::bcs_test::bytes_u64
// CHECK: results: 0x2a00000000000000
// RUN: execute 0x42::bcs_test::bytes_u128
// CHECK: results: 0x2a000000000000000000000000000000
// RUN: execute 0x42::bcs_test::bytes_bool
// CHECK: results: 0x01
// RUN: execute 0x42::bcs_test::bytes_addr
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000001
// RUN: execute 0x42::bcs_test::bytes_vec_u8
// CHECK: results: 0x03010203
// RUN: execute 0x42::bcs_test::bytes_vec_u64
// CHECK: results: 0x0201000000000000000200000000000000
// RUN: execute 0x42::bcs_test::bytes_struct
// CHECK: results: 0x01000000000000000200000000000000
// RUN: execute 0x42::bcs_test::bytes_nested
// CHECK: results: 0x0100000000000000020000000000000009

// --- serialized_size ---
// RUN: execute 0x42::bcs_test::size_u8
// CHECK: results: 1
// RUN: execute 0x42::bcs_test::size_u64
// CHECK: results: 8
// RUN: execute 0x42::bcs_test::size_u128
// CHECK: results: 16
// RUN: execute 0x42::bcs_test::size_bool
// CHECK: results: 1
// RUN: execute 0x42::bcs_test::size_addr
// CHECK: results: 32
// RUN: execute 0x42::bcs_test::size_vec_u8
// CHECK: results: 4
// RUN: execute 0x42::bcs_test::size_vec_u64
// CHECK: results: 17
// RUN: execute 0x42::bcs_test::size_struct
// CHECK: results: 16
// RUN: execute 0x42::bcs_test::size_nested
// CHECK: results: 17
