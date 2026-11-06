// Differential test for the `debug` natives. The testsuite runs both VMs in
// debug mode. Their stack traces differ in detail (V1 also dumps code, locals
// and the operand stack), so tests compare only which frames a trace names.

// RUN: publish
module 0x1::debug {
    use std::string::String;

    native fun native_print(x: String);
    native fun native_stack_trace(): String;

    public fun print(x: String) {
        native_print(x)
    }

    public fun stack_trace(): String {
        native_stack_trace()
    }
}

module 0x1::main {
    use std::string::{Self, String};
    use 0x1::debug;

    fun contains(s: &String, sub: vector<u8>): bool {
        string::index_of(s, &string::utf8(sub)) < string::length(s)
    }

    public fun print_then_add(a: u64, b: u64): u64 {
        debug::print(string::utf8(b"hello"));
        a + b
    }

    public fun print_in_loop(n: u64): u64 {
        let i = 0;
        while (i < n) {
            debug::print(string::utf8(b"tick"));
            i = i + 1;
        };
        i
    }

    fun middle(): String {
        debug::stack_trace()
    }

    // The trace names every caller of `debug::stack_trace`, by its full name.
    public fun trace_names_callers(): bool {
        let trace = middle();
        contains(&trace, b"0x0000000000000000000000000000000000000000000000000000000000000001::main::middle")
            && contains(&trace, b"0x0000000000000000000000000000000000000000000000000000000000000001::main::trace_names_callers")
    }

    // The trace excludes the function calling the native.
    public fun trace_excludes_native_caller(): bool {
        let trace = middle();
        !contains(&trace, b"::debug::stack_trace")
    }
}

// RUN: execute 0x1::main::print_then_add --args 2, 3
// CHECK: results: 5

// RUN: execute 0x1::main::print_in_loop --args 3
// CHECK: results: 3

// RUN: execute 0x1::main::trace_names_callers
// CHECK: results: true

// RUN: execute 0x1::main::trace_excludes_native_caller
// CHECK: results: true
