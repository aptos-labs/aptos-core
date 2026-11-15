// A behavioral predicate over a function value needs the function's parameter tuple as a
// Boogie type. A callback with more parameters than the prelude's largest tuple cannot
// carry a contract at all: the generated `$Tuple<n>` is undeclared and Boogie fails name
// resolution, so a caller stating `aborts_of<f>(..)` on it is rejected before verification.
// The prelude now declares tuples up to twenty elements. This exercises a fifteen-parameter
// callback (an exchange's settle callback has that many) with an abort condition stated and
// discharged through the predicate, plus a negative case.
module 0x42::behavioral_wide_arity {
    struct Callbacks has copy, drop, store {
        settle: |u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64| u64
            has copy + drop + store
    }

    #[persistent]
    fun settle(
        a: u64, b: u64, c: u64, _d: u64, e: u64, _f: u64, g: u64, _h: u64,
        i: u64, _j: u64, k: u64, _l: u64, m: u64, _n: u64, o: u64
    ): u64 {
        assert!(a != b, 1);
        assert!(o > 0, 2);
        a + c + e + g + i + k + m
    }
    spec settle {
        pragma opaque;
        aborts_if a == b;
        aborts_if o == 0;
        aborts_if a + c + e + g + i + k + m > MAX_U64;
        ensures result == a + c + e + g + i + k + m;
    }

    fun callbacks(): Callbacks {
        Callbacks { settle: |a, b, c, d, e, f, g, h, i, j, k, l, m, n, o| settle(a, b, c, d, e, f, g, h, i, j, k, l, m, n, o) }
    }

    fun run(cb: &Callbacks, a: u64, b: u64, o: u64): u64 {
        (cb.settle)(a, b, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, o)
    }
    spec run {
        aborts_if aborts_of<cb.settle>(a, b, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, o);
        ensures result == result_of<cb.settle>(a, b, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, o);
    }

    /// Through the concrete callbacks the predicate resolves to `settle`'s contract.
    fun run_concrete(a: u64, b: u64, o: u64): u64 {
        let cb = callbacks();
        run(&cb, a, b, o)
    }
    spec run_concrete {
        aborts_if a == b;
        aborts_if o == 0;
        ensures result == a;
    }

    /// Negative: claims a self-trade does not abort.
    fun run_concrete_wrong(a: u64, b: u64, o: u64): u64 {
        let cb = callbacks();
        run(&cb, a, b, o)
    }
    spec run_concrete_wrong {
        aborts_if o == 0;
        ensures result == a;
    }
}
