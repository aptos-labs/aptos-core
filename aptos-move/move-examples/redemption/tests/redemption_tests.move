#[test_only]
module redemption::redemption_tests {
    use aptos_framework::account;
    use aptos_framework::aptos_account;
    use aptos_framework::coin::Coin;
    use aptos_framework::coin;
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::stake;
    use redemption::redemption;
    use std::option;
    use std::signer;
    use std::string;

    struct WrappedCoin {}

    #[test(deployer = @0xcafe, operator = @0x123, user = @0x234)]
    public entry fun test_e2e(deployer: &signer, operator: &signer, user: &signer) {
        stake::initialize_for_test(&account::create_signer_for_test(@0x1));

        let user_addr = signer::address_of(user);
        let coins = create_coin_and_mint(deployer, 1000);
        aptos_account::deposit_coins(user_addr, coins);
        let (redemption_fa, native_tokens) = create_fungible_asset_and_mint(deployer, 1000);
        primary_fungible_store::deposit(signer::address_of(operator), native_tokens);
        redemption::create_pool<WrappedCoin>(deployer, redemption_fa);

        // Operator deposits 1000 native FA into the pool.
        let operator_addr = signer::address_of(operator);
        assert!(primary_fungible_store::balance(operator_addr, redemption_fa) == 1000, 0);
        redemption::deposit_native<WrappedCoin>(operator, 1000);
        assert!(primary_fungible_store::balance(operator_addr, redemption_fa) == 0, 0);
        assert!(redemption::native_balance<WrappedCoin>() == 1000, 0);
        assert!(redemption::wrapped_balance<WrappedCoin>() == 0, 0);

        // User redeems 500
        assert!(primary_fungible_store::balance(user_addr, redemption_fa) == 0, 0);
        assert!(coin::balance<WrappedCoin>(user_addr) == 1000, 0);
        redemption::redeem<WrappedCoin>(user, 500);
        assert!(primary_fungible_store::balance(user_addr, redemption_fa) == 500, 0);
        assert!(coin::balance<WrappedCoin>(user_addr) == 500, 0);
        assert!(redemption::native_balance<WrappedCoin>() == 500, 0);
        assert!(redemption::wrapped_balance<WrappedCoin>() == 500, 0);

        // Operator withdraws 500 wrapped coins
        assert!(primary_fungible_store::balance(operator_addr, redemption_fa) == 0, 0);
        assert!(coin::balance<WrappedCoin>(operator_addr) == 0, 0);
        redemption::withdraw_wrapped<WrappedCoin>(operator, 500);
        assert!(primary_fungible_store::balance(operator_addr, redemption_fa) == 0, 0);
        assert!(coin::balance<WrappedCoin>(operator_addr) == 500, 0);
        assert!(redemption::wrapped_balance<WrappedCoin>() == 0, 0);

        // User redeems the remaining 500
        assert!(primary_fungible_store::balance(operator_addr, redemption_fa) == 0, 0);
        redemption::redeem<WrappedCoin>(user, 500);
        assert!(coin::balance<WrappedCoin>(user_addr) == 0, 0);
        assert!(primary_fungible_store::balance(user_addr, redemption_fa) == 1000, 0);
        assert!(redemption::native_balance<WrappedCoin>() == 0, 0);
        assert!(redemption::wrapped_balance<WrappedCoin>() == 500, 0);

        // Operator withdraws the remaining 500 wrapped coins
        redemption::withdraw_wrapped<WrappedCoin>(operator, 500);
        assert!(coin::balance<WrappedCoin>(operator_addr) == 1000, 0);
        assert!(primary_fungible_store::balance(operator_addr, redemption_fa) == 0, 0);
        assert!(redemption::native_balance<WrappedCoin>() == 0, 0);
        assert!(redemption::wrapped_balance<WrappedCoin>() == 0, 0);
    }

    fun create_coin_and_mint(creator: &signer, amount: u64): Coin<WrappedCoin> {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<WrappedCoin>(
            creator,
            string::utf8(b"Test"),
            string::utf8(b"Test"),
            8,
            true,
        );
        let coin = coin::mint<WrappedCoin>(amount, &mint_cap);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);
        coin::destroy_mint_cap(mint_cap);
        coin
    }

    fun create_fungible_asset_and_mint(creator: &signer, amount: u64): (Object<Metadata>, FungibleAsset) {
        let token_metadata = &object::create_named_object(creator, b"");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            token_metadata,
            option::none(),
            string::utf8(b"FA"),
            string::utf8(b"FA"),
            8,
            string::utf8(b""),
            string::utf8(b""),
        );
        let mint_ref = &fungible_asset::generate_mint_ref(token_metadata);
        (fungible_asset::mint_ref_metadata(mint_ref), fungible_asset::mint(mint_ref, amount))
    }
}
