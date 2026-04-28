module defi::vault {
    /// A fungible asset — linear and hot potato (no store, copy, or drop).
    struct FungibleAsset {
        amount: u64,
    }

    /// A store holding a fungible asset balance.
    struct FungibleStore has store {
        balance: u64,
    }

    /// A yield strategy that transforms assets and guarantees
    /// the returned amount is at least the input amount.
    struct Strategy(|FungibleAsset|FungibleAsset) has store, copy, drop;
    spec Strategy {
        modifies_of<self.0> *;
        invariant forall input: FungibleAsset, result: FungibleAsset:
            ensures_of<self.0>(input, result) ==> result.amount >= input.amount;
    }

    struct Vault has key {
        store: FungibleStore,
        strategy: Strategy,
    }

    fun withdraw(store: &mut FungibleStore, amount: u64): FungibleAsset {
        assert!(store.balance >= amount, 1);
        store.balance = store.balance - amount;
        FungibleAsset { amount }
    }
    spec withdraw {
        pragma opaque;
        aborts_if store.balance < amount;
        ensures result.amount == amount;
        ensures store.balance == old(store.balance) - amount;
    }

    fun deposit(store: &mut FungibleStore, fa: FungibleAsset) {
        let FungibleAsset { amount } = fa;
        store.balance = store.balance + amount;
    }
    spec deposit {
        pragma opaque;
        aborts_if store.balance + fa.amount > MAX_U64;
        ensures store.balance == old(store.balance) + fa.amount;
    }

    public fun harvest(caller: &signer, vault_addr: address) {
        let vault = &mut Vault[vault_addr];

        // Withdraw all assets from the vault's store
        let balance = vault.store.balance;
        let assets = withdraw(&mut vault.store, balance);

        // Execute the dynamically dispatched strategy
        let strategy = vault.strategy;
        let returned = (strategy.0)(assets);

        // Deposit the results back into the store
        deposit(&mut vault.store, returned);
    }

    spec harvest {
        modifies global<Vault>(vault_addr);
        ensures Vault[vault_addr].store.balance >= old(Vault[vault_addr].store.balance);
    }
}
