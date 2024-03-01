#[test_only]
module token_minter::coin_utils {
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::coin;
    use aptos_framework::coin::{BurnCapability, MintCapability};

    public fun fund_account<CoinType>(mint_cap: &MintCapability<CoinType>, to: &signer, amount: u64) {
        let to_address = signer::address_of(to);
        if (!account::exists_at(to_address)) {
            account::create_account_for_test(to_address);
        };

        coin::register<CoinType>(to);
        coin::deposit(to_address, coin::mint(amount, mint_cap));
    }

    public fun clean_up_caps<CoinType>(burn_cap: BurnCapability<CoinType>, mint_cap: MintCapability<CoinType>) {
        coin::destroy_burn_cap<CoinType>(burn_cap);
        coin::destroy_mint_cap<CoinType>(mint_cap);
    }
}
