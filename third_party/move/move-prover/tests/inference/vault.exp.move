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
        pragma aborts_if_is_partial = true;
        ensures [inferred = sathard] ({
            let a = {
                let b = {
                    let c = ..S1 |~ result_of<fungible_asset::balance<fungible_asset::FungibleStore>>(Vault[object::object_address<Vault>(vault_obj)].store);
                    S1..S2 |~ result_of<fungible_asset::withdraw<fungible_asset::FungibleStore>>(caller, Vault[object::object_address<Vault>(vault_obj)].store, c)
                };
                S2..S3 |~ result_of<Vault[object::object_address<Vault>(vault_obj)].strategy.0>(b)
            };
            S3.. |~ ensures_of<fungible_asset::deposit<fungible_asset::FungibleStore>>(Vault[object::object_address<Vault>(vault_obj)].store, a)
        });
        aborts_if [inferred] !exists<Vault>(object::object_address<Vault>(vault_obj));
    }
}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `vault::harvest` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition did not survive a memory-havocking loop
   ┌─ tests/inference/vault.move:21:5
   │
21 │ ╭     public fun harvest(caller: &signer, vault_obj: Object<Vault>) {
22 │ │         let vault = &mut Vault[vault_obj.object_address()];
23 │ │
24 │ │         // Withdraw all assets from the vault's store
   · │
32 │ │         fungible_asset::deposit(vault.store, returned_assets);
33 │ │     }
   │ ╰─────^

Verification: exiting with condition generation errors
warning: unused alias
  ┌─ vault.enriched.move:5:26
  │
5 │     use aptos_framework::object::{Self, Object};
  │                          ^^^^^^ Unused 'use' of alias 'object'. Consider removing it

error: this function has no specification but is referenced by a behavioral predicate
    ┌─ ../../../aptos-move/framework/aptos-framework/sources/fungible_asset.move:675:5
    │
675 │ ╭     public fun balance<T: key>(
676 │ │         store: Object<T>
677 │ │     ): u64 {
678 │ │         balance_impl(
    · │
686 │ │         )
687 │ │     }
    │ ╰─────^

error: this function has no specification but is referenced by a behavioral predicate
    ┌─ ../../../aptos-move/framework/aptos-framework/sources/fungible_asset.move:984:5
    │
984 │ ╭     public fun withdraw<T: key>(
985 │ │         owner: &signer, store: Object<T>, amount: u64
986 │ │     ): FungibleAsset acquires FungibleStore, DispatchFunctionStore, ConcurrentFungibleBalance {
987 │ │         withdraw_sanity_check(owner, store, true);
988 │ │         unchecked_withdraw(store.object_address(), amount)
989 │ │     }
    │ ╰─────^

error: this function has no specification but is referenced by a behavioral predicate
     ┌─ ../../../aptos-move/framework/aptos-framework/sources/fungible_asset.move:1033:5
     │
1033 │ ╭     public fun deposit<T: key>(
1034 │ │         store: Object<T>, fa: FungibleAsset
1035 │ │     ) acquires FungibleStore, DispatchFunctionStore, ConcurrentFungibleBalance {
1036 │ │         deposit_sanity_check(store, true);
1037 │ │         unchecked_deposit(store.object_address(), fa);
1038 │ │     }
     │ ╰─────^
*/
