// `reflect::resolve` checking ability constraints on structs whose modules nothing else in
// the transaction uses: the structs appear only in the expected function type, and no code
// of their modules runs. Covered: `key` on a plain struct with and without it; `key` on a
// generic struct, which needs `store` of a non-phantom argument and nothing of a phantom
// one; and `store + drop` on vectors of elements with and without `store`. Each unmet
// constraint gives 3 (incompatible type).

// RUN: publish
module 0x42::keyed {
    struct K has key, drop { count: u64 }
}

module 0x42::unkeyed {
    struct NoKey has copy, drop, store { count: u64 }
}

module 0x42::holders {
    struct Holder<T> has key, drop { item: T }
    struct Tagged<phantom T> has key, drop { count: u64 }
}

module 0x42::leaves {
    struct StoreDrop has store, drop { count: u64 }
    struct DropOnly has drop { count: u64 }
}

module 0x42::target {
    public fun needs_key<T: key + drop>(value: T): u64 { let _ = value; 1 }
    public fun needs_store<T: store + drop>(value: T): u64 { let _ = value; 1 }
}

module 0x42::resolver {
    use std::reflect;
    use std::string::utf8;
    use 0x42::holders::{Holder, Tagged};
    use 0x42::leaves::{StoreDrop, DropOnly};

    public fun key_satisfied(): u64 {
        let func = reflect::resolve<|0x42::keyed::K|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"needs_key")).unwrap();
        let _ = func;
        1
    }

    public fun key_missing(): u64 {
        reflect::resolve<|0x42::unkeyed::NoKey|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"needs_key")).unwrap_err().error_code()
    }

    // `Holder<T>` has `key` when T has `store`.
    public fun key_from_store_arg(): u64 {
        let func = reflect::resolve<|Holder<StoreDrop>|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"needs_key")).unwrap();
        let _ = func;
        1
    }

    public fun key_without_store_arg(): u64 {
        reflect::resolve<|Holder<DropOnly>|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"needs_key")).unwrap_err().error_code()
    }

    // The same argument in a phantom position imposes nothing.
    public fun key_phantom_arg(): u64 {
        let func = reflect::resolve<|Tagged<DropOnly>|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"needs_key")).unwrap();
        let _ = func;
        1
    }

    public fun store_on_vector(): u64 {
        let func = reflect::resolve<|vector<StoreDrop>|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"needs_store")).unwrap();
        let _ = func;
        1
    }

    public fun store_missing_on_vector(): u64 {
        reflect::resolve<|vector<DropOnly>|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"needs_store")).unwrap_err().error_code()
    }
}

// RUN: execute 0x42::resolver::key_satisfied
// CHECK: results: 1

// RUN: execute 0x42::resolver::key_missing
// CHECK: results: 3

// RUN: execute 0x42::resolver::key_from_store_arg
// CHECK: results: 1

// RUN: execute 0x42::resolver::key_without_store_arg
// CHECK: results: 3

// RUN: execute 0x42::resolver::key_phantom_arg
// CHECK: results: 1

// RUN: execute 0x42::resolver::store_on_vector
// CHECK: results: 1

// RUN: execute 0x42::resolver::store_missing_on_vector
// CHECK: results: 3
