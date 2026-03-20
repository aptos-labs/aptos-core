// Test spec inference for loops
// Set a tight timeout to test that inference does not cause solver timeouts
// flag: -T=20
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
            invariant sum == i * (i + 1) / 2;
            invariant i <= n;
        };
        sum
    }

    // Count down to zero
    fun count_down(n: u64): u64 {
        let count = n;
        while (count > 0) {
            count = count - 1;
        };
        count
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
    spec inc_n_times {
        pragma unroll = 5;
    }

    // Double a value n times through a reference
    spec module {
        fun pow2(n: u64): u64 {
            if (n == 0) { 1 } else { pow2(n - 1) * 2 }
        }
        // pow2 is always positive
        axiom forall a: u64 {pow2(a)}: pow2(a) >= 1;
        // pow2 is monotone
        axiom forall a: u64, b: u64 {pow2(a), pow2(b)}: a <= b ==> pow2(a) <= pow2(b);
    }

    // Default: infer both ensures and aborts_if
    fun double_n_times(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r * 2;
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant r == old(r) * pow2(i);
        };
    }
    spec double_n_times {
        pragma verify = false; // timeout
    }

    // Ensures only: skip aborts inference
    fun double_n_times_ensures(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r * 2;
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant r == old(r) * pow2(i);
        };
    }
    spec double_n_times_ensures {
        pragma inference = only_ensures;
        pragma verify = false; // timeout
    }

    // Aborts only: skip ensures inference
    fun double_n_times_aborts(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r * 2;
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant r == old(r) * pow2(i);
        };
    }
    spec double_n_times_aborts {
        pragma inference = only_aborts;
        pragma verify = false; // timeout
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
    fun inc_global_n_times(addr: address, n: u64) acquires Counter {
        let i = 0;
        while (i < n) {
            let c = &mut Counter[addr];
            c.value = c.value + 1;
            i = i + 1;
        };
    }
    spec inc_global_n_times {
        // Test with unrolling. Notice that the inferred spec is incomplete and will
        // fail verification.
        pragma unroll = 3;
    }

    // Increment global counter n times with a user-provided loop invariant
    fun inc_global_with_invariant(addr: address, n: u64) acquires Counter {
        let i = 0;
        let start = Counter[addr].value;
        while (i < n) {
            let c = &mut Counter[addr];
            c.value = c.value + 1;
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant global<Counter>(addr).value == start + i;
        };
    }

    // ==================== Nested Loops ====================

    // ==================== Agent-Inferred Invariants ====================

    // Loop with agent-inferred invariant
    fun agent_invariant_loop(n: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) {
            i = i + 1;
            sum = sum + i;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] sum == i * (i + 1) / 2;
        };
        sum
    }

    // ==================== Empty Spec Block ====================

    // Empty spec block: conditions should be injected inside the braces.
    fun empty_spec_block(n: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) {
            i = i + 1;
            sum = sum + i;
        } spec {
            invariant i <= n;
            invariant sum == i * (i + 1) / 2;
        };
        sum
    }
    spec empty_spec_block {}

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
                invariant j <= n;
                invariant count == i * n + j;
            };
            i = i + 1;
        } spec {
            invariant i <= m;
            invariant count == i * n;
        };
        count
    }
}
