// Copyright © Aptos Foundation
// Tests that state labels are rejected on functions with non-linear control flow.
module 0x42::nonlinear_cfg_error {
    struct Counter has key { value: u64 }

    fun opaque_inc(addr: address) acquires Counter {
        Counter[addr].value = Counter[addr].value + 1;
    }
    spec opaque_inc {
        pragma opaque;
        modifies Counter[addr];
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
    }

    fun branching_with_labels(addr: address, cond: bool) acquires Counter {
        opaque_inc(addr);
        if (cond) {
            opaque_inc(addr);
        };
    }
    spec branching_with_labels {
        pragma aborts_if_is_partial;
        ensures ..S |~ Counter[addr].value == old(Counter[addr].value) + 1; // error: non-linear CFG
    }
}
