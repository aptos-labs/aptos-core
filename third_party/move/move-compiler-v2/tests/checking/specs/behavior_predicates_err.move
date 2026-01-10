// Tests for behavior predicate parsing and error messages.
// Behavior predicates (requires_of, aborts_of, ensures_of, modifies_of)
// are parsed but not yet supported in the model builder.

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

    // Another function to test with state labels
    fun apply2(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply2 {
        // Test with pre-state label
        ensures old@ensures_of<f>(x, result);

        // Test with post-state label
        ensures ensures_of<f>(x, result)@post;

        // Test with both pre and post state labels
        ensures pre@ensures_of<f>(x, result)@post;
    }
}
