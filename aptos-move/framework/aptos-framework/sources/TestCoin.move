/// This module defines a minimal and generic Coin and Balance.
/// modified from https://github.com/move-language/move/tree/main/language/documentation/tutorial
module AptosFramework::TestCoin {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use Std::Event::{Self, EventHandle};
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

    /// Struct representing the balance of each address.
    struct Balance has key {
        coin: Coin
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

    /// Events handles.
    struct TransferEvents has key {
        sent_events: EventHandle<SentEvent>,
        received_events: EventHandle<ReceivedEvent>,
    }

    struct SentEvent has drop, store {
        amount: u64,
        to: address,
    }

    struct ReceivedEvent has drop, store {
        amount: u64,
        from: address,
    }

    public fun initialize(core_resource: &signer, scaling_factor: u64) {
        SystemAddresses::assert_core_resource(core_resource);
        move_to(core_resource, MintCapability {});
        move_to(core_resource, BurnCapability {});
        move_to(core_resource, CoinInfo { scaling_factor });
        move_to(core_resource, Delegations { inner: Vector::empty() });
        register(core_resource);
    }

    /// Publish an empty balance resource under `account`'s address. This function must be called before
    /// minting or transferring to the account.
    public fun register(account: &signer) {
        let empty_coin = Coin { value: 0 };
        assert!(!exists<Balance>(Signer::address_of(account)), Errors::already_published(EALREADY_HAS_BALANCE));
        move_to(account, Balance { coin:  empty_coin });
        move_to(
            account,
            TransferEvents {
                sent_events: Event::new_event_handle<SentEvent>(account),
                received_events: Event::new_event_handle<ReceivedEvent>(account),
            }
        );
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

    /// Mint coins with capability.
    public(script) fun mint(
        account: &signer,
        mint_addr: address,
        amount: u64
    ) acquires Balance, MintCapability
    {
        mint_internal(account, mint_addr, amount);
    }

    public fun mint_internal(account: &signer, mint_addr: address, amount: u64) acquires Balance, MintCapability {
        let sender_addr = Signer::address_of(account);
        let _cap = borrow_global<MintCapability>(sender_addr);
        // Deposit `amount` of tokens to `mint_addr`'s balance
        deposit(mint_addr, Coin { value: amount });
    }

    public fun exists_at(addr: address): bool{
        exists<Balance>(addr)
    }

    /// Returns the balance of `owner`.
    public fun balance_of(owner: address): u64 acquires Balance {
        assert!(exists<Balance>(owner), Errors::not_published(EBALANCE_NOT_PUBLISHED));
        borrow_global<Balance>(owner).coin.value
    }

    /// Transfers `amount` of tokens from `from` to `to`.
    public(script) fun transfer(from: &signer, to: address, amount: u64) acquires Balance, TransferEvents {
        let check = withdraw(from, amount);
        deposit(to, check);
        // emit events
        let sender_handle = borrow_global_mut<TransferEvents>(Signer::address_of(from));
        Event::emit_event<SentEvent>(
            &mut sender_handle.sent_events,
            SentEvent { amount, to },
        );
        let receiver_handle = borrow_global_mut<TransferEvents>(to);
        Event::emit_event<ReceivedEvent>(
            &mut receiver_handle.received_events,
            ReceivedEvent { amount, from: Signer::address_of(from) },
        );
    }

    /// Withdraw `amount` number of tokens from the balance under `addr`.
    public fun withdraw(signer: &signer, amount: u64) : Coin acquires Balance {
        let addr = Signer::address_of(signer);
        let balance = balance_of(addr);
        // balance must be greater than the withdraw amount
        assert!(balance >= amount, Errors::limit_exceeded(EINSUFFICIENT_BALANCE));
        let balance_ref = &mut borrow_global_mut<Balance>(addr).coin.value;
        *balance_ref = balance - amount;
        Coin { value: amount }
    }

    /// Deposit `amount` number of tokens to the balance under `addr`.
    public fun deposit(addr: address, check: Coin) acquires Balance {
        let balance = balance_of(addr);
        let balance_ref = &mut borrow_global_mut<Balance>(addr).coin.value;
        let Coin { value } = check;
        *balance_ref = balance + value;
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

    public fun merge(lhs: &mut Coin, rhs: Coin) {
        let Coin { value } = rhs;
        lhs.value = lhs.value + value;
    }

    public fun value(coin: &Coin): u64 {
        coin.value
    }

    #[test(account = @0x1)]
    #[expected_failure] // This test should abort
    fun mint_non_owner(account: signer) acquires Balance, MintCapability {
        // Make sure the address we've chosen doesn't match the module
        // owner address
        register(&account);
        assert!(Signer::address_of(&account) != @CoreResources, 0);
        mint_internal(&account, @0x1, 10);
    }

    #[test(account = @CoreResources)]
    public(script) fun mint_check_balance_and_supply(
        account: signer,
    ) acquires Balance, MintCapability {
        initialize(&account, 1000000);
        let addr = Signer::address_of(&account);
        mint(&account, @CoreResources, 42);
        assert!(balance_of(addr) == 42, 0);
    }

    #[test(account = @0x1)]
    fun register_has_zero(account: signer) acquires Balance {
        let addr = Signer::address_of(&account);
        register(&account);
        assert!(balance_of(addr) == 0, 0);
    }

    #[test(account = @0x1)]
    #[expected_failure(abort_code = 262)] // Can specify an abort code
    fun register_already_exists(account: signer) {
        register(&account);
        register(&account);
    }

    #[test]
    #[expected_failure]
    fun balance_of_dne() acquires Balance {
        balance_of(@0x1);
    }

    #[test(account = @0x1)]
    #[expected_failure]
    fun withdraw_dne(account: signer) acquires Balance {
        // Need to unpack the coin since `Coin` is a resource
        Coin { value: _ } = withdraw(&account, 0);
    }

    #[test(account = @0x1)]
    #[expected_failure]
    fun withdraw_too_much(account: signer) acquires Balance {
        register(&account);
        Coin { value: _ } = withdraw(&account, 1);
    }

    #[test(account = @CoreResources)]
    fun can_withdraw_amount(account: signer) acquires Balance, MintCapability {
        initialize(&account, 1000000);
        let amount = 1000;
        let addr = Signer::address_of(&account);
        mint_internal(&account, addr, amount);
        let Coin { value } = withdraw(&account, amount);
        assert!(value == amount, 0);
    }

    #[test(account = @CoreResources)]
    fun successful_burn(account: signer) acquires Balance, MintCapability, BurnCapability {
        initialize(&account, 1000000);
        let amount = 1000;
        let addr = Signer::address_of(&account);
        mint_internal(&account, addr, amount);
        burn(&account, withdraw(&account, 100));
        assert!(balance_of(addr) == 900, 0);
    }

    #[test(account = @CoreResources, another = @0x1)]
    #[expected_failure]
    fun failed_burn(account: signer, another: signer) acquires Balance, MintCapability, BurnCapability {
        initialize(&account, 1000000);
        let amount = 1000;
        let addr = Signer::address_of(&another);
        mint_internal(&account, addr, amount);
        burn(&another, withdraw(&another, 100));
    }

    #[test(account = @CoreResources, receiver = @0x1)]
    public(script) fun transfer_test(
        account: signer,
        receiver: signer,
    ) acquires Balance, MintCapability, TransferEvents {
        initialize(&account, 1000000);
        register(&receiver);
        let amount = 1000;
        let addr = Signer::address_of(&account);
        let addr1 = Signer::address_of(&receiver);
        mint_internal(&account, addr, amount);

        transfer(&account, addr1, 400);
        assert!(balance_of(addr) == 600, 0);
        assert!(balance_of(addr1) == 400, 0);
    }

    #[test(account = @CoreResources, account_clone = @CoreResources, delegatee = @0x1)]
    public(script) fun mint_delegation_success(account: signer, account_clone: signer, delegatee: signer) acquires Balance, Delegations, MintCapability  {
        initialize(&account, 1000000);
        register(&delegatee);
        let addr = Signer::address_of(&delegatee);
        let addr1 = @0x2;
        // make sure can delegate more than one
        delegate_mint_capability(account_clone, addr1);
        delegate_mint_capability(account, addr);
        claim_mint_capability(&delegatee);

        mint(&delegatee, addr, 1000);
        assert!(balance_of(addr) == 1000, 0);
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
    public fun mint_for_test(account: &signer, amount: u64) acquires Balance {
        register(account);
        deposit(Signer::address_of(account), Coin {value: amount});
    }
}
