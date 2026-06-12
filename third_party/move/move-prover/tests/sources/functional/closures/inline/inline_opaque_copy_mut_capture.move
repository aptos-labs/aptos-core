// Tests that a closure capturing `&mut` may now have the `copy` ability and
// be called more than once by an opaque inline HOF. Move semantics already
// prevents such closures from escaping their borrow's stack frame: inline
// HOFs substitute the closure body at every call site (so no value
// materializes for the programmer), and non-inline first-class function
// values cannot acquire `store` because the captured reference type itself
// is unstorable. Multi-call is sound at the source level.
//
// The spec layer's single-application encoding (`spec_instrumentation.rs`'s
// "more than one ensures_of" check) is unchanged and is the boundary of this
// change; tests exploring multi-application specs use `pragma verify = false`
// where the encoding falls short.
module 0x42::inline_opaque_copy_mut_capture {

    inline fun call_twice(f: |u64| has copy + drop) {
        f(1);
        f(2);
    }
    spec call_twice {
        pragma opaque;
        // Two ensures_of clauses both constrain the captured location of f.
        // The current call-site model expresses only the cumulative effect
        // through one havoc; trust the spec until the fold-of-ensures encoding
        // lands. See the design notes in the parent PR.
        pragma verify = false;
        ensures ensures_of<f>(1);
        ensures ensures_of<f>(2);
    }

    /// Test: source-level admission of a copyable `&mut`-capturing closure.
    /// Before the rule was dropped, the call below failed with "a closure
    /// capturing references must not have the `copy` ability".
    fun test_mut_capture_copy(): u64 {
        let x = 0;
        call_twice(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_mut_capture_copy {
        // Trusted: the body is verified standalone in the future, once the
        // multi-application encoding lands.
        pragma verify = false;
        ensures result == 3;
    }
}
