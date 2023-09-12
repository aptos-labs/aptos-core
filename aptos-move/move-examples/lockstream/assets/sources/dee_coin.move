/// Can only be minted to Dee's account.
module assets::dee_coin {
    use aptos_framework::coin;
    use aptos_std::string;

    struct DeeCoin {}

    const NAME: vector<u8> = b"Dee Coin";
    const SYMBOL: vector<u8> = b"DEE";
    const DECIMALS: u8 = 8;
    const MONITOR_SUPPLY: bool = false;

    fun init_module(account: &signer) {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<DeeCoin>(
            account,
            NAME,
            SYMBOL,
            DECIMALS,
            MONITOR_SUPPLY,
        );
        move_to(account, CapabilityStore<CoinType> {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
        coin::register<DeeCoin>(account)
    }

    public entry fun mint<CoinType>(
        account: &signer,
        amount: u64,
    ) acquires CapabilityStore {
        assert!(address_of(account) == @dee, 0);
        let mint_cap_ref =
            &borrow_global<CapabilityStore<CoinType>>(@dee).mint_cap;
        coin::deposit(@dee, coin::mint(amount, mint_cap_ref));
    }
}