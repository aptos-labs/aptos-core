// `reflect::resolve` signature matching: a type parameter bound inconsistently, reference
// parameters (concrete and as a type argument), a function-typed parameter, a `key`
// constraint, and a generic struct argument. Each mismatch gives 3 (incompatible type).

// RUN: publish
module 0x42::target {
    struct K has key, drop { n: u64 }
    struct Box<T> has drop { t: T }

    public fun pair<T: drop>(left: T, right: T): u64 { let _ = left; let _ = right; 1 }
    public fun by_ref(value: &u64): u64 { *value + 1 }
    public fun by_mut(value: &mut u64): u64 { *value = *value + 10; *value }
    public fun gen<T: drop>(value: T): u64 { let _ = value; 2 }
    public fun apply(func: |u64|u64 has copy + drop, value: u64): u64 { func(value) }
    public fun needs_key<T: key + drop>(value: T): u64 { let _ = value; 3 }
    public fun unbox<T: drop>(boxed: Box<T>): u64 { let Box { t: _ } = boxed; 4 }
    public fun double(value: u64): u64 { value * 2 }
    public fun mk_box(): Box<u8> { Box { t: 1 } }
    public fun mk_k(): K { K { n: 0 } }
}

module 0x42::m {
    use std::reflect;
    use std::string::utf8;
    use 0x42::target::{K, Box};

    struct NoKey has drop { n: u64 }

    public fun inconsistent_binding(): u64 {
        reflect::resolve<|u64, u8|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"pair"))
            .unwrap_err().error_code()
    }

    public fun consistent_binding(): u64 {
        let func = reflect::resolve<|u8, u8|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"pair")).unwrap();
        func(1, 2)
    }

    public fun ref_param(): u64 {
        let func = reflect::resolve<|&u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"by_ref")).unwrap();
        let value = 41;
        func(&value)
    }

    public fun mut_ref_param(): u64 {
        let func = reflect::resolve<|&mut u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"by_mut")).unwrap();
        let value = 5;
        let result = func(&mut value);
        result * 100 + value
    }

    public fun ref_vs_mut_ref(): u64 {
        reflect::resolve<|&u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"by_mut"))
            .unwrap_err().error_code()
    }

    public fun ref_as_type_arg(): u64 {
        reflect::resolve<|&u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"gen"))
            .unwrap_err().error_code()
    }

    public fun function_param(): u64 {
        let func = reflect::resolve<|(|u64|u64 has copy + drop), u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"apply")).unwrap();
        func(0x42::target::double, 21)
    }

    public fun key_satisfied(): u64 {
        let func = reflect::resolve<|K|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"needs_key")).unwrap();
        func(0x42::target::mk_k())
    }

    public fun key_missing(): u64 {
        reflect::resolve<|NoKey|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"needs_key"))
            .unwrap_err().error_code()
    }

    public fun generic_struct_arg(): u64 {
        let func = reflect::resolve<|Box<u8>|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"unbox")).unwrap();
        func(0x42::target::mk_box())
    }
}

// RUN: execute 0x42::m::inconsistent_binding
// CHECK: results: 3

// RUN: execute 0x42::m::consistent_binding
// CHECK: results: 1

// RUN: execute 0x42::m::ref_param
// CHECK: results: 42

// RUN: execute 0x42::m::mut_ref_param
// CHECK: results: 1515

// RUN: execute 0x42::m::ref_vs_mut_ref
// CHECK: results: 3

// RUN: execute 0x42::m::ref_as_type_arg
// CHECK: results: 3

// RUN: execute 0x42::m::function_param
// CHECK: results: 42

// RUN: execute 0x42::m::key_satisfied
// CHECK: results: 3

// RUN: execute 0x42::m::key_missing
// CHECK: results: 3

// RUN: execute 0x42::m::generic_struct_arg
// CHECK: results: 4
