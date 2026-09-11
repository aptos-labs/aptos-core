// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// A state transition under negation or in an implication antecedent is not a
// positive definition. Assuming it would contradict the condition being
// checked and let the following false postcondition verify vacuously.
module 0x42::negated_definition {
    struct R has key { value: u64 }

    fun write_negated(addr: address) acquires R {
        R[addr].value = 1;
    }
    spec write_negated {
        modifies R[addr];
        aborts_if !exists<R>(addr);
        ensures !(..S |~ update<R>(addr, R { value: 1 })) && (S |~ exists<R>(addr));
        ensures false;
    }

    fun write_in_antecedent(addr: address) acquires R {
        R[addr].value = 1;
    }
    spec write_in_antecedent {
        modifies R[addr];
        aborts_if !exists<R>(addr);
        ensures ((..S |~ update<R>(addr, R { value: 1 })) ==> false) &&
            (S |~ exists<R>(addr));
        ensures false;
    }
}
