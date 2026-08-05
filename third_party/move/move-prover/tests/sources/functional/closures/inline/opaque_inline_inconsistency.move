// flag: --check-inconsistency
//
// Guards against vacuity in the verification context created for retained
// inline-opaque calls: the havocs and `ensures_of` assumptions injected for
// value, immutable-reference, and `&mut` captures (including combined with
// non-captured `&mut` parameters) must be satisfiable.
module 0x42::opaque_inline_inconsistency {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun value_capture(x: u64, c: u64): u64 {
        apply(|y| y + c spec { ensures result == y + c; }, x)
    }
    spec value_capture {
        ensures result == x + c;
    }

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    fun mut_capture(): u64 {
        let x = 0;
        call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec mut_capture {
        ensures result == 1;
    }

    inline fun update_via(f: |&mut u64|, r: &mut u64) {
        f(r)
    }
    spec update_via {
        pragma opaque;
        ensures ensures_of<f>(r);
    }

    fun mut_capture_and_mut_param(p: u64): u64 {
        let count = 0;
        let v = p;
        update_via(|q| {
            *q = *q + 1;
            count = count + 2;
        } spec {
            ensures q == old(q) + 1;
            ensures count == old(count) + 2;
        }, &mut v);
        v + count
    }
    spec mut_capture_and_mut_param {
        requires p < 1000;
        ensures result == p + 3;
    }
}
