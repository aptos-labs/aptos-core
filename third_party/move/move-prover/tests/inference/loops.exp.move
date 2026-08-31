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
        pragma opaque = true;
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
        pragma opaque = true;
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
        pragma opaque = true;
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
        aborts_if [inferred] 0 < n && r == MAX_U64;
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
        pragma opaque = true;
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
        pragma opaque = true;
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
        pragma opaque = true;
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
        } spec {
            invariant i <= n;
            invariant do_inc ==> r == old(r) + i;
            invariant !do_inc ==> r == old(r);
        };
    }
    spec cond_inc_loop(r: &mut u64, n: u64, do_inc: bool) {
        pragma opaque = true;
        ensures [inferred] r == (if (do_inc) old(r) + n else old(r));
        aborts_if [inferred] do_inc && r + n > MAX_U64;
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
        pragma opaque = true;
        modifies Counter[addr];
        ensures [inferred] 3 < n ==> Counter[addr].value == Counter[addr].value + 1;
        ensures [inferred] 2 < n ==> Counter[addr].value == Counter[addr].value + 1;
        ensures [inferred] ({
            let a = S1 |~ global<Counter>(addr);
            1 < n ==> Counter[addr].value == a.value + 1
        });
        ensures [inferred] 0 < n ==> {
            let a = update_field(old(Counter[addr]), value, old(Counter[addr]).value + 1);
            ..S1 |~ update<Counter>(addr, a)
        };
        aborts_if [inferred] 3 < n && !exists<Counter>(addr);
        aborts_if [inferred] 2 < n && !exists<Counter>(addr);
        aborts_if [inferred] ({
            let a = S1 |~ exists<Counter>(addr);
            1 < n && !a
        });
        aborts_if [inferred] 0 < n && !exists<Counter>(addr);
        aborts_if [inferred] 3 < n && Counter[addr].value == MAX_U64;
        aborts_if [inferred] 2 < n && Counter[addr].value == MAX_U64;
        aborts_if [inferred] ({
            let a = S1 |~ global<Counter>(addr);
            1 < n && a.value == MAX_U64
        });
        aborts_if [inferred] 0 < n && Counter[addr].value == MAX_U64;
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
        pragma opaque = true, aborts_if_is_partial = true;
        modifies Counter[addr];
        ensures [inferred] Counter[addr].value == old(Counter[addr]).value ==> (forall x: u64, y: Counter: Counter[addr].value == old(Counter[addr]).value + x && x < n ==> update<Counter>(addr, update_field(y, value, y.value + 1)));
        aborts_if [inferred] !exists<Counter>(addr);
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
    spec agent_invariant_loop(n: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == n * (n + 1) / 2;
        aborts_if [inferred] n > 0 && (n - 1) * n / 2 + n > MAX_U64;
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
    spec empty_spec_block {
        pragma opaque = true;
        ensures [inferred] result == n * (n + 1) / 2;
        aborts_if [inferred] n > 0 && (n - 1) * n / 2 + n > MAX_U64;
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
        pragma opaque = true;
        ensures [inferred] result == m * n;
        aborts_if [inferred] n > 0 && m > 0 && m * n > MAX_U64;
    }

}
/*
Inference diagnostics:
warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
   ┌─ tests/inference/loops.move:25:16
   │
25 │         while (count > 0) {
   │                ^^^^^
   │
   = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
   = source-visible loop-carried state: n
   = bounded WP status: exact within the displayed bound
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].n == n
       head[1]: n > 0 ==> head[1].n == n - 1
       head[2]: n > 1 ==> head[2].n == n - 2
       head[3]: n > 2 ==> head[3].n == n - 3
   = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

warning: WP could not characterize the aborts of `loops::inc_global_with_invariant` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition did not survive a memory-havocking loop
    ┌─ tests/inference/loops.move:142:5
    │
142 │ ╭     fun inc_global_with_invariant(addr: address, n: u64) acquires Counter {
143 │ │         let i = 0;
144 │ │         let start = Counter[addr].value;
145 │ │         while (i < n) {
    · │
152 │ │         };
153 │ │     }
    │ ╰─────^

Verification: exiting with verification errors
error: post-condition does not hold
    ┌─ loops.enriched.move:178:9
    │
178 │         ensures [inferred] 2 < n ==> Counter[addr].value == Counter[addr].value + 1;
    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    │
    =     at loops.enriched.move:163: inc_global_n_times
    =         addr = <redacted>
    =         n = <redacted>
    =     at loops.enriched.move:164: inc_global_n_times
    =         i = <redacted>
    =     at loops.enriched.move:165: inc_global_n_times
    =     at loops.enriched.move:166: inc_global_n_times
    =         c = <redacted>
    =     at loops.enriched.move:167: inc_global_n_times
    =     at loops.enriched.move:168: inc_global_n_times
    =         i = <redacted>
    =     at loops.enriched.move:165: inc_global_n_times
    =     at loops.enriched.move:166: inc_global_n_times
    =         c = <redacted>
    =     at loops.enriched.move:167: inc_global_n_times
    =     at loops.enriched.move:168: inc_global_n_times
    =         i = <redacted>
    =     at loops.enriched.move:165: inc_global_n_times
    =     at loops.enriched.move:166: inc_global_n_times
    =         c = <redacted>
    =     at loops.enriched.move:167: inc_global_n_times
    =     at loops.enriched.move:168: inc_global_n_times
    =         i = <redacted>
    =     at loops.enriched.move:165: inc_global_n_times
    =     at loops.enriched.move:170: inc_global_n_times
    =     at loops.enriched.move:187: inc_global_n_times (spec)
    =     at loops.enriched.move:188: inc_global_n_times (spec)
    =     at loops.enriched.move:189: inc_global_n_times (spec)
    =     at loops.enriched.move:193: inc_global_n_times (spec)
    =     at loops.enriched.move:194: inc_global_n_times (spec)
    =     at loops.enriched.move:195: inc_global_n_times (spec)
    =     at loops.enriched.move:196: inc_global_n_times (spec)
    =     at loops.enriched.move:200: inc_global_n_times (spec)
    =     at loops.enriched.move:177: inc_global_n_times (spec)
    =     at loops.enriched.move:178: inc_global_n_times (spec)

error: abort not covered by any of the `aborts_if` clauses
    ┌─ loops.enriched.move:171:5
    │
167 │               c.value = c.value + 1;
    │                         ----------- abort happened here with execution failure
    ·
171 │ ╭     spec inc_global_n_times {
172 │ │         // Test with unrolling. Notice that the inferred spec is incomplete and will
173 │ │         // fail verification.
174 │ │         pragma unroll = 3;
    · │
200 │ │         aborts_if [inferred] 0 < n && Counter[addr].value == MAX_U64;
201 │ │     }
    │ ╰─────^
    │
    =     at loops.enriched.move:163: inc_global_n_times
    =         addr = <redacted>
    =         n = <redacted>
    =     at loops.enriched.move:164: inc_global_n_times
    =         i = <redacted>
    =     at loops.enriched.move:165: inc_global_n_times
    =     at loops.enriched.move:166: inc_global_n_times
    =         c = <redacted>
    =     at loops.enriched.move:167: inc_global_n_times
    =     at loops.enriched.move:168: inc_global_n_times
    =         i = <redacted>
    =     at loops.enriched.move:165: inc_global_n_times
    =     at loops.enriched.move:166: inc_global_n_times
    =         c = <redacted>
    =     at loops.enriched.move:167: inc_global_n_times
    =     at loops.enriched.move:168: inc_global_n_times
    =         i = <redacted>
    =     at loops.enriched.move:165: inc_global_n_times
    =     at loops.enriched.move:166: inc_global_n_times
    =         c = <redacted>
    =     at loops.enriched.move:167: inc_global_n_times
    =         ABORTED
*/
