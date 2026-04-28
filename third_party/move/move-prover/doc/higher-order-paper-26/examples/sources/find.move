// Copyright © Aptos Foundation
// Higher-order vector search demonstrating specification of a
// closure-parameterized `find` via behavioral predicates. The loop
// invariant quantifies `result_of<pred>` and `aborts_of<pred>`
// symbolically on prefix elements; the function spec characterises
// the return value entirely via those predicates.

module defi::collection {
    use std::vector;

    spec module {
        /// `no_match(v, pred, k)` holds iff `pred` returns false on every
        /// element of the prefix `v[0..k]`.
        fun no_match<T>(v: vector<T>, pred: |&T|bool, k: u64): bool {
            forall j in 0..k: !result_of<pred>(v[j])
        }

        /// `no_abort(v, pred, k)` holds iff `pred` does not abort on any
        /// element of the prefix `v[0..k]`.
        fun no_abort<T>(v: vector<T>, pred: |&T|bool, k: u64): bool {
            forall j in 0..k: !aborts_of<pred>(v[j])
        }
    }

    /// Find the first index `i < length(v)` such that `pred(&v[i])`
    /// holds; returns `length(v)` if no such index exists.
    public fun find<T>(v: &vector<T>, pred: |&T|bool has copy + drop): u64 {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            if (pred(vector::borrow(v, i))) {
                return i
            };
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant no_match(v, pred, i);
            invariant no_abort(v, pred, i);
        };
        n
    }
    spec find {
        pragma opaque;
        // `pred` must be callable on every element.
        requires forall j in 0..len(v): requires_of<pred>(v[j]);
        // `find` aborts iff there is a visited index where `pred` aborts:
        // the scan reaches `j` iff all earlier calls neither aborted nor matched.
        aborts_if exists j in 0..len(v):
            no_abort(v, pred, j) && no_match(v, pred, j) &&
            aborts_of<pred>(v[j]);
        // Result is within [0, len(v)].
        ensures result <= len(v);
        // If found, `pred` holds at `result` and nowhere before.
        ensures result < len(v) ==>
            result_of<pred>(v[result]) && no_match(v, pred, result);
        // If not found, `pred` holds nowhere.
        ensures result == len(v) ==> no_match(v, pred, len(v));
    }

    // -------------------------------------------------------
    // Usage example: find the first zero in a vector
    // -------------------------------------------------------

    /// Named predicate, given an explicit spec so callers of `find`
    /// can propagate its contract.
    public fun is_zero(x: &u64): bool {
        *x == 0
    }
    spec is_zero {
        pragma opaque;
        aborts_if false;
        ensures result == (x == 0);
    }

    /// Zero-predicate specialisation of `find`.
    public fun find_zero(v: &vector<u64>): u64 {
        find(v, is_zero)
    }
    spec find_zero {
        aborts_if false;
        ensures result <= len(v);
        ensures result < len(v) ==>
            v[result] == 0 &&
            (forall j in 0..result: v[j] != 0);
        ensures result == len(v) ==>
            (forall j in 0..len(v): v[j] != 0);
    }
}
