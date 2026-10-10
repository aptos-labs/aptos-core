// Context parameters of specialized spec functions are named by the
// lambda's free-variable symbols. When such a symbol collides with a
// retained parameter of the spec function, or is shadowed by a binder in
// its body around the substitution site, the context parameter must be
// freshened — otherwise the specialized body conflates the caller's
// captured value with the like-named parameter or binder.
module 0x42::spec_fun_ctx_collision {
    use std::vector;

    spec fun spec_apply(f: |u64| u64, v: u64): u64 {
        result_of<f>(v)
    }

    inline fun apply(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            assert r == spec_apply(f, x);
        };
        r
    }

    // The captured local `v` collides with the retained parameter `v` of
    // `spec_apply`.
    fun add_captured_local(w: u64): u64 {
        let v = 3;
        apply(|x| x + v, w)
    }
    spec add_captured_local {
        requires w < 1000;
        ensures result == w + 3;
    }

    // Non-vacuity canary for the collision case.
    fun add_captured_local_wrong(w: u64): u64 {
        let v = 3;
        apply(|x| x + v, w)
    }
    spec add_captured_local_wrong {
        requires w < 1000;
        ensures result == w + w; // error: off by w - 3
    }

    // The captured symbol is a parameter of the enclosing function (a
    // temporary in the lambda material), also named like the retained
    // parameter.
    fun add_captured_param(v: u64): u64 {
        apply(|x| x + v, 7)
    }
    spec add_captured_param {
        requires v < 1000;
        ensures result == 7 + v;
    }

    // A binder in the spec function's body around the substitution site
    // shadows the capture: `result_of<f>` is resolved under `let v`, so the
    // spliced captured `v` must not be captured by that binder.
    spec fun spec_apply_succ(f: |u64| u64, u: u64): u64 {
        {
            let v = u + 1;
            result_of<f>(v)
        }
    }

    inline fun apply_succ(f: |u64| u64, x: u64): u64 {
        let r = f(x + 1);
        spec {
            assert r == spec_apply_succ(f, x);
        };
        r
    }

    fun add_shadowed(w: u64): u64 {
        let v = 3;
        apply_succ(|y| y + v, w)
    }
    spec add_shadowed {
        requires w < 1000;
        ensures result == w + 1 + 3;
    }

    // Recursion under freshening: the capture `init` collides with the
    // retained parameter `init` of `spec_fold`. The recursive redirect must
    // stay on the single specialization — passing the retained `init` and
    // the freshened context parameter apart — and the ensures literal must
    // unify with it.
    spec fun spec_fold(f: |u64, u64| u64, v: vector<u64>, init: u64, end: u64): u64 {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    inline fun fold(v: &vector<u64>, init: u64, f: |u64, u64| u64): u64 {
        let acc = init;
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            acc = f(acc, *vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant acc == spec_fold(f, v, init, i);
        };
        acc
    }

    fun sum_scaled(u: &vector<u64>, init: u64): u64 {
        fold(u, 0, |acc, e| acc + e * init)
    }
    spec sum_scaled {
        pragma aborts_if_is_partial;
        ensures result == spec_fold(|acc, e| acc + e * init, u, 0, len(u));
    }
}
