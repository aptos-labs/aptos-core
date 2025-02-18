/// This module defines a minimal and generic Coin and Balance.
/// modified from https://github.com/move-language/move/tree/main/language/documentation/tutorial
module supra_framework::supra_coin {
    use std::error;
    use std::signer;
    use std::string;
    use std::vector;
    use std::option::{Self, Option};

    use supra_framework::coin::{Self, BurnCapability, MintCapability};
    use supra_framework::system_addresses;

    friend supra_framework::genesis;


	/// Max supply of Supra Coin to be 100 billion with 8 decimal places fraction
	const MAX_SUPRA_COIN_SUPPLY: u128 = 100_000_000_000_00_000_000u128;
	//const MAX_SUPRA_COIN_SUPPLY: u128 = 340282366920938463463374607431768211455u128; 
    /// Account does not have mint capability
    const ENO_CAPABILITIES: u64 = 1;
    /// Mint capability has already been delegated to this specified address
    const EALREADY_DELEGATED: u64 = 2;
    /// Cannot find delegation of mint capability to this account
    const EDELEGATION_NOT_FOUND: u64 = 3;

    struct SupraCoin has key {}

    struct MintCapStore has key {
        mint_cap: MintCapability<SupraCoin>,
    }

    /// Delegation token created by delegator and can be claimed by the delegatee as MintCapability.
    struct DelegatedMintCapability has store {
        to: address
    }

    /// The container stores the current pending delegations.
    struct Delegations has key {
        inner: vector<DelegatedMintCapability>,
    }

    /// Can only called during genesis to initialize the Supra coin.
    public(friend) fun initialize(supra_framework: &signer): (BurnCapability<SupraCoin>, MintCapability<SupraCoin>) {
        system_addresses::assert_supra_framework(supra_framework);

        let (burn_cap, freeze_cap, mint_cap) =coin::initialize_with_parallelizable_supply_with_limit<SupraCoin>(
            supra_framework,
            string::utf8(b"Supra Coin"),
            string::utf8(b"SUPRA"),
            8, // decimals
            true, // monitor_supply
			MAX_SUPRA_COIN_SUPPLY,
        );

        // Supra framework needs mint cap to mint coins to initial validators. This will be revoked once the validators
        // have been initialized.
        move_to(supra_framework, MintCapStore { mint_cap });

        coin::destroy_freeze_cap(freeze_cap);
        (burn_cap, mint_cap)
    }

    public fun has_mint_capability(account: &signer): bool {
        exists<MintCapStore>(signer::address_of(account))
    }

    /// Only called during genesis to destroy the supra framework account's mint capability once all initial validators
    /// and accounts have been initialized during genesis.
    public(friend) fun destroy_mint_cap(supra_framework: &signer) acquires MintCapStore {
        system_addresses::assert_supra_framework(supra_framework);
        let MintCapStore { mint_cap } = move_from<MintCapStore>(@supra_framework);
        coin::destroy_mint_cap(mint_cap);
    }

    /// Can only be called during genesis for tests to grant mint capability to supra framework and core resources
    /// accounts.
    /// Expects account and SUPRA store to be registered before calling.
    public(friend) fun configure_accounts_for_test(
        supra_framework: &signer,
        core_resources: &signer,
        mint_cap: MintCapability<SupraCoin>,
    ) {
        system_addresses::assert_supra_framework(supra_framework);

        // Mint the core resource account SupraCoin for gas so it can execute system transactions.
        let coins = coin::mint<SupraCoin>(
            ((MAX_SUPRA_COIN_SUPPLY)/10 as u64),
            &mint_cap,
        );
        coin::deposit<SupraCoin>(signer::address_of(core_resources), coins);

        move_to(core_resources, MintCapStore { mint_cap });
        move_to(core_resources, Delegations { inner: vector::empty() });
    }

