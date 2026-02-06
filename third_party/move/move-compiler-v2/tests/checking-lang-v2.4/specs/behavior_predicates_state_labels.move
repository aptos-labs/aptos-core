// Tests for behavior predicates with state labels.
// State labels allow reasoning about state at different program points.
// Rules:
// 1. Pre-labels must reference post-labels defined by other predicates in the same spec
// 2. Post-labels must be referenced by some pre-label
// 3. No cycles in state label dependencies

module 0x42::M {

    // Test a simple two-predicate chain: one defines, one references
    fun apply_simple_chain(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_simple_chain {
        // First predicate defines post-state "s1"
        ensures ensures_of<f>(x, result)@s1;
        // Second predicate reads from "s1" (must reference s1 so it's not orphaned)
        ensures s1@ensures_of<f>(x, result);
    }

    // Test a longer chain: s1 -> s2 -> end
    fun apply_longer_chain(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_longer_chain {
        // First predicate defines post-state "s1"
        ensures ensures_of<f>(x, result)@s1;
        // Second predicate reads from "s1" and defines "s2"
        ensures s1@ensures_of<f>(x, result)@s2;
        // Third predicate reads from "s2"
        ensures s2@ensures_of<f>(x, result);
    }

    // Test result_of with state labels
    fun apply_result_chain(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_result_chain {
        // First predicate defines post-state "r1"
        ensures result_of<f>(x)@r1 == result;
        // Second predicate reads from "r1"
        ensures r1@result_of<f>(x) == result;
    }
}
