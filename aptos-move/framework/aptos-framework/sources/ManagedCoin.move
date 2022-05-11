/// ManagedCoin is built to make a simple walkthrough of the Coins module.
/// It contains scripts you will need to initialize, mint, burn, transfer coins.
/// By utilizing this current module, a developer can create his own coin and care less about mint and burn capabilities,
module AptosFramework::ManagedCoin {
    use Std::ASCII;
    use Std::Errors;
    use Std::Signer;

    use AptosFramework::Coin::{Self, BurnCapability, MintCapability};
    use AptosFramework::TypeInfo;

    // Errors

    /// When no capabilities (burn/mint) found on an account.
    const ENO_CAPABILITIES: u64 = 0;

    // Data structures

    /// Capabilities resource storing mint and burn capabilities.
    /// The resource is stored on the account that initialized coin `CoinType`.
    struct Capabilities<phantom CoinType> has key {
        mint_cap: MintCapability<CoinType>,
        burn_cap: BurnCapability<CoinType>,
    }

    // Public functions

    /// Withdraw an `amount` of coin `CoinType` from `account` and burn it.
    public(script) fun burn<CoinType>(
        account: &signer,
        amount: u64,
    ) acquires Capabilities {
        let type_info = TypeInfo::type_of<CoinType>();
        let coin_address = TypeInfo::account_address(&type_info);

        assert!(
            exists<Capabilities<CoinType>>(coin_address),
            Errors::not_published(ENO_CAPABILITIES),
        );

        let capabilities = borrow_global<Capabilities<CoinType>>(coin_address);

        let to_burn = Coin::withdraw<CoinType>(account, amount);
        Coin::burn(to_burn, &capabilities.burn_cap);
    }

    /// Initialize new coin `CoinType` in Aptos Blockchain.
    /// Mint and Burn Capabilities will be stored under `account` in `Capabilities` resource.
    public fun initialize<CoinType>(
        account: &signer,
        name: vector<u8>,
        symbol: vector<u8>,
        decimals: u64,
        monitor_supply: bool,
    ) {
        let (mint_cap, burn_cap) = Coin::initialize<CoinType>(
            account,
            ASCII::string(name),
            ASCII::string(symbol),
            decimals,
            monitor_supply,
        );

        move_to(account, Capabilities<CoinType>{
            mint_cap,
            burn_cap,
        });
    }

    /// Create new coins `CoinType` and deposit them into dst_addr's account.
    public(script) fun mint<CoinType>(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) acquires Capabilities {
        let account_addr = Signer::address_of(account);

        assert!(
            exists<Capabilities<CoinType>>(account_addr),
            Errors::not_published(ENO_CAPABILITIES),
        );

        let capabilities = borrow_global<Capabilities<CoinType>>(account_addr);
        let coins_minted = Coin::mint(amount, &capabilities.mint_cap);
        Coin::deposit(dst_addr, coins_minted);
    }

    /// Creating a resource that stores balance of `CoinType` on user's account, withdraw and deposit event handlers.
    /// Required if user wants to start accepting deposits of `CoinType` in his account.
    public(script) fun register<CoinType>(account: &signer) {
        Coin::register<CoinType>(account);
    }

     /// Transfers `amount` of coins `CoinType` from `from` to `to`.
    public(script) fun transfer<CoinType>(
        from: &signer,
        to: address,
        amount: u64,
    ) {
         Coin::transfer<CoinType>(from, to, amount);
    }

    //
    // Tests
    //
    #[test_only]
    use Std::Option;

    #[test_only]
    struct FakeMoney { }

    #[test(source = @0x1, destination = @0x2)]
    public(script) fun test_end_to_end(
        source: signer,
        destination: signer,
    ) acquires Capabilities {
        let source_addr = Signer::address_of(&source);
        let destination_addr = Signer::address_of(&destination);

        initialize<FakeMoney>(
            &source,
            b"Fake Money",
            b"FMD",
            10,
            true
        );
        assert!(Coin::is_registered<FakeMoney>(), 0);

        register<FakeMoney>(&source);
        register<FakeMoney>(&destination);

        mint<FakeMoney>(&source, source_addr, 50);
        mint<FakeMoney>(&source, destination_addr, 10);
        assert!(Coin::balance<FakeMoney>(source_addr) == 50, 1);
        assert!(Coin::balance<FakeMoney>(destination_addr) == 10, 2);

        let supply = Coin::supply<FakeMoney>();
        assert!(Option::is_some(&supply), 1);
        assert!(Option::extract(&mut supply) == 60, 2);

        transfer<FakeMoney>(&source, destination_addr, 10);
        assert!(Coin::balance<FakeMoney>(source_addr) == 40, 3);
        assert!(Coin::balance<FakeMoney>(destination_addr) == 20, 4);

        burn<FakeMoney>(&source, 40);
        burn<FakeMoney>(&destination, 10);

        assert!(Coin::balance<FakeMoney>(source_addr) == 0, 1);
        assert!(Coin::balance<FakeMoney>(destination_addr) == 10, 2);

        let new_supply = Coin::supply<FakeMoney>();
        assert!(Option::extract(&mut new_supply) == 10, 2);
    }
}
