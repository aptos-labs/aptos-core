/// ManagedCoin is built to make a simple walkthrough of the Coins module.
/// It contains scripts you will need to initialize, mint, burn, transfer coins.
/// By utilizing this current module, a developer can create his own coin and care less about mint and burn capabilities,
module aptos_framework::managed_coin {
    use std::string;
    use std::error;
    use std::signer;

    use aptos_framework::coin::{Self, BurnCapability, FreezeCapability, MintCapability, destroy_burn_cap,
        destroy_freeze_cap, destroy_mint_cap
    };

    //
    // Errors
    //

    /// Account has no capabilities (burn/mint).
    const ENO_CAPABILITIES: u64 = 1;

    //
    // Data structures
    //

    /// Capabilities resource storing mint and burn capabilities.
    /// The resource is stored on the account that initialized coin `CoinType`.
    struct Capabilities<phantom CoinType> has key {
        burn_cap: BurnCapability<CoinType>,
        freeze_cap: FreezeCapability<CoinType>,
        mint_cap: MintCapability<CoinType>,
    }

    //
    // Public functions
    //

    /// Withdraw an `amount` of coin `CoinType` from `account` and burn it.
    public entry fun burn<CoinType>(
        account: &signer,
        amount: u64,
    ) acquires Capabilities {
        let account_addr = signer::address_of(account);

        assert!(
            exists<Capabilities<CoinType>>(account_addr),
            error::not_found(ENO_CAPABILITIES),
        );

        let capabilities = borrow_global<Capabilities<CoinType>>(account_addr);

        let to_burn = coin::withdraw<CoinType>(account, amount);
        coin::burn(to_burn, &capabilities.burn_cap);
    }

