// This file tests loop invariants on `for` loops. `for` loops are lowered such
// that invariants are checked in the classical form: at the loop head, the
// iterator holds the next value to be processed and no compiler-internal state
// influences the induction argument.
//
// The invariants here use the trailing spec-block form, written after the loop
// body like a `while` loop: `for (i in lb..ub) body spec { invariant ... }`.
module 0x42::VerifyForLoopInvariants {
    use std::vector;

    // --- Positive cases: all of the following are expected to verify. ---

    // Textbook accumulator invariant.
    public fun sum_range(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            sum = sum + i;
        } spec {
            invariant i <= n;
            invariant sum == i * (i - 1) / 2;
        };
        sum
    }
    spec sum_range {
        requires n < 1000;
        aborts_if false;
        ensures result == n * (n - 1) / 2;
    }

    // Non-zero lower bound: the iterator starts at `a`, not 0.
    public fun sum_from(a: u64, b: u64): u64 {
        let sum = 0;
        for (i in a..b) {
            sum = sum + i;
        } spec {
            invariant a <= i && i <= b;
            invariant sum == i * (i - 1) / 2 - a * (a - 1) / 2;
        };
        sum
    }
    spec sum_from {
        requires a <= b && b < 1000;
        aborts_if false;
        ensures result == b * (b - 1) / 2 - a * (a - 1) / 2;
    }

    // `continue` advances the iterator, so the invariant must be preserved
    // on the continue back edge as well.
    public fun count_odd(n: u64): u64 {
        let count = 0;
        for (i in 0..n) {
            if (i % 2 == 0) continue;
            count = count + 1;
        } spec {
            invariant i <= n;
            invariant count == i / 2;
        };
        count
    }
    spec count_odd {
        aborts_if false;
        ensures result == n / 2;
    }

    // `break` exits the loop early; the invariant must hold up to the break point,
    // and the elements scanned before breaking are all below the threshold.
    public fun find_first_ge(v: &vector<u64>, threshold: u64): u64 {
        let n = vector::length(v);
        let found = n;
        for (i in 0..n) {
            if (*vector::borrow(v, i) >= threshold) {
                found = i;
                break
            };
        } spec {
            invariant i <= n;
            invariant found == n;
            invariant forall j in 0..i: v[j] < threshold;
        };
        found
    }
    spec find_first_ge {
        aborts_if false;
        ensures result <= len(v);
        // Everything before the returned index is below the threshold.
        ensures forall j in 0..result: v[j] < threshold;
    }

    // A `continue` taken from within a scope that shadows the iterator. The
    // invariant is maintained only because the iterator is still advanced on the
    // `continue` back edge -- a regression check that the lowering advances the
    // real iterator, not the shadow. Skipping `i == 0` does not change the sum,
    // so the accumulator invariant is the same as for a plain range sum.
    public fun sum_skip_first(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            let i = i; // shadow the iterator inside the body
            if (i == 0) continue;
            sum = sum + i;
        } spec {
            invariant i <= n;
            invariant sum == i * (i - 1) / 2;
        };
        sum
    }
    spec sum_skip_first {
        requires n < 1000;
        aborts_if false;
        ensures result == n * (n - 1) / 2;
    }

    // `break` exits as soon as adding the next element would exceed `cap`. The
    // invariant bounds the accumulator, which is what discharges the `ensures`.
    public fun sum_until(n: u64, cap: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            if (sum + i > cap) break;
            sum = sum + i;
        } spec {
            invariant i <= n;
            invariant sum <= cap;
        };
        sum
    }
    spec sum_until {
        requires n < 1000 && cap < 1000000;
        aborts_if false;
        ensures result <= cap;
    }

    // Prefix `forall` invariant relating processed elements to the iterator.
    public fun init_zeros(n: u64): vector<u64> {
        let v = vector::empty<u64>();
        for (i in 0..n) {
            vector::push_back(&mut v, 0);
        } spec {
            invariant i <= n;
            invariant len(v) == i;
            invariant forall j in 0..i: v[j] == 0;
        };
        v
    }
    spec init_zeros {
        aborts_if false;
        ensures len(result) == n;
        ensures forall j in 0..n: result[j] == 0;
    }

    // Mutable-reference parameter with `old(..)`: the invariant relates the
    // running value of `x` to its entry value. For a scalar reference, `x` in
    // the spec is the dereferenced value and `old(x)` is its value on entry.
    public fun add_repeatedly(x: &mut u64, n: u64) {
        for (i in 0..n) {
            *x = *x + 1;
        } spec {
            invariant i <= n;
            invariant x == old(x) + i;
        };
    }
    spec add_repeatedly {
        requires x + n <= MAX_U64;
        aborts_if false;
        ensures x == old(x) + n;
    }

    // Two sequential `for` loops in the same function, each with its own
    // invariant, sharing an accumulator.
    public fun count_twice(n: u64): u64 {
        let c = 0;
        for (i in 0..n) {
            c = c + 1;
        } spec {
            invariant i <= n;
            invariant c == i;
        };
        for (i in 0..n) {
            c = c + 1;
        } spec {
            invariant i <= n;
            invariant c == n + i;
        };
        c
    }
    spec count_twice {
        requires n < 1000000;
        aborts_if false;
        ensures result == 2 * n;
    }

    // Nested `for` loops with invariants on both levels, each in a trailing
    // spec block.
    public fun sum_grid(n: u64, m: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            for (j in 0..m) {
                sum = sum + 1;
            } spec {
                invariant j <= m;
                invariant sum == i * m + j;
            };
        } spec {
            invariant i <= n;
            invariant sum == i * m;
        };
        sum
    }
    spec sum_grid {
        requires n < 1000 && m < 1000;
        aborts_if false;
        ensures result == n * m;
    }

    // The invariant is what discharges `aborts_if false`: tracking the exact
    // value of `sum` bounds it below `MAX_U64`, so `sum + 10` cannot overflow.
    public fun bounded_sum(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            sum = sum + 10;
        } spec {
            invariant i <= n;
            invariant sum == i * 10;
        };
        sum
    }
    spec bounded_sum {
        requires n < 100;
        aborts_if false;
        ensures result == n * 10;
    }

    // --- Negative cases: each is expected to produce a verification error. ---

    // Induction step fails: the invariant holds on entry but is not preserved.
    public fun sum_range_incorrect(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            sum = sum + i;
        } spec {
            invariant sum == i;
        };
        sum
    }
    spec sum_range_incorrect {
        requires n < 1000;
    }

    // Base case fails: the invariant does not hold when the loop is first
    // entered (at `i == 0`, `sum == 0`, but the invariant claims `sum == 1`).
    public fun base_case_incorrect(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            sum = sum + i;
        } spec {
            invariant sum == i + 1;
        };
        sum
    }
    spec base_case_incorrect {
        requires n < 1000;
    }
}
