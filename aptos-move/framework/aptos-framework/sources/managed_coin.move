/// ManagedCoin is built to make a simple walkthrough of the Coins module.
/// It contains scripts you will need to initialize, mint, burn, transfer coins.
/// By utilizing this current module, a developer can create his own coin and care less about mint and burn capabilities,
///
/// ## High-Level Properties
///
///
/// | No. | Property                                                                                                              | Criticality | Implementation                                                                                                                                                           | Enforcement                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
/// |-----|-----------------------------------------------------------------------------------------------------------------------|-------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
/// | 1   | The initializing account should hold the capabilities to operate the coin.                                            | Critical    | The capabilities are stored under the initializing account under the Capabilities resource, which is distinct for a distinct type of coin.                               | Enforced via: [initialize](https://github.com/aptos-labs/aptos-core/blob/cdb1f27868890a49075356d626e91d73f8ee3170/aptos-move/framework/aptos-framework/sources/managed_coin.spec.move#L60)                                                                                                                                                                                                                                                                                                                                                  |
/// | 2   | A new coin should be properly initialized.                                                                            | High        | In the initialize function, a new coin is initialized via the coin module with the specified properties.                                                                 | Enforced via: [initialize\_internal](https://github.com/aptos-labs/aptos-core/blob/37d7a428eaadf6ff99eb9fd302a689405b20c2c5/aptos-move/framework/aptos-framework/sources/coin.spec.move#L305).                                                                                                                                                                                                                                                                                                                                              |
/// | 3   | Minting/Burning should only be done by the account who hold the valid capabilities.                                   | High        | The mint and burn capabilities are moved under the initializing account and retrieved, while minting/burning                                                             | Enforced via: [initialize](https://github.com/aptos-labs/aptos-core/blob/cdb1f27868890a49075356d626e91d73f8ee3170/aptos-move/framework/aptos-framework/sources/managed_coin.spec.move#L60), [burn](https://github.com/aptos-labs/aptos-core/blob/cdb1f27868890a49075356d626e91d73f8ee3170/aptos-move/framework/aptos-framework/sources/managed_coin.spec.move#L24), [mint](https://github.com/aptos-labs/aptos-core/blob/cdb1f27868890a49075356d626e91d73f8ee3170/aptos-move/framework/aptos-framework/sources/managed_coin.spec.move#L71). |
/// | 4   | If the total supply of coins is being monitored, burn and mint operations will appropriately adjust the total supply. | High        | The coin::burn and coin::mint functions, when tracking the supply, adjusts the total coin supply accordingly.                                                            | Formally Verified: [TotalSupplyNoChange](https://github.com/aptos-labs/aptos-core/blob/005aca2ae22a1200871c4679b606c84210fdfb94/aptos-move/framework/aptos-framework/sources/coin.spec.move#L8).                                                                                                                                                                                                                                                                                                                                            |
/// | 5   | Before burning coins, exact amount of coins are withdrawn.                                                            | High        | After utilizing the coin::withdraw function to withdraw coins, they are then burned, and the function ensures the precise return of the initially specified coin amount. | Enforced via: [burn\_from](https://github.com/aptos-labs/aptos-core/blob/37d7a428eaadf6ff99eb9fd302a689405b20c2c5/aptos-move/framework/aptos-framework/sources/coin.spec.move#L179).                                                                                                                                                                                                                                                                                                                                                        |
/// | 6   | Minted coins are deposited to the provided destination address.                                                       | High        | After the coins are minted via coin::mint they are deposited into the coinstore of the destination address.                                                              | _This should be formally verified via a post condition to ensure that coins are deposited to the destination address._                                                                                                                                                                                                                                                                                                                                                                                                                      |
///
module aptos_framework::managed_coin {
    use std::string;
    use std::error;
    use std::signer;

    use aptos_framework::coin::{Self, BurnCapability, FreezeCapability, MintCapability};

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

    //
    // Tests
    //

    #[test_only]
    use std::option;

    #[test_only]
    use aptos_framework::aggregator_factory;

    #[test_only]
    struct FakeMoney {}

    #[test(source = @0xa11ce, destination = @0xb0b, mod_account = @0x1)]
    public entry fun test_end_to_end(
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
