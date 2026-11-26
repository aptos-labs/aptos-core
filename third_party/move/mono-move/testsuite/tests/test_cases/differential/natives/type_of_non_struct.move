// `type_info::type_of` on non-struct types (a vector of a generic struct, a vector of a
// struct at a full-length address, a function type, signer) aborts with a message naming
// only the outer kind. An enum is accepted like a struct.

// RUN: publish
module 0x1::type_info {
    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }

    public native fun type_of<T>(): TypeInfo;

    public fun parts(info: &TypeInfo): (address, vector<u8>, vector<u8>) {
        (info.account_address, info.module_name, info.struct_name)
    }
}
module 0xcafe::m {
    struct S has drop {}

    struct G<phantom T> has drop {}

    enum E has drop { A, B { x: u64 } }

    public fun vector_of_generic() {
        0x1::type_info::type_of<vector<G<vector<u8>>>>();
    }

    public fun function_type() {
        0x1::type_info::type_of<|S, u8|G<u64> has copy + drop>();
    }

    public fun signer_type() {
        0x1::type_info::type_of<signer>();
    }

    public fun enum_type(): (address, vector<u8>, vector<u8>) {
        0x1::type_info::parts(&0x1::type_info::type_of<E>())
    }

    public fun long_address_struct() {
        0x1::type_info::type_of<vector<0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef::n::T>>();
    }
}
module 0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef::n {
    struct T has drop {}
}

// RUN: execute 0xcafe::m::vector_of_generic
// CHECK: aborted: code 1 (Expected a struct type, found: vector) in 0x1::type_info

// RUN: execute 0xcafe::m::function_type
// CHECK: aborted: code 1 (Expected a struct type, found: function) in 0x1::type_info

// RUN: execute 0xcafe::m::signer_type
// CHECK: aborted: code 1 (Expected a struct type, found: signer) in 0x1::type_info

// RUN: execute 0xcafe::m::enum_type
// CHECK: results: 0xcafe, 0x6d, 0x45

// RUN: execute 0xcafe::m::long_address_struct
// CHECK: aborted: code 1 (Expected a struct type, found: vector) in 0x1::type_info
