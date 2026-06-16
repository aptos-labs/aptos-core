// In verify mode, a lambda passed to a retained inline-opaque function may not
// modify a captured variable: the lambda lifter rejects it (a captured variable
// is passed by value, so a modification could not be observed by the caller). In
// normal compilation the inline function is expanded, so the lambda is never
// lifted and no error arises.
module 0x42::retained_mut_capture {

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    fun caller(): u64 {
        let x = 0;
        call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec caller {
        ensures result == 1;
    }
}
