// Comprehensive tests for behavioral predicates in the Move Prover.
// Tests how specifications of function parameters via behavioral predicates
// affect the specification of the enclosing function.
//
// Key concepts:
// - ensures_of<f>(args, result) constrains the postcondition of f
// - aborts_of<f>(args) constrains the abort conditions of f
// - These behavioral predicates allow reasoning about higher-order functions
// - requires_of<f>(args) constrains the precondition of f

module 0x42::behavioral_predicates {

    // =========================================================================
    // Ensures_of with lambdas
    // =========================================================================

    /// Basic higher-order function with spec using ensures_of
    fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        ensures ensures_of<f>(x, result);
    }

    /// Test: ensures_of propagates postcondition from lambda
    fun test_ensures_of_basic(x: u64): u64 {
        apply(|y| y + 5, x)
    }
    spec test_ensures_of_basic {
        ensures result == x + 5;
    }

    /// Test: ensures_of fails with wrong postcondition
    fun test_ensures_of_fail(x: u64): u64 {
        apply(|y| y + 5, x)
    }
    spec test_ensures_of_fail {
        ensures result == x + 10; // error: post-condition does not hold
    }

    // =========================================================================
    // Multiple arguments to function parameter
    // =========================================================================

    /// Higher-order function where f takes two arguments
    fun apply2(f: |u64, u64| u64, x: u64, y: u64): u64 {
        f(x, y)
    }
    spec apply2 {
        ensures ensures_of<f>(x, y, result);
    }

    /// Test with binary operation - addition
    fun test_apply2_add(x: u64, y: u64): u64 {
        apply2(|a, b| a + b, x, y)
    }
    spec test_apply2_add {
        ensures result == x + y;
    }

    /// Test with binary operation - multiplication
    fun test_apply2_mul(x: u64, y: u64): u64 {
        apply2(|a, b| a * b, x, y)
    }
    spec test_apply2_mul {
        ensures result == x * y;
    }

    // =========================================================================
    // Generic higher-order functions
    // =========================================================================

    /// Generic map function
    fun map<T, R>(f: |T| R, x: T): R {
        f(x)
    }
    spec map {
        ensures ensures_of<f>(x, result);
    }

    /// Test: generic map with u64 -> u64
    fun test_generic_map_u64(x: u64): u64 {
        map(|y: u64| y * 3, x)
    }
    spec test_generic_map_u64 {
        ensures result == x * 3;
    }

    /// Test: generic map with bool -> u64
    fun test_generic_map_bool_to_u64(b: bool): u64 {
        map(|flag: bool| if (flag) 1 else 0, b)
    }
    spec test_generic_map_bool_to_u64 {
        ensures b ==> result == 1;
        ensures !b ==> result == 0;
    }

    /// Generic binary apply
    fun apply2_generic<T1, T2, R>(f: |T1, T2| R, x: T1, y: T2): R {
        f(x, y)
    }
    spec apply2_generic {
        ensures ensures_of<f>(x, y, result);
    }

    /// Test generic binary with different types
    fun test_apply2_generic_mixed(x: u64, b: bool): u64 {
        apply2_generic(|n: u64, flag: bool| if (flag) n else 0, x, b)
    }
    spec test_apply2_generic_mixed {
        ensures b ==> result == x;
        ensures !b ==> result == 0;
    }

    // =========================================================================
    // Chained applications with lambdas
    // =========================================================================

    /// Test chained application directly with lambdas
    fun test_chained_application_ok(x: u64): u64 {
        apply(|y| y + 1, apply(|z| z + 2, x))
    }
    spec test_chained_application_ok {
        ensures result == x + 3;
    }

    /// Test nested apply calls
    fun test_nested_apply_ok(x: u64): u64 {
        apply(|y| y * 2, apply(|z| z + 5, x))
    }
    spec test_nested_apply_ok {
        ensures result == (x + 5) * 2;
    }

    // =========================================================================
    // Functions returning functions (currying)
    // =========================================================================

    /// Curried addition - returns a function
    fun make_adder(n: u64): |u64| u64 {
        |x| x + n
    }

    /// Test currying pattern
    fun test_currying_ok(x: u64): u64 {
        let add5 = make_adder(5);
        apply(add5, x)
    }
    spec test_currying_ok {
        ensures result == x + 5;
    }

    /// Test currying with different values
    fun test_currying_10(x: u64): u64 {
        let add10 = make_adder(10);
        apply(add10, x)
    }
    spec test_currying_10 {
        ensures result == x + 10;
    }

    // =========================================================================
    // Identity and constant lambdas
    // =========================================================================

    /// Test with identity lambda
    fun test_identity(x: u64): u64 {
        apply(|y| y, x)
    }
    spec test_identity {
        ensures result == x;
    }

    /// Test with constant lambda
    fun test_constant(x: u64): u64 {
        apply(|_y| 42, x)
    }
    spec test_constant {
        ensures result == 42;
    }

    /// Test with a more complex lambda expression
    fun test_complex_lambda(x: u64, y: u64): u64 {
        apply(|z| if (z > y) z - y else y - z, x)
    }
    spec test_complex_lambda {
        ensures x > y ==> result == x - y;
        ensures x <= y ==> result == y - x;
    }

    // =========================================================================
    // Known function targets
    // =========================================================================

    /// Helper function with spec
    fun double(x: u64): u64 {
        x * 2
    }
    spec double {
        ensures result == x * 2;
    }

    /// Test using ensures_of with known function target
    fun test_known_function_ensures(x: u64): u64 {
        double(x)
    }
    spec test_known_function_ensures {
        // ensures_of<double> reduces to: result == x * 2
        ensures ensures_of<double>(x, result);
    }

    /// Another helper function
    fun increment(x: u64): u64 {
        x + 1
    }
    spec increment {
        ensures result == x + 1;
    }

    /// Test with increment
    fun test_known_increment(x: u64): u64 {
        increment(x)
    }
    spec test_known_increment {
        ensures ensures_of<increment>(x, result);
    }

    // =========================================================================
    // Negative tests (expected failures)
    // =========================================================================

    /// Test: Wrong ensures postcondition
    fun test_wrong_ensures(x: u64): u64 {
        apply(|y| y + 1, x)
    }
    spec test_wrong_ensures {
        ensures result == x + 2; // error: post-condition does not hold
    }

    /// Test: Postcondition too strong
    fun test_postcondition_too_strong(x: u64): u64 {
        apply(|y| y + 1, x)
    }
    spec test_postcondition_too_strong {
        ensures result == x + 1;
        ensures result > 100; // error: post-condition does not hold
    }

    /// Test: Wrong binary result
    fun test_wrong_binary(x: u64, y: u64): u64 {
        apply2(|a, b| a + b, x, y)
    }
    spec test_wrong_binary {
        ensures result == x * y; // error: should be x + y
    }

    // =========================================================================
    // Aborts_of with lambdas
    // =========================================================================

    /// Higher-order function with abort specification
    fun apply_may_abort(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_may_abort {
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    /// Test: lambda that may abort
    fun test_may_abort(x: u64): u64 {
        apply_may_abort(|y| if (y == 0) abort 1 else y, x)
    }
    spec test_may_abort {
        aborts_if x == 0;
        ensures result == x;
    }

    /// Test: lambda that aborts on condition
    fun test_may_abort_on_large(x: u64): u64 {
        apply_may_abort(|y| if (y > 1000) abort 1 else y + 10, x)
    }
    spec test_may_abort_on_large {
        aborts_if x > 1000;
        ensures result == x + 10;
    }

    // =========================================================================
    // Combined ensures_of and aborts_of
    // =========================================================================

    /// Function with both aborts_of and ensures_of
    fun guarded_apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec guarded_apply {
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    /// Test: lambda with both abort and post condition
    fun test_guarded_apply(x: u64): u64 {
        guarded_apply(|y| {
            if (y > 500) abort 1;
            y * 2
        }, x)
    }
    spec test_guarded_apply {
        aborts_if x > 500;
        ensures result == x * 2;
    }

    // =========================================================================
    // Lambda with captured variables
    // =========================================================================

    /// Test lambda capturing a local variable
    fun test_captured_var(x: u64, offset: u64): u64 {
        apply(|y| y + offset, x)
    }
    spec test_captured_var {
        ensures result == x + offset;
    }

    /// Test lambda capturing multiple variables
    fun test_captured_multiple(x: u64, a: u64, b: u64): u64 {
        apply(|y| y + a + b, x)
    }
    spec test_captured_multiple {
        ensures result == x + a + b;
    }

    // =========================================================================
    // Requires_of with lambdas
    // =========================================================================

    /// Higher-order function with requires_of specification
    fun apply_with_requires(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_with_requires {
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    /// Test: lambda with no precondition (requires_of reduces to true)
    fun test_requires_of_trivial(x: u64): u64 {
        apply_with_requires(|y| y + 1, x)
    }
    spec test_requires_of_trivial {
        ensures result == x + 1;
    }

    /// Test: lambda with multiplication (no precondition)
    fun test_requires_of_mul(x: u64): u64 {
        apply_with_requires(|y| y * 2, x)
    }
    spec test_requires_of_mul {
        ensures result == x * 2;
    }

    /// Test: lambda with conditional (no precondition from the condition itself)
    fun test_requires_of_conditional(x: u64): u64 {
        apply_with_requires(|y| if (y > 10) y - 5 else y + 5, x)
    }
    spec test_requires_of_conditional {
        ensures x > 10 ==> result == x - 5;
        ensures x <= 10 ==> result == x + 5;
    }

    // =========================================================================
    // Chained and nested requires_of
    // =========================================================================

    /// Test: lambda with abort condition via guarded_apply (has requires_of in spec)
    fun test_requires_via_guarded(x: u64): u64 {
        guarded_apply(|y| {
            if (y > 100) abort 1;
            y * 2
        }, x)
    }
    spec test_requires_via_guarded {
        aborts_if x > 100;
        ensures result == x * 2;
    }

    /// Test: chained application with requires_of
    fun test_requires_chain(x: u64): u64 {
        apply_with_requires(|y| apply_with_requires(|z| z + 1, y), x)
    }
    spec test_requires_chain {
        ensures result == x + 1;
    }

    /// Test: deeply nested lambdas with requires_of
    fun test_requires_deep_nest(x: u64): u64 {
        apply_with_requires(|a|
            apply_with_requires(|b|
                apply_with_requires(|c| c * 2, b), a), x)
    }
    spec test_requires_deep_nest {
        ensures result == x * 2;
    }
}
