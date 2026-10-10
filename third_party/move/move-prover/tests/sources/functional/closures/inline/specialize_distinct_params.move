// Distinct function parameters passed to the same spec function at the same
// argument position must resolve to distinct specializations: the
// specialization identity within a context is the bound parameter, not just
// the argument position. (Spec-equivalent lambdas still unify globally onto
// one specialized function.)
module 0x42::specialize_distinct_params {

    spec fun spec_apply(f: |u64| u64, v: u64): u64 {
        result_of<f>(v)
    }

    inline fun apply2(f: |u64| u64, g: |u64| u64, x: u64): u64 {
        let r1 = f(x);
        let r2 = g(x);
        spec {
            assert r1 == spec_apply(f, x);
            assert r2 == spec_apply(g, x);
        };
        r1 + r2
    }

    fun call(x: u64): u64 {
        apply2(|a| a + 1, |a| a + 2, x)
    }
    spec call {
        requires x < 1000;
        ensures result == 2 * x + 3;
    }

    // Both parameters bound to spec-equivalent lambdas: the specializations
    // unify onto one function.
    fun call_same(x: u64): u64 {
        apply2(|a| a + 1, |a| a + 1, x)
    }
    spec call_same {
        requires x < 1000;
        ensures result == 2 * x + 2;
    }

    // Non-vacuity canary.
    fun call_wrong(x: u64): u64 {
        apply2(|a| a + 1, |a| a + 2, x)
    }
    spec call_wrong {
        requires x < 1000;
        ensures result == 2 * x + 4; // error: off by one
    }
}
