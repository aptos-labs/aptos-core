// Aliasing a mut-capturing function value through a spec `let` is not yet
// supported: the `f_post` threading of `ensures_of` requires a directly
// referenced function value (a spec-let binding would alias the pre-call
// closure value).
module 0x42::opaque_inline_mut_capture_alias_fail {

    /// Test: the callee spec may alias the function value through a spec `let`.
    inline fun call_aliased(f: |u64|) {
        f(1)
    }
    spec call_aliased {
        pragma opaque;
        let g = f;
        ensures ensures_of<g>(1); // error: only supported for directly referenced function values
    }
    fun test_spec_let_alias(): u64 {
        let x = 0;
        call_aliased(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_spec_let_alias {
        ensures result == 1;
    }

}
