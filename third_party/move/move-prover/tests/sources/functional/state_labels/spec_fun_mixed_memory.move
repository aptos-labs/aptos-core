// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests that `uses_old` spec functions correctly evaluate non-old memory in the
// current (post) state rather than the pre state.  A spec function that mixes
// `old(global<R1>(...))` with `global<R2>(...)` must pass the pre-state memory
// for R1 and the current-state memory for R2.

module 0x42::spec_fun_mixed_memory {
    struct Balance has key { value: u64 }
    struct Limit has key { max: u64 }

    // =========================================================================
    // Spec function mixing old and current memory
    // =========================================================================

    /// Balance increased (compared to old state) and stays within the limit
    /// (read from current state).
    spec fun increased_within_limit(addr: address): bool {
        old(global<Balance>(addr).value) < global<Balance>(addr).value
            && global<Balance>(addr).value <= global<Limit>(addr).max
    }

    // =========================================================================
    // Positive: increment balance and raise limit — should verify
    // =========================================================================

    fun increment_and_raise_limit(addr: address) acquires Balance, Limit {
        Balance[addr].value = Balance[addr].value + 1;
        Limit[addr].max = Limit[addr].max + 10;
    }
    spec increment_and_raise_limit {
        requires Balance[addr].value + 1 <= Limit[addr].max + 10;
        ensures increased_within_limit(addr);
    }

    // =========================================================================
    // Positive: balance and limit both start at same value, bump balance by 1
    // while setting limit to old_limit + 1 — exactly at limit
    // =========================================================================

    fun increment_and_set_limit(addr: address) acquires Balance, Limit {
        Balance[addr].value = Balance[addr].value + 1;
        Limit[addr].max = Balance[addr].value;
    }
    spec increment_and_set_limit {
        ensures increased_within_limit(addr);
    }

    // =========================================================================
    // Negative: lower the limit below new balance — should fail because
    // current-state Limit.max < Balance.value.  If the bug were present
    // (non-old memory read from pre-state), this would incorrectly pass
    // because old Limit.max might still be >= Balance.value.
    // =========================================================================

    fun increment_and_lower_limit(addr: address) acquires Balance, Limit {
        Balance[addr].value = Balance[addr].value + 1;
        Limit[addr].max = 0;
    }
    spec increment_and_lower_limit {
        requires Balance[addr].value > 0;
        ensures increased_within_limit(addr); // error: limit lowered below balance
    }
}
