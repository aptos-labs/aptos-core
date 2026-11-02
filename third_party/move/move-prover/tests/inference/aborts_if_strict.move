// flag: --aborts-if-is-strict
// With strict aborts, an abort characterization WP cannot make exact is an
// error rather than an `aborts_if_is_partial` contract.
module 0x42::aborts_if_strict {
    struct Counter has key {
        n: u64,
    }

    // The loop writes global memory, so its abort conditions do not survive
    // the loop's memory havoc.
    fun bump(a: address, k: u64) {
        let i = 0;
        while (i < k) {
            Counter[a].n += 1;
            i += 1;
        } spec {
            invariant i <= k;
        };
    }
}
