/// This module defines a minimal and generic Coin and Balance.
/// modified from https://github.com/move-language/move/tree/main/language/documentation/tutorial
module aptos_framework::aptos_coin {
    use std::string;
    use std::error;
    use std::signer;
    use std::vector;
    use std::option::{Self, Option};

    use aptos_framework::coin::{Self, BurnCapability, MintCapability};
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;

    /// Error codes
    const ENO_CAPABILITIES: u64 = 1;
    const EALREADY_DELEGATED: u64 = 2;
    const EDELEGATION_NOT_FOUND: u64 = 3;

    struct AptosCoin has key { }

    struct Capabilities has key {
        mint_cap: MintCapability<AptosCoin>,
    }

    /// Delegation token created by delegator and can be claimed by the delegatee as MintCapability.
    struct DelegatedMintCapability has store {
        to: address
    }

    /// The container stores the current pending delegations.
    struct Delegations has key {
        inner: vector<DelegatedMintCapability>,
    }

    /// Can only called during genesis to initialize the Aptos coin.
    public(friend) fun initialize(aptos_framework: &signer): (MintCapability<AptosCoin>, BurnCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(aptos_framework);

        let (mint_cap, burn_cap) = coin::initialize<AptosCoin>(
            aptos_framework,
            string::utf8(b"Aptos Coin"),
            string::utf8(b"APT"),
            8, /* decimals */
            false, /* monitor_supply */
        );

        (mint_cap, burn_cap)
    }

    /// Can only be called during genesis for tests to grant mint capability to aptos framework and core resources
    /// accounts.
    public(friend) fun configure_accounts_for_test(
        aptos_framework: &signer,
        core_resources: &signer,
        mint_cap: MintCapability<AptosCoin>,
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);

        // Aptos framework needs mint cap to mint coins to initial validators.
        move_to(aptos_framework, Capabilities { mint_cap });

        // Mint the core resource account AptosCoin for gas so it can execute system transactions.
        coin::register<AptosCoin>(core_resources);
        let coins = coin::mint<AptosCoin>(
            18446744073709551615,
            &mint_cap,
        );
        coin::deposit<AptosCoin>(signer::address_of(core_resources), coins);

        move_to(core_resources, Capabilities { mint_cap });
        move_to(core_resources, Delegations { inner: vector::empty() });
    }

    /// Only callable in tests and testnets where the core resources account exists.
    /// Create new coins and deposit them into dst_addr's account.
    public entry fun mint(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) acquires Capabilities {
        let account_addr = signer::address_of(account);

        assert!(
            exists<Capabilities>(account_addr),
            error::not_found(ENO_CAPABILITIES),
        );

        let capabilities = borrow_global<Capabilities>(account_addr);
        let coins_minted = coin::mint<AptosCoin>(amount, &capabilities.mint_cap);
        coin::deposit<AptosCoin>(dst_addr, coins_minted);
    }

    /// Only callable in tests and testnets where the core resources account exists.
    /// Create delegated token for the address so the account could claim MintCapability later.
    public entry fun delegate_mint_capability(account: signer, to: address) acquires Delegations {
        system_addresses::assert_core_resource(&account);
        let delegations = &mut borrow_global_mut<Delegations>(@core_resources).inner;
        let i = 0;
        while (i < vector::length(delegations)) {
            let element = vector::borrow(delegations, i);
            assert!(element.to != to, error::invalid_argument(EALREADY_DELEGATED));
            i = i + 1;
        };
        vector::push_back(delegations, DelegatedMintCapability { to });
    }

    /// Only callable in tests and testnets where the core resources account exists.
    /// Claim the delegated mint capability and destroy the delegated token.
    public entry fun claim_mint_capability(account: &signer) acquires Delegations, Capabilities {
        let maybe_index = find_delegation(signer::address_of(account));
        assert!(option::is_some(&maybe_index), EDELEGATION_NOT_FOUND);
        let idx = *option::borrow(&maybe_index);
        let delegations = &mut borrow_global_mut<Delegations>(@core_resources).inner;
        let DelegatedMintCapability { to: _} = vector::swap_remove(delegations, idx);

        // Make a copy of mint cap and give it to the specified account.
        let mint_cap = borrow_global<Capabilities>(@core_resources).mint_cap;
        move_to(account, Capabilities { mint_cap });
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
    public fun initialize_for_test(aptos_framework: &signer): (MintCapability<AptosCoin>, BurnCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(aptos_framework);

        let (mint_cap, burn_cap) = coin::initialize<AptosCoin>(
            aptos_framework,
            string::utf8(b"Aptos Coin"),
            string::utf8(b"APT"),
            8, /* decimals */
            false, /* monitor_supply */
        );

        (mint_cap, burn_cap)
    }
}
