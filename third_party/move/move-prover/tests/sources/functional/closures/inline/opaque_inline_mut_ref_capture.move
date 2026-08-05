// Tests for lambdas capturing mutable references, passed to retained
// inline-opaque functions. The prover models the captured mutation as carried
// by the closure value: havoced across the call, constrained by the callee's
// behavioral post conditions, and written back to its source location when
// the closure dies.
module 0x42::opaque_inline_mut_ref_capture {
    struct S has drop {
        f: u64,
        g: u64,
    }

    inline fun modify(f: |u64|) {
        f(1)
    }
    spec modify {
        pragma opaque;
        requires !aborts_of<f>(1);
        aborts_if false;
        ensures ensures_of<f>(1);
    }

    /// Test: lambda capturing a mutable reference to a local.
    fun test_local_mut_capture(): u64 {
        let x = 10;
        let r = &mut x;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
        x
    }
    spec test_local_mut_capture {
        ensures result == 11;
    }

    /// Test: lambda capturing a mutable reference parameter.
    fun test_param_mut_capture(r: &mut u64) {
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
    }
    spec test_param_mut_capture {
        requires r < MAX_U64;
        aborts_if false;
        ensures r == old(r) + 1;
    }

    /// Test: lambda capturing a mutable field borrow (a `Field` edge above the
    /// `Capture` edge in the write-back chain).
    fun test_field_mut_capture(): S {
        let s = S { f: 1, g: 2 };
        let r = &mut s.f;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
        s
    }
    spec test_field_mut_capture {
        ensures result.f == 2;
        ensures result.g == 2;
    }

    /// Test: two sequential retained calls mutating the same root.
    fun test_sequential_calls(): u64 {
        let x = 0;
        let r = &mut x;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
        let r2 = &mut x;
        modify(|y| *r2 = *r2 + y spec {
            aborts_if r2 + y > MAX_U64;
            ensures r2 == old(r2) + y;
        });
        x
    }
    spec test_sequential_calls {
        ensures result == 2;
    }

    inline fun apply_ref(f: |&mut u64| &mut u64, x: &mut u64): u64 {
        *f(x)
    }
    spec apply_ref {
        // Not honored: `apply_ref` has a `&mut`-returning function parameter, so
        // it is not retained; calls are expanded and verified through the body.
        pragma opaque;
    }

    /// Test: a lambda capturing a `&mut` and returning a reference is admitted
    /// because the callee is expanded rather than retained.
    fun test_expanded_ref_result(): u64 {
        let x = 10;
        let z = 5;
        let r = &mut z;
        let v = apply_ref(|s| {
            *s = *s + 1;
            r
        }, &mut x);
        v + x + z
    }
    spec test_expanded_ref_result {
        ensures result == 21;
    }

    inline fun scale(f: |u64|, k: u64) {
        f(k)
    }
    spec scale {
        pragma opaque;
        requires !aborts_of<f>(k);
        aborts_if false;
        ensures ensures_of<f>(k);
    }

    /// Test: mixed capture of a mutable reference and a value, with a
    /// non-captured parameter.
    fun test_mixed_capture(k: u64): u64 {
        let x = 10;
        let b = 3;
        let r = &mut x;
        scale(|y| *r = *r * y + b spec {
            aborts_if r * y + b > MAX_U64;
            ensures r == old(r) * y + b;
        }, k);
        x
    }
    spec test_mixed_capture {
        requires k < 1000;
        ensures result == 10 * k + 3;
    }
}
