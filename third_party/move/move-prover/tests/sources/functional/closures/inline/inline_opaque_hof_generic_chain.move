// (See issue #20273.)
// The once-and-for-all generic chain lemma: paired with `for_each_ref`,
// generic in the step function `g`.
module 0x42::inline_opaque_hof_generic_chain {
    use std::vector;

    spec fun apply_all(f: |&u64| has copy + drop, v: vector<u64>, end: u64): |&u64| has copy + drop {
        if (end == 0) f else fun_post_of<apply_all(f, v, end - 1)>(v[end - 1])
    }

    // Value-level fold of g's write effect — generic in g.
    spec fun iterate(g: |&mut u64, &u64| has drop, c0: u64, v: vector<u64>, k: u64): u64 {
        if (k == 0) c0 else write_of<g>(iterate(g, c0, v, k - 1), v[k - 1])
    }

    inline fun for_each_ref(v: &vector<u64>, f: |&u64| has copy + drop) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= len(v);
            invariant f == apply_all(old(f), v, i);
        };
    }
    spec for_each_ref {
        pragma opaque;
        requires forall j in 0..len(v): !aborts_of<apply_all(f, v, j)>(v[j]);
        aborts_if false;
        ensures f == apply_all(old(f), v, len(v));
    }

    spec module {
        // Generic chain lemma: after k applications, the captured state is
        // the k-fold of g's (deterministic) write effect.
        lemma for_each_chain(
            f: |&u64| has copy + drop,
            g: |&mut u64, &u64| has drop,
            v: vector<u64>,
            k: u64
        ) {
            requires partial_of<f>(g);
            requires forall c: u64, e: u64, w: u64:
                ensures_of<g>(c, e, w) ==> w == write_of<g>(c, e);
            requires k <= len(v);
            ensures partial_of<apply_all(f, v, k)>(g);
            ensures captures_of<apply_all(f, v, k)>(g)
                 == iterate(g, captures_of<f>(g), v, k);
        } proof {
            if (k > 0) {
                apply for_each_chain(f, g, v, k - 1);
            }
        }
    }

    // ================= use case: sum =================

    fun step(s: &mut u64, e: &u64) {
        *s = *s + *e;
    }
    spec step {
        aborts_if s + e > MAX_U64;
        ensures s == old(s) + e;
    }

    spec fun spec_sum(v: vector<u64>, k: u64): u64 {
        if (k == 0) 0 else spec_sum(v, k - 1) + v[k - 1]
    }

    spec module {
        // Step-specific arithmetic (inherent per accumulator operation).
        lemma iterate_step_is_sum(v: vector<u64>, c0: u64, k: u64) {
            requires k <= len(v);
            ensures iterate(step, c0, v, k) == c0 + spec_sum(v, k);
        } proof {
            if (k > 0) {
                apply iterate_step_is_sum(v, c0, k - 1);
            }
        }

        lemma sum_prefix_bound(v: vector<u64>, j: u64, k: u64) {
            requires j < k;
            requires k <= len(v);
            ensures spec_sum(v, j) + v[j] <= spec_sum(v, k);
        } proof {
            if (j + 1 < k) {
                apply sum_prefix_bound(v, j, k - 1);
            }
        }
    }

    /// Test: a PURE lambda through the chain-spec'd HOF — non-capturing
    /// values do not advance (fun_post_of is the identity per variant), so
    /// the chain collapses and the abort requires discharges (bounded
    /// length: the collapse is per unfolded step; symbolic depth would need
    /// a lemma like everything else).
    fun noop_all(v: &vector<u64>): u64 {
        for_each_ref(v, |_e| ());
        7
    }
    spec noop_all {
        requires len(v) < 8;
        aborts_if false;
        ensures result == 7;
    }

    fun sum_all(v: &vector<u64>): u64 {
        let s = 0;
        for_each_ref(v, |e| step(&mut s, e));
        s
    }
    spec sum_all {
        requires spec_sum(v, len(v)) <= MAX_U64;
        aborts_if false;
        ensures result == spec_sum(v, len(v));
    } proof {
        forall f: |&u64| has copy + drop, k: u64 {apply_all(f, v, k)}
            apply for_each_chain(f, step, v, k);
        forall c0: u64, k: u64 {iterate(step, c0, v, k)} apply iterate_step_is_sum(v, c0, k);
        forall j: u64, k: u64 {spec_sum(v, j), spec_sum(v, k)} apply sum_prefix_bound(v, j, k);
    }
}
