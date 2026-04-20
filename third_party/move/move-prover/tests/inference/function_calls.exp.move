// Test spec inference for function calls and closures using behavioral predicates
module 0x42::function_calls {

    // Helper function with spec for testing behavioral predicates
    fun callee(x: u64): u64 {
        x + 1
    }
    spec callee {
        ensures result == x + 1;
        aborts_if x == 18446744073709551615; // MAX_U64
        pragma opaque = true;
        ensures [inferred] x != 18446744073709551615 ==> result == x + 1;
        aborts_if [inferred] x == MAX_U64;
    }

    // Direct function call - should infer using ensures_of/aborts_of
    fun test_call(x: u64): u64 {
        callee(x)
    }
    spec test_call(x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == callee(x);
        aborts_if [inferred] aborts_of<callee>(x);
    }


    // Chained function calls
    fun test_call_chain(x: u64): u64 {
        callee(callee(x))
    }
    spec test_call_chain(x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == callee(callee(x));
        aborts_if [inferred] aborts_of<callee>(callee(x));
        aborts_if [inferred] aborts_of<callee>(x);
    }


    // Function call result stored in variable
    fun test_call_assign(x: u64): u64 {
        let y = callee(x);
        y
    }
    spec test_call_assign(x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == callee(x);
        aborts_if [inferred] aborts_of<callee>(x);
    }


    // Higher-order function with closure invocation
    fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply(f: |u64|u64, x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == result_of<f>(x);
        aborts_if [inferred] aborts_of<f>(x);
    }


    // Function that uses apply with callee
    fun test_higher_order(x: u64): u64 {
        apply(|v| callee(v), x)
    }
    spec test_higher_order(x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == result_of<apply>(|x| callee(x), x);
        aborts_if [inferred] aborts_of<apply>(|x| callee(x), x);
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
        ensures [inferred] n == 0 ==> result == 1;
        ensures [inferred] n != 0 ==> result == n * factorial(n - 1);
        aborts_if [inferred] n != 0 && n * factorial(n - 1) > MAX_U64;
        aborts_if [inferred] n != 0 && aborts_of<factorial>(n - 1);
        aborts_if [inferred] n != 0 && n == 0;
    }

    // Caller of recursive function - should infer using behavioral predicates
    fun test_factorial_call(n: u64): u64 {
        factorial(n)
    }
    spec test_factorial_call(n: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == factorial(n);
        aborts_if [inferred] aborts_of<factorial>(n);
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
        ensures [inferred] n == 0 ==> result == true;
        ensures [inferred] n != 0 ==> result == is_odd(n - 1);
        aborts_if [inferred] n != 0 && aborts_of<is_odd>(n - 1);
        aborts_if [inferred] n != 0 && n == 0;
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
        ensures [inferred] n == 0 ==> result == false;
        ensures [inferred] n != 0 ==> result == is_even(n - 1);
        aborts_if [inferred] n != 0 && aborts_of<is_even>(n - 1);
        aborts_if [inferred] n != 0 && n == 0;
    }

    // Caller of mutually recursive functions
    fun test_parity(n: u64): bool {
        if (is_even(n)) {
            !is_odd(n)
        } else {
            is_odd(n)
        }
    }
    spec test_parity(n: u64): bool {
        pragma opaque = true;
        ensures [inferred] is_even(n) ==> result == !is_odd(n);
        ensures [inferred] !is_even(n) ==> result == is_odd(n);
        aborts_if [inferred] aborts_of<is_odd>(n);
        aborts_if [inferred] aborts_of<is_even>(n);
    }

}
/*
Verification: Succeeded.
*/
