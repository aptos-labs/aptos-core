// Copyright © Aptos Foundation
// An exclusively borrowed global resource survives an invoke of a stored function
// value that may modify any memory (`modifies_of *`): mutations made through the
// borrow before and after the call, and post-conditions relating the resource to
// its pre-state, hold across the call.
module 0x42::borrowed_global_across_invoke {
    struct Strat(|u64|u64) has store, copy, drop;
    spec Strat {
        modifies_of<self.0> *;
        invariant forall S in *, x: u64: S |~ !aborts_of<self.0>(x);
        invariant forall S in *, x: u64, r: u64:
            S.. |~ ensures_of<self.0>(x, r) ==> r >= x;
    }

    struct Vault has key {
        balance: u64,
        strat: Strat,
    }

    #[persistent]
    fun id(x: u64): u64 { x }
    spec id {
        aborts_if false;
        ensures result == x;
    }

    public fun make(owner: &signer) {
        move_to(owner, Vault { balance: 0, strat: Strat(id) });
    }

    public fun harvest(addr: address): u64 acquires Vault {
        let vault = &mut Vault[addr];
        let taken = vault.balance;
        vault.balance = 0;
        let s = vault.strat;
        let returned = (s.0)(taken);
        vault.balance = vault.balance + returned;
        returned
    }
    spec harvest {
        aborts_if !exists<Vault>(addr);
        ensures Vault[addr].balance >= old(Vault[addr].balance);
        ensures result >= old(Vault[addr].balance);
    }
}
