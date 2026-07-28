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

    struct V has drop {
        items: vector<u64>,
    }

    inline fun count(self: &V, f: |u64|) {
        f(1)
    }
    spec count {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    /// The motivating shape: receiver-style call with compound assignment on a
    /// context local.
    fun receiver_caller(v: &V): u64 {
        let n = 0;
        v.count(|_x| n += 1 spec { ensures n == old(n) + 1; });
        n
    }
    spec receiver_caller {
        ensures result == 1;
    }

    /// Modification of a captured parameter of the enclosing function; behaves
    /// like a local.
    fun param_caller(x: u64): u64 {
        call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec param_caller {
        requires x < 1000;
        ensures result == x + 1;
    }

    /// A converted variable that is also read through an immutable borrow
    /// (rewritten to a freeze of the `&mut` parameter).
    fun freeze_caller(): u64 {
        let x = 1;
        call_once(|i| x = x + read(&x) + i spec { ensures x == old(x) * 2 + i; });
        x
    }
    spec freeze_caller {
        ensures result == 3;
    }

    fun read(r: &u64): u64 {
        *r
    }
    spec read {
        ensures result == r;
    }

    inline fun call_once_copy(f: |u64| has copy + drop) {
        f(1)
    }
    spec call_once_copy {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    /// Conversion into a closure satisfying a `copy` ability bound: the
    /// lifted closure captures `&mut x` and remains linear at the model
    /// level, but the bound itself is admitted.
    fun copy_bound_caller(): u64 {
        let x = 0;
        call_once_copy(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec copy_bound_caller {
        ensures result == 1;
    }
}
