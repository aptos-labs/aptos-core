// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests that generic spec functions using `old()` work correctly when called
// with concrete type instantiations.  The `old_memory` set must be instantiated
// with the call-site type arguments so that the `contains` check matches the
// instantiated `used_memory` entries, producing the correct Boogie argument list
// (pre-state *and* post-state memory parameters).

module 0x42::generic_spec_fun_old {
    struct Coin<phantom T> has key { value: u64 }

    struct USD {}
    struct EUR {}

    // =========================================================================
    // Generic spec function that uses old()
    // =========================================================================

    /// Returns true when the coin balance strictly increased.
    spec fun balance_increased<T>(addr: address): bool {
        old(global<Coin<T>>(addr).value) < global<Coin<T>>(addr).value
    }

    /// Returns true when the coin balance did not decrease.
    spec fun balance_non_decreasing<T>(addr: address): bool {
        old(global<Coin<T>>(addr).value) <= global<Coin<T>>(addr).value
    }

    // =========================================================================
    // Concrete-type callers
    // =========================================================================

    fun increment_usd(addr: address) acquires Coin {
        Coin<USD>[addr].value = Coin<USD>[addr].value + 1;
    }
    spec increment_usd {
        ensures balance_increased<USD>(addr);
    }

    fun increment_eur(addr: address) acquires Coin {
        Coin<EUR>[addr].value = Coin<EUR>[addr].value + 1;
    }
    spec increment_eur {
        ensures balance_increased<EUR>(addr);
    }

    // =========================================================================
    // Negative: wrong type argument should fail
    // =========================================================================

    fun increment_usd_claim_eur(addr: address) acquires Coin {
        Coin<USD>[addr].value = Coin<USD>[addr].value + 1;
    }
    spec increment_usd_claim_eur {
        // We increment USD but claim EUR increased — should fail.
        ensures balance_increased<EUR>(addr); // error: wrong type
    }

    // =========================================================================
    // Combined: increment both, verify both
    // =========================================================================

    fun increment_both(addr: address) acquires Coin {
        Coin<USD>[addr].value = Coin<USD>[addr].value + 1;
        Coin<EUR>[addr].value = Coin<EUR>[addr].value + 1;
    }
    spec increment_both {
        ensures balance_increased<USD>(addr);
        ensures balance_increased<EUR>(addr);
    }

    // =========================================================================
    // Non-decreasing: no change is fine
    // =========================================================================

    fun noop(addr: address) acquires Coin {
        let _v = Coin<USD>[addr].value;
    }
    spec noop {
        ensures balance_non_decreasing<USD>(addr);
    }
}
