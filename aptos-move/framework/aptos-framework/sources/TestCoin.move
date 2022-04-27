/// This module defines a minimal and generic Coin and Balance.
/// modified from https://github.com/move-language/move/tree/main/language/documentation/tutorial
module AptosFramework::TestCoin {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use Std::Option::{Self, Option};

    use AptosFramework::SystemAddresses;

    friend AptosFramework::TransactionFee;

    /// Error codes
    const EINSUFFICIENT_BALANCE: u64 = 0;
    const EALREADY_HAS_BALANCE: u64 = 1;
    const EBALANCE_NOT_PUBLISHED: u64 = 2;
    const EALREADY_DELEGATED: u64 = 3;
    const EDELEGATION_NOT_FOUND: u64 = 4;

    struct Coin has store {
        value: u64
    }

    /// Represnets the metadata of the coin, store @CoreResources.
    struct CoinInfo has key {
        scaling_factor: u64,
    }

    /// Capability required to mint coins.
    struct MintCapability has key, store { }
    /// Capability required to burn coins.
    struct BurnCapability has key, store { }

    /// Delegation token created by delegator and can be claimed by the delegatee as MintCapability.
    struct DelegatedMintCapability has store {
        to: address
    }

    /// The container stores the current pending delegations.
    struct Delegations has key {
        inner: vector<DelegatedMintCapability>,
    }

    public fun initialize(core_resource: &signer, scaling_factor: u64) {
        SystemAddresses::assert_core_resource(core_resource);
        move_to(core_resource, MintCapability {});
        move_to(core_resource, BurnCapability {});
        move_to(core_resource, CoinInfo { scaling_factor });
        move_to(core_resource, Delegations { inner: Vector::empty() });
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
    public(script) fun claim_mint_capability(account: &signer) acquires Delegations {
        let maybe_index = find_delegation(Signer::address_of(account));
        assert!(Option::is_some(&maybe_index), EDELEGATION_NOT_FOUND);
        let idx = *Option::borrow(&maybe_index);
        let delegations = &mut borrow_global_mut<Delegations>(@CoreResources).inner;
        let DelegatedMintCapability { to: _} = Vector::swap_remove(delegations, idx);

        move_to(account, MintCapability {});
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

    /// Mint with Capability.
    public fun mint(account: &signer, amount: u64): Coin acquires  MintCapability {
        let sender_addr = Signer::address_of(account);
        let _cap = borrow_global<MintCapability>(sender_addr);
        Coin { value: amount }
    }

    /// Split `amount` number of coins.
    public fun split(coin: &mut Coin, amount: u64): Coin {
        // balance must be greater than the withdraw amount
        assert!(coin.value >= amount, Errors::limit_exceeded(EINSUFFICIENT_BALANCE));
        coin.value = coin.value - amount;
        Coin { value: amount }
    }

    /// Merge coins.
    public fun merge(coin: &mut Coin, check: Coin){
        let Coin { value } = check;
        coin.value = coin.value + value
    }

    /// Burn coins with capability.
    public fun burn(account: &signer, coins: Coin) acquires BurnCapability {
        let cap = borrow_global<BurnCapability>(Signer::address_of(account));
        burn_with_capability(coins, cap);
    }

    fun burn_with_capability(coins: Coin, _cap: &BurnCapability) {
        let Coin { value: _ } = coins;
    }

    /// Burn transaction gas.
    public(friend) fun burn_gas(fee: Coin) acquires BurnCapability {
        let cap = borrow_global<BurnCapability>(@CoreResources);
        burn_with_capability(fee, cap);
    }

    public fun scaling_factor(): u64 acquires  CoinInfo {
        borrow_global<CoinInfo>(@CoreResources).scaling_factor
    }

    public fun zero(): Coin {
        Coin {value: 0}
    }

    public fun value(coin: &Coin): u64 {
        coin.value
    }

    #[test(account = @0x1)]
    #[expected_failure] // This test should abort
    fun mint_non_owner(account: signer) acquires MintCapability {
        // Make sure the address we've chosen doesn't match the module
        // owner address
        assert!(Signer::address_of(&account) != @CoreResources, 0);
        let Coin { value: _ } = mint(&account, 10);
    }

    #[test(account = @CoreResources)]
    public(script) fun mint_check_balance_and_supply(
        account: signer,
    ) acquires MintCapability {
        initialize(&account, 1000000);
        let Coin{ value } = mint(&account, 42);
        assert!(value == 42, 0);
    }

    #[test(account = @CoreResources)]
    fun can_split_amount(account: signer) acquires MintCapability {
        initialize(&account, 1000000);
        let amount = 1000;
        let coins = mint(&account, amount);
        let Coin { value } = split(&mut coins, 100);
        assert!(value == 100, 0);
        let Coin { value } = coins;
        assert!(value == 900, 0);
    }

    #[test(account = @CoreResources)]
    fun successful_burn(account: signer) acquires MintCapability, BurnCapability {
        initialize(&account, 1000000);
        let amount = 1000;
        let coins = mint(&account, amount);
        burn(&account, coins);
    }

    #[test(account = @CoreResources, another = @0x1)]
    #[expected_failure]
    fun failed_burn(account: signer, another: signer) acquires MintCapability, BurnCapability {
        initialize(&account, 1000000);
        let amount = 1000;
        let coins = mint(&account, amount);
        burn(&another, coins);
    }

    #[test(account = @CoreResources, account_clone = @CoreResources, delegatee = @0x1)]
    public(script) fun mint_delegation_success(account: signer, account_clone: signer, delegatee: signer) acquires Delegations, MintCapability  {
        initialize(&account, 1000000);
        let addr = Signer::address_of(&delegatee);
        let addr1 = @0x2;
        // make sure can delegate more than one
        delegate_mint_capability(account_clone, addr1);
        delegate_mint_capability(account, addr);
        claim_mint_capability(&delegatee);

        let Coin { value } = mint(&delegatee, 1000);
        assert!(value == 1000, 0);
    }

    #[test(account = @CoreResources, random = @0x1)]
    #[expected_failure]
    public(script) fun mint_delegation_claim_fail(account: signer, random: signer) acquires Delegations  {
        initialize(&account, 1000000);
        let delegatee = @0x1234;
        delegate_mint_capability(account, delegatee);
        claim_mint_capability(&random);
    }

    #[test(account = @CoreResources, random = @0x1)]
    #[expected_failure]
    public(script) fun mint_delegation_delegate_fail(account: signer, random: signer) acquires Delegations  {
        initialize(&account, 1000000);
        delegate_mint_capability(random, @0x1);
    }

    #[test_only]
    public fun mint_for_test(amount: u64): Coin {
        Coin {value: amount}
    }
}
