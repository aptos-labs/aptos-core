module aptos_framework::fee_destribution {
    use std::error;
    use std::option::{Self, Option};

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, AggregatableCoin};
    use aptos_framework::system_addresses;
    use aptos_framework::transaction_fee;

    friend aptos_framework::transaction_validation;

    /// When struct holding distribution ifnormation already exists.
    const EDISTRIBUTION_INFO_EXISTS: u64 = 1;

    /// Resource which holds the collected transaction fees and their receiver.
    struct DistributionInfo has key {
        balance: AggregatableCoin<AptosCoin>,
        receiver: Option<address>,
    }

    /// Initializes the resource holding information for gas fees distribution.
    /// Should be called by on-chain governance.
    public fun initialize_distribution_info(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<DistributionInfo>(@aptos_framework),
            error::already_exists(EDISTRIBUTION_INFO_EXISTS)
        );

        let zero = coin::initialize_aggregator_coin(aptos_framework);
        let info = DistributionInfo {
            balance: zero,
            receiver: option::none(),
        };
        move_to(aptos_framework, info);
    }

    /// Called by transaction epilogue to collect the gas fees from the specified account.
    public(friend) fun collect_fee(account: address, fee: u64) acquires DistributionInfo {
        let distribution_info = borrow_global_mut<DistributionInfo>(@aptos_framework);
        let dst_coin = &mut distribution_info.balance;
        coin::collect_from(account, fee, dst_coin);
    }

    /// Sets the receiver of the collected fees for the next block.
    public fun set_receiver(vm: &signer, receiver_addr: address) acquires DistributionInfo {
        // Can only be called by the VM.
        system_addresses::assert_vm(vm);
        let distribution_info = borrow_global_mut<DistributionInfo>(@aptos_framework);
        let _ = option::swap_or_fill(&mut distribution_info.receiver, receiver_addr);
    }

    /// Distributes collected transaction fees to the receiver. Should be called
    /// at the beginning of each block.
    public fun maybe_distribute_fees(vm: &signer) acquires DistributionInfo {
        // Can only be called by the VM.
        system_addresses::assert_vm(vm);
        let distribution_info = borrow_global_mut<DistributionInfo>(@aptos_framework);

        // First, do nothing if there are no collected fees.
        if (coin::is_zero(&distribution_info.balance)) {
            return
        };

        let coin = coin::drain(&mut distribution_info.balance);
        if (option::is_some(&distribution_info.receiver)) {
            let receiver_addr = *option::borrow(&distribution_info.receiver);

            // There is a receiver, but it might not have account registered for storing
            // coins, so check for that.
            let receiver_has_account = coin::is_account_registered<AptosCoin>(receiver_addr);
            if (receiver_has_account) {
                // If all checks passed, deposit coins to the receiver's account.
                coin::deposit(receiver_addr, coin);
                return
            };
        };

        // Otherwise, burn the collected coins.
        transaction_fee::burn_collected_fee(coin);
    }

    #[test_only]
    use aptos_framework::aggregator_factory;

    #[test(aptos_framework = @aptos_framework)]
    fun test_initialize_distribution_info(aptos_framework: signer) acquires DistributionInfo {
        aggregator_factory::initialize_aggregator_factory_for_test(&aptos_framework);
        initialize_distribution_info(&aptos_framework);

        // Check struct has been published.
        assert!(exists<DistributionInfo>(@aptos_framework), 0);

        // Check that initial balance is 0 and there is no proposer set.
        let distribution_info = borrow_global<DistributionInfo>(@aptos_framework);
        assert!(coin::is_zero(&distribution_info.balance), 0);
        assert!(option::is_none(&distribution_info.receiver), 0);
    }

    #[test(aptos_framework = @aptos_framework, vm = @vm_reserved, alice = @0xa11ce, bob = @0xb0b, carol = @0xca101)]
    fun test_fees_distribution(
        aptos_framework: signer,
        vm: signer,
        alice: signer,
        bob: signer,
        carol: signer,
    ) acquires DistributionInfo {
        use std::signer;
        use aptos_framework::aptos_account;
        use aptos_framework::aptos_coin;

        // Initialization.
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(&aptos_framework);
        transaction_fee::store_aptos_coin_burn_cap(&aptos_framework, burn_cap);
        initialize_distribution_info(&aptos_framework);

        // Create dummy accounts.
        let alice_addr = signer::address_of(&alice);
        let bob_addr = signer::address_of(&bob);
        let carol_addr = signer::address_of(&carol);
        aptos_account::create_account(alice_addr);
        aptos_account::create_account(bob_addr);
        coin::deposit(alice_addr, coin::mint(10000, &mint_cap));
        coin::deposit(bob_addr, coin::mint(10000, &mint_cap));
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 20000, 0);

        // Block 1 starts.
        maybe_distribute_fees(&vm);
        set_receiver(&vm, alice_addr);

        // Check that there was no fees distribution.
        let distribution_info = borrow_global<DistributionInfo>(@aptos_framework);
        assert!(coin::is_zero(&distribution_info.balance), 0);
        assert!(*option::borrow(&distribution_info.receiver) == alice_addr, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 20000, 0);

        // Simulate transaction fee collection - here we simply collect some fees from Bob.
        collect_fee(bob_addr, 100);
        collect_fee(bob_addr, 500);
        collect_fee(bob_addr, 400);

        // Now Bob must have 1000 less in his account.
        assert!(coin::balance<AptosCoin>(alice_addr) == 10000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 9000, 0);

        // Block 2 starts.
        maybe_distribute_fees(&vm);
        set_receiver(&vm, bob_addr);

        // Collected fees from Bob must have been sent to Alice.
        assert!(coin::balance<AptosCoin>(alice_addr) == 11000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 9000, 0);

        // Also, aggregator coin is drained and total supply is unchanged (nothing is burnt).
        let distribution_info = borrow_global<DistributionInfo>(@aptos_framework);
        assert!(coin::is_zero(&distribution_info.balance), 0);
        assert!(*option::borrow(&distribution_info.receiver) == bob_addr, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 20000, 0);

        // Simulate transaction fee collection one more time.
        collect_fee(bob_addr, 5000);
        collect_fee(bob_addr, 4000);

        assert!(coin::balance<AptosCoin>(alice_addr) == 11000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 0, 0);

        // Block 3 starts.
        maybe_distribute_fees(&vm);
        set_receiver(&vm, carol_addr);

        // Collected fees should have been returned back to Bob because he was set as
        // the receiver.
        assert!(coin::balance<AptosCoin>(alice_addr) == 11000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 9000, 0);

        // Again, aggregator coin is drained and total supply unchanged.
        let distribution_info = borrow_global<DistributionInfo>(@aptos_framework);
        assert!(coin::is_zero(&distribution_info.balance), 0);
        assert!(*option::borrow(&distribution_info.receiver) == carol_addr, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 20000, 0);

        // Simulate transaction fee collection one last time.
        collect_fee(bob_addr, 1000);
        collect_fee(bob_addr, 1000);

        // Block 4 starts.
        maybe_distribute_fees(&vm);
        set_receiver(&vm, alice_addr);

        // Check that 2000 was collected from Bob.
        assert!(coin::balance<AptosCoin>(alice_addr) == 11000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 7000, 0);

        // Since carol has no account registered, fees should be burnt and total supply
        // should reflect that.
        let distribution_info = borrow_global<DistributionInfo>(@aptos_framework);
        assert!(coin::is_zero(&distribution_info.balance), 0);
        assert!(*option::borrow(&distribution_info.receiver) == alice_addr, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 18000, 0);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
}
