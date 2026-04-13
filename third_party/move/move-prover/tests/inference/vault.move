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
        let store_addr = object::object_address(Vault[vault_obj.object_address()].store);
        ensures FungibleStore[store_addr].balance >= old(FungibleStore[store_addr].balance);
    }
}
