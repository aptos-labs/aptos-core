// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::field_update_labels {
    struct Counter has key { value: u64 }

    fun bump(addr: address) acquires Counter {
        Counter[addr].value = Counter[addr].value + 1;
    }
    spec bump {
        pragma opaque;
        modifies Counter[addr];
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value == MAX_U64;
        ensures ..S |~ update<Counter>(addr,
            update_field(old(Counter[addr]), value, old(Counter[addr].value) + 1));
        ensures Counter[addr].value == (S |~ Counter[addr].value);
    }

    fun caller(addr: address) acquires Counter { bump(addr); }
    spec caller {
        modifies Counter[addr];
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value == MAX_U64;
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
    }

    // A caller cannot infer an extra update from the named state.
    fun caller_wrong(addr: address) acquires Counter { bump(addr); }
    spec caller_wrong {
        modifies Counter[addr];
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value == MAX_U64;
        ensures Counter[addr].value == old(Counter[addr].value) + 2;
    }
}
