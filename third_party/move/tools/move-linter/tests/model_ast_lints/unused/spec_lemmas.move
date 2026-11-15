module 0x42::m {
    public fun double(x: u64): u64 {
        x + x
    }

    public fun triple(x: u64): u64 {
        x + x + x
    }

    // Still reported: an ordinary private function with no callers.
    fun unused_helper(): u64 {
        1
    }
}

spec 0x42::m {
    spec fun spec_double(x: num): num {
        2 * x
    }

    spec fun spec_triple(x: num): num {
        3 * x
    }

    // Applied directly from a function's proof block.
    spec lemma double_is_sum(x: num) {
        ensures spec_double(x) == x + x;
    }

    // Applied only from another lemma's proof block.
    spec lemma triple_is_sum(x: num) {
        ensures spec_triple(x) == x + x + x;
    }

    spec lemma triple_via_double(x: num) {
        ensures spec_triple(x) == spec_double(x) + x;
    } proof {
        apply triple_is_sum(x);
        apply double_is_sum(x);
    }

    // Applied only under `forall .. apply`.
    spec lemma double_monotone(x: num, y: num) {
        requires x <= y;
        ensures spec_double(x) <= spec_double(y);
    }

    // Applied by nothing; still verified by the prover on its own.
    spec lemma double_nonneg(x: num) {
        requires x >= 0;
        ensures spec_double(x) >= 0;
    }

    spec double {
        aborts_if x + x > MAX_U64;
        ensures result == spec_double(x);
    } proof {
        apply double_is_sum(x);
        forall a: num, b: num {spec_double(a), spec_double(b)} apply double_monotone(a, b);
    }

    spec triple {
        aborts_if x + x + x > MAX_U64;
        ensures result == spec_triple(x);
    } proof {
        apply triple_via_double(x);
    }
}
