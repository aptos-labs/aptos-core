// A `folds_of` invariant forwarded through a wrapper HOF, compiled in
// regular (non-verify) mode: the anchored deferral machinery is
// verify-only — no `FoldsCaptureAnchor` markers or deferred occurrences
// may leak into regular compilation, where conditions with unresolved
// behavioral predicates are replaced by `true`.
module 0x42::behavior_folds_of_wrapper {

    inline fun each_idx(n: u64, f: |u64|) {
        let i = 0;
        while (i < n) {
            f(i);
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant folds_of<f>(|j| j, i);
        };
    }

    /// Forwarding wrapper with a pure (aborting) prelude.
    inline fun each_idx_shifted(n: u64, f: |u64|) {
        each_idx(n, |x| f(x + 1));
    }

    fun sum_shifted(n: u64): u64 {
        let sum = 0;
        each_idx_shifted(n, |x| sum = sum + x);
        sum
    }
}
