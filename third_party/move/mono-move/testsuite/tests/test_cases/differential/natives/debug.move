// Differential test for the `debug` natives. The testsuite enables them on
// both VMs: MonoMove in debug mode, V1 via its `testing` cargo feature. Their
// stack traces differ in detail (V1 also dumps code, locals
// and the operand stack), so tests compare only which frames a trace names,
// and in what order.

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

module 0x1::chain {
    use std::string::String;
    use 0x1::debug;

    public fun outer(): String {
        middle()
    }

    fun middle(): String {
        inner()
    }

    fun inner(): String {
        debug::stack_trace()
    }
}

module 0x1::main {
    use std::string::{Self, String};
    use 0x1::chain;
    use 0x1::debug;

    // Byte index of the first `sub` in `s`, or `s`'s length if absent.
    fun find(s: &String, sub: vector<u8>): u64 {
        string::index_of(s, &string::utf8(sub))
    }

    fun contains(s: &String, sub: vector<u8>): bool {
        find(s, sub) < string::length(s)
    }

    // Number of non-overlapping occurrences of `sub` in `s`.
    fun count(s: &String, sub: vector<u8>): u64 {
        let sub = string::utf8(sub);
        let rest = *s;
        let n = 0;
        loop {
            let i = string::index_of(&rest, &sub);
            if (i == string::length(&rest)) break;
            n = n + 1;
            rest = string::sub_string(&rest, i + string::length(&sub), string::length(&rest));
        };
        n
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

    // A chain across modules lists every frame, outermost first.
    public fun trace_orders_callers(): bool {
        let trace = chain::outer();
        let entry = find(&trace, b"::main::trace_orders_callers");
        let outer = find(&trace, b"::chain::outer");
        let middle = find(&trace, b"::chain::middle");
        let inner = find(&trace, b"::chain::inner");
        entry < outer && outer < middle && middle < inner && inner < string::length(&trace)
    }

    fun rec(n: u64): String {
        if (n == 0) debug::stack_trace() else rec(n - 1)
    }

    // Each recursive call is its own frame: `rec(n)` down to `rec(0)`.
    public fun trace_counts_recursion(n: u64): u64 {
        count(&rec(n), b"::main::rec")
    }

    fun generic_caller<T>(): String {
        middle()
    }

    // A generic instantiation is named by its function.
    public fun trace_names_generic_caller(): bool {
        let trace = generic_caller<u64>();
        contains(&trace, b"::main::generic_caller") && contains(&trace, b"::main::middle")
    }

    // A call through a function value keeps the frames on both sides of it.
    public fun trace_through_closure(): bool {
        let f: || String has drop = || middle();
        let trace = f();
        contains(&trace, b"::main::trace_through_closure") && contains(&trace, b"::main::middle")
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

// RUN: execute 0x1::main::trace_orders_callers
// CHECK: results: true

// RUN: execute 0x1::main::trace_counts_recursion --args 0
// CHECK: results: 1

// RUN: execute 0x1::main::trace_counts_recursion --args 5
// CHECK: results: 6

// RUN: execute 0x1::main::trace_names_generic_caller
// CHECK: results: true

// RUN: execute 0x1::main::trace_through_closure
// CHECK: results: true
