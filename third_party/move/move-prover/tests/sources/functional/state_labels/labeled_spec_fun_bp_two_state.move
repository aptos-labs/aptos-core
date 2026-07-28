// Copyright © Aptos Foundation
// Two-state state-labeled calls (`..S`, `S..`, `S1..S2`) of spec functions
// containing two-state behavioral predicates: the pre- and post-label
// memories are passed for the spec function's dual-state parameter pairs.
module 0x42::labeled_spec_fun_bp_two_state {
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

    // Dual-state wrapper: `ensures_of` spans pre- and post-state.
    spec fun incremented(addr: address): bool {
        ensures_of<increment>(addr)
    }

    // Single-state wrapper, observed at intermediate states.
    spec fun wont_abort_inc(addr: address): bool {
        !aborts_of<increment>(addr)
    }

    fun two_increments(addr: address) acquires Counter {
        increment(addr);
        increment(addr);
    }
    spec two_increments {
        pragma aborts_if_is_partial;
        // S is the state between the increments, defined by a mutation.
        ensures ..S |~ update<Counter>(addr,
            update_field(old(Counter[addr]), value, old(Counter[addr].value) + 1));
        // Second increment: S → exit satisfies increment's postconditions.
        ensures S.. |~ incremented(addr);
        // At S the counter exists and cannot overflow, so increment won't abort.
        ensures S |~ wont_abort_inc(addr);
    }

    fun three_increments(addr: address) acquires Counter {
        increment(addr);
        increment(addr);
        increment(addr);
    }
    spec three_increments {
        pragma aborts_if_is_partial;
        ensures ..S1 |~ update<Counter>(addr,
            update_field(old(Counter[addr]), value, old(Counter[addr].value) + 1));
        ensures S1..S2 |~ update<Counter>(addr,
            update_field(S1 |~ Counter[addr], value, (S1 |~ Counter[addr].value) + 1));
        // Explicit pre/post pair on the dual-state wrapper.
        ensures S1..S2 |~ incremented(addr);
        ensures S2.. |~ incremented(addr);
        ensures S1.. |~ incremented(addr); // error: two increments from S1 to exit
    }
}
