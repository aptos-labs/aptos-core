// In verify mode, a lambda passed to a retained inline-opaque function may capture
// mutable references rooted anywhere (locals, parameters, or global storage). The
// closure must not require the `copy` ability (a mutable capture makes the value
// linear). An inline function with a `&mut`-returning function parameter does not
// get the retained treatment at all — its opaque spec is not honored and calls are
// expanded, so no lambda is lifted. In normal compilation every inline function is
// expanded, so no capture arises anywhere.
module 0x42::retained_mut_ref_capture {

    struct R has key {
        value: u64,
    }

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    inline fun apply_copy(f: |u64| u64 has copy, x: u64): u64 {
        f(x)
    }
    spec apply_copy {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    inline fun apply_ref(f: |u64| &mut u64, x: u64): u64 {
        *f(x)
    }
    spec apply_ref {
        pragma opaque;
    }

    /// Admitted: capture of a `&mut` parameter in the retained direct-argument position.
    fun direct_mut_capture(r: &mut u64): u64 {
        apply(|y| { *r = *r + y; *r }, 5)
    }

    /// Admitted: capture rooted in global storage.
    fun global_rooted_capture(a: address): u64 {
        let r = &mut R[a].value;
        apply(|y| { *r = *r + y; *r }, 5)
    }

    /// Rejected: closure requiring the `copy` ability.
    fun copy_mut_capture(r: &mut u64): u64 {
        apply_copy(|y| { *r = *r + y; *r }, 5)
    }

    fun deref_helper(r: &mut u64, y: u64): &mut u64 {
        *r = *r + y;
        r
    }

    /// Admitted via expansion: `apply_ref` has a `&mut`-returning function
    /// parameter, so it is not retained; the call is expanded and the lambda
    /// inlined instead of lifted.
    fun ret_mut_capture(r: &mut u64): u64 {
        apply_ref(|y| deref_helper(r, y), 5)
    }
}
