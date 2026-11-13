// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// One clause of a data invariant at both sites a function owes it, a construction and a store.
module 0x42::proof_stored_clauses {
    struct Even has copy, drop, key {
        value: u64,
        small: u64,
    }
    spec Even {
        invariant exists half: u64: value == 2 * half;
        invariant small < 10;
    }

    fun save(account: &signer, half: u64) {
        let value = Even { value: half + half, small: 3 };
        move_to(account, value);
    }
    spec save {
        pragma verify = true;
        requires half + half <= MAX_U64;
    }
}
