// Tests for valid behavior predicate type checking with qualified function targets.
// Note: Function-typed parameters require inline functions, but inline functions
// don't support spec blocks yet. So we test with qualified function targets only.

module 0x42::M {

    // ========================================
    // Test functions to use as targets
    // ========================================

    // Simple unary function
    public fun increment(x: u64): u64 {
        x + 1
    }

    spec increment {
        requires x < 18446744073709551615;
        ensures result == x + 1;
    }

    // Binary function
    public fun add(a: u64, b: u64): u64 {
        a + b
    }

    spec add {
        requires a + b <= 18446744073709551615;
        ensures result == a + b;
    }

    // Function with no return value
    fun do_nothing(_x: u64) {
    }

    spec do_nothing {
        ensures true;
    }

    // Function with multiple return values
    fun split(x: u64): (u64, u64) {
        (x / 2, x - x / 2)
    }

    spec split {
        ensures result_1 + result_2 == x;
    }

    // Function with different param types
    fun mixed_params(x: u64, flag: bool): u64 {
        if (flag) { x + 1 } else { x }
    }

    // ========================================
    // Tests using behavior predicates with qualified function targets
    // ========================================

    // Test requires_of with unary function
    fun test_requires_unary(x: u64): u64 {
        increment(x)
    }

    spec test_requires_unary {
        ensures requires_of<increment>(x);
    }

    // Test requires_of with binary function
    fun test_requires_binary(a: u64, b: u64): u64 {
        add(a, b)
    }

    spec test_requires_binary {
        ensures requires_of<add>(a, b);
    }

    // Test aborts_of
    fun test_aborts(x: u64): u64 {
        increment(x)
    }

    spec test_aborts {
        aborts_if aborts_of<increment>(x);
    }

    // Test ensures_of with unary function (input + result)
    fun test_ensures_unary(x: u64): u64 {
        increment(x)
    }

    spec test_ensures_unary {
        ensures ensures_of<increment>(x, result);
    }

    // Test ensures_of with binary function (two inputs + result)
    fun test_ensures_binary(a: u64, b: u64): u64 {
        add(a, b)
    }

    spec test_ensures_binary {
        ensures ensures_of<add>(a, b, result);
    }

    // Test ensures_of with function returning unit (only inputs)
    fun test_ensures_unit(x: u64) {
        do_nothing(x)
    }

    spec test_ensures_unit {
        ensures ensures_of<do_nothing>(x);
    }

    // Test ensures_of with multiple return values
    fun test_ensures_multi(x: u64): (u64, u64) {
        split(x)
    }

    spec test_ensures_multi {
        ensures ensures_of<split>(x, result_1, result_2);
    }

    // Test with mixed parameter types
    fun test_mixed_params(x: u64, b: bool): u64 {
        mixed_params(x, b)
    }

    spec test_mixed_params {
        ensures requires_of<mixed_params>(x, b);
        ensures ensures_of<mixed_params>(x, b, result);
    }
}

// Test cross-module function references
module 0x42::N {
    use 0x42::M;

    fun call_increment(x: u64): u64 {
        M::increment(x)
    }

    spec call_increment {
        // Cross-module qualified function reference
        ensures requires_of<M::increment>(x);
        ensures ensures_of<M::increment>(x, result);
    }

    fun call_add(a: u64, b: u64): u64 {
        M::add(a, b)
    }

    spec call_add {
        ensures requires_of<M::add>(a, b);
        ensures ensures_of<M::add>(a, b, result);
    }
}
