// In verify mode, lambdas passed to retained inline-opaque functions may modify
// captured variables. The lambda lifter converts such captures into `&mut`
// parameters: `|i| x = x + i` becomes a closure capturing `&mut x` over a lifted
// function with body `*x = *x + i`. In normal compilation mode this is an error.
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

    struct S has copy, drop {
        x: u64,
        y: u64,
    }

    fun field_caller(): u64 {
        let s = S { x: 1, y: 7 };
        call_once(|i| s.x = s.x + i spec {
            ensures s.x == old(s).x + i;
            ensures s.y == old(s).y;
        });
        s.x + s.y
    }
    spec field_caller {
        ensures result == 9;
    }
}
