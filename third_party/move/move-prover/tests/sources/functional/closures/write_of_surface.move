// Surface `write_of<g, j>(args)`: the post-state of `g`'s j-th `&mut`
// parameter (j defaults to 0), a value-level step function usable to define
// fold-style spec functions over captured-state updates (see issue #20273).
module 0x42::write_of_surface {

    fun step(s: &mut u64, e: &u64) {
        *s = *s + *e;
    }
    spec step {
        aborts_if s + e > MAX_U64;
        ensures s == old(s) + e;
    }

    /// Test: nested `write_of` describes sequential applications.
    fun run2(a: u64, b: u64): u64 {
        let s = 0;
        step(&mut s, &a);
        step(&mut s, &b);
        s
    }
    spec run2 {
        requires a < 1000 && b < 1000;
        ensures result == write_of<step>(write_of<step>(0, a), b);
    }

    spec fun spec_sum(v: vector<u64>, k: u64): u64 {
        if (k == 0) 0 else spec_sum(v, k - 1) + v[k - 1]
    }

    /// A value-level fold of `step`'s write effect.
    spec fun iterate(v: vector<u64>, c0: u64, k: u64): u64 {
        if (k == 0) c0 else write_of<step>(iterate(v, c0, k - 1), v[k - 1])
    }

    spec module {
        /// The fold of `step` is the prefix sum.
        lemma iterate_is_sum(v: vector<u64>, c0: u64, k: u64) {
            requires k <= len(v);
            ensures iterate(v, c0, k) == c0 + spec_sum(v, k);
        } proof {
            if (k > 0) {
                apply iterate_is_sum(v, c0, k - 1);
            }
        }
    }
}
