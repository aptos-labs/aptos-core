// flag: --infer-lambda-specs
//
// A lambda which modifies a captured variable gets its `&mut`-converted
// signature processed by spec inference: WP produces `ensures x == old(x) + i;
// aborts_if old(x) + i > MAX_U64;` so the caller proves the captured variable's
// post-state.
module 0x42::inferred_mut_capture {

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    fun test_mut_capture(): u64 {
        let x = 5;
        call_once(|i| x = x + i);
        x
    }
    spec test_mut_capture {
        ensures result == 6;
    }

    fun test_mut_capture_twice(): u64 {
        let x = 0;
        call_once(|i| x = x + i);
        call_once(|i| x = x + i);
        x
    }
    spec test_mut_capture_twice {
        ensures result == 2;
    }
}
