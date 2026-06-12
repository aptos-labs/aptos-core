// Tests for lambdas which modify captured variables, passed to retained
// inline-opaque functions. The lambda lifter converts such captures into
// `&mut` captures: `|i| x = x + i` becomes a closure capturing `&mut x` over
// a lifted function `fun lifted(x: &mut u64, i: u64) { *x = *x + i }`. At the
// opaque call site, the captured locations are havoced and constrained by the
// `ensures_of` conditions of the callee's spec.
module 0x42::opaque_inline_mut_capture {

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    /// Test: the effect of the lambda on the captured variable propagates
    /// through the opaque inline call via `ensures_of`.
    fun test_mut_capture(): u64 {
        let x = 0;
        call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_mut_capture {
        ensures result == 1;
    }

    /// Test: chained mutations accumulate.
    fun test_mut_capture_twice(): u64 {
        let x = 0;
        call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_mut_capture_twice {
        ensures result == 2;
    }

    /// Test: lambda modifying two captured variables.
    fun test_two_mut_captures(): u64 {
        let x = 0;
        let y = 10;
        call_once(|i| {
            x = x + i;
            y = y + i;
        } spec {
            ensures x == old(x) + i;
            ensures y == old(y) + i;
        });
        x + y
    }
    spec test_two_mut_captures {
        ensures result == 12;
    }

    /// Test: mixed value and mutating captures.
    fun test_mixed_captures(c: u64): u64 {
        let x = 0;
        call_once(|i| x = x + i + c spec { ensures x == old(x) + i + c; });
        x
    }
    spec test_mixed_captures {
        requires c < 1000;
        ensures result == 1 + c;
    }

    struct S has copy, drop {
        x: u64,
        y: u64,
    }

    /// Test: lambda assigning to a field of a captured struct variable. The
    /// whole struct is converted to a `&mut` capture; unmodified fields are
    /// framed by the lambda's spec.
    fun test_field_capture(): S {
        let s = S { x: 1, y: 7 };
        call_once(|i| s.x = s.x + i spec {
            ensures s.x == old(s).x + i;
            ensures s.y == old(s).y;
        });
        s
    }
    spec test_field_capture {
        ensures result.x == 2;
        ensures result.y == 7;
    }

    fun bump(r: &mut u64) {
        *r = *r + 1
    }
    spec bump {
        ensures r == old(r) + 1;
    }

    /// Test: lambda passing a mutable borrow of a field of a captured struct
    /// variable to a function. The borrow chain marks the whole struct as a
    /// `&mut` capture; the lambda's spec is proven from `bump`'s spec.
    fun test_field_borrow_to_fun(): S {
        let s = S { x: 1, y: 7 };
        call_once(|_i| bump(&mut s.x) spec {
            ensures s.x == old(s).x + 1;
            ensures s.y == old(s).y;
        });
        s
    }
    spec test_field_borrow_to_fun {
        ensures result.x == 2;
        ensures result.y == 7;
    }

    /// Test: lambda mutating an element of a captured vector.
    fun test_vector_capture(): vector<u64> {
        let v = vector[1, 5];
        call_once(|i| v[0] = v[0] + i spec {
            ensures v[0] == old(v)[0] + i;
            ensures v[1] == old(v)[1];
            ensures len(v) == len(old(v));
        });
        v
    }
    spec test_vector_capture {
        ensures result[0] == 2;
        ensures result[1] == 5;
    }

    inline fun update_via(f: |&mut u64|, r: &mut u64) {
        f(r)
    }
    spec update_via {
        pragma opaque;
        ensures ensures_of<f>(r);
    }

    /// Test: lambda with a `&mut` parameter combined with a modified capture.
    /// The lifted function has both a captured `&mut` (the counter) and a
    /// non-captured `&mut` parameter (from the function type), whose post-state
    /// slots must be merged in parameter order.
    fun test_mut_param_and_mut_capture(p: u64): u64 {
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
    spec test_mut_param_and_mut_capture {
        requires p < 1000;
        ensures result == p + 3;
    }
}
