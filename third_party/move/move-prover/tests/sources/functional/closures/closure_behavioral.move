// Tests for behavioral predicate code generation in the Move Prover.
// This verifies that behavioral predicates compile correctly and the Boogie
// datatype variants for function-typed parameters are generated.

module 0x42::behavioral {

    // A simple higher-order function.
    fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    // Test with a lambda - verification works because lambda is inlined
    fun test_apply_lambda_ok(x: u64): u64 {
        apply(|y| y + 3, x)
    }
    spec test_apply_lambda_ok {
        ensures result == x + 3;
    }

    // Test with incorrect spec - should fail
    fun test_apply_lambda_fail(x: u64): u64 {
        apply(|y| y + 3, x)
    }
    spec test_apply_lambda_fail {
        // This should fail because the lambda returns y + 3, not y + 5
        ensures result == x + 5; // error: post-condition does not hold
    }
}
