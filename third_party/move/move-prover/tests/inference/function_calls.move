// Test spec inference for function calls and closures using behavioral predicates
module 0x42::function_calls {

    // Helper function with spec for testing behavioral predicates
    fun callee(x: u64): u64 {
        x + 1
    }
    spec callee {
        ensures result == x + 1;
        aborts_if x == 18446744073709551615; // MAX_U64
    }

    // Direct function call - should infer using ensures_of/aborts_of
    fun test_call(x: u64): u64 {
        callee(x)
    }

    // Chained function calls
    fun test_call_chain(x: u64): u64 {
        callee(callee(x))
    }

    // Function call result stored in variable
    fun test_call_assign(x: u64): u64 {
        let y = callee(x);
        y
    }

    // Higher-order function with closure invocation
    fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    // Function that uses apply with callee
    fun test_higher_order(x: u64): u64 {
        apply(|v| callee(v), x)
    }

    // ==================== Inference Pragma ====================

    // Test pragma inference = "none" to explicitly disable inference
    fun no_inference(x: u64): u64 {
        x + 1
    }
    spec no_inference {
        pragma inference = none;
    }

    // ==================== Recursive Functions ====================

    // Recursive functions need pragma opaque for verification, but inference
    // should still run on them (opaque no longer skips inference).
    fun factorial(n: u64): u64 {
        if (n == 0) {
            1
        } else {
            n * factorial(n - 1)
        }
    }
    spec factorial {
        pragma opaque;
    }

    // Caller of recursive function - should infer using behavioral predicates
    fun test_factorial_call(n: u64): u64 {
        factorial(n)
    }

    // Mutual recursion: is_even / is_odd
    fun is_even(n: u64): bool {
        if (n == 0) {
            true
        } else {
            is_odd(n - 1)
        }
    }
    spec is_even {
        pragma opaque;
    }

    fun is_odd(n: u64): bool {
        if (n == 0) {
            false
        } else {
            is_even(n - 1)
        }
    }
    spec is_odd {
        pragma opaque;
    }

    // Caller of mutually recursive functions
    fun test_parity(n: u64): bool {
        if (is_even(n)) {
            !is_odd(n)
        } else {
            is_odd(n)
        }
    }
}
// TODO(#18762): opaque recursive functions produce expected boogie errors
// in the verification step because Boogie doesn't generate procedure bodies for them.
