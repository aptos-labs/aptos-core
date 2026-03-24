// Tests for behavior predicate error checking.
// This file consolidates various error cases for behavioral predicates
// (requires_of, aborts_of, ensures_of, result_of).

module 0x42::M {

    struct R has key { value: u64 }

    fun helper(x: u64): u64 { x + 1 }
    fun binary(a: u64, b: u64): u64 { a + b }

    // ========================================
    // Error: Non-function name used as predicate target
    // ========================================

    fun test_non_fun_target(x: u64): u64 { x }

    spec test_non_fun_target {
        // Error: x is u64, not a function
        ensures requires_of<x>(x);
    }

    // ========================================
    // Error: Wrong number of arguments for requires_of
    // ========================================

    fun test_requires_wrong_count(a: u64, b: u64): u64 {
        binary(a, b)
    }

    spec test_requires_wrong_count {
        // Error: binary takes 2 args, but only 1 provided
        ensures requires_of<binary>(a);
    }

    // ========================================
    // Error: Wrong number of arguments for ensures_of
    // ========================================

    fun test_ensures_wrong_count(x: u64): u64 {
        helper(x)
    }

    spec test_ensures_wrong_count {
        // Error: helper has 1 input + 1 result = 2 args needed, but only 1 provided
        ensures ensures_of<helper>(x);
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

    // ========================================
    // Error: Wrong argument type
    // ========================================

    fun test_wrong_arg_type(a: u64, b: bool): u64 { a }

    spec test_wrong_arg_type {
        // Error: binary expects (u64, u64), but (u64, bool) provided
        ensures requires_of<binary>(a, b);
    }

    // ========================================
    // Error: Wrong number of arguments for function parameter ensures_of
    // ========================================

    fun apply_ensures_wrong(f: |address|, addr: address) {
        f(addr)
    }

    spec apply_ensures_wrong {
        // Error: f returns unit, so ensures_of<f>(addr, 42) has too many args
        ensures ensures_of<f>(addr, 42);
    }

    // ========================================
    // Error: result_of wrong arity
    // ========================================

    fun test_result_wrong_count(x: u64): u64 {
        helper(x)
    }

    spec test_result_wrong_count {
        // Error: helper takes 1 arg, but 2 provided
        ensures result == result_of<helper>(x, x);
    }

    // ========================================
    // Error: Unknown function target
    // ========================================

    fun test_unknown_function(x: u64): u64 { x }

    spec test_unknown_function {
        // Error: nonexistent_function doesn't exist
        ensures requires_of<nonexistent_function>(x);
    }
}
