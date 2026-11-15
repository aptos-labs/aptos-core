// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A quantifier's binder is in scope in its clause alone: a later binding of
// the same name is a different variable.
module 0x42::contract_binder_scope {
    struct R has key { v: u64 }

    fun owner_address(owner: address): address { owner }
    spec owner_address {
        aborts_if false;
        ensures forall addr: address: old(exists<R>(addr)) ==> exists<R>(addr);
        let addr = owner;
        ensures result == addr;
    }

    fun other_address(owner: address): address { owner }
    spec other_address {
        aborts_if false;
        ensures forall addr: address: old(exists<R>(addr)) ==> exists<R>(addr);
        let addr = owner;
        ensures result != addr; // error: the binding is the owner, not the binder
    }
}
