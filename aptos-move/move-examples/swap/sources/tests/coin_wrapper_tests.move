#[test_only]
module swap::coin_wrapper_tests {
    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin, FakeMoney};
    use aptos_framework::fungible_asset::{Self, FungibleAsset};
    use swap::coin_wrapper;
    use swap::test_helpers;
    use std::signer;

    #[test(user = @0x1, deployer = @0xcafe)]
    fun test_e2e(user: &signer, deployer: &signer) {
        test_helpers::set_up(deployer);
        account::create_account_for_test(signer::address_of(user));
        coin::create_fake_money(user, user, 1000);
        let coins = coin::withdraw<FakeMoney>(user, 1000);
        let fa = coin_wrapper::wrap(coins);
        let metadata = fungible_asset::asset_metadata(&fa);
        assert!(fungible_asset::amount(&fa) == 1000, 0);
        assert!(
            fungible_asset::name(metadata) == coin::name<FakeMoney>(),
            0
        );
        assert!(
            fungible_asset::symbol(metadata) == coin::symbol<FakeMoney>(),
            0
        );
        assert!(
            fungible_asset::decimals(metadata) == coin::decimals<FakeMoney>(),
            0
        );
        let coins = coin_wrapper::unwrap<FakeMoney>(fa);
        assert!(coin::value(&coins) == 1000, 0);
        coin::deposit(signer::address_of(user), coins);
    }

    public fun wrap<CoinType>(coin: Coin<CoinType>): FungibleAsset {
        coin_wrapper::wrap(coin)
    }
}
