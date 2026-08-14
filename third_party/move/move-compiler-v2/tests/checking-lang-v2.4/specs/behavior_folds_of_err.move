// Tests for error checking of the `folds_of` behavior predicate.

module 0x42::M {

    fun apply(f: |u64| u64, v: vector<u64>, x: u64): u64 {
        f(x)
    }

    spec apply {
        // Error: folds_of takes 2 args, but 3 provided
        ensures folds_of<f>(v, x, x);
        // Error: bool result used as integer
        ensures folds_of<f>(v, x) + 1 == 2;
        // Error: target must have function type
        ensures folds_of<x>(v, x);
        // Error: post-state label not allowed on folds_of
        ensures ..post |~ folds_of<f>(v, x);
        // Error: two-state predicate requires range notation
        ensures a..a |~ folds_of<f>(v, x);
    }

    fun apply_binary(f: |u64, u64|, v: vector<u64>, i: u64) {
        f(i, i)
    }

    spec apply_binary {
        // Error: element form requires a unary target
        ensures folds_of<f>(v, i);
        // Error: index lambda must take exactly one parameter
        ensures folds_of<f>(|j, k| (j, k), i);
        // Error: index lambda result must be the target's argument tuple
        ensures folds_of<f>(|j| j, i);
    }

    fun apply_indirect(f: |u64|, g: |u64| u64, v: vector<bool>, i: u64) {
        f(g(i))
    }

    spec apply_indirect {
        // Error: the index function must be a literal lambda; a
        // function-typed variable fails the element-form vector check
        ensures folds_of<f>(g, i);
        // Error: element type mismatch (vector<bool> vs u64 parameter)
        ensures folds_of<f>(v, i);
    }
}
