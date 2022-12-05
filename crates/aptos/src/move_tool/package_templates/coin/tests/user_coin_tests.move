#[test_only]
module coin_address::user_coin_tests {
    use aptos_framework::account;
    use aptos_framework::coin;

    use coin_address::user_coin::{Self, UserCoin};

    #[test(coin_admin = @coin_address)]
    fun test_mint_burn_coins(coin_admin: signer) {
        user_coin::initialize(&coin_admin);

        let user_addr = @0x41;
        let user = account::create_account_for_test(user_addr);
        coin::register<UserCoin>(&user);
        user_coin::mint(&coin_admin, user_addr, 100);

        assert!(coin::balance<UserCoin>(user_addr) == 100, 1);

        user_coin::burn(&user, 30);

        assert!(coin::balance<UserCoin>(user_addr) == 70, 1);
    }
}
