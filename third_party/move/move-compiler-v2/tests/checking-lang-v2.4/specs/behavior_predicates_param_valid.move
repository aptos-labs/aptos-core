// Tests for behavior predicates with function parameter targets.
// These generate uninterpreted specification functions.

module 0x42::M {

    // Test requires_of with function parameter
    fun apply_requires(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_requires {
        ensures requires_of<f>(x);
    }

    // Test aborts_of with function parameter
    fun apply_aborts(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_aborts {
        aborts_if aborts_of<f>(x);
    }

    // Test ensures_of with function parameter
    fun apply_ensures(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_ensures {
        ensures ensures_of<f>(x, result);
    }

    // Test with binary function parameter
    fun apply_binary(f: |u64, u64| u64, a: u64, b: u64): u64 {
        f(a, b)
    }

    spec apply_binary {
        ensures requires_of<f>(a, b);
        ensures ensures_of<f>(a, b, result);
        aborts_if aborts_of<f>(a, b);
    }

    // Test with generic function parameter - verifies type instantiation is preserved
    fun apply_generic<T>(f: |T| T, x: T): T {
        f(x)
    }

    spec apply_generic {
        ensures requires_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    // Another function to test with valid state label chains
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
