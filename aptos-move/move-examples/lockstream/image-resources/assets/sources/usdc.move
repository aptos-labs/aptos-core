/// Can be minted to anyone's account.
module dee::usdc {
    use aptos_std::coin::{
        Self,
        BurnCapability,
        FreezeCapability,
        MintCapability,
    };
    use aptos_std::signer;
    use aptos_std::string;

    struct USDC {}

    struct CapabilityStore<phantom CoinType> has key {
        burn_cap: BurnCapability<CoinType>,
        freeze_cap: FreezeCapability<CoinType>,
        mint_cap: MintCapability<CoinType>,
    }

    const NAME: vector<u8> = b"USD Coin";
    const SYMBOL: vector<u8> = b"USDC";
    const DECIMALS: u8 = 6;
    const MONITOR_SUPPLY: bool = false;

    fun init_module(account: &signer) {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<USDC>(
            account,
            string::utf8(NAME),
            string::utf8(SYMBOL),
            DECIMALS,
            MONITOR_SUPPLY,
        );
        move_to(account, CapabilityStore<USDC> {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    public entry fun mint(
        account: &signer,
        amount: u64,
    ) acquires CapabilityStore {
        let account_addr = signer::address_of(account);
        if (!coin::is_account_registered<USDC>(account_addr)) {
            coin::register<USDC>(account)
        };
        let mint_cap_ref =
            &borrow_global<CapabilityStore<USDC>>(@dee).mint_cap;
        coin::deposit(account_addr, coin::mint(amount, mint_cap_ref));
    }
}