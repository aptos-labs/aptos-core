// flag: --aptos
// flag: --no-inference-opaque
module 0x42::vault {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, FungibleStore};
    use aptos_framework::object::{Self, Object};

    struct Strategy(|FungibleAsset|FungibleAsset) has store, copy;
    spec Strategy {
        modifies_of<self.0> *;
        invariant forall input: FungibleAsset, result: FungibleAsset:
            ensures_of<self.0>(input, result) ==>
                fungible_asset::asset_metadata(result) == fungible_asset::asset_metadata(input) &&
                fungible_asset::amount(result) >= fungible_asset::amount(input);
    }

    struct Vault has key {
        store: Object<FungibleStore>,
        strategy: Strategy
    }

    public fun harvest(caller: &signer, vault_obj: Object<Vault>) {
        let vault = &mut Vault[vault_obj.object_address()];

        // Withdraw all assets from the vault's store
        let balance = fungible_asset::balance(vault.store);
        let assets = fungible_asset::withdraw(caller, vault.store, balance);

        // Execute the dynamically dispatched strategy
        let returned_assets = (vault.strategy.0)(assets);

        // Deposit the results back into the store
        fungible_asset::deposit(vault.store, returned_assets);
    }

    spec harvest {
        use 0x1::object;
        use 0x1::fungible_asset;
        let store_addr = object::object_address(Vault[vault_obj.object_address()].store);
        ensures FungibleStore[store_addr].balance >= old(FungibleStore[store_addr].balance);
        ensures [inferred] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) ==> {
            let a = {
                let b = {
                    let c = ..S1 |~ result_of<fungible_asset::balance<fungible_asset::FungibleStore>>(old(Vault[object::object_address<Vault>(vault_obj)]).store);
                    S1..S2 |~ result_of<fungible_asset::withdraw<fungible_asset::FungibleStore>>(caller, old(Vault[object::object_address<Vault>(vault_obj)]).store, c)
                };
                S2..S3 |~ result_of<old(Vault[object::object_address<Vault>(vault_obj)]).strategy.0>(b)
            };
            S3.. |~ ensures_of<fungible_asset::deposit<fungible_asset::FungibleStore>>(old(Vault[object::object_address<Vault>(vault_obj)]).store, a)
        };
        ensures [inferred] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) ==> {
            let a = ..S1 |~ result_of<fungible_asset::balance<fungible_asset::FungibleStore>>(old(Vault[object::object_address<Vault>(vault_obj)]).store);
            let b = S1..S2 |~ result_of<fungible_asset::withdraw<fungible_asset::FungibleStore>>(caller, old(Vault[object::object_address<Vault>(vault_obj)]).store, a);
                let c = S1..S2 |~ result_of<fungible_asset::withdraw<fungible_asset::FungibleStore>>(caller, old(Vault[object::object_address<Vault>(vault_obj)]).store, a);
                S1..S2 |~ ensures_of<fungible_asset::withdraw<fungible_asset::FungibleStore>>(caller, old(Vault[object::object_address<Vault>(vault_obj)]).store, a, c)
        };
        aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && !exists<Vault>(object::object_address<Vault>(vault_obj));
        aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && aborts_of<fungible_asset::balance<fungible_asset::FungibleStore>>(Vault[object::object_address<Vault>(vault_obj)].store);
        aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && {
            let a = ..S1 |~ result_of<fungible_asset::balance<fungible_asset::FungibleStore>>(Vault[object::object_address<Vault>(vault_obj)].store);
            S1 |~ aborts_of<fungible_asset::withdraw<fungible_asset::FungibleStore>>(caller, Vault[object::object_address<Vault>(vault_obj)].store, a)
        };
        aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && {
            let a = {
                let b = ..S1 |~ result_of<fungible_asset::balance<fungible_asset::FungibleStore>>(Vault[object::object_address<Vault>(vault_obj)].store);
                S1..S2 |~ result_of<fungible_asset::withdraw<fungible_asset::FungibleStore>>(caller, Vault[object::object_address<Vault>(vault_obj)].store, b)
            };
            S2 |~ aborts_of<Vault[object::object_address<Vault>(vault_obj)].strategy.0>(a)
        };
        aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && {
            let a = {
                let b = {
                    let c = ..S1 |~ result_of<fungible_asset::balance<fungible_asset::FungibleStore>>(Vault[object::object_address<Vault>(vault_obj)].store);
                    S1..S2 |~ result_of<fungible_asset::withdraw<fungible_asset::FungibleStore>>(caller, Vault[object::object_address<Vault>(vault_obj)].store, c)
                };
                S2..S3 |~ result_of<Vault[object::object_address<Vault>(vault_obj)].strategy.0>(b)
            };
            S3 |~ aborts_of<fungible_asset::deposit<fungible_asset::FungibleStore>>(Vault[object::object_address<Vault>(vault_obj)].store, a)
        };
    }
}
/*
Verification: exiting with compilation errors
warning: unused alias
  ┌─ vault.enriched.move:5:26
  │
5 │     use aptos_framework::object::{Self, Object};
  │                          ^^^^^^ Unused 'use' of alias 'object'. Consider removing it

error: undeclared `addr`
   ┌─ vault.enriched.move:40:98
   │
40 │         ensures [inferred] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) ==> {
   │                                                                                                  ^^^^

error: undeclared `addr`
   ┌─ vault.enriched.move:50:98
   │
50 │         ensures [inferred] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) ==> {
   │                                                                                                  ^^^^

error: undeclared `addr`
   ┌─ vault.enriched.move:56:110
   │
56 │         aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && !exists<Vault>(object::object_address<Vault>(vault_obj));
   │                                                                                                              ^^^^

error: undeclared `addr`
   ┌─ vault.enriched.move:57:110
   │
57 │         aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && aborts_of<fungible_asset::balance<fungible_asset::FungibleStore>>(Vault[object::object_address<Vault>(vault_obj)].store);
   │                                                                                                              ^^^^

error: undeclared `addr`
   ┌─ vault.enriched.move:58:110
   │
58 │         aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && {
   │                                                                                                              ^^^^

error: undeclared `addr`
   ┌─ vault.enriched.move:62:110
   │
62 │         aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && {
   │                                                                                                              ^^^^

error: undeclared `addr`
   ┌─ vault.enriched.move:69:110
   │
69 │         aborts_if [inferred = sathard] (forall x: address {exists<fungible_asset::ConcurrentFungibleBalance>(addr)}: exists<fungible_asset::ConcurrentFungibleBalance>(x) ==> fungible_asset::FungibleStore[x].balance == 0) && !exists<fungible_asset::DispatchFunctionStore>(@0xa) && {
   │                                                                                                              ^^^^
*/
