// The compositional form of the accumulator use case: unbounded symbolic
// length, closed via `partial_of`/`captures_of` chain lemmas paired with the
// named step function, instantiated by `forall .. apply` at the call site.
// (See issue #20273.)
// Unbounded symbolic-length accumulator through a chain-spec'd HOF,
// composed via a step-paired recursive lemma.
module 0x42::inline_opaque_hof_symbolic_sum {
    use std::vector;

    spec fun spec_sum(v: vector<u64>, k: u64): u64 {
        if (k == 0) 0 else spec_sum(v, k - 1) + v[k - 1]
    }

    spec fun apply_all(f: |&u64| has copy + drop, v: vector<u64>, end: u64): |&u64| has copy + drop {
        if (end == 0) f else fun_post_of<apply_all(f, v, end - 1)>(v[end - 1])
    }

    fun step(s: &mut u64, e: &u64) {
        *s = *s + *e;
    }
    spec step {
        aborts_if s + e > MAX_U64;
        ensures s == old(s) + e;
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
        // Chain lemma, paired with `step`: after k applications the captured
        // accumulator has grown by the prefix sum.
        lemma chain_sum(f: |&u64| has copy + drop, v: vector<u64>, k: u64) {
            requires partial_of<f>(step);
            requires k <= len(v);
            ensures partial_of<apply_all(f, v, k)>(step);
            ensures captures_of<apply_all(f, v, k)>(step)
                 == captures_of<f>(step) + spec_sum(v, k);
        } proof {
            if (k > 0) {
                apply chain_sum(f, v, k - 1);
            }
        }

        // A prefix sum plus its next element is bounded by any longer
        // prefix sum.
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
        forall f: |&u64| has copy + drop, k: u64 {apply_all(f, v, k)} apply chain_sum(f, v, k);
        forall j: u64, k: u64 {spec_sum(v, j), spec_sum(v, k)} apply sum_prefix_bound(v, j, k);
    }
}
