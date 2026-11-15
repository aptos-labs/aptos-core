// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// State labels: a label defined by a state-change predicate (`..S |~ publish`),
// read by behavioral predicates at a one-state (`S |~`) and a post-state
// (`S.. |~`) position, and labels quantified over the state domain
// (`exists S in *`) splitting a two-state specification function.
module 0x42::state_labels {
    use std::signer;

    struct Resource has key { value: u64 }
    struct Counter has copy, drop, store { value: u64 }

    fun read_resource(addr: address): u64 acquires Resource {
        Resource[addr].value
    }
    spec read_resource {
        pragma opaque;
        ensures result == Resource[addr].value;
        aborts_if !exists<Resource>(addr);
    }

    fun create_then_read(account: &signer, addr: address): u64 acquires Resource {
        move_to(account, Resource { value: 42 });
        read_resource(addr)
    }
    spec create_then_read {
        modifies Resource[signer::address_of(account)];
        ensures ..S |~ publish<Resource>(signer::address_of(account), Resource { value: 42 });
        ensures S.. |~ result == result_of<read_resource>(addr);
        aborts_if S |~ aborts_of<read_resource>(addr);
        aborts_if exists<Resource>(signer::address_of(account));
    }

    spec fun counter_increased(c: &mut Counter): bool {
        old(c).value < c.value
    }

    fun inc(c: &mut Counter) {
        c.value = c.value + 1;
    }

    fun inc_twice(c: &mut Counter) {
        inc(c);
        inc(c)
    }
    spec inc_twice {
        ensures exists S in *: (..S |~ counter_increased(c)) && (S.. |~ counter_increased(c));
    }
}
