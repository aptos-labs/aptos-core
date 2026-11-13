// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The compiler accepts this specification: its purity check does not count a write
// of global memory. Verex rejects it, since it would read the write as nothing.
module 0x42::impure_global_errors {
    struct R has key, drop {
        value: u64,
    }

    fun added(account: &signer): bool {
        move_to(account, R { value: 1 });
        exists<R>(std::signer::address_of(account))
    }

    fun check(account: &signer): bool {
        exists<R>(std::signer::address_of(account))
    }
    spec check {
        ensures result == added(account);
    }
}
