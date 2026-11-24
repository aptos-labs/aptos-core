// Function values handed to natives, and closures over natives. Checks the BCS encoding and
// size of a function value, and a `from_bytes` round trip whose result is called. MonoMove
// refuses the other steps: cmp does not order function values, string_utils reports
// `unsupported`, calling the result of `reflect::resolve` on a native fails, and a function
// containing a closure over a native fails to load even when the closure is never built.

// RUN: publish
module 0x1::test_natives {
    native public fun u64_add(left: u64, right: u64): u64;
}
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x1::string_utils {
    use std::string::String;

    public native fun native_format<T>(
        value: &T,
        type_tag: bool,
        canonicalize: bool,
        single_line: bool,
        include_int_types: bool,
    ): String;
}
module 0x42::target {
    public fun add1(value: u64): u64 {
        value + 1
    }

    public fun add(left: u64, right: u64): u64 {
        left + right
    }
}
module 0x42::m {
    use std::bcs;
    use std::cmp;
    use std::reflect;
    use std::string::{String, utf8};

    struct Holder has copy, drop, store {
        f: |u64|u64 has copy + drop + store,
    }

    fun storable(): |u64|u64 has copy + drop + store {
        0x42::target::add1
    }

    public fun to_bytes_fn(): vector<u8> {
        bcs::to_bytes(&storable())
    }

    // Captures `offset` as the first argument of a public function.
    public fun to_bytes_capturing(offset: u64): vector<u8> {
        let func: |u64|u64 has copy + drop + store = |value| 0x42::target::add(offset, value);
        bcs::to_bytes(&func)
    }

    public fun serialized_size_fn(): u64 {
        bcs::serialized_size(&storable())
    }

    public fun compare_fn(): bool {
        let func = storable();
        cmp::compare(&func, &func).is_eq()
    }

    public fun compare_holder(): bool {
        let holder = Holder { f: storable() };
        cmp::compare(&holder, &holder).is_eq()
    }

    public fun format_fn(): String {
        0x1::string_utils::native_format(&storable(), false, false, true, false)
    }

    public fun from_bytes_fn(): u64 {
        let func = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop + store>(bcs::to_bytes(&storable()));
        func(1)
    }

    public fun reflect_native(): u64 {
        let func = reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0x1, &utf8(b"test_natives"), &utf8(b"u64_add")
        ).unwrap();
        func(1, 2)
    }

    public fun closure_over_native(): u64 {
        let func: |u64, u64|u64 has copy + drop = 0x1::test_natives::u64_add;
        func(1, 2)
    }

    // A lambda that only forwards to the native compiles to a closure over it.
    public fun lambda_calls_native(): u64 {
        let func: |u64, u64|u64 has copy + drop = |left, right| 0x1::test_natives::u64_add(left, right);
        func(1, 2)
    }

    // The closure is never built at run time; MonoMove still fails to load the function.
    public fun native_closure_untaken(value: u64): u64 {
        if (value > 100) {
            let func: |u64, u64|u64 has drop = |left, right| 0x1::test_natives::u64_add(left, right);
            func(value, 1)
        } else {
            value
        }
    }
}

// RUN: execute 0x42::m::to_bytes_fn
// CHECK: results: 0x0501000000000000000000000000000000000000000000000000000000000000000042067461726765740461646431000000000000000000

// RUN: execute 0x42::m::to_bytes_capturing --args 5
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000420674617267657403616464000100000000000000020500000000000000

// RUN: execute 0x42::m::serialized_size_fn
// CHECK: results: 56

// RUN: execute 0x42::m::compare_fn
// CHECK-V1: results: true
// CHECK-V2: error: function values cannot be ordered

// RUN: execute 0x42::m::compare_holder
// CHECK-V1: results: true
// CHECK-V2: error: function values cannot be ordered

// RUN: execute 0x42::m::format_fn
// CHECK-V1: results: "0x0000000000000000000000000000000000000000000000000000000000000042::target::add1()"
// CHECK-V2: error: unsupported: function values are not yet supported

// RUN: execute 0x42::m::from_bytes_fn
// CHECK: results: 2

// RUN: execute 0x42::m::reflect_native
// CHECK-V1: results: 3
// CHECK-V2: error: Function 0x1::test_natives::u64_add is a native and cannot be loaded as code

// RUN: execute 0x42::m::closure_over_native
// CHECK-V1: results: 3
// CHECK-V2: error: failed to load function: Failed to lower function: native closure target not yet lowered

// RUN: execute 0x42::m::lambda_calls_native
// CHECK-V1: results: 3
// CHECK-V2: error: failed to load function: Failed to lower function: native closure target not yet lowered

// RUN: execute 0x42::m::native_closure_untaken --args 0
// CHECK-V1: results: 0
// CHECK-V2: error: failed to load function: Failed to lower function: native closure target not yet lowered
