/// This module defines a minimal and generic Coin and Balance.
/// modified from https://github.com/diem/move/tree/main/language/documentation/tutorial
module AptosFramework::TestCoin {
    use Std::Errors;
    use Std::Signer;

    use CoreFramework::SystemAddresses;

    friend AptosFramework::TransactionFee;

    /// Error codes
    const EINSUFFICIENT_BALANCE: u64 = 0;
    const EALREADY_HAS_BALANCE: u64 = 1;
    const EBALANCE_NOT_PUBLISHED: u64 = 2;

    struct Coin has store {
        value: u64
    }

    /// Struct representing the balance of each address.
    struct Balance has key {
        coin: Coin
    }

    struct MintCapability has key, store { }
    struct BurnCapability has key, store { }

    public fun initialize(core_resource: &signer) {
        SystemAddresses::assert_core_resource(core_resource);
        move_to(core_resource, MintCapability {});
        move_to(core_resource, BurnCapability {});
        register(core_resource);
    }

    /// Publish an empty balance resource under `account`'s address. This function must be called before
    /// minting or transferring to the account.
    public fun register(account: &signer) {
        let empty_coin = Coin { value: 0 };
        assert!(!exists<Balance>(Signer::address_of(account)), Errors::already_published(EALREADY_HAS_BALANCE));
        move_to(account, Balance { coin:  empty_coin });
    }

    /// Mint coins with capability.
    public fun mint(account: &signer, mint_addr: address, amount: u64) acquires Balance, MintCapability {
        let _cap = borrow_global<MintCapability>(Signer::address_of(account));
        // Deposit `amount` of tokens to `mint_addr`'s balance
        deposit(mint_addr, Coin { value: amount });
    }

    /// Returns the balance of `owner`.
    public fun balance_of(owner: address): u64 acquires Balance {
        assert!(exists<Balance>(owner), Errors::not_published(EBALANCE_NOT_PUBLISHED));
        borrow_global<Balance>(owner).coin.value
    }

    /// Transfers `amount` of tokens from `from` to `to`.
    public fun transfer(from: &signer, to: address, amount: u64) acquires Balance {
        let check = withdraw(from, amount);
        deposit(to, check);
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
        let Coin { value: _value } = coins;
    }

    /// Burn transaction gas.
    public(friend) fun burn_gas(fee: Coin) acquires BurnCapability {
        let cap = borrow_global<BurnCapability>(@CoreResources);
        burn_with_capability(fee, cap);
    }

    #[test(account = @0x1)]
    #[expected_failure] // This test should abort
    fun mint_non_owner(account: signer) acquires Balance, MintCapability {
        // Make sure the address we've chosen doesn't match the module
        // owner address
        register(&account);
        assert!(Signer::address_of(&account) != @CoreResources, 0);
        mint(&account, @0x1, 10);
    }

    #[test(account = @CoreResources)]
    fun mint_check_balance(account: signer) acquires Balance, MintCapability {
        initialize(&account);
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
        initialize(&account);
        let amount = 1000;
        let addr = Signer::address_of(&account);
        mint(&account, addr, amount);
        let Coin { value } = withdraw(&account, amount);
        assert!(value == amount, 0);
    }

    #[test(account = @CoreResources)]
    fun successful_burn(account: signer) acquires Balance, MintCapability, BurnCapability {
        initialize(&account);
        let amount = 1000;
        let addr = Signer::address_of(&account);
        mint(&account, addr, amount);
        burn(&account, withdraw(&account, 100));
        assert!(balance_of(addr) == 900, 0);
    }

    #[test(account = @CoreResources, another = @0x1)]
    #[expected_failure]
    fun failed_burn(account: signer, another: signer) acquires Balance, MintCapability, BurnCapability {
        initialize(&account);
        let amount = 1000;
        let addr = Signer::address_of(&another);
        mint(&account, addr, amount);
        burn(&another, withdraw(&another, 100));
    }

    #[test(account = @CoreResources, receiver = @0x1)]
    fun transfer_test(account: signer, receiver: signer) acquires Balance, MintCapability {
        initialize(&account);
        register(&receiver);
        let amount = 1000;
        let addr = Signer::address_of(&account);
        let addr1 = Signer::address_of(&receiver);
        mint(&account, addr, amount);

        transfer(&account, addr1, 400);
        assert!(balance_of(addr) == 600, 0);
        assert!(balance_of(addr1) == 400, 0);
    }
}