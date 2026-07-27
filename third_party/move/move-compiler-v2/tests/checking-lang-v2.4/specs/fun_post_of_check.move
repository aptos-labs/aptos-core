// Type checking for `fun_post_of<f>(args)`: the result type is the subject's
// own function type; arguments are the function's inputs (references
// stripped).
module 0x42::fun_post_of_check {

    fun apply(f: |u64|u64 has drop, x: u64): u64 {
        f(x)
    }
    spec apply {
        // Result compares against the subject's own fun type.
        ensures f == fun_post_of<old(f)>(x);
        // Chains nest through the subject.
        ensures f == fun_post_of<fun_post_of<old(f)>(1)>(2);
    }

    fun apply_ref(f: |&mut u64| has drop, r: &mut u64) {
        f(r)
    }
    spec apply_ref {
        // `&mut` inputs are passed as values.
        ensures f == fun_post_of<old(f)>(r);
    }
}
