// `type_info::type_name` for bool, signer, u16, u32, u128, u256, every signed width, and
// structs at non-special addresses, alone and nested in a generic. Struct addresses render
// with leading zeros trimmed, so one with a non-zero first hex digit prints all 64 digits.

// RUN: publish
module 0x1::type_info {
    use std::string::String;
    public native fun type_name<T>(): String;
}
module 0x42::m {
    struct S has drop {}
    struct G<phantom T> has drop {}
}
module 0xa11ce00000000000000000000000000000000000000000000000000000000b0b::far {
    struct F has drop {}
}
module 0x42::main {
    use std::string::String;
    use 0x1::type_info::type_name;

    public fun bool_name(): String { type_name<bool>() }
    public fun signer_name(): String { type_name<signer>() }
    public fun u16_name(): String { type_name<u16>() }
    public fun u32_name(): String { type_name<u32>() }
    public fun u128_name(): String { type_name<u128>() }
    public fun u256_name(): String { type_name<u256>() }
    public fun i8_name(): String { type_name<i8>() }
    public fun i16_name(): String { type_name<i16>() }
    public fun i32_name(): String { type_name<i32>() }
    public fun i64_name(): String { type_name<i64>() }
    public fun i128_name(): String { type_name<i128>() }
    public fun i256_name(): String { type_name<i256>() }

    public fun struct_at_0x42(): String {
        type_name<0x42::m::S>()
    }

    public fun struct_at_full_address(): String {
        type_name<0xa11ce00000000000000000000000000000000000000000000000000000000b0b::far::F>()
    }

    public fun generic_across_addresses(): String {
        type_name<0x42::m::G<vector<0xa11ce00000000000000000000000000000000000000000000000000000000b0b::far::F>>>()
    }
}

// RUN: execute 0x42::main::bool_name
// CHECK: results: "bool"

// RUN: execute 0x42::main::signer_name
// CHECK: results: "signer"

// RUN: execute 0x42::main::u16_name
// CHECK: results: "u16"

// RUN: execute 0x42::main::u32_name
// CHECK: results: "u32"

// RUN: execute 0x42::main::u128_name
// CHECK: results: "u128"

// RUN: execute 0x42::main::u256_name
// CHECK: results: "u256"

// RUN: execute 0x42::main::i8_name
// CHECK: results: "i8"

// RUN: execute 0x42::main::i16_name
// CHECK: results: "i16"

// RUN: execute 0x42::main::i32_name
// CHECK: results: "i32"

// RUN: execute 0x42::main::i64_name
// CHECK: results: "i64"

// RUN: execute 0x42::main::i128_name
// CHECK: results: "i128"

// RUN: execute 0x42::main::i256_name
// CHECK: results: "i256"

// RUN: execute 0x42::main::struct_at_0x42
// CHECK: results: "0x42::m::S"

// RUN: execute 0x42::main::struct_at_full_address
// CHECK: results: "0xa11ce00000000000000000000000000000000000000000000000000000000b0b::far::F"

// RUN: execute 0x42::main::generic_across_addresses
// CHECK: results: "0x42::m::G<vector<0xa11ce00000000000000000000000000000000000000000000000000000000b0b::far::F>>"
