#[test_only]
module supra_framework::supra_coin_tests {
    use supra_framework::supra_coin;
    use supra_framework::coin;
    use supra_framework::fungible_asset::{Self, FungibleStore, Metadata};
    use supra_framework::primary_fungible_store;
    use supra_framework::object::{Self, Object};

    public fun mint_sup_fa_to_for_test<T: key>(store: Object<T>, amount: u64) {
        fungible_asset::deposit(store, supra_coin::mint_sup_fa_for_test(amount));
    }

    public fun mint_sup_fa_to_primary_fungible_store_for_test(
        owner: address,
        amount: u64,
    ) {
        primary_fungible_store::deposit(owner, supra_coin::mint_sup_fa_for_test(amount));
    }

    #[test(supra_framework = @supra_framework)]
    fun test_sup_setup_and_mint(supra_framework: &signer) {
        let (burn_cap, mint_cap) = supra_coin::initialize_for_test(supra_framework);
        let coin = coin::mint(100, &mint_cap);
        let fa = coin::coin_to_fungible_asset_for_test(coin);
        primary_fungible_store::deposit(@supra_framework, fa);
        assert!(
            primary_fungible_store::balance(
                @supra_framework,
                object::address_to_object<Metadata>(@supra_fungible_asset)
            ) == 100,
            0
        );
        coin::destroy_mint_cap(mint_cap);
        coin::destroy_burn_cap(burn_cap);
    }

    #[test]
    fun test_fa_helpers_for_test() {
        assert!(!object::object_exists<Metadata>(@supra_fungible_asset), 0);
        supra_coin::ensure_initialized_with_sup_fa_metadata_for_test();
        assert!(object::object_exists<Metadata>(@supra_fungible_asset), 0);
        mint_sup_fa_to_primary_fungible_store_for_test(@supra_framework, 100);
        let metadata = object::address_to_object<Metadata>(@supra_fungible_asset);
        assert!(primary_fungible_store::balance(@supra_framework, metadata) == 100, 0);
        let store_addr = primary_fungible_store::primary_store_address(@supra_framework, metadata);
        mint_sup_fa_to_for_test(object::address_to_object<FungibleStore>(store_addr), 100);
        assert!(primary_fungible_store::balance(@supra_framework, metadata) == 200, 0);
    }
}
