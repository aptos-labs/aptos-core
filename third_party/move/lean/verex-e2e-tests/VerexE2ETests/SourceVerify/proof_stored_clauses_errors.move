// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A `witness` applies to every obligation its `on` names; a key naming an obligation without
/// the existential is an error at the witness.
module 0x42::proof_stored_clauses_errors {
    spec fun held(x: u64): bool;

    struct Even has copy, drop, key {
        value: u64,
        small: u64,
    }
    spec Even {
        invariant exists half: u64: value == 2 * half;
        invariant held(small);
    }

    fun save(account: &signer, half: u64, small: u64) {
        let value = Even { value: half + half, small };
        move_to(account, value);
    }
    spec save {
        pragma verify = true;
        requires half + half <= MAX_U64;
    }
}
