// Tests for valid parsing and typing of the `folds_of` behavior predicate,
// in both surface forms, inside loop invariants of inline functions.

module 0x42::M {

    // Element form: `folds_of<f>(v, i)` — unary `f`, `v` a vector of the
    // parameter's (reference-stripped) type, `i` the iteration count.
    inline fun apply_elem(f: |u64|, v: vector<u64>, n: u64) {
        let i = 0;
        while (i < n) {
            f(i);
            i = i + 1;
        } spec {
            invariant folds_of<f>(v, i);
        };
    }

    // Element form with a reference parameter: the element type is the
    // reference-stripped parameter type.
    inline fun apply_elem_ref(f: |&u64|, v: vector<u64>, n: u64) {
        let i = 0;
        while (i < n) {
            f(&i);
            i = i + 1;
        } spec {
            invariant folds_of<f>(v, i);
        };
    }

    // General form: `folds_of<f>(g, i)` with a literal index lambda `g`
    // producing the target's argument tuple for iteration `j`.
    inline fun apply_zip(f: |u64, u64|, v1: vector<u64>, v2: vector<u64>, n: u64) {
        let i = 0;
        while (i < n) {
            f(i, i);
            i = i + 1;
        } spec {
            invariant folds_of<f>(|j| (v1[j], v2[j]), i);
        };
    }

    // General form with a unary target (e.g. reversed iteration order).
    inline fun apply_rev(f: |u64|, v: vector<u64>, n: u64) {
        let i = 0;
        while (i < n) {
            f(i);
            i = i + 1;
        } spec {
            invariant folds_of<f>(|j| v[n - 1 - j], i);
        };
    }

    // Callers with capture-writing lambdas (the motivating shape); in
    // regular compilation the unresolved predicates reduce to `true`.
    fun caller(v: vector<u64>, w: vector<u64>, n: u64): u64 {
        let sum = 0;
        apply_elem(|x| sum = sum + x, v, n);
        apply_elem_ref(|x| sum = sum + *x, v, n);
        apply_zip(|x, y| sum = sum + x + y, v, w, n);
        apply_rev(|x| sum = sum + x, v, n);
        sum
    }
}
