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
    spec sum_to_n(n: u64): u64 {
        ensures [inferred] result == n * (n + 1) / 2;
        aborts_if [inferred] n > 0 && (n - 1) * n / 2 + n > MAX_U64;
    }


    // Count down to zero
    fun count_down(n: u64): u64 {
        let count = n;
        while (count > 0) {
            count = count - 1;
        };
        count
    }
    spec count_down(n: u64): u64 {
        ensures [inferred = vacuous] forall x: u64: x <= 0 ==> result == x;
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
        ensures [inferred] 5 < n ==> r == old(r) + 6;
        ensures [inferred] n == 5 ==> r == old(r) + 5;
        ensures [inferred] n == 4 ==> r == old(r) + 4;
        ensures [inferred] n == 3 ==> r == old(r) + 3;
        ensures [inferred] n == 2 ==> r == old(r) + 2;
        ensures [inferred] n == 1 ==> r == old(r) + 1;
        ensures [inferred] n == 0 ==> r == old(r);
        aborts_if [inferred] 5 < n && r > MAX_U64 - 6;
        aborts_if [inferred] 4 < n && r > MAX_U64 - 5;
        aborts_if [inferred] 3 < n && r > MAX_U64 - 4;
        aborts_if [inferred] 2 < n && r > MAX_U64 - 3;
        aborts_if [inferred] 1 < n && r > MAX_U64 - 2;
        aborts_if [inferred] 0 < n && r > MAX_U64 - 1;
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
        ensures [inferred] r == old(r) * pow2(n);
        aborts_if [inferred] n > 0 && r * pow2(n - 1) * 2 > MAX_U64;
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
        ensures [inferred] r == old(r) * pow2(n);
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
        aborts_if [inferred] n > 0 && r * pow2(n - 1) * 2 > MAX_U64;
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
    spec cond_inc_loop(r: &mut u64, n: u64, do_inc: bool) {
        ensures [inferred = vacuous] forall x: u64, y: u64: x < n && do_inc ==> r == y + 1;
        ensures [inferred = vacuous] forall x: u64, y: u64: x < n && !do_inc ==> r == y;
        ensures [inferred = vacuous] forall x: u64: r == x;
        aborts_if [inferred] do_inc && n > 0;
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
        ensures [inferred] 3 < n ==> global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, old(global<Counter>(addr)).value + 4);
        ensures [inferred] n == 3 ==> global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, old(global<Counter>(addr)).value + 3);
        ensures [inferred] n == 2 ==> global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, old(global<Counter>(addr)).value + 2);
        ensures [inferred] n == 1 ==> global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, old(global<Counter>(addr)).value + 1);
        ensures [inferred] n == 0 ==> global<Counter>(addr) == old(global<Counter>(addr));
        aborts_if [inferred] 3 < n && global<Counter>(addr).value > MAX_U64 - 4;
        aborts_if [inferred] 3 < n && !exists<Counter>(addr);
        aborts_if [inferred] 2 < n && global<Counter>(addr).value > MAX_U64 - 3;
        aborts_if [inferred] 2 < n && !exists<Counter>(addr);
        aborts_if [inferred] 1 < n && global<Counter>(addr).value > MAX_U64 - 2;
        aborts_if [inferred] 1 < n && !exists<Counter>(addr);
        aborts_if [inferred] 0 < n && global<Counter>(addr).value > MAX_U64 - 1;
        aborts_if [inferred] 0 < n && !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
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
    spec inc_global_with_invariant(addr: address, n: u64) {
        ensures [inferred] global<Counter>(addr) == Counter{value: old(global<Counter>(addr)).value + n};
        aborts_if [inferred] n > 0 && global<Counter>(addr).value + (n - 1) > MAX_U64 - 1;
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
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
    spec nested_count(m: u64, n: u64): u64 {
        ensures [inferred] result == m * n;
        aborts_if [inferred] n > 0 && (m > 0 && m * n > MAX_U64);
    }

}
/*
Verification: exiting with verification errors
error: post-condition does not hold
    ┌─ tests/inference/loops.enriched.move:143:9
    │
143 │         ensures [inferred = vacuous] forall x: u64, y: u64: x < n && !do_inc ==> r == y;
    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    │
    =     at tests/inference/loops.enriched.move:132: cond_inc_loop
    =         r = <redacted>
    =         n = <redacted>
    =         do_inc = <redacted>
    =     at tests/inference/loops.enriched.move:133: cond_inc_loop
    =         i = <redacted>
    =     at tests/inference/loops.enriched.move:134: cond_inc_loop
    =     enter loop, variable(s) r, i havocked and reassigned
    =         r = <redacted>
    =         i = <redacted>
    =     at tests/inference/loops.enriched.move:134: cond_inc_loop
    =     at tests/inference/loops.enriched.move:132: cond_inc_loop
    =         r = <redacted>
    =     at tests/inference/loops.enriched.move:145: cond_inc_loop (spec)
    =     at tests/inference/loops.enriched.move:142: cond_inc_loop (spec)
    =     at tests/inference/loops.enriched.move:143: cond_inc_loop (spec)

error: post-condition does not hold
    ┌─ tests/inference/loops.enriched.move:144:9
    │
144 │         ensures [inferred = vacuous] forall x: u64: r == x;
    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    │
    =     at tests/inference/loops.enriched.move:132: cond_inc_loop
    =         r = <redacted>
    =         n = <redacted>
    =         do_inc = <redacted>
    =     at tests/inference/loops.enriched.move:133: cond_inc_loop
    =         i = <redacted>
    =     at tests/inference/loops.enriched.move:134: cond_inc_loop
    =     enter loop, variable(s) r, i havocked and reassigned
    =         r = <redacted>
    =         i = <redacted>
    =     at tests/inference/loops.enriched.move:134: cond_inc_loop
    =     at tests/inference/loops.enriched.move:132: cond_inc_loop
    =         r = <redacted>
    =     at tests/inference/loops.enriched.move:145: cond_inc_loop (spec)
    =     at tests/inference/loops.enriched.move:142: cond_inc_loop (spec)
    =     at tests/inference/loops.enriched.move:143: cond_inc_loop (spec)
    =     at tests/inference/loops.enriched.move:144: cond_inc_loop (spec)

error: function does not abort under this condition
    ┌─ tests/inference/loops.enriched.move:145:9
    │
145 │         aborts_if [inferred] do_inc && n > 0;
    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    │
    =     at tests/inference/loops.enriched.move:132: cond_inc_loop
    =         r = <redacted>
    =         n = <redacted>
    =         do_inc = <redacted>
    =     at tests/inference/loops.enriched.move:133: cond_inc_loop
    =         i = <redacted>
    =     at tests/inference/loops.enriched.move:134: cond_inc_loop
    =     enter loop, variable(s) r, i havocked and reassigned
    =         r = <redacted>
    =         i = <redacted>
    =     at tests/inference/loops.enriched.move:134: cond_inc_loop
    =     at tests/inference/loops.enriched.move:132: cond_inc_loop
    =         r = <redacted>
    =     at tests/inference/loops.enriched.move:145: cond_inc_loop (spec)
*/
