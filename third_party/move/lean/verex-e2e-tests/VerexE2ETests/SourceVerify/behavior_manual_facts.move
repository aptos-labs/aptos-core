// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::behavior_manual_facts {
    struct Counter has key { value: u64 }
    struct Flag has key { active: bool }

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

    fun flip_flag(addr: address) acquires Flag {
        Flag[addr].active = !Flag[addr].active;
    }
    spec flip_flag {
        pragma opaque;
        modifies Flag[addr];
        ensures Flag[addr].active != old(Flag[addr].active);
        aborts_if !exists<Flag>(addr);
    }

    fun caller(addr: address) acquires Counter, Flag {
        increment(addr);
        flip_flag(addr);
    }
    spec caller {
        pragma aborts_if_is_partial;
        modifies Counter[addr], Flag[addr];
        ensures ..S |~ ensures_of<increment>(addr);
        ensures S.. |~ ensures_of<flip_flag>(addr);
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        ensures Flag[addr].active != old(Flag[addr].active);
    }

    // Existing call facts must not establish an incorrect final value.
    fun wrong_final_value(addr: address) acquires Counter, Flag {
        increment(addr);
        flip_flag(addr);
    }
    spec wrong_final_value {
        pragma aborts_if_is_partial;
        modifies Counter[addr], Flag[addr];
        ensures ..S |~ ensures_of<increment>(addr);
        ensures S.. |~ ensures_of<flip_flag>(addr);
        ensures Counter[addr].value == old(Counter[addr].value) + 2;
    }
}
