// Tests for invalid state label usage in behavior predicates.

module 0x42::M {

    // Error: requires_of should not have post-state label
    fun apply_requires_err(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_requires_err {
        ensures requires_of<f>(x)@post; // Error: post-state label not allowed on requires_of
    }

    // Error: aborts_of should not have post-state label
    fun apply_aborts_err(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_aborts_err {
        aborts_if aborts_of<f>(x)@post; // Error: post-state label not allowed on aborts_of
    }

    // Error: pre-state label references undefined post-state
    fun apply_undefined_label(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_undefined_label {
        ensures undefined@ensures_of<f>(x, result); // Error: 'undefined' label not defined
    }

    // Error: post-state label defined but never referenced
    fun apply_orphan_post(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_orphan_post {
        ensures ensures_of<f>(x, result)@orphan; // Error: 'orphan' is never referenced
    }

    // Error: cyclic state label reference
    fun apply_cycle(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply_cycle {
        // a references b's post-state, b references a's post-state - cycle!
        ensures a@ensures_of<f>(x, result)@b;
        ensures b@ensures_of<f>(x, result)@a;
    }
}
