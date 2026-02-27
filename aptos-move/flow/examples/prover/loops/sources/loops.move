module 0x42::loops {

    // ==================== Simple Counting Loops ====================

    // Simple loop summing 1 to n
    fun sum_to_n(n: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) {
            i = i + 1;
            sum = sum + i;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] sum == sum_up_to(i);
        };
        sum
    }
    spec sum_to_n {
        ensures [inferred] result == sum_up_to(n);
        aborts_if [inferred] sum_up_to(n) > MAX_U64;
    } proof {
        forall x: u64, y: u64 {sum_up_to(x), sum_up_to(y)} apply sum_up_to_monotone(x, y);
    }

    /// Recursive definition: sum of 1 + 2 + ... + n
    spec fun sum_up_to(n: u64): u64 {
        if (n == 0) { 0 } else { n + sum_up_to(n - 1) }
    }

    /// sum_up_to is monotonically increasing
    spec lemma sum_up_to_monotone(x: u64, y: u64) {
        requires x <= y;
        ensures sum_up_to(x) <= sum_up_to(y);
    } proof {
        if (x < y) {
            apply sum_up_to_monotone(x, y - 1);
        }
    }


    // Count down to zero
    fun count_down(n: u64): u64 {
        let count = n;
        while (count > 0) {
            count = count - 1;
        } spec {
            invariant [inferred] count <= n;
        };
        count
    }
    spec count_down {
        ensures [inferred] result == 0;
    }


    // ==================== Mutation in Loops ====================

    // Increment a reference in a loop
    fun inc_n_times(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r + 1;
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] r == old(r) + i;
        };
    }
    spec inc_n_times {
        ensures [inferred] r == old(r) + n;
        aborts_if [inferred] r + n > MAX_U64;
    }


    // Default: infer both ensures and aborts_if
    fun double_n_times(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r * 2;
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] r == doubled(old(r), i);
        };
    }
    spec double_n_times {
        ensures [inferred] r == doubled(old(r), n);
        aborts_if [inferred] doubled(r, n) > MAX_U64;
    } proof {
        forall v: u64, x: u64, y: u64 {doubled(v, x), doubled(v, y)} apply doubled_monotone(v, x, y);
    }

    /// Iterated doubling: doubled(v, n) = v * 2^n, avoiding multiplication of two variables
    spec fun doubled(v: u64, n: u64): u64 {
        if (n == 0) { v } else { 2 * doubled(v, n - 1) }
    }

    /// doubled is always non-negative
    spec lemma doubled_nonneg(v: u64, n: u64) {
        ensures doubled(v, n) >= 0;
    } proof {
        if (n > 0) {
            apply doubled_nonneg(v, n - 1);
        }
    }

    /// doubled is monotone in the iteration count
    spec lemma doubled_monotone(v: u64, x: u64, y: u64) {
        requires x <= y;
        ensures doubled(v, x) <= doubled(v, y);
    } proof {
        if (x < y) {
            apply doubled_nonneg(v, y - 1);
            apply doubled_monotone(v, x, y - 1);
        }
    }


    // Ensures only: skip aborts inference
    fun double_n_times_ensures(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r * 2;
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] r == doubled(old(r), i);
        };
    }
    spec double_n_times_ensures {
        ensures [inferred] r == doubled(old(r), n);
        aborts_if [inferred] doubled(r, n) > MAX_U64;
    } proof {
        forall v: u64, x: u64, y: u64 {doubled(v, x), doubled(v, y)} apply doubled_monotone(v, x, y);
    }


    // Aborts only: skip ensures inference
    fun double_n_times_aborts(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r * 2;
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] r == doubled(old(r), i);
        };
    }
    spec double_n_times_aborts {
        ensures [inferred] r == doubled(old(r), n);
        aborts_if [inferred] doubled(r, n) > MAX_U64;
    } proof {
        forall v: u64, x: u64, y: u64 {doubled(v, x), doubled(v, y)} apply doubled_monotone(v, x, y);
    }


    // ==================== Loop with Conditional ====================

    // Conditionally increment in loop
    fun cond_inc_loop(r: &mut u64, n: u64, do_inc: bool) {
        let i = 0;
        while (i < n) {
            if (do_inc) {
                *r = *r + 1;
            };
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] r == if (do_inc) { old(r) + i } else { old(r) };
        };
    }
    spec cond_inc_loop {
        ensures [inferred] r == if (do_inc) { old(r) + n } else { old(r) };
        aborts_if [inferred] do_inc && r + n > MAX_U64;
    }


    // ==================== Global Mutation in Loops ====================

    struct Counter has key {
        value: u64,
    }

    // Increment global counter n times in a loop (unrolled)
    fun inc_global_n_times(addr: address, n: u64) {
        let init = if (n > 0) { Counter[addr].value } else { 0 };
        let i = 0;
        while (i < n) {
            let c = &mut Counter[addr];
            c.value = c.value + 1;
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] n > 0 ==> exists<Counter>(addr);
            invariant [inferred] n > 0 ==> global<Counter>(addr).value == init + i;
        };
    }
    spec inc_global_n_times {
        aborts_if [inferred] n > 0 && !exists<Counter>(addr);
        aborts_if [inferred] n > 0 && global<Counter>(addr).value + n > MAX_U64;
        ensures [inferred] n > 0 ==> global<Counter>(addr).value == old(global<Counter>(addr)).value + n;
    }


    // Increment global counter n times with a user-provided loop invariant
    fun inc_global_with_invariant(addr: address, n: u64) {
        let init = if (n > 0) { Counter[addr].value } else { 0 };
        let i = 0;
        while (i < n) {
            let c = &mut Counter[addr];
            c.value = c.value + 1;
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] n > 0 ==> exists<Counter>(addr);
            invariant [inferred] n > 0 ==> global<Counter>(addr).value == init + i;
        };
    }
    spec inc_global_with_invariant {
        aborts_if [inferred] n > 0 && !exists<Counter>(addr);
        aborts_if [inferred] n > 0 && global<Counter>(addr).value + n > MAX_U64;
        ensures [inferred] n > 0 ==> global<Counter>(addr).value == old(global<Counter>(addr)).value + n;
    }


    // ==================== Nested Loops ====================

    // Simple nested loop counting iterations
    fun nested_count(m: u64, n: u64): u64 {
        let count = 0;
        let i = 0;
        while (i < m) {
            let j = 0;
            while (j < n) {
                count = count + 1;
                j = j + 1;
            } spec {
                invariant [inferred] j <= n;
                invariant [inferred] count == i * n + j;
            };
            i = i + 1;
        } spec {
            invariant [inferred] i <= m;
            invariant [inferred] count == i * n;
        };
        count
    }
    spec nested_count {
        ensures [inferred] result == m * n;
        aborts_if [inferred] m * n > MAX_U64;
    }

}
