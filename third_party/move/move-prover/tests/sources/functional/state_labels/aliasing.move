// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests that the prover correctly reasons about intermediate state between
// a move_to (which creates a resource) and a subsequent opaque call that
// reads from potentially the same address.

module 0x42::state_labels {
    use std::signer;

    struct Resource has key {
        value: u64,
    }

    /// Opaque read: returns Resource.value at addr.
    fun read_resource(addr: address): u64 acquires Resource {
        Resource[addr].value
    }
    spec read_resource {
        pragma opaque;
        ensures result == Resource[addr].value;
        aborts_if !exists<Resource>(addr);
    }

    // MoveFrom: return value must reference pre-state (resource is removed)
    // Expected:
    //   ensures result == global<Resource, @pre>(addr)
    //   aborts_if !exists<Resource, @pre>(addr)
    fun remove_resource(addr: address): Resource acquires Resource {
        move_from<Resource>(addr)
    }
    spec remove_resource(addr: address): Resource {
        pragma opaque = true;
        modifies Resource[addr];
        ensures [inferred] result == old(Resource[addr]);
        ensures [inferred] remove<Resource>(addr);
        aborts_if [inferred] !exists<Resource>(addr);
    }

    /// Creates a Resource at account's address, then reads from addr.
    /// If addr is the same as the account, the read sees the freshly created resource.
    fun create_then_read(account: &signer, addr: address): u64 acquires Resource {
        move_to(account, Resource { value: 42 });
        read_resource(addr)
    }
    spec create_then_read {
        modifies Resource[signer::address_of(account)];
        ensures S.. |~ result == result_of<read_resource>(addr);
        ensures ..S |~ publish<Resource>(signer::address_of(account), Resource{value: 42});
        aborts_if S |~ aborts_of<read_resource>(addr);
        aborts_if exists<Resource>(signer::address_of(account));
    }


    // Chained function calls with state dependency:
    // The second call sees the state after the first call modified it
    // Expected:
    //   ensures result == result_of<read_resource, @s2>(addr)
    //   where @s2 is state after remove_resource
    fun remove_then_try_read(addr1: address, addr2: address): u64 acquires Resource {
        // Remove resource from addr1
        let Resource { value: _ } = remove_resource(addr1);
        // Try to read from addr2 (intermediate state: addr1 no longer has resource)
        read_resource(addr2)
    }
    spec remove_then_try_read(addr1: address, addr2: address): u64 {
        pragma opaque = true;
        modifies Resource[addr1];
        ensures ..S |~ remove<Resource>(addr1);
        ensures S.. |~ result == result_of<read_resource>(addr2);
        aborts_if S |~ aborts_of<read_resource>(addr2);
        aborts_if aborts_of<remove_resource>(addr1);
    }

    // Two writes to same resource type at different (but potentially aliased) addresses.
    // When a1 == a2, the second write overwrites the first.
    struct Counter has key { value: u64 }

    fun different_addr_global(a1: address, a2: address, v1: u64, v2: u64) acquires Counter {
        Counter[a1].value = v1;
        Counter[a2].value = v2;
    }
    spec different_addr_global {
        modifies Counter[a1];
        modifies Counter[a2];
        ensures ..S |~ update<Counter>(a1, update_field(old(Counter[a1]), value, v1));
        ensures S.. |~ update<Counter>(a2, update_field(Counter[a2], value, v2));
        aborts_if !exists<Counter>(a1);
        aborts_if !exists<Counter>(a2);
    }
}
