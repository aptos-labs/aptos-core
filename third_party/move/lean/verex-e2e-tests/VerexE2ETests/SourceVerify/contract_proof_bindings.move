// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::contract_proof_bindings {
    fun increment(x: &mut u64) {
        *x = *x + 1;
    }
    spec increment {
        let before = x;
        let next = before + 1;
        let post after = x;
        aborts_if before == MAX_U64;
        ensures [inferred = sathard] x == next;
    } proof {
        assert next == before + 1;
        post assert x == before + 1;
        post assert after == next;
    }
}
