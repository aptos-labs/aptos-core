module 0x42::m {
    // Warns: the upper bound `i` refers to the enclosing parameter `i`, which
    // the loop iterator `i` shadows. Its meaning is version-dependent (it reads
    // the iterator in older versions, the enclosing `i` in newer ones).
    fun ub_self_ref(i: u64): u64 {
        let s = 0;
        for (i in 0..i) {
            s = s + i;
        };
        s
    }

    // No warning: only the upper bound's meaning changed across versions. The
    // lower bound is evaluated before the iterator is bound in every version, so
    // `for (i in i..n)` reads the enclosing `i` regardless.
    fun lb_self_ref(i: u64, n: u64): u64 {
        let s = 0;
        for (i in i..n) {
            s = s + i;
        };
        s
    }

    // No warning: a `_` iterator gets a fresh name that no bound can reference.
    fun underscore_ok(n: u64): u64 {
        let s = 0;
        for (_ in 0..n) {
            s = s + 1;
        };
        s
    }

    // No warning: the bounds do not mention the iterator's name.
    fun no_shadow_ok(n: u64): u64 {
        let s = 0;
        for (i in 0..n) {
            s = s + i;
        };
        s
    }
}
