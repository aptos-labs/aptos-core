// Tests for error checking of the `unchanged_of` behavior predicate.

module 0x42::M {

    fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply {
        // Error: f takes 1 arg, but 2 provided
        ensures unchanged_of<f>(x, x);
        // Error: bool result used as integer
        ensures unchanged_of<f>(x) + 1 == 2;
        // Error: target must have function type
        ensures unchanged_of<x>(x);
        // Error: post-state label not allowed on unchanged_of
        ensures ..post |~ unchanged_of<f>(x);
        // Error: two-state predicate requires range notation
        ensures a..a |~ unchanged_of<f>(x);
    }
}
