module 0x42::m {
    // Only `invariant` is allowed in a for-loop header spec block; `assert` is rejected.
    fun bad_assert(n: u64): u64 {
        let s = 0;
        for (i in 0..n spec { assert s == i; }) {
            s = s + i;
        };
        s
    }
}
