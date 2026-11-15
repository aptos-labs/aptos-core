// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Under a model (a spec variable makes the unit one), an inlined callee starts
// as it does otherwise: its loop reads its function start, and its caller owes
// its precondition. `owes_not_established` fails that precondition.
module 0x42::inlined_start_model {
    struct R has key { owner: address, v: u64 }
    spec R { invariant v <= 10; }
    spec module { global ticks: u64; }

    fun walk(a: address, o: address, n: u64): bool {
        let c = R[a].owner;
        let count = 0;
        while (o != c) {
            count += 1;
            assert!(count < n, 2);
            if (!exists<R>(c)) return false;
            R[c].v = 1;
            c = R[c].owner;
        };
        true
    }
    spec walk {
        requires n > 0; // error: `owes_not_established` does not establish it
    }

    fun establishes(a: address, o: address): bool {
        walk(a, o, 8)
    }
    spec establishes {
        pragma aborts_if_is_partial;
    }

    fun owes_not_established(a: address, o: address, n: u64): bool {
        walk(a, o, n)
    }
    spec owes_not_established {
        pragma aborts_if_is_partial;
    }
}
