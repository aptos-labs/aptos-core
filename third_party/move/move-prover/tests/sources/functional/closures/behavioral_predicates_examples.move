// Examples demonstrating behavioral predicates with higher-order functions.

module 0x42::behavioral_predicates_examples {

    // =========================================================================
    // Apply_twice (opaque): applying a function twice with opaque pragma
    // =========================================================================

    /// Applies function f twice: f(f(x))
    /// This version is opaque - verification relies on behavioral predicates.
    fun apply_twice_opaque(f: |u64| u64 has copy, x: u64): u64 {
        f(f(x))
    }
    spec apply_twice_opaque {
        // Make this opaque so we get semantics from spec at caller side.
        pragma opaque = true;
        // Without knowing `f`, we can still specify how its
        // behavior effects the calling function.
        let y = choose y: u64 where ensures_of<f>(x, y);
        requires requires_of<f>(x) && requires_of<f>(y);
        aborts_if aborts_of<f>(x) || aborts_of<f>(y);
        ensures ensures_of<f>(y, result);
    }

    fun add_opaque(): u64 {
        apply_twice_opaque(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1;},
        1)
    }
    spec add_opaque {
        ensures result == 3;
    }

    // =========================================================================
    // Apply_twice (transparent): applying a function twice without opaque
    // =========================================================================

    /// Applies function f twice: f(f(x))
    /// This version is transparent - verification inlines the implementation.
    fun apply_twice_transparent(f: |u64| u64 has copy, x: u64): u64 {
        f(f(x))
    }
    spec apply_twice_transparent {
        // NOT opaque - the implementation is inlined at call sites.
        // Behavioral predicates still define the abstract contract.
        let y = choose y: u64 where ensures_of<f>(x, y);
        requires requires_of<f>(x) && requires_of<f>(y);
        aborts_if aborts_of<f>(x) || aborts_of<f>(y);
        ensures ensures_of<f>(y, result);
    }

    fun add_transparent(): u64 {
        apply_twice_transparent(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1;},
        1)
    }
    spec add_transparent {
        ensures result == 3;
    }

    // =========================================================================
    // Apply with no abort: using aborts_of to guarantee no abort
    // =========================================================================

    /// Applies function f to x, requiring that f won't abort on x.
    fun apply_with_no_abort(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_with_no_abort {
        requires !aborts_of<f>(x);
        aborts_if false;
        ensures ensures_of<f>(x, result);
    }

    fun test_no_abort(): u64 {
        // This closure never aborts (for values < MAX_U64)
        apply_with_no_abort(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            5  // 5 != MAX_U64, so !aborts_of is satisfied
        )
    }
    spec test_no_abort {
        ensures result == 6;
    }

    fun test_no_abort_fail(): u64 {
        // This should FAIL verification: passing MAX_U64 violates !aborts_of<f>(x)
        // because the closure aborts when x == MAX_U64
        apply_with_no_abort(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            MAX_U64  // aborts_of<f>(MAX_U64) is true, so !aborts_of is violated
        )
    }
    spec test_no_abort_fail {
        ensures result == 0;  // unreachable, but spec needed
    }

    // =========================================================================
    // Contains: behavioral predicates with reference-typed closure and vectors
    // =========================================================================

    /// Checks if any element in the vector satisfies the predicate.
    /// Demonstrates behavioral predicates with reference-typed closure parameter.
    /// Uses opaque pragma so callers rely on the spec rather than implementation.
    fun contains(v: &vector<u64>, pred: |&u64| bool has copy + drop): bool {
        let i = 0;
        let len = std::vector::length(v);
        while ({
            spec {
                invariant i <= len;
                invariant forall j in 0..i: !ensures_of<pred>(v[j], true);
            };
            i < len
        }) {
            if (pred(std::vector::borrow(v, i))) {
                return true
            };
            i = i + 1;
        };
        false
    }
    spec contains {
        // Make opaque so callers use spec, not implementation
        pragma opaque = true;
        // Require the predicate won't abort for any element
        requires forall x in 0..len(v): !aborts_of<pred>(v[x]);
        aborts_if false;
        // The result indicates whether any element satisfies the predicate
        ensures result == (exists k in 0..len(v): ensures_of<pred>(v[k], true));
    }

    /// Test: contains with a concrete predicate checking for value > 2
    fun test_contains_found(): bool {
        let v = vector[1, 4, 2];
        contains(
            &v,
            |x| (*x > 2) spec { ensures result == (x > 2); }
        )
    }
    spec test_contains_found {
        ensures result == true;  // 4 > 2
    }

    /// Test: contains with a predicate that matches nothing
    fun test_contains_not_found(): bool {
        let v = vector[1, 2, 3];
        contains(
            &v,
            |x| (*x > 5) spec { ensures result == (x > 5); }
        )
    }
    spec test_contains_not_found {
        ensures result == false;  // no element > 5
    }
}
