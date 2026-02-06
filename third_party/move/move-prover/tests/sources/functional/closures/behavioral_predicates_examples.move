// Examples demonstrating behavioral predicates with higher-order functions.
// Each section contains both transparent and opaque versions of the same function.

module 0x42::behavioral_predicates_examples {

    // =========================================================================
    // Apply_seq: applying a function sequentially f(f(x))
    // =========================================================================

    /// Applies function f twice: f(f(x))
    /// This version is transparent - verification inlines the implementation.
    fun apply_seq(f: |u64| u64 has copy, x: u64): u64 {
        f(f(x))
    }
    spec apply_seq {
        let y = result_of<f>(x);
        requires requires_of<f>(x) && requires_of<f>(y);
        aborts_if aborts_of<f>(x) || aborts_of<f>(y);
        ensures result == result_of<f>(y);
    }

    /// Applies function f twice: f(f(x))
    /// This version is opaque - verification relies on behavioral predicates.
    fun apply_seq_opaque(f: |u64| u64 has copy, x: u64): u64 {
        f(f(x))
    }
    spec apply_seq_opaque {
        pragma verify = false;
        pragma opaque = true;
        let y = result_of<f>(x);
        requires requires_of<f>(x) && requires_of<f>(y);
        aborts_if aborts_of<f>(x) || aborts_of<f>(y);
        ensures result == result_of<f>(y);
    }

    fun apply_seq_test(): u64 {
        apply_seq(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            1
        )
    }
    spec apply_seq_test {
        ensures result == 3;
    }

    fun apply_seq_opaque_test(): u64 {
        apply_seq_opaque(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            1
        )
    }
    spec apply_seq_opaque_test {
        ensures result == 3;
    }

    // =========================================================================
    // Apply_parallel: applying a function twice and adding results f(x) + f(y)
    // =========================================================================

    /// Applies function f to two arguments and adds results: f(x) + f(y)
    /// This version is transparent - verification inlines the implementation.
    fun apply_parallel(f: |u64| u64 has copy, x: u64, y: u64): u64 {
        f(x) + f(y)
    }
    spec apply_parallel {
        requires requires_of<f>(x) && requires_of<f>(y);
        ensures result == result_of<f>(x) + result_of<f>(y);
    }

    /// Applies function f to two arguments and adds results: f(x) + f(y)
    /// This version is opaque - verification relies on behavioral predicates.
    fun apply_parallel_opaque(f: |u64| u64 has copy, x: u64, y: u64): u64 {
        f(x) + f(y)
    }
    spec apply_parallel_opaque {
        pragma opaque = true;
        requires requires_of<f>(x) && requires_of<f>(y);
        ensures result == result_of<f>(x) + result_of<f>(y);
    }

    fun apply_parallel_test(): u64 {
        apply_parallel(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            1, 1
        )
    }
    spec apply_parallel_test {
        ensures result == 4;
    }

    fun apply_parallel_opaque_test(): u64 {
        apply_parallel_opaque(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            1, 1
        )
    }
    spec apply_parallel_opaque_test {
        ensures result == 4;
    }

    // =========================================================================
    // Apply_no_abort: using aborts_of to guarantee no abort
    // =========================================================================

    /// Applies function f to x, requiring that f won't abort on x.
    /// This version is transparent - verification inlines the implementation.
    fun apply_no_abort(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_no_abort {
        requires !aborts_of<f>(x); // expected fail from test_fail
        aborts_if false;
        ensures ensures_of<f>(x, result);
    }

    /// Applies function f to x, requiring that f won't abort on x.
    /// This version is opaque - verification relies on behavioral predicates.
    fun apply_no_abort_opaque(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_no_abort_opaque {
        pragma opaque = true;
        requires !aborts_of<f>(x); // expected fail from test_fail
        aborts_if false;
        ensures ensures_of<f>(x, result);
    }

    fun apply_no_abort_test(): u64 {
        apply_no_abort(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            5
        )
    }
    spec apply_no_abort_test {
        ensures result == 6;
    }

    fun apply_no_abort_opaque_test(): u64 {
        apply_no_abort_opaque(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            5
        )
    }
    spec apply_no_abort_opaque_test {
        ensures result == 6;
    }

    fun apply_no_abort_test_fail(): u64 {
        // This should FAIL verification: passing MAX_U64 violates !aborts_of<f>(x)
        apply_no_abort(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            MAX_U64
        )
    }
    spec apply_no_abort_test_fail {
        ensures result == 0;  // unreachable, but spec needed
    }

    fun apply_no_abort_opaque_test_fail(): u64 {
        // This should FAIL verification: passing MAX_U64 violates !aborts_of<f>(x)
        apply_no_abort_opaque(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            MAX_U64
        )
    }
    spec apply_no_abort_opaque_test_fail {
        ensures result == 0;  // unreachable, but spec needed
    }

    // =========================================================================
    // Contains: checking if any element satisfies a predicate
    // =========================================================================

    /// Checks if any element in the vector satisfies the predicate.
    /// This version is transparent - verification inlines the implementation.
    fun contains(v: &vector<u64>, pred: |&u64| bool has copy + drop): bool {
        let i = 0;
        let len = std::vector::length(v);
        while (i < len) {
            if (pred(std::vector::borrow(v, i))) {
                return true;
            };
            i = i + 1;
        }
            spec {
                invariant i <= len;
                invariant forall j in 0..i: !result_of<pred>(v[j]);
            };
        false
    }
    spec contains {
        requires forall x in 0..len(v): !aborts_of<pred>(v[x]);
        aborts_if false;
        ensures result == (exists k in 0..len(v): result_of<pred>(v[k]));
    }

    /// Checks if any element in the vector satisfies the predicate.
    /// This version is opaque - verification relies on behavioral predicates.
    fun contains_opaque(v: &vector<u64>, pred: |&u64| bool has copy + drop): bool {
        let i = 0;
        let len = std::vector::length(v);
        while (i < len) {
            if (pred(std::vector::borrow(v, i))) {
                return true;
            };
            i = i + 1;
        }
            spec {
                invariant i <= len;
                invariant forall j in 0..i: !result_of<pred>(v[j]);
            };
        false
    }
    spec contains_opaque {
        pragma opaque = true;
        requires forall x in 0..len(v): !aborts_of<pred>(v[x]);
        aborts_if false;
        ensures result == (exists k in 0..len(v): result_of<pred>(v[k]));
    }

    fun contains_test_found(): bool {
        let v = vector[1, 4, 2];
        contains(&v, |x| (*x > 2) spec { ensures result == (x > 2); })
    }
    spec contains_test_found {
        ensures result == true;  // TODO(#18560)
    }

    fun contains_opaque_test_found(): bool {
        let v = vector[1, 4, 2];
        contains_opaque(&v, |x| (*x > 2) spec { ensures result == (x > 2); })
    }
    spec contains_opaque_test_found {
        ensures result == true;  // TODO(#18560)
    }

    fun contains_test_not_found(): bool {
        let v = vector[1, 2, 3];
        contains(&v, |x| (*x > 5) spec { ensures result == (x > 5); })
    }
    spec contains_test_not_found {
        ensures result == false;
    }

    fun contains_opaque_test_not_found(): bool {
        let v = vector[1, 2, 3];
        contains_opaque(&v, |x| (*x > 5) spec { ensures result == (x > 5); })
    }
    spec contains_opaque_test_not_found {
        ensures result == false;
    }

    // =========================================================================
    // Index: finding the first element satisfying a predicate
    // =========================================================================

    /// Finds the index of the first element satisfying the predicate.
    /// Returns len(v) if not found.
    /// This version is transparent - verification inlines the implementation.
    fun index(v: &vector<u64>, pred: |&u64| bool has copy + drop): u64 {
        let i = 0;
        let len = std::vector::length(v);
        while (i < len) {
            if (pred(&v[i])) return i;
            i = i + 1;
        }
        spec {
            invariant i <= len;
            invariant forall j in 0..i: !result_of<pred>(v[j]);
        };
        len
    }
    spec index {
        ensures result >= 0 && result <= len(v);
        ensures forall j in 0..result: !result_of<pred>(v[j]);
        ensures result < len(v) ==> result_of<pred>(v[result]);
    }

    /// Finds the index of the first element satisfying the predicate.
    /// Returns len(v) if not found.
    /// This version is opaque - verification relies on behavioral predicates.
    fun index_opaque(v: &vector<u64>, pred: |&u64| bool has copy + drop): u64 {
        let i = 0;
        let len = std::vector::length(v);
        while (i < len) {
            if (pred(&v[i])) return i;
            i = i + 1;
        }
        spec {
            invariant i >= 0 && i <= len;
            invariant forall j in 0..i: !result_of<pred>(v[j]);
        };
        len
    }
    spec index_opaque {
        pragma opaque = true;
        ensures result >= 0 && result <= len(v);
        ensures forall j in 0..result: !result_of<pred>(v[j]);
        ensures result < len(v) ==> result_of<pred>(v[result]);
    }

    fun index_test_found(): u64 {
        let v = vector[1, 4, 2];
        spec { assert v[1] == 4; }; // TODO(#18560): need witness
        index(&v, |x| (*x > 2) spec { ensures result == (x > 2); })
    }
    spec index_test_found {
        ensures result == 1;
    }

    fun index_opaque_test_found(): u64 {
        let v = vector[1, 4, 2];
        spec { assert v[1] == 4; };
        index_opaque(&v, |x| (*x > 2) spec { ensures result == (x > 2); })
    }
    spec index_opaque_test_found {
        ensures result == 1;
    }

    // =========================================================================
    // Reduce: folding over a vector with a reducer function
    // =========================================================================

    /// Reduces a vector using a reducer function: reducer(...reducer(reducer(start, v[0]), v[1])..., v[n-1])
    /// This version is transparent - verification inlines the implementation.
    fun reduce(vec: vector<u64>, start: u64, reducer: |u64, u64|u64 has copy + drop): u64 {
        let i = 0;
        let len = std::vector::length(&vec);
        let result = start;
        while (i < len) {
            result = reducer(result, vec[i]);
            i = i + 1;
        }
        spec {
            invariant i >= 0 && i <= len;
            invariant result == spec_reduce(reducer, vec, start, i);
        };
        result
    }
    spec reduce {
        ensures result == spec_reduce(reducer, vec, start, len(vec));
    }

    /// Reduces a vector using a reducer function.
    /// This version is opaque - verification relies on behavioral predicates.
    fun reduce_opaque(vec: vector<u64>, start: u64, reducer: |u64, u64|u64 has copy + drop): u64 {
        let i = 0;
        let len = std::vector::length(&vec);
        let result = start;
        while (i < len) {
            result = reducer(result, vec[i]);
            i = i + 1;
        }
        spec {
            invariant i >= 0 && i <= len;
            invariant result == spec_reduce(reducer, vec, start, i);
        };
        result
    }
    spec reduce_opaque {
        pragma opaque;
        ensures result == spec_reduce(reducer, vec, start, len(vec));
    }

    /// Spec function defining reduce semantics recursively.
    spec fun spec_reduce(reducer: |u64, u64|u64, v: vector<u64>, val: u64, end: u64): u64 {
        if (end == 0) val
        else {
            let val = spec_reduce(reducer, v, val, end - 1);
            result_of<reducer>(val, v[end - 1])
        }
    }

    fun reduce_test_ok(): u64 {
        let v = vector[2, 4, 1];
        reduce(v, 1, |x, y| x + y spec { ensures result == x + y; })
    }
    spec reduce_test_ok {
        ensures result == 8;
    }

    fun reduce_opaque_test_ok(): u64 {
        let v = vector[2, 4, 1];
        reduce_opaque(v, 1, |x, y| x + y spec { ensures result == x + y; })
    }
    spec reduce_opaque_test_ok {
        ensures result == 8;
    }

    fun reduce_test_fail(): u64 {
        let v = vector[2, 4, 1];
        reduce(v, 1, |x, y| x + y spec { ensures result == x + y; })
    }
    spec reduce_test_fail {
        ensures result == 7; // expected failure: result == 8
    }

    fun reduce_opaque_test_fail(): u64 {
        let v = vector[2, 4, 1];
        reduce_opaque(v, 1, |x, y| x + y spec { ensures result == x + y; })
    }
    spec reduce_opaque_test_fail {
        ensures result == 7; // expected failure: result == 8
    }
}
