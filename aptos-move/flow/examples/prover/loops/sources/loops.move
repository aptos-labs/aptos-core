module 0x42::loops {

    // ==================== Simple Counting Loops ====================

    // Simple loop summing 1 to n
    fun sum_to_n(n: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) {
            i = i + 1;
            sum = sum + i;
        };
        sum
    }


    // Count down to zero
    fun count_down(n: u64): u64 {
        let count = n;
        while (count > 0) {
            count = count - 1;
        } spec {
            invariant count <= n;
        };
        count
    }
    spec count_down {
        aborts_if false;
        ensures result == 0;
    }

    // ==================== Mutation in Loops ====================

    // Increment a reference in a loop
    fun inc_n_times(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r + 1;
            i = i + 1;
        };
    }


    // Default: infer both ensures and aborts_if
    fun double_n_times(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r * 2;
        } spec {
            invariant [inferred] i == 0;
        };
    }
    spec double_n_times {
        aborts_if n > 0;
        ensures r == old(r);
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
        };
    }


    // ==================== Global Mutation in Loops ====================

    struct Counter has key {
        value: u64,
    }

    // Increment global counter n times in a loop (unrolled)
    fun inc_global_n_times(addr: address, n: u64) {
        let i = 0;
        while (i < n) {
            let c = &mut Counter[addr];
            c.value = c.value + 1;
            i = i + 1;
        };
    }


    // Increment global counter n times with a user-provided loop invariant
    fun inc_global_with_invariant(addr: address, n: u64) {
        let i = 0;
        while (i < n) {
            let c = &mut Counter[addr];
            c.value = c.value + 1;
            i = i + 1;
        };
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
            };
            i = i + 1;
        };
        count
    }

}
