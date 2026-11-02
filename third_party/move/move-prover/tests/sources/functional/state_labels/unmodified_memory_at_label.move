// Copyright © Aptos Foundation
// A memory which a state label's definition does not change is, at that
// label, the memory of the label's pre-state: here the function entry.
module 0x42::unmodified_memory_at_label {
    use std::signer;

    struct Resource has key {
        value: u64,
    }

    struct Container has key {
        inner: u64,
    }

    fun swap(account: &signer, addr: address): Resource acquires Resource {
        let r = move_from<Resource>(addr);
        move_to(account, Container { inner: r.value });
        r
    }
    spec swap(account: &signer, addr: address): Resource {
        pragma opaque;
        modifies Container[signer::address_of(account)];
        modifies Resource[addr];
        ensures result == old(Resource[addr]);
        ensures ..S1 |~ remove<Resource>(addr);
        ensures S1.. |~ publish<Container>(
            signer::address_of(account), Container { inner: old(Resource[addr]).value }
        );
        aborts_if !exists<Resource>(addr);
        aborts_if S1 |~ exists<Container>(signer::address_of(account));
    }

    fun swap_wrong(account: &signer, addr: address): Resource acquires Resource {
        let r = move_from<Resource>(addr);
        move_to(account, Container { inner: r.value });
        r
    }
    spec swap_wrong(account: &signer, addr: address): Resource {
        pragma opaque;
        modifies Container[signer::address_of(account)];
        modifies Resource[addr];
        ensures ..S1 |~ remove<Resource>(addr);
        ensures S1 |~ exists<Container>(signer::address_of(account)); // error: published only after S1
        aborts_if !exists<Resource>(addr);
        aborts_if S1 |~ exists<Container>(signer::address_of(account));
    }
}
