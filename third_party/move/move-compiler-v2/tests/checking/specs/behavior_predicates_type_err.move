// Tests for behavior predicate type checking errors.

module 0x42::M {

    // Helper functions
    fun unary(x: u64): u64 { x }
    fun binary(a: u64, b: u64): u64 { a + b }

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
}
