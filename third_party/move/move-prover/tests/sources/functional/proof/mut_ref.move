// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for proof blocks on functions with `&mut` parameters,
// exercising pre/post state observations and result values.
module 0x42::proof_mut_ref {

    struct Counter has drop {
        value: u64,
    }

    // ==================================================================
    // Pre-state entry assert on &mut param, mutation verified via ensures.

    fun increment(c: &mut Counter) {
        c.value = c.value + 1;
    }
    spec increment {
        requires c.value < MAX_U64;
        ensures c.value == old(c.value) + 1;
    } proof {
        // Pre-state observation: c.value is the original value here.
        assert c.value < MAX_U64;
        assert c.value + 1 <= MAX_U64;
    }

    // ==================================================================
    // Mutation + return result: post assert observes result.

    fun add_and_return(c: &mut Counter, n: u64): u64 {
        c.value = c.value + n;
        c.value
    }
    spec add_and_return {
        requires c.value + n <= MAX_U64;
        ensures c.value == old(c.value) + n;
        ensures result == c.value;
    } proof {
        // Pre-state: entry observation on &mut param.
        assert c.value + n <= MAX_U64;
        // Post-state: old() observes pre-mutation value, result the post value.
        post assert c.value == old(c.value) + n;
        post assert result == c.value;
    }

    // ==================================================================
    // Conditional mutation returning bool: pre-state branching with result.

    fun double_if_small(c: &mut Counter): bool {
        if (c.value < 1000) {
            c.value = c.value * 2;
            true
        } else {
            false
        }
    }
    spec double_if_small {
        ensures old(c.value) < 1000 ==> (c.value == old(c.value) * 2 && result == true);
        ensures old(c.value) >= 1000 ==> (c.value == old(c.value) && result == false);
    } proof {
        // Pre-state branching on &mut param value.
        if (c.value < 1000) {
            assert c.value * 2 <= MAX_U64;
        }
    }

    // ==================================================================
    // Multiple fields: swap returning old first field via result.

    struct Pair has drop {
        x: u64,
        y: u64,
    }

    fun swap_and_return_old_x(p: &mut Pair): u64 {
        let old_x = p.x;
        p.x = p.y;
        p.y = old_x;
        old_x
    }
    spec swap_and_return_old_x {
        ensures p.x == old(p.y);
        ensures p.y == old(p.x);
        ensures result == old(p.x);
    } proof {
        // Pre-state: observe both fields before swap.
        assert p.x <= MAX_U64;
        assert p.y <= MAX_U64;
        // Post-state: old() observes pre-swap fields, result captures original p.x.
        post assert p.x == old(p.y);
        post assert result == old(p.x);
    }

    // ==================================================================
    // Decrement returning new value: pre + post observations.

    fun decrement(c: &mut Counter): u64 {
        c.value = c.value - 1;
        c.value
    }
    spec decrement {
        requires c.value > 0;
        ensures c.value == old(c.value) - 1;
        ensures result == c.value;
    } proof {
        // Pre-state: original value is positive.
        assert c.value > 0;
        // Post-state: old() relates pre/post, result matches mutated field.
        post assert c.value == old(c.value) - 1;
        post assert result == c.value;
    }

    // ==================================================================
    // Apply lemma relating pre and post state of &mut param.
    // A pre-state `let` captures the original value; at the return
    // point the lemma is applied with the saved pre-value and the
    // mutated post-value.

    spec module {
        lemma strict_increase(a: u64, b: u64) {
            requires b == a + 1;
            ensures a < b;
        } proof {
            assume [trusted] true;
        }
    }

    fun bump(c: &mut Counter) {
        c.value = c.value + 1;
    }
    spec bump {
        requires c.value < MAX_U64;
        ensures c.value == old(c.value) + 1;
        ensures old(c.value) < c.value;
    } proof {
        // old(c.value) is the saved pre-state, c.value is the mutated post-state.
        post apply strict_increase(old(c.value), c.value);
    }

    // ==================================================================
    // FAILURE: False post assertion on mutated &mut param.
    // After set_to_zero, c.value == 0, but the proof claims it equals old value.

    fun set_to_zero(c: &mut Counter) {
        c.value = 0;
    }
    spec set_to_zero {
        ensures c.value == 0;
    } proof {
        post assert c.value == old(c.value);  // error: c.value == 0, not old(c.value)
    }
}
