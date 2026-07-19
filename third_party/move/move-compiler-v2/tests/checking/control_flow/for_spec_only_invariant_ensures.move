module 0x42::m {
    // Only `invariant` is allowed in a for-loop header spec block; `ensures` is rejected.
    fun bad_ensures(n: u64): u64 {
        let s = 0;
        for (i in 0..n spec { ensures s == i; }) {
            s = s + i;
        };
        s
    }
}
