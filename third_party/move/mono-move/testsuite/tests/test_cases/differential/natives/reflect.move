// RUN: publish
module 0x42::target {
    public fun add(x: u64, y: u64): u64 {
        x + y
    }

    public fun generic<T: drop>(x: T, y: u64): u64 {
        let _ = x;
        y + 1
    }

    // `U` appears in no parameter or return type, so it cannot be inferred.
    public fun phantom_generic<T: drop, U>(x: T): u64 {
        let _ = x;
        7
    }

    public fun needs_store<T: drop + store>(x: T): u64 {
        let _ = x;
        8
    }

    fun private_add(x: u64, y: u64): u64 {
        x + y
    }

    public fun call_private(x: u64): u64 {
        private_add(x, 1)
    }
}

module 0x42::m {
    use std::reflect;
    use std::string::utf8;

    struct NoStore has copy, drop {}

    public fun resolve_and_call(x: u64, y: u64): u64 {
        let f = reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"add")
        ).unwrap();
        f(x, y)
    }

    public fun resolve_generic_and_call(x: u64): u64 {
        let f = reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"generic")
        ).unwrap();
        f(x, 7)
    }

    public fun bad_module_name(): u64 {
        reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"not a module"), &utf8(b"add")
        ).unwrap_err().error_code()
    }

    public fun bad_function_name(): u64 {
        reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"1add")
        ).unwrap_err().error_code()
    }

    public fun missing_module(): u64 {
        reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"missing"), &utf8(b"add")
        ).unwrap_err().error_code()
    }

    public fun missing_function(): u64 {
        reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"missing")
        ).unwrap_err().error_code()
    }

    public fun private_function(): u64 {
        reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"private_add")
        ).unwrap_err().error_code()
    }

    public fun missing_module_twice(): u64 {
        let first = reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"missing"), &utf8(b"add")
        ).unwrap_err().error_code();
        let second = reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x42, &utf8(b"missing"), &utf8(b"add")
        ).unwrap_err().error_code();
        first + second
    }

    public fun forbidden_event_emit(): u64 {
        reflect::resolve<|u64|>(
            @0x1, &utf8(b"event"), &utf8(b"emit")
        ).unwrap_err().error_code()
    }

    public fun forbidden_init(): u64 {
        reflect::resolve<|u64|>(
            @0x1, &utf8(b"init"), &utf8(b"internal_maybe_initialize")
        ).unwrap_err().error_code()
    }

    public fun wrong_arity(): u64 {
        reflect::resolve<|u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"add")
        ).unwrap_err().error_code()
    }

    public fun wrong_param_type(): u64 {
        reflect::resolve<|u64, u8|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"add")
        ).unwrap_err().error_code()
    }

    public fun ability_not_satisfied(): u64 {
        reflect::resolve<|NoStore|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"needs_store")
        ).unwrap_err().error_code()
    }

    public fun not_instantiated(): u64 {
        reflect::resolve<|u64|u64 has copy + drop>(
            @0x42, &utf8(b"target"), &utf8(b"phantom_generic")
        ).unwrap_err().error_code()
    }
}

// RUN: execute 0x42::m::resolve_and_call --args 40, 2
// CHECK: results: 42

// RUN: execute 0x42::m::resolve_generic_and_call --args 1
// CHECK: results: 8

// RUN: execute 0x42::m::bad_module_name
// CHECK: results: 0

// RUN: execute 0x42::m::bad_function_name
// CHECK: results: 0

// RUN: execute 0x42::m::missing_module
// CHECK: results: 1

// RUN: execute 0x42::m::missing_function
// CHECK: results: 1

// RUN: execute 0x42::m::missing_module_twice
// CHECK: results: 2

// RUN: execute 0x42::m::private_function
// CHECK: results: 2

// RUN: execute 0x42::m::forbidden_event_emit
// CHECK: results: 2

// RUN: execute 0x42::m::forbidden_init
// CHECK: results: 2

// RUN: execute 0x42::m::wrong_arity
// CHECK: results: 3

// RUN: execute 0x42::m::wrong_param_type
// CHECK: results: 3

// RUN: execute 0x42::m::ability_not_satisfied
// CHECK: results: 3

// RUN: execute 0x42::m::not_instantiated
// CHECK: results: 4
