// Copyright © Aptos Foundation
// Tests for `reads` declaration enforcement on functions.
module 0x42::reads_check {
    struct R has key { val: u64 }
    struct S has key { val: u64 }

    // =========================================================================
    // 1. reads R — accesses only R: OK
    // =========================================================================

    fun reads_ok(addr: address): u64 acquires R {
        R[addr].val
    }
    spec reads_ok {
        reads R;
    }

    // =========================================================================
    // 2. reads R — but accesses R and S: ERROR
    // =========================================================================

    fun reads_incomplete(addr: address): u64 acquires R, S {
        R[addr].val + S[addr].val
    }
    spec reads_incomplete {
        reads R;  // error: S not covered
    }

    // =========================================================================
    // 3. reads R, modifies S — covers both: OK
    // =========================================================================

    fun reads_and_modifies(addr: address): u64 acquires R, S {
        S[addr].val = 42;
        R[addr].val + S[addr].val
    }
    spec reads_and_modifies {
        reads R;
        modifies S[addr];
    }
}
