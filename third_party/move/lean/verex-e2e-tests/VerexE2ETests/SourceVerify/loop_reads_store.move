// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop that only reads the store keeps it, invariant or none.
module 0x42::loop_reads_store {
    struct Link has key { next: address, count: u64 }

    fun walk(start: address, stop: address) {
        let current = start;
        let steps = 0u8;
        while (current != stop) {
            steps = steps + 1;
            assert!(steps < 8, 1);
            assert!(exists<Link>(current), 2);
            current = Link[current].next;
        };
    }
    spec walk {
        pragma aborts_if_is_partial;
        modifies global<Link>(start);
        ensures global<Link>(start) == old(global<Link>(start));
    }

    fun rewrite(start: address, stop: address) {
        let current = start;
        let steps = 0u8;
        while (current != stop) {
            steps = steps + 1;
            assert!(steps < 8, 1);
            let link = &mut Link[current];
            link.count = link.count + 1;
            current = link.next;
        };
    }
    spec rewrite {
        pragma aborts_if_is_partial;
        ensures global<Link>(start) == old(global<Link>(start)); // error: the loop writes the store
    }
}
