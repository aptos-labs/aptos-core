// A GENERIC retained inline-opaque callee with a `&mut`-capturing closure:
// spec instantiation re-mints node ids for type-parameter-dependent nodes,
// and the live fun-value clone's save exemption must survive that (else the
// call site would assume the entry value for the post state).
module 0x42::opaque_inline_generic_callee {

    inline fun call_once<T>(f: |&T| has copy + drop, x: &T) {
        f(x)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(x);
    }

    /// Test: the captured accumulator advances through the generic callee.
    fun test_generic_mut_capture(): u64 {
        let s = 0;
        call_once(|e| s = s + *e spec { ensures s == old(s) + e; }, &2);
        s
    }
    spec test_generic_mut_capture {
        ensures result == 2;
    }
}
