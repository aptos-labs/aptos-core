module econia::faucet {
    use aptos_std::coin::{
        Self,
        BurnCapability,
        FreezeCapability,
        MintCapability,
    };
    use aptos_std::string;
    use aptos_std::type_info;
    use std::signer::{address_of};

    /// No capability store at coin type publisher's account.
    const E_NO_CAP_STORE: u64 = 0;

    /// A wrapper for coin type capabilities.
    struct CapabilityStore<phantom CoinType> has key {
        burn_cap: BurnCapability<CoinType>,
        freeze_cap: FreezeCapability<CoinType>,
        mint_cap: MintCapability<CoinType>,
    }

    /// Init and store coin capabilities, at type publisher's account.
    public entry fun initialize<CoinType>(
        account: &signer,
        name: string::String,
        symbol: string::String,
        decimals: u8,
        monitor_supply: bool,
    ) {
        // Initialize coin info at coin type publisher's account,
        // returning coin capabilities (this fails is the calling
        // account is not the coin type publisher).
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<CoinType>(
            account,
            name,
            symbol,
            decimals,
            monitor_supply
        );
        // Store capabilities under the publisher's account.
        move_to(account, CapabilityStore<CoinType> {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    /// Permissionlessly mint a coin type to a caller's coin store.
    public entry fun mint<CoinType>(
        account: &signer,
        amount: u64,
    ) acquires CapabilityStore {
        // Get caller's account address.
        let account_addr = address_of(account);
        // If caller does not have a coin store for coin type:
        if (!coin::is_account_registered<CoinType>(account_addr)) {
            // Regiser one.
            coin::register<CoinType>(account)
        };
        // Get the coin type publisher's address.
        let coin_type_publisher_address =
            type_info::account_address(&type_info::type_of<CoinType>());
        // Assert a coin cap store exists at the publisher's address.
        assert!(
            exists<CapabilityStore<CoinType>>(coin_type_publisher_address),
            E_NO_CAP_STORE
        );
        // Immutably borrow the mint capability for the coin type.
        let mint_cap_ref = &borrow_global<CapabilityStore<CoinType>>(
            coin_type_publisher_address
        ).mint_cap;
        // Deposit to caller's coin store the minted coins.
        coin::deposit(account_addr, coin::mint(amount, mint_cap_ref));
    }
}