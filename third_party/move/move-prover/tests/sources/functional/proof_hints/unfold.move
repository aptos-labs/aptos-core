// flag: --language-version=2.4
module 0x42::proof_hint_unfold {

    // ============================================================
    // Basic unfold on a spec function.

    spec fun helper(x: u64): bool { x > 0 }

    fun check_positive(x: u64): bool {
        x > 0
    }
    spec check_positive {
        ensures result == helper(x);

        proof {
            unfold helper;
        }
    }

    // ============================================================
    // Unfold with depth: recursive spec function.

    spec fun sum(n: u64): u64 {
        if (n == 0) { 0 } else { n + sum(n - 1) }
    }

    fun test_depth_1(n: u64): u64 {
        if (n == 0) { 0 }
        else if (n == 1) { 1 }
        else if (n == 2) { 3 }
        else { n }
    }
    spec test_depth_1 {
        requires n <= 2;
        ensures result == sum(n);

        proof {
            unfold sum;
        }
    }

    fun test_depth_3(n: u64): u64 {
        if (n == 0) { 0 }
        else if (n == 1) { 1 }
        else if (n == 2) { 3 }
        else { n }
    }
    spec test_depth_3 {
        requires n <= 2;
        ensures result == sum(n);

        proof {
            unfold sum depth 3;
        }
    }

    // ============================================================
    // Error: unfold on a non-existent spec function.

    fun test_unfold_nonexistent(x: u64): u64 {
        x
    }
    spec test_unfold_nonexistent {
        proof {
            unfold nonexistent_function;
        }
    }
}
