// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// A call must retain the successful-return property of a clause which also
// defines an intermediate state. Assuming the definition before the call
// does not replace the full postcondition after the call.
module 0x42::mixed_callee_postcondition {
    use std::signer::address_of;

    struct R has key, drop { value: u64 }

    fun replace(s: &signer) acquires R {
        let R { value: _ } = move_from<R>(address_of(s));
        move_to(s, R { value: 1 });
    }
    spec replace {
        pragma opaque;
        aborts_if !exists<R>(address_of(s));
        modifies R[address_of(s)];
        ensures {
            let addr = address_of(s);
            (..S |~ remove<R>(addr)) &&
            (S.. |~ publish<R>(addr, R { value: 1 }))
        };
    }

    fun caller(s: &signer) acquires R {
        replace(s)
    }
    spec caller {
        aborts_if !exists<R>(address_of(s));
        ensures exists<R>(address_of(s));
        ensures R[address_of(s)].value == 1;
    }

    fun create_then_call(s: &signer) acquires R {
        move_to(s, R { value: 0 });
        replace(s)
    }
    spec create_then_call {
        aborts_if exists<R>(address_of(s));
        ensures R[address_of(s)].value == 1;
    }
}
