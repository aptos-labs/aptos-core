/// Can be minted to anyone's account.
module assets::usdc {
    use aptos_framework::coin;
    use aptos_std::string;

    struct USDC {}

    const NAME: vector<u8> = b"USD Coin";
    const SYMBOL: vector<u8> = b"USDC";
    const DECIMALS: u8 = 6;
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
    }

    public entry fun mint<CoinType>(
        account: &signer,
        amount: u64,
    ) acquires CapabilityStore {
        let account_addr = address_of(account);
        if (!coin::is_account_registered<CoinType>(account_addr)) {
            coin::register<CoinType>(account)
        };
        let mint_cap_ref =
            &borrow_global<CapabilityStore<CoinType>>(@dee).mint_cap;
        coin::deposit(account_addr, coin::mint(amount, mint_cap_ref));
    }
}