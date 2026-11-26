// `bcs::constant_serialized_size` of signed and unsigned integer widths and of structs built
// from them: each fixed-width integer serializes to its width in bytes. Vectors, enums,
// function values and structs holding them give `None`, even an enum whose variants all
// encode to the same number of bytes. Each struct holding an enum or a function value has a
// control with that field replaced by a fixed-width integer.

// RUN: publish
module 0x42::shapes {
    struct Box<T> { value: T }
    struct Pair<A, B> { first: A, second: B }

    enum SameSize { A { left: u64 }, B { right: u64 } }
    enum Unit { Red, Green }

    struct HoldsEnum { tag: u8, payload: SameSize }
    struct HoldsU64 { tag: u8, payload: u64 }
    struct HoldsUnit { payload: Unit, tail: u16 }
    struct HoldsU8 { payload: u8, tail: u16 }
    struct HoldsNested { inner: Box<HoldsEnum> }
    struct HoldsFn { tag: u8, callback: |u64|u64 has copy + drop }
    struct HoldsFnControl { tag: u8, callback: u64 }
}

module 0x42::sizes {
    use std::bcs;
    use std::option::{Self, Option};
    use 0x42::shapes::{Box, Pair, SameSize, HoldsEnum, HoldsU64, HoldsUnit, HoldsU8, HoldsNested, HoldsFn, HoldsFnControl};

    fun size_of(size: Option<u64>): u64 {
        option::destroy_some(size)
    }

    public fun unsigned(): (u64, u64, u64, u64, u64, u64) {
        (
            size_of(bcs::constant_serialized_size<u8>()),
            size_of(bcs::constant_serialized_size<u16>()),
            size_of(bcs::constant_serialized_size<u32>()),
            size_of(bcs::constant_serialized_size<u128>()),
            size_of(bcs::constant_serialized_size<u256>()),
            size_of(bcs::constant_serialized_size<Box<u256>>()),
        )
    }

    public fun signed(): (u64, u64, u64, u64, u64, u64) {
        (
            size_of(bcs::constant_serialized_size<i8>()),
            size_of(bcs::constant_serialized_size<i16>()),
            size_of(bcs::constant_serialized_size<i32>()),
            size_of(bcs::constant_serialized_size<i64>()),
            size_of(bcs::constant_serialized_size<i128>()),
            size_of(bcs::constant_serialized_size<i256>()),
        )
    }

    public fun signed_structs(): (u64, u64) {
        (
            size_of(bcs::constant_serialized_size<Pair<i8, i256>>()),
            size_of(bcs::constant_serialized_size<Box<Pair<i16, Pair<i32, i128>>>>()),
        )
    }

    public fun enums(): (bool, bool, bool, bool, bool) {
        (
            option::is_none(&bcs::constant_serialized_size<SameSize>()),
            option::is_none(&bcs::constant_serialized_size<HoldsEnum>()),
            option::is_none(&bcs::constant_serialized_size<HoldsUnit>()),
            option::is_none(&bcs::constant_serialized_size<HoldsNested>()),
            option::is_none(&bcs::constant_serialized_size<Box<Option<u64>>>()),
        )
    }

    public fun enum_controls(): (u64, u64, u64) {
        (
            size_of(bcs::constant_serialized_size<HoldsU64>()),
            size_of(bcs::constant_serialized_size<HoldsU8>()),
            size_of(bcs::constant_serialized_size<Box<HoldsU64>>()),
        )
    }

    public fun vectors(): (bool, bool) {
        (
            option::is_none(&bcs::constant_serialized_size<vector<u64>>()),
            option::is_none(&bcs::constant_serialized_size<Pair<u8, vector<u8>>>()),
        )
    }

    public fun functions(): (bool, bool, bool) {
        (
            option::is_none(&bcs::constant_serialized_size<|u64|u64 has copy + drop>()),
            option::is_none(&bcs::constant_serialized_size<HoldsFn>()),
            option::is_none(&bcs::constant_serialized_size<Box<HoldsFn>>()),
        )
    }

    public fun function_controls(): (u64, u64) {
        (
            size_of(bcs::constant_serialized_size<HoldsFnControl>()),
            size_of(bcs::constant_serialized_size<Box<HoldsFnControl>>()),
        )
    }
}

// RUN: execute 0x42::sizes::unsigned
// CHECK: results: 1, 2, 4, 16, 32, 32

// RUN: execute 0x42::sizes::signed
// CHECK: results: 1, 2, 4, 8, 16, 32

// RUN: execute 0x42::sizes::signed_structs
// CHECK: results: 33, 22

// RUN: execute 0x42::sizes::enums
// CHECK: results: true, true, true, true, true

// RUN: execute 0x42::sizes::enum_controls
// CHECK: results: 9, 3, 9

// RUN: execute 0x42::sizes::vectors
// CHECK: results: true, true

// RUN: execute 0x42::sizes::functions
// CHECK: results: true, true, true

// RUN: execute 0x42::sizes::function_controls
// CHECK: results: 9, 9
