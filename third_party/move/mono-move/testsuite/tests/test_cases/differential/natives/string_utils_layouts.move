// `string_utils::native_format` rendering of: integer suffixes for u16, u32, u128, i8, i16,
// i32 and i128 (extreme values, so a wrong read width shows); the multi-line layout, with
// each element of a vector of vectors, Strings or Options on its own line and a vector of
// primitives on one line; and an address inside a struct, canonical and short.

// RUN: publish
module 0x1::string_utils {
    use std::string::String;

    public native fun native_format<T>(
        s: &T,
        type_tag: bool,
        canonicalize: bool,
        single_line: bool,
        include_int_types: bool,
    ): String;
}
module 0x42::main {
    use std::option;
    use std::string::{String, utf8};
    use 0x1::string_utils::native_format;

    struct Ints has drop {
        unsigned16: u16,
        unsigned32: u32,
        unsigned128: u128,
        signed8: i8,
        signed16: i16,
        signed32: i32,
        signed128: i128,
    }

    struct Holder has drop {
        addr: address,
    }

    fun ints(): Ints {
        Ints {
            unsigned16: 65535,
            unsigned32: 4294967295,
            unsigned128: 340282366920938463463374607431768211455,
            signed8: -128,
            signed16: -32768,
            signed32: -2147483648,
            signed128: -170141183460469231731687303715884105728,
        }
    }

    fun multi_line<T>(value: &T): String {
        native_format(value, false, false, false, false)
    }

    public fun ints_typed(): String {
        native_format(&ints(), false, false, true, true)
    }

    public fun ints_untyped(): String {
        native_format(&ints(), false, false, true, false)
    }

    public fun vec_of_vecs(): String {
        multi_line(&vector[vector[1u64], vector[2u64, 3u64]])
    }

    public fun vec_of_strings(): String {
        multi_line(&vector[utf8(b"a"), utf8(b"bc")])
    }

    public fun vec_of_options(): String {
        multi_line(&vector[option::some(1u64), option::none()])
    }

    public fun vec_of_u64s(): String {
        multi_line(&vector[1u64, 2u64])
    }

    public fun vec_of_addresses(): String {
        multi_line(&vector[@0x1, @0x2])
    }

    public fun address_in_struct_canonical(): String {
        native_format(&Holder { addr: @0x42 }, false, true, true, false)
    }

    public fun address_in_struct_short(): String {
        native_format(&Holder { addr: @0x42 }, false, false, true, false)
    }
}

// RUN: execute 0x42::main::ints_typed
// CHECK: results: "Ints { unsigned16: 65535u16, unsigned32: 4294967295u32, unsigned128: 340282366920938463463374607431768211455u128, signed8: -128i8, signed16: -32768i16, signed32: -2147483648i32, signed128: -170141183460469231731687303715884105728i128 }"

// RUN: execute 0x42::main::ints_untyped
// CHECK: results: "Ints { unsigned16: 65535, unsigned32: 4294967295, unsigned128: 340282366920938463463374607431768211455, signed8: -128, signed16: -32768, signed32: -2147483648, signed128: -170141183460469231731687303715884105728 }"

// RUN: execute 0x42::main::vec_of_vecs
// CHECK: results: "[\n  [ 1 ],\n  [ 2, 3 ]\n]"

// RUN: execute 0x42::main::vec_of_strings
// CHECK: results: "[\n  \"a\",\n  \"bc\"\n]"

// RUN: execute 0x42::main::vec_of_options
// CHECK: results: "[\n  Some(1),\n  None\n]"

// RUN: execute 0x42::main::vec_of_u64s
// CHECK: results: "[ 1, 2 ]"

// RUN: execute 0x42::main::vec_of_addresses
// CHECK: results: "[ @0x1, @0x2 ]"

// RUN: execute 0x42::main::address_in_struct_canonical
// CHECK: results: "Holder { addr: @0000000000000000000000000000000000000000000000000000000000000042 }"

// RUN: execute 0x42::main::address_in_struct_short
// CHECK: results: "Holder { addr: @0x42 }"
