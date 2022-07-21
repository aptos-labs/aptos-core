module aptos_framework::transaction_fee {
    use aptos_framework::coin::{Self, BurnCapability};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::system_addresses;

    friend aptos_framework::account;

    struct AptosCoinCapabilities has key {
        burn_cap: BurnCapability<AptosCoin>,
    }

    /// Burn transaction fees in epilogue.
    public(friend) fun burn_fee(account: address, fee: u64) acquires AptosCoinCapabilities {
        coin::burn_from<AptosCoin>(
            account,
            fee,
            &borrow_global<AptosCoinCapabilities>(@aptos_framework).burn_cap,
        );
    }

    public fun store_aptos_coin_burn_cap(account: &signer, burn_cap: BurnCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(account);
        move_to(account, AptosCoinCapabilities { burn_cap })
    }
}
