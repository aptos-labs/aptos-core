// `reflect::resolve` against generic targets whose type parameter sits behind `&`, behind
// `&mut` or inside a function-typed parameter: the binding is read from the referent and
// from the function type's arguments and result. A mismatch in reference kind, function
// abilities or shape, or a binding there that disagrees with the rest of the signature,
// gives 3 (incompatible type). Each failing step has one position that disagrees.

// RUN: publish
module 0x42::target {
    public fun peek<T: copy + drop>(value: &T): T { *value }
    public fun poke<T: drop>(value: &mut T, next: T): u64 { *value = next; 5 }
    public fun apply<T>(func: |T|T has copy + drop, value: T): T { func(value) }
    public fun double(value: u64): u64 { value * 2 }
}

module 0x42::resolver {
    use std::reflect;
    use std::string::utf8;

    public fun ref_binds(): u64 {
        let func = reflect::resolve<|&u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"peek")).unwrap();
        let value = 41;
        func(&value)
    }

    public fun ref_binds_u8(): u8 {
        let func = reflect::resolve<|&u8|u8 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"peek")).unwrap();
        let value = 7;
        func(&value)
    }

    // `&T` against `&mut u64`.
    public fun ref_vs_mut(): u64 {
        reflect::resolve<|&mut u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"peek"))
            .unwrap_err().error_code()
    }

    // `&T` against a value type.
    public fun ref_vs_value(): u64 {
        reflect::resolve<|u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"peek"))
            .unwrap_err().error_code()
    }

    // T is bound to u64 behind `&` and then to u8 by the result.
    public fun ref_inconsistent(): u64 {
        reflect::resolve<|&u64|u8 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"peek"))
            .unwrap_err().error_code()
    }

    public fun mut_ref_binds(): u64 {
        let func = reflect::resolve<|&mut u64, u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"poke")).unwrap();
        let value = 3;
        let result = func(&mut value, 90);
        result * 1000 + value
    }

    // T is bound to u8 behind `&mut` and then to u64 by the second argument.
    public fun mut_ref_inconsistent(): u64 {
        reflect::resolve<|&mut u8, u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"poke"))
            .unwrap_err().error_code()
    }

    public fun fn_binds(): u64 {
        let func = reflect::resolve<|(|u64|u64 has copy + drop), u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"apply")).unwrap();
        func(0x42::target::double, 21)
    }

    // A function-typed parameter against a value type.
    public fun fn_vs_value(): u64 {
        reflect::resolve<|u64, u64|u64 has copy + drop>(@0x42, &utf8(b"target"), &utf8(b"apply"))
            .unwrap_err().error_code()
    }

    // The parameter's function type has `copy + drop`, the expected one only `copy`.
    public fun fn_abilities_differ(): u64 {
        reflect::resolve<|(|u64|u64 has copy), u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"apply")).unwrap_err().error_code()
    }

    // Only the function's argument disagrees: T is u8 there and u64 everywhere else.
    public fun fn_arg_inconsistent(): u64 {
        reflect::resolve<|(|u8|u64 has copy + drop), u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"apply")).unwrap_err().error_code()
    }

    // Only the function's result disagrees: T is u64 there and u8 everywhere else.
    public fun fn_result_inconsistent(): u64 {
        reflect::resolve<|(|u8|u64 has copy + drop), u8|u8 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"apply")).unwrap_err().error_code()
    }
}

// RUN: execute 0x42::resolver::ref_binds
// CHECK: results: 41

// RUN: execute 0x42::resolver::ref_binds_u8
// CHECK: results: 7

// RUN: execute 0x42::resolver::ref_vs_mut
// CHECK: results: 3

// RUN: execute 0x42::resolver::ref_vs_value
// CHECK: results: 3

// RUN: execute 0x42::resolver::ref_inconsistent
// CHECK: results: 3

// RUN: execute 0x42::resolver::mut_ref_binds
// CHECK: results: 5090

// RUN: execute 0x42::resolver::mut_ref_inconsistent
// CHECK: results: 3

// RUN: execute 0x42::resolver::fn_binds
// CHECK: results: 42

// RUN: execute 0x42::resolver::fn_vs_value
// CHECK: results: 3

// RUN: execute 0x42::resolver::fn_abilities_differ
// CHECK: results: 3

// RUN: execute 0x42::resolver::fn_arg_inconsistent
// CHECK: results: 3

// RUN: execute 0x42::resolver::fn_result_inconsistent
// CHECK: results: 3