    /// Initialize new coin `CoinType` in Aptos Blockchain.
    /// Mint and Burn Capabilities will be stored under `account` in `Capabilities` resource.
    public entry fun initialize<CoinType>(
        account: &signer,
        name: vector<u8>,
        symbol: vector<u8>,
        decimals: u8,
        monitor_supply: bool,
    ) {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<CoinType>(
            account,
            string::utf8(name),
            string::utf8(symbol),
            decimals,
            monitor_supply,
        );

        move_to(account, Capabilities<CoinType> {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    /// Create new coins `CoinType` and deposit them into dst_addr's account.
    public entry fun mint<CoinType>(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) acquires Capabilities {
        let account_addr = signer::address_of(account);

        assert!(
            exists<Capabilities<CoinType>>(account_addr),
            error::not_found(ENO_CAPABILITIES),
        );

        let capabilities = borrow_global<Capabilities<CoinType>>(account_addr);
        let coins_minted = coin::mint(amount, &capabilities.mint_cap);
        coin::deposit(dst_addr, coins_minted);
    }

    /// Creating a resource that stores balance of `CoinType` on user's account, withdraw and deposit event handlers.
    /// Required if user wants to start accepting deposits of `CoinType` in his account.
    public entry fun register<CoinType>(account: &signer) {
        coin::register<CoinType>(account);
    }

    /// Destroys capabilities from the account, so that the user no longer has access to mint or burn.
    public entry fun destroy_caps<CoinType>(account: &signer) acquires Capabilities {
        let (burn_cap, freeze_cap, mint_cap) = remove_caps<CoinType>(account);
        destroy_burn_cap(burn_cap);
        destroy_freeze_cap(freeze_cap);
        destroy_mint_cap(mint_cap);
    }

    /// Removes capabilities from the account to be stored or destroyed elsewhere
    public fun remove_caps<CoinType>(
        account: &signer
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) acquires Capabilities {
        let account_addr = signer::address_of(account);
        assert!(
            exists<Capabilities<CoinType>>(account_addr),
            error::not_found(ENO_CAPABILITIES),
        );

        let Capabilities<CoinType> {
            burn_cap,
            freeze_cap,
            mint_cap,
        } = move_from<Capabilities<CoinType>>(account_addr);
        (burn_cap, freeze_cap, mint_cap)
    }

    //
    // Tests
    //

    #[test_only]
    use std::option;

    #[test_only]
    use aptos_framework::aggregator_factory;

    #[test_only]
    struct FakeMoney {}

    #[test(framework = @aptos_framework, source = @0xa11ce, destination = @0xb0b, mod_account = @0x1)]
    public entry fun test_end_to_end(
        framework: signer,
        source: signer,
        destination: signer,
        mod_account: signer
    ) acquires Capabilities {
        let source_addr = signer::address_of(&source);
        let destination_addr = signer::address_of(&destination);
        aptos_framework::account::create_account_for_test(source_addr);
        aptos_framework::account::create_account_for_test(destination_addr);
        aptos_framework::account::create_account_for_test(signer::address_of(&mod_account));
        aggregator_factory::initialize_aggregator_factory_for_test(&mod_account);
        aptos_framework::coin::create_coin_conversion_map(&framework);

        initialize<FakeMoney>(
            &mod_account,
            b"Fake Money",
            b"FMD",
            10,
            true
        );
        assert!(coin::is_coin_initialized<FakeMoney>(), 0);

        coin::register<FakeMoney>(&mod_account);
        register<FakeMoney>(&source);
        register<FakeMoney>(&destination);

        mint<FakeMoney>(&mod_account, source_addr, 50);
        mint<FakeMoney>(&mod_account, destination_addr, 10);
        assert!(coin::balance<FakeMoney>(source_addr) == 50, 1);
        assert!(coin::balance<FakeMoney>(destination_addr) == 10, 2);

        let supply = coin::supply<FakeMoney>();
        assert!(option::is_some(&supply), 1);
        assert!(option::extract(&mut supply) == 60, 2);

        coin::transfer<FakeMoney>(&source, destination_addr, 10);
        assert!(coin::balance<FakeMoney>(source_addr) == 40, 3);
        assert!(coin::balance<FakeMoney>(destination_addr) == 20, 4);

        coin::transfer<FakeMoney>(&source, signer::address_of(&mod_account), 40);
        burn<FakeMoney>(&mod_account, 40);

        assert!(coin::balance<FakeMoney>(source_addr) == 0, 1);

        let new_supply = coin::supply<FakeMoney>();
        assert!(option::extract(&mut new_supply) == 20, 2);

        // Destroy mint capabilities
        destroy_caps<FakeMoney>(&mod_account);
        assert!(!exists<Capabilities<FakeMoney>>(signer::address_of(&mod_account)), 3);
    }

    #[test(source = @0xa11ce, destination = @0xb0b, mod_account = @0x1)]
    public entry fun test_end_to_end_caps_removal(
        source: signer,
        destination: signer,
        mod_account: signer
    ) acquires Capabilities {
        let source_addr = signer::address_of(&source);
        let destination_addr = signer::address_of(&destination);
        aptos_framework::account::create_account_for_test(source_addr);
        aptos_framework::account::create_account_for_test(destination_addr);
        aptos_framework::account::create_account_for_test(signer::address_of(&mod_account));
        aggregator_factory::initialize_aggregator_factory_for_test(&mod_account);

        initialize<FakeMoney>(
            &mod_account,
            b"Fake Money",
            b"FMD",
            10,
            true
        );
        assert!(coin::is_coin_initialized<FakeMoney>(), 0);

        // Remove capabilities
        let (burn_cap, freeze_cap, mint_cap) = remove_caps<FakeMoney>(&mod_account);
        assert!(!exists<Capabilities<FakeMoney>>(signer::address_of(&mod_account)), 3);
        coin::destroy_mint_cap(mint_cap);
        coin::destroy_freeze_cap(freeze_cap);
        coin::destroy_burn_cap(burn_cap);
    }

    #[test(source = @0xa11ce, destination = @0xb0b, mod_account = @0x1)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun fail_mint(
        source: signer,
        destination: signer,
        mod_account: signer,
    ) acquires Capabilities {
        let source_addr = signer::address_of(&source);

        aptos_framework::account::create_account_for_test(source_addr);
        aptos_framework::account::create_account_for_test(signer::address_of(&destination));
        aptos_framework::account::create_account_for_test(signer::address_of(&mod_account));
        aggregator_factory::initialize_aggregator_factory_for_test(&mod_account);

        initialize<FakeMoney>(&mod_account, b"Fake money", b"FMD", 1, true);
        coin::register<FakeMoney>(&mod_account);
        register<FakeMoney>(&source);
        register<FakeMoney>(&destination);

        mint<FakeMoney>(&destination, source_addr, 100);
    }

    #[test(source = @0xa11ce, destination = @0xb0b, mod_account = @0x1)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun fail_burn(
        source: signer,
        destination: signer,
        mod_account: signer,
    ) acquires Capabilities {
        let source_addr = signer::address_of(&source);

        aptos_framework::account::create_account_for_test(source_addr);
        aptos_framework::account::create_account_for_test(signer::address_of(&destination));
        aptos_framework::account::create_account_for_test(signer::address_of(&mod_account));
        aggregator_factory::initialize_aggregator_factory_for_test(&mod_account);

        initialize<FakeMoney>(&mod_account, b"Fake money", b"FMD", 1, true);
        coin::register<FakeMoney>(&mod_account);
        register<FakeMoney>(&source);
        register<FakeMoney>(&destination);

        mint<FakeMoney>(&mod_account, source_addr, 100);
        burn<FakeMoney>(&destination, 10);
    }
}