    /// Only callable in tests and testnets where the core resources account exists.
    /// Create new coins and deposit them into dst_addr's account.
    public entry fun mint(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) acquires MintCapStore {
        let account_addr = signer::address_of(account);

        assert!(
            exists<MintCapStore>(account_addr),
            error::not_found(ENO_CAPABILITIES),
        );

        let mint_cap = &borrow_global<MintCapStore>(account_addr).mint_cap;
        let coins_minted = coin::mint<SupraCoin>(amount, mint_cap);
        coin::deposit<SupraCoin>(dst_addr, coins_minted);
    }

    /// Only callable in tests and testnets where the core resources account exists.
    /// Create delegated token for the address so the account could claim MintCapability later.
    public entry fun delegate_mint_capability(account: signer, to: address) acquires Delegations {
        system_addresses::assert_core_resource(&account);
        let delegations = &mut borrow_global_mut<Delegations>(@core_resources).inner;
        vector::for_each_ref(delegations, |element| {
            let element: &DelegatedMintCapability = element;
            assert!(element.to != to, error::invalid_argument(EALREADY_DELEGATED));
        });
        vector::push_back(delegations, DelegatedMintCapability { to });
    }

    /// Only callable in tests and testnets where the core resources account exists.
    /// Claim the delegated mint capability and destroy the delegated token.
    public entry fun claim_mint_capability(account: &signer) acquires Delegations, MintCapStore {
        let maybe_index = find_delegation(signer::address_of(account));
        assert!(option::is_some(&maybe_index), EDELEGATION_NOT_FOUND);
        let idx = *option::borrow(&maybe_index);
        let delegations = &mut borrow_global_mut<Delegations>(@core_resources).inner;
        let DelegatedMintCapability { to: _ } = vector::swap_remove(delegations, idx);

        // Make a copy of mint cap and give it to the specified account.
        let mint_cap = borrow_global<MintCapStore>(@core_resources).mint_cap;
        move_to(account, MintCapStore { mint_cap });
    }

    fun find_delegation(addr: address): Option<u64> acquires Delegations {
        let delegations = &borrow_global<Delegations>(@core_resources).inner;
        let i = 0;
        let len = vector::length(delegations);
        let index = option::none();
        while (i < len) {
            let element = vector::borrow(delegations, i);
            if (element.to == addr) {
                index = option::some(i);
                break
            };
            i = i + 1;
        };
        index
    }

    #[test_only]
    use supra_framework::account;
    #[test_only]
    use supra_framework::aggregator_factory;
    #[test_only]
    use supra_framework::fungible_asset::FungibleAsset;

    #[test_only]
    public fun mint_apt_fa_for_test(amount: u64): FungibleAsset acquires MintCapStore {
        ensure_initialized_with_apt_fa_metadata_for_test();
        coin::coin_to_fungible_asset_for_test(
            coin::mint(
                amount,
                &borrow_global<MintCapStore>(@supra_framework).mint_cap
            )
        )
    }

    #[test_only]
    public fun ensure_initialized_with_apt_fa_metadata_for_test() {
        let supra_framework = account::create_signer_for_test(@supra_framework);
        if (!exists<MintCapStore>(@supra_framework)) {
            if (!aggregator_factory::aggregator_factory_exists_for_testing()) {
                aggregator_factory::initialize_aggregator_factory_for_test(&supra_framework);
            };
            let (burn_cap, mint_cap) = initialize(&supra_framework);
            coin::destroy_burn_cap(burn_cap);
            coin::destroy_mint_cap(mint_cap);
        };
        coin::create_coin_conversion_map(&supra_framework);
        coin::create_pairing<SupraCoin>(&supra_framework);
    }

    #[test_only]
    public fun initialize_for_test(supra_framework: &signer): (BurnCapability<SupraCoin>, MintCapability<SupraCoin>) {
        aggregator_factory::initialize_aggregator_factory_for_test(supra_framework);
        let (burn_cap, mint_cap) = initialize(supra_framework);
        coin::create_coin_conversion_map(supra_framework);
        coin::create_pairing<SupraCoin>(supra_framework);
        (burn_cap, mint_cap)
    }

