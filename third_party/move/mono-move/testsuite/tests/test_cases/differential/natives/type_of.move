// Differential test for `type_info::type_of`.
//
// TODO: `module_name` / `struct_name` are `vector<u8>`, so they render as hex
// byte dumps below. Render them as strings once the V2 test harness has the
// `string` natives (e.g. `string::utf8`) needed to decode them.

// RUN: publish
module 0x1::type_info {
    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }
    struct Foo has drop {}
    struct Bar<phantom T> has drop {}

    public native fun type_of<T>(): TypeInfo;

    public fun foo_address(): address {
        let t = type_of<Foo>();
        t.account_address
    }

    public fun foo_module(): vector<u8> {
        let t = type_of<Foo>();
        t.module_name
    }

    public fun foo_struct(): vector<u8> {
        let t = type_of<Foo>();
        t.struct_name
    }

    // `struct_name` must carry the generic instantiation, not just `Bar`.
    public fun bar_struct(): vector<u8> {
        let t = type_of<Bar<u64>>();
        t.struct_name
    }

    // Aborts on a non-struct type, matching the legacy VM's code and message.
    public fun non_struct_aborts(): address {
        let t = type_of<u64>();
        t.account_address
    }
}

// RUN: execute 0x1::type_info::foo_address
// CHECK: results: 0x1

// RUN: execute 0x1::type_info::foo_module
// CHECK: results: 0x747970655f696e666f

// RUN: execute 0x1::type_info::foo_struct
// CHECK: results: 0x466f6f

// RUN: execute 0x1::type_info::bar_struct
// CHECK: results: 0x4261723c7536343e

// RUN: execute 0x1::type_info::non_struct_aborts
// CHECK: aborted: code 1 (Expected a struct type, found: u64)
