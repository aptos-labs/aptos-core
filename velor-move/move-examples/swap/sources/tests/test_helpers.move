#[test_only]
module swap::test_helpers {
    use velor_framework::coin::{Self, Coin};
    use velor_framework::fungible_asset::{Self, FungibleAsset};
    use velor_framework::object;
    use velor_framework::primary_fungible_store;
    use std::option;
    use std::string;
    use swap::package_manager;
    use swap::liquidity_pool;

    public fun set_up(deployer: &signer) {
        package_manager::initialize_for_test(deployer);
        liquidity_pool::initialize();
    }

    public fun create_fungible_asset_and_mint(creator: &signer, name: vector<u8>, amount: u64): FungibleAsset {
        let token_metadata = &object::create_named_object(creator, name);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            token_metadata,
            option::none(),
            string::utf8(name),
            string::utf8(name),
            8,
            string::utf8(b""),
            string::utf8(b""),
        );
        let mint_ref = &fungible_asset::generate_mint_ref(token_metadata);
        fungible_asset::mint(mint_ref, amount)
    }

    public fun create_coin_and_mint<CoinType>(creator: &signer, amount: u64): Coin<CoinType> {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<CoinType>(
            creator,
            string::utf8(b"Test"),
            string::utf8(b"Test"),
            8,
            true,
        );
        let coin = coin::mint<CoinType>(amount, &mint_cap);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);
        coin::destroy_mint_cap(mint_cap);
        coin
    }
}
