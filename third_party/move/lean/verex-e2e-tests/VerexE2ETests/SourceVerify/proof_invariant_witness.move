// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `witness` on the invariant a construction and a store owe: an invariant's
/// clause is the one the leaf owes, so a witness has its own type.
module 0x42::proof_invariant_witness {
    struct Even has copy, drop, key {
        value: u64,
    }
    spec Even {
        invariant exists half: u64: value == 2 * half;
    }

    fun make(half: u64): Even {
        Even { value: half + half }
    }
    spec make {
        requires half + half <= MAX_U64;
    }

    fun save(account: &signer, half: u64) {
        let value = Even { value: half + half };
        move_to(account, value);
    }
    spec save {
        pragma verify = true;
        requires half + half <= MAX_U64;
    }
}
