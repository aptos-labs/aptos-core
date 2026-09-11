// Differential test for `type_info::type_of`.

// RUN: publish
module 0x1::type_info {
    use std::string::{Self, String};

    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }
    struct Foo has drop {}
    struct Bar<phantom T> has drop {}
    struct Pair<phantom T, phantom U> has drop {}

    public native fun type_of<T>(): TypeInfo;

    public fun foo_address(): address {
        let info = type_of<Foo>();
        info.account_address
    }

    public fun foo_module(): String {
        let info = type_of<Foo>();
        string::utf8(info.module_name)
    }

    public fun foo_struct(): String {
        let info = type_of<Foo>();
        string::utf8(info.struct_name)
    }

    // `struct_name` must carry the generic instantiation, not just `Bar`.
    public fun bar_struct(): String {
        let info = type_of<Bar<u64>>();
        string::utf8(info.struct_name)
    }

    // Struct type arguments are fully qualified; type arguments are comma separated.
    public fun pair_struct(): String {
        let info = type_of<Pair<u64, Bar<u8>>>();
        string::utf8(info.struct_name)
    }

    // Function type arguments render canonically, with parenthesized results.
    public fun bar_function_struct(): String {
        let info = type_of<Bar<|u8|(u8, u16) has copy + drop>>();
        string::utf8(info.struct_name)
    }

    // Aborts on a non-struct type, matching the legacy VM's code and message.
    public fun non_struct_aborts(): address {
        let info = type_of<u64>();
        info.account_address
    }
}

// RUN: execute 0x1::type_info::foo_address
// CHECK: results: 0x1

// RUN: execute 0x1::type_info::foo_module
// CHECK: results: "type_info"

// RUN: execute 0x1::type_info::foo_struct
// CHECK: results: "Foo"

// RUN: execute 0x1::type_info::bar_struct
// CHECK: results: "Bar<u64>"

// RUN: execute 0x1::type_info::pair_struct
// CHECK: results: "Pair<u64, 0x1::type_info::Bar<u8>>"

// RUN: execute 0x1::type_info::bar_function_struct
// CHECK: results: "Bar<|u8|(u8, u16) has copy + drop>"

// RUN: execute 0x1::type_info::non_struct_aborts
// CHECK: aborted: code 1 (Expected a struct type, found: u64) in 0x1::type_info
