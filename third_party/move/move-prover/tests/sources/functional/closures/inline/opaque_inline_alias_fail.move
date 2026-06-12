// Tests for rejection of conflicting accesses to one location through different
// arguments of a retained inline-opaque call. Such programs can be legal Move —
// after inline expansion the accesses happen sequentially — but the behavioral
// predicates in the callee's opaque spec relate the call's entry and exit
// states and cannot express sequential effects on the same location; the
// prover's call-site model (one pre-state value and one havoced post-state
// value per location) would be unsound.
module 0x42::opaque_inline_alias_fail {

    inline fun call2(f: |u64|, g: |u64|) {
        f(1);
        g(1)
    }
    spec call2 {
        pragma opaque;
        ensures ensures_of<f>(1);
        ensures ensures_of<g>(1);
    }

    /// Test: two lambdas must not mutate the same captured variable in one call.
    fun test_aliasing_captures(): u64 {
        let x = 0;
        call2( // error: mutated through more than one argument
            |i| x = x + i spec { ensures x == old(x) + i; },
            |i| x = x + 2 * i spec { ensures x == old(x) + 2 * i; }
        );
        x
    }
    spec test_aliasing_captures {
        ensures result == 0;
    }

    inline fun update_via(f: |&mut u64|, r: &mut u64) {
        f(r)
    }
    spec update_via {
        pragma opaque;
        ensures ensures_of<f>(r);
    }

    /// Test: a variable mutated via capture must not also be passed as a direct
    /// `&mut` argument of the same call.
    fun test_aliasing_capture_and_arg(): u64 {
        let x = 0;
        update_via(|q| { // error: mutated through more than one argument
            *q = *q + 1;
            x = x + 2;
        } spec {
            ensures q == old(q) + 1;
            ensures x == old(x) + 2;
        }, &mut x);
        x
    }
    spec test_aliasing_capture_and_arg {
        ensures result == 0;
    }

    inline fun observe(f: |u64|, r: &u64): u64 {
        f(1);
        *r
    }
    spec observe {
        pragma opaque;
        ensures ensures_of<f>(1);
        ensures result == r;
    }

    /// Test: a variable mutated via capture must not also be passed by immutable
    /// reference (the spec would reason from a stale pre-state value).
    fun test_mut_capture_and_imm_ref(): (u64, u64) {
        let x = 0;
        let r = observe(|i| x = x + i spec { ensures x == old(x) + i; }, &x); // error: mutated and referenced
        (x, r)
    }
    spec test_mut_capture_and_imm_ref {
        ensures result_2 == 0;
    }

    /// Test: a variable mutated via capture must not be value-captured by another
    /// lambda of the same call (the capture snapshots the variable at closure
    /// construction, while inline expansion would read it at its use sites).
    fun test_mut_and_value_capture(): (u64, u64) {
        let x = 1;
        let y = 0;
        call2( // error: mutated and captured
            |i| x = x + i spec { ensures x == old(x) + i; },
            |_i| y = x spec { ensures y == x; }
        );
        (x, y)
    }
    spec test_mut_and_value_capture {
        ensures result_2 == 1;
    }

    /// Test: the same conflict through a reference-typed local binding.
    fun test_mut_capture_and_imm_carrier(): (u64, u64) {
        let x = 0;
        let rx = &x;
        let r = observe(|i| x = x + i spec { ensures x == old(x) + i; }, rx); // error: mutated and referenced
        (x, r)
    }
    spec test_mut_capture_and_imm_carrier {
        ensures result_2 == 0;
    }

    struct S has copy, drop {
        x: u64,
        y: u64,
    }

    /// Test: a chain of reference-typed bindings resolves to the root.
    fun test_mut_capture_and_chained_carrier(): (u64, u64) {
        let s = S { x: 0, y: 7 };
        let rs = &s;
        let rx = &rs.x;
        let r = observe(|i| s.x = s.x + i spec { // error: mutated and referenced
            ensures s.x == old(s).x + i;
            ensures s.y == old(s).y;
        }, rx);
        (s.x, r)
    }
    spec test_mut_capture_and_chained_carrier {
        ensures result_2 == 0;
    }

    /// Test: a carrier binding is checked with the bindings in effect at the
    /// call, regardless of later rebindings.
    fun test_carrier_rebound_after_call(): (u64, u64) {
        let x = 0;
        let y = 100;
        let rx = &x;
        let r = observe(|i| x = x + i spec { ensures x == old(x) + i; }, rx); // error: mutated and referenced
        let rx = &y;
        let _ = *rx;
        (x, r)
    }
    spec test_carrier_rebound_after_call {
        ensures result_2 == 0;
    }

    /// Test: a reference computed by a helper call derives from the references
    /// inside it; mutating the same vector through a capture conflicts.
    fun test_mut_capture_and_helper_borrow(): u64 {
        let v = vector[1u64, 2];
        let r = observe(|i| { // error: mutated through more than one argument
            let e = std::vector::borrow_mut(&mut v, 0);
            *e = *e + i;
        } spec {
            ensures v[0] == old(v)[0] + i;
            ensures v[1] == old(v)[1];
            ensures len(v) == len(old(v));
        }, std::vector::borrow(&v, 1));
        r
    }
    spec test_mut_capture_and_helper_borrow {
        ensures result == 2;
    }

    inline fun update_field(f: |u64|, r: &mut u64) {
        f(1);
        *r = *r + 1
    }
    spec update_field {
        pragma opaque;
        ensures ensures_of<f>(1);
        ensures r == old(r) + 1;
    }

    /// Test: a direct field borrow conflicts with a mutation of the whole struct.
    fun test_mut_capture_and_field_borrow(): u64 {
        let s = S { x: 0, y: 7 };
        update_field(|i| s.x = s.x + i spec { // error: mutated through more than one argument
            ensures s.x == old(s).x + i;
            ensures s.y == old(s).y;
        }, &mut s.y);
        s.x + s.y
    }
    spec test_mut_capture_and_field_borrow {
        ensures result == 9;
    }
}