    #[test_only]
    fun initialize_with_aggregator(supra_framework: &signer) {
        let (burn_cap, freeze_cap, mint_cap) =coin::initialize_with_parallelizable_supply_with_limit<SupraCoin>(
            supra_framework,
            string::utf8(b"Supra Coin"),
            string::utf8(b"SUPRA"),
            8, // decimals
            true, // monitor_supply
            MAX_SUPRA_COIN_SUPPLY,
        );
        coin::destroy_freeze_cap(freeze_cap);
        move_to(supra_framework, SupraCoinCapabilities {
            burn_cap,
            mint_cap,
        });
    }

    // This is particularly useful if the aggregator_factory is already initialized via another call path.
    #[test_only]
    public fun initialize_for_test_without_aggregator_factory(
        supra_framework: &signer
    ): (BurnCapability<SupraCoin>, MintCapability<SupraCoin>) {
        let (burn_cap, mint_cap) = initialize(supra_framework);
        coin::create_coin_conversion_map(supra_framework);
        coin::create_pairing<SupraCoin>(supra_framework);
        (burn_cap, mint_cap)
    }

    #[test_only]
    struct SupraCoinCapabilities has key {
        burn_cap: BurnCapability<SupraCoin>,
        mint_cap: MintCapability<SupraCoin>,
    }

	#[test(source = @0x1, destination = @0x2)]
    public entry fun end_to_end(
        source: signer,
        destination: signer,
    )  {
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);
        let destination_addr = signer::address_of(&destination);
        account::create_account_for_test(destination_addr);

        let name = string::utf8(b"Supra Coin");
        let symbol = string::utf8(b"SUPRA");

        aggregator_factory::initialize_aggregator_factory_for_test(&source);
        let (burn_cap,  mint_cap) = initialize(
            &source,
        );
        coin::register<SupraCoin>(&source);
        coin::register<SupraCoin>(&destination);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 0, 0);

        assert!(coin::name<SupraCoin>() == name, 1);
        assert!(coin::symbol<SupraCoin>() == symbol, 2);
        assert!(coin::decimals<SupraCoin>() == 8, 3);

        let coins_minted = coin::mint<SupraCoin>(100, &mint_cap);
        coin::deposit(source_addr, coins_minted);
        coin::transfer<SupraCoin>(&source, destination_addr, 50);

        assert!(coin::balance<SupraCoin>(source_addr) == 50, 4);
        assert!(coin::balance<SupraCoin>(destination_addr) == 50, 5);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 100, 6);

        let coin = coin::withdraw<SupraCoin>(&source, 10);
        assert!(coin::value(&coin) == 10, 7);
        coin::burn(coin, &burn_cap);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 90, 8);

        move_to(&source, SupraCoinCapabilities {
            burn_cap,
            mint_cap,
        });
    }

    #[test(source = @0x1, destination = @0x2)]
    public entry fun test_mint_no_overflow(
        source: signer,
        destination: signer,
    ){
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);
        let destination_addr = signer::address_of(&destination);
        account::create_account_for_test(destination_addr);

        aggregator_factory::initialize_aggregator_factory_for_test(&source);
        let (burn_cap,  mint_cap) = initialize(
            &source,
        );
        coin::register<SupraCoin>(&source);
        coin::register<SupraCoin>(&destination);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 0, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 0, 0);

        let coins_minted = coin::mint<SupraCoin>((MAX_SUPRA_COIN_SUPPLY as u64), &mint_cap);
        coin::deposit(source_addr, coins_minted);
        coin::transfer<SupraCoin>(&source, destination_addr, (MAX_SUPRA_COIN_SUPPLY as u64));
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(source = @0x1)]
    #[expected_failure(abort_code = 0x20001, location = supra_framework::aggregator)]
    public entry fun test_mint_overflow(
        source: signer,
    ) {
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);

        aggregator_factory::initialize_aggregator_factory_for_test(&source);
        let (burn_cap, mint_cap) = initialize(
            &source,
        );
        coin::register<SupraCoin>(&source);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 0, 0);

        let coins_minted = coin::mint<SupraCoin>((MAX_SUPRA_COIN_SUPPLY as u64)+1, &mint_cap);
        coin::deposit(source_addr, coins_minted);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
}
