// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::projected_global_reads {
    struct Counter has key { value: u64 }
    fun increment(addr: address) acquires Counter {
        Counter[addr].value = Counter[addr].value + 1;
    }
    spec increment {
        pragma opaque;
        modifies Counter[addr];
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
    }

    spec fun unchanged(addr: address): bool {
        Counter[addr].value == old(Counter[addr].value)
    }
    fun invoke(addr: address) acquires Counter { increment(addr) }
    spec invoke {
        pragma opaque;
        pragma aborts_if_is_partial;
        modifies Counter[addr];
        ensures ..S |~ ensures_of<increment>(addr);
        ensures S.. |~ unchanged(addr);
    }
    // The labelled contract replaces the call; no callee program point is available.
    fun caller(addr: address) acquires Counter { invoke(addr) }
    spec caller {
        pragma aborts_if_is_partial;
        modifies Counter[addr];
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
    }
    fun read(addr: address): u64 acquires Counter { Counter[addr].value }
    fun via_read(addr: address): u64 acquires Counter { read(addr) }
    spec via_read {
        aborts_if !exists<Counter>(addr);
        ensures result == read(addr);
    }
    fun wrong(addr: address) acquires Counter { increment(addr) }
    spec wrong {
        pragma aborts_if_is_partial;
        modifies Counter[addr];
        ensures Counter[addr].value == old(Counter[addr].value) + 2;
    }
}
