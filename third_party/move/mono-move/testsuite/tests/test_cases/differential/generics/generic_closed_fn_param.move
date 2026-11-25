// A generic function with a parameter of function type `|u64|u64`, which does not involve
// its type parameter, called with two different type arguments.

// RUN: publish
module 0x42::generic_closed_fn_param {
    fun apply<T: drop>(func: |u64|u64 has drop, ignored: T, arg: u64): u64 {
        let _ = ignored;
        func(arg)
    }

    fun twice(value: u64): u64 { value * 2 }

    public fun run_u8(): u64 { apply(|value| twice(value), 1u8, 20) }

    public fun run_vec(): u64 { apply(|value| value * 3 + 1, vector[1u64], 41) }
}

// RUN: execute 0x42::generic_closed_fn_param::run_u8
// CHECK: results: 40

// RUN: execute 0x42::generic_closed_fn_param::run_vec
// CHECK: results: 124
