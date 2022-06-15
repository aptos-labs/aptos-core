/// This module defines a minimal and generic Coin and Balance.
/// modified from https://github.com/move-language/move/tree/main/language/documentation/tutorial
module AptosFramework::TestCoin {
    use Std::ASCII;
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use Std::Option::{Self, Option};

    use AptosFramework::Coin::{Self, BurnCapability, MintCapability};
    use AptosFramework::SystemAddresses;

    /// Error codes
    const ENO_CAPABILITIES: u64 = 1;
    const EALREADY_DELEGATED: u64 = 2;
    const EDELEGATION_NOT_FOUND: u64 = 3;

    struct TestCoin has key { }

    struct Capabilities has key {
        mint_cap: MintCapability<TestCoin>,
    }

    /// Delegation token created by delegator and can be claimed by the delegatee as MintCapability.
    struct DelegatedMintCapability has store {
        to: address
    }

    /// The container stores the current pending delegations.
    struct Delegations has key {
        inner: vector<DelegatedMintCapability>,
    }

    public fun initialize(
        core_framework: &signer,
        core_resource: &signer,
    ): (MintCapability<TestCoin>, BurnCapability<TestCoin>) {
        SystemAddresses::assert_core_resource(core_resource);

        let (mint_cap, burn_cap) = Coin::initialize<TestCoin>(
            core_framework,
            ASCII::string(b"Test Coin"),
            ASCII::string(b"TC"),
            6, /* decimals */
            false, /* monitor_supply */
        );

        // Mint the core resource account TestCoin for gas so it can execute system transactions.
        Coin::register_internal<TestCoin>(core_resource);
        let coins = Coin::mint<TestCoin>(
            18446744073709551615,
            &mint_cap,
        );
        Coin::deposit<TestCoin>(Signer::address_of(core_resource), coins);

        // Save MintCapability so we can give Faucet mint capability when one claims.
        move_to(core_framework, Capabilities { mint_cap: copy mint_cap });

        // Also give Core resources account mint capability so it can mint test coins if needed.
        move_to(core_resource, Capabilities { mint_cap: copy mint_cap });

        // Give Core Resources ability to delegate/create mint capability so it can grant the
        // faucet account mint cap.
        move_to(core_resource, Delegations { inner: Vector::empty() });

        (mint_cap, burn_cap)
    }

    /// Create new test coins and deposit them into dst_addr's account.
    public(script) fun mint(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) acquires Capabilities {
        let account_addr = Signer::address_of(account);

        assert!(
            exists<Capabilities>(account_addr),
            Errors::not_published(ENO_CAPABILITIES),
        );

        let capabilities = borrow_global<Capabilities>(account_addr);
        let coins_minted = Coin::mint<TestCoin>(amount, &capabilities.mint_cap);
        Coin::deposit<TestCoin>(dst_addr, coins_minted);
    }

    /// Create delegated token for the address so the account could claim MintCapability later.
    public(script) fun delegate_mint_capability(account: signer, to: address) acquires Delegations {
        SystemAddresses::assert_core_resource(&account);
        let delegations = &mut borrow_global_mut<Delegations>(@CoreResources).inner;
        let i = 0;
        while (i < Vector::length(delegations)) {
            let element = Vector::borrow(delegations, i);
            assert!(element.to != to, Errors::invalid_argument(EALREADY_DELEGATED));
            i = i + 1;
        };
        Vector::push_back(delegations, DelegatedMintCapability { to });
    }

    /// Claim the delegated mint capability and destroy the delegated token.
    public(script) fun claim_mint_capability(account: &signer) acquires Delegations, Capabilities {
        let maybe_index = find_delegation(Signer::address_of(account));
        assert!(Option::is_some(&maybe_index), EDELEGATION_NOT_FOUND);
        let idx = *Option::borrow(&maybe_index);
        let delegations = &mut borrow_global_mut<Delegations>(@CoreResources).inner;
        let DelegatedMintCapability { to: _} = Vector::swap_remove(delegations, idx);

        // Make a copy of mint cap and give it to the specified account.
        let mint_cap = borrow_global<Capabilities>(@AptosFramework).mint_cap;
        move_to(account, Capabilities { mint_cap });
    }

    fun find_delegation(addr: address): Option<u64> acquires Delegations {
        let delegations = &borrow_global<Delegations>(@CoreResources).inner;
        let i = 0;
        let len = Vector::length(delegations);
        let index = Option::none();
        while (i < len) {
            let element = Vector::borrow(delegations, i);
            if (element.to == addr) {
                index = Option::some(i);
                break
            };
            i = i + 1;
        };
        index
    }
}
