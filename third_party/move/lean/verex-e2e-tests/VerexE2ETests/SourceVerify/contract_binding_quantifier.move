// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A quantifier's binder is in scope in its own clause only: a contract binding
// of the same name is another local.
module 0x42::contract_binding_quantifier {
    struct R has key { value: u64 }

    fun read(account: &signer): u64 acquires R {
        R[std::signer::address_of(account)].value
    }
    spec read {
        ensures forall addr: address: exists<R>(addr) ==> global<R>(addr).value >= 0;
        let addr = std::signer::address_of(account);
        aborts_if !exists<R>(addr);
        ensures result == global<R>(addr).value;
    }
}
