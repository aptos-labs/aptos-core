// Tests for behavior predicate type checking errors.

module 0x42::M {

    // Helper functions
    fun unary(x: u64): u64 { x }
    fun binary(a: u64, b: u64): u64 { a + b }

    // Generic helper function
    fun generic_id<T>(x: T): T { x }

    // ========================================
    // Error: Wrong argument count for requires_of
    // ========================================

    fun test_requires_wrong_count(a: u64, b: u64): u64 {
        binary(a, b)
    }

    spec test_requires_wrong_count {
        // Error: binary takes 2 args, but only 1 provided
        ensures requires_of<binary>(a);
    }

    // ========================================
    // Error: Wrong argument count for ensures_of
    // ========================================

    fun test_ensures_wrong_count(x: u64): u64 {
        unary(x)
    }

    spec test_ensures_wrong_count {
        // Error: unary has 1 input + 1 result = 2 args needed, but only 1 provided
        ensures ensures_of<unary>(x);
    }

    // ========================================
    // Error: Unknown function target
    // ========================================

    fun test_unknown_function(x: u64): u64 {
        x
    }

    spec test_unknown_function {
        // Error: nonexistent_function doesn't exist
        ensures requires_of<nonexistent_function>(x);
    }

    // ========================================
    // Error: Type arguments provided for function parameter
    // ========================================

    fun apply_with_type_args(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_with_type_args {
        // Error: type arguments cannot be provided for function parameter
        ensures requires_of<f<u64>>(x);
    }

    // ========================================
    // Error: Non-function type used as target
    // ========================================

    fun test_non_function_target(x: u64): u64 {
        x
    }

    spec test_non_function_target {
        // Error: x is not a function type
        ensures requires_of<x>(x);
    }

    // ========================================
    // Error: Wrong type argument arity for generic function
    // ========================================

    fun test_wrong_type_arity(x: u64): u64 {
        generic_id(x)
    }

    spec test_wrong_type_arity {
        // Error: generic_id takes 1 type param, but 2 provided
        ensures requires_of<generic_id<u64, u64>>(x);
    }

    // ========================================
    // Error: Wrong argument type
    // ========================================

    fun test_wrong_arg_type(a: u64, b: bool): u64 {
        a
    }

    spec test_wrong_arg_type {
        // Error: binary expects (u64, u64), but (u64, bool) provided
        ensures requires_of<binary>(a, b);
    }

    // ========================================
    // Error: Too many arguments for requires_of
    // ========================================

    fun test_too_many_args(a: u64, b: u64, c: u64): u64 {
        binary(a, b)
    }

    spec test_too_many_args {
        // Error: binary takes 2 args, but 3 provided
        ensures requires_of<binary>(a, b, c);
    }
}
