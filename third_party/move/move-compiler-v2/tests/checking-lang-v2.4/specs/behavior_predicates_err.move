// Tests for behavior predicate type checking.
// Behavior predicates (requires_of, aborts_of, ensures_of, modifies_of)
// are parsed and type-checked. This test verifies correct type checking
// and expected error messages for invalid usage.

module 0x42::M {

    // Function with a function parameter
    fun apply(f: |u64, u64| u64, x: u64, y: u64): u64 {
        f(x, y)
    }

    spec apply {
        // Test basic behavior predicate syntax - requires_of
        ensures requires_of<f>(x, y);

        // Test basic behavior predicate syntax - ensures_of
        ensures ensures_of<f>(x, y, result);

        // Test basic behavior predicate syntax - aborts_of
        aborts_if aborts_of<f>(x, y);

        // Test basic behavior predicate syntax - modifies_of
        modifies modifies_of<f>(x);
    }

    // Another function to test with state labels (valid chains)
    fun apply2(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply2 {
        // First predicate defines post-state "s1"
        ensures ensures_of<f>(x, result)@s1;

        // Second predicate reads from "s1" (completes the chain)
        ensures s1@ensures_of<f>(x, result);
    }
}
