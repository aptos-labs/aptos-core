/// Can only be minted to Dee's account.
module dee::dee_coin {
    use aptos_std::coin::{
        Self,
        BurnCapability,
        FreezeCapability,
        MintCapability,
    };
    use aptos_std::signer;
    use aptos_std::string;

    struct DeeCoin {}

    struct CapabilityStore<phantom CoinType> has key {
        burn_cap: BurnCapability<CoinType>,
        freeze_cap: FreezeCapability<CoinType>,
        mint_cap: MintCapability<CoinType>,
    }

    const NAME: vector<u8> = b"Dee Coin";
    const SYMBOL: vector<u8> = b"DEE";
    const DECIMALS: u8 = 8;
    const MONITOR_SUPPLY: bool = false;

    const E_NOT_DEE: u64 = 0;

    fun init_module(account: &signer) {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<DeeCoin>(
            account,
            string::utf8(NAME),
            string::utf8(SYMBOL),
            DECIMALS,
            MONITOR_SUPPLY,
        );
        move_to(account, CapabilityStore<DeeCoin> {
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
        assert!(signer::address_of(account) == @dee, E_NOT_DEE);
        let mint_cap_ref =
            &borrow_global<CapabilityStore<CoinType>>(@dee).mint_cap;
        coin::deposit(@dee, coin::mint(amount, mint_cap_ref));
    }
}