// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// The capped vault with contracts its code does not meet: each failure is
/// reported at the Move clause it belongs to.
module 0x42::vault_errors {
    struct Vault has key {
        balance: u64,
        cap: u64,
    }

    const E_OVER_CAP: u64 = 1;
    const E_INSUFFICIENT: u64 = 2;

    public fun deposit(addr: address, amount: u64) acquires Vault {
        let vault = &mut Vault[addr];
        assert!(vault.balance + amount <= vault.cap, E_OVER_CAP);
        vault.balance = vault.balance + amount;
    }
    spec deposit {
        aborts_if global<Vault>(addr).balance + amount > MAX_U64;
        aborts_if global<Vault>(addr).balance + amount > global<Vault>(addr).cap
            with E_OVER_CAP;
        ensures global<Vault>(addr).balance == old(global<Vault>(addr).balance) + amount;
        ensures global<Vault>(addr).cap == old(global<Vault>(addr).cap);
        modifies global<Vault>(addr);
    }

    public fun withdraw(addr: address, amount: u64): u64 acquires Vault {
        let vault = &mut Vault[addr];
        assert!(vault.balance >= amount, E_INSUFFICIENT);
        vault.balance = vault.balance - amount;
        vault.balance
    }
    spec withdraw {
        aborts_if !exists<Vault>(addr);
        aborts_if global<Vault>(addr).balance < amount with E_INSUFFICIENT;
        ensures result == old(global<Vault>(addr).balance) - amount + 1;
        ensures global<Vault>(addr).balance == result;
        modifies global<Vault>(addr);
    }

    public fun headroom(addr: address): u64 acquires Vault {
        let vault = &Vault[addr];
        if (vault.balance >= vault.cap) 0 else vault.cap - vault.balance
    }
    spec headroom {
        aborts_if !exists<Vault>(addr);
        ensures global<Vault>(addr).balance + result > global<Vault>(addr).cap;
    }

    /// Accrues a fixed fee of 5 for each of `periods` periods.
    public fun fees(periods: u64): u64 {
        let i = 0;
        let total = 0;
        while (i < periods) {
            total = total + 5;
            i = i + 1;
        } spec {
            invariant i <= periods;
            invariant i < periods;
            invariant total == 5 * i;
        };
        total
    }
    spec fees {
        aborts_if 5 * periods > MAX_U64;
        ensures result == 5 * periods;
    }
}
