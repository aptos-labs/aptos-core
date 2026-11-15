// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop that only reads the store keeps it: its function's closed frame holds
// and a caller reads the state of its entry. `writes` changes the store.
module 0x42::loop_reads_keep_store {
    struct R has key { owner: address, v: u64 }

    fun reads(a: address, o: address): bool {
        let c = R[a].owner;
        let count = 0;
        while (o != c) {
            count += 1;
            assert!(count < 8, 2);
            if (!exists<R>(c)) return false;
            c = R[c].owner;
        };
        true
    }
    spec reads {
        pragma aborts_if_is_partial;
        ensures global<R>(a) == old(global<R>(a));
    }

    fun caller(a: address, o: address): u64 {
        if (reads(a, o)) R[a].v else 0
    }
    spec caller {
        pragma aborts_if_is_partial;
        ensures global<R>(o) == old(global<R>(o));
    }

    fun writes(a: address, o: address): bool {
        let c = R[a].owner;
        while (o != c) {
            R[c].v = 1;
            c = R[c].owner;
        };
        true
    }
    spec writes {
        pragma aborts_if_is_partial;
        ensures global<R>(a) == old(global<R>(a)); // error: the loop writes every `R` it walks
    }
}
