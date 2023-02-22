/// This module provides an interface to burn or collect and redistribute transaction fees.
module aptos_framework::transaction_fee {
    use aptos_framework::coin::{Self, AggregatableCoin, BurnCapability, Coin};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::stake;
    use aptos_framework::system_addresses;
    use std::error;
    use std::option::{Self, Option};
    use std::vector;

    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;
    friend aptos_framework::transaction_validation;

    /// Gas fees are already being collected and the struct holding
    /// information about collected amounts is already published.
    const EALREADY_COLLECTING_FEES: u64 = 1;

    /// The burn percentage is out of range [0, 100].
    const EINVALID_BURN_PERCENTAGE: u64 = 3;

    /// Stores burn capability to burn the gas fees.
    struct AptosCoinCapabilities has key {
        burn_cap: BurnCapability<AptosCoin>,
    }

    /// Stores information about the block proposer and the amount of fees
    /// collected when executing the block.
    struct CollectedFeesPerBlockAndBatches has key {
        amount: vector<AggregatableCoin<AptosCoin>>,
        block_proposer: Option<address>,
        batch_proposers: vector<address>,
        block_distribution_percentage: u8,
        batch_distribution_percentage: u8,
    }

    /// Initializes the resource storing information about gas fees collection and
    /// distribution. Should be called by on-chain governance.
    public fun initialize_fee_collection_and_distributions(aptos_framework: &signer, block_distribution_percentage: u8, batch_distribution_percentage: u8) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<CollectedFeesPerBlockAndBatches>(@aptos_framework),
            error::already_exists(EALREADY_COLLECTING_FEES)
        );
        assert!(block_distribution_percentage + batch_distribution_percentage <= 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

        // Make sure stakng module is aware of transaction fees collection.
        stake::initialize_validator_fees(aptos_framework);

        // Initially, no fees are collected and the block proposer is not set.
        let collected_fees = CollectedFeesPerBlockAndBatches {
            amount: vector::empty(),
            block_proposer: option::none(),
            batch_proposers: vector::empty(),
            block_distribution_percentage,
            batch_distribution_percentage,
        };
        move_to(aptos_framework, collected_fees);
    }

    fun is_fees_collection_enabled(): bool {
        exists<CollectedFeesPerBlockAndBatches>(@aptos_framework)
    }

    /// Sets the burn percentage for collected fees to a new value. Should be called by on-chain governance.
    public fun upgrade_distribution_percentages(
        aptos_framework: &signer,
        new_block_distribution_percentage: u8,
        new_batch_distribution_percentage: u8
    ) acquires CollectedFeesPerBlockAndBatches { // AptosCoinCapabilities
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(new_block_distribution_percentage + new_batch_distribution_percentage <= 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

        // Prior to upgrading the burn percentage, make sure to process collected
        // fees. Otherwise we would use the new (incorrect) burn_percentage when
        // processing fees later!
        process_collected_fees();

        if (is_fees_collection_enabled()) {
            // Upgrade has no effect unless fees are being collected.
            let config = borrow_global_mut<CollectedFeesPerBlockAndBatches>(@aptos_framework);
            config.block_distribution_percentage = new_block_distribution_percentage;
            config.batch_distribution_percentage = new_batch_distribution_percentage;
        }
    }

    /// Registers the proposer of the block for gas fees collection. This function
    /// can only be called at the beginning of the block.
    public(friend) fun register_proposers_for_fee_collection(block_proposer_addr: address, batch_proposers_addr: vector<address>) acquires CollectedFeesPerBlockAndBatches {
        if (is_fees_collection_enabled()) {
            let collected_fees = borrow_global_mut<CollectedFeesPerBlockAndBatches>(@aptos_framework);
            // TODO implement
            //  coin::initialize_aggregatable_coin(aptos_framework)
            // let _ = option::swap_or_fill(&mut collected_fees.proposer, proposer_addr);
        }
    }

    /// Burns a specified fraction of the coin.
    fun burn_coin_fraction(coin: &mut Coin<AptosCoin>, burn_percentage: u8) acquires AptosCoinCapabilities {
        assert!(burn_percentage <= 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

        let collected_amount = coin::value(coin);
        spec {
            // We assume that `burn_percentage * collected_amount` does not overflow.
            assume burn_percentage * collected_amount <= MAX_U64;
        };
        let amount_to_burn = (burn_percentage as u64) * collected_amount / 100;
        if (amount_to_burn > 0) {
            let coin_to_burn = coin::extract(coin, amount_to_burn);
            coin::burn(
                coin_to_burn,
                &borrow_global<AptosCoinCapabilities>(@aptos_framework).burn_cap,
            );
        }
    }

    /// Calculates the fee which should be distributed to the block proposer at the
    /// end of an epoch, and records it in the system. This function can only be called
    /// at the beginning of the block or during reconfiguration.
    public(friend) fun process_collected_fees() acquires CollectedFeesPerBlockAndBatches { // AptosCoinCapabilities
        if (!is_fees_collection_enabled()) {
            return
        };
        let collected_fees = borrow_global_mut<CollectedFeesPerBlockAndBatches>(@aptos_framework);

        // // If there are no collected fees, only unset the proposer. See the rationale for
        // // setting proposer to option::none() below.
        // if (coin::is_aggregatable_coin_zero(&collected_fees.amount)) {
        //     if (option::is_some(&collected_fees.proposer)) {
        //         let _ = option::extract(&mut collected_fees.proposer);
        //     };
        //     return
        // };

        // // Otherwise get the collected fee, and check if it can distributed later.
        // let coin = coin::drain_aggregatable_coin(&mut collected_fees.amount);
        // if (option::is_some(&collected_fees.proposer)) {
        //     // Extract the address of proposer here and reset it to option::none(). This
        //     // is particularly useful to avoid any undesired side-effects where coins are
        //     // collected but never distributed or distributed to the wrong account.
        //     // With this design, processing collected fees enforces that all fees will be burnt
        //     // unless the proposer is specified in the block prologue. When we have a governance
        //     // proposal that triggers reconfiguration, we distribute pending fees and burn the
        //     // fee for the proposal. Otherwise, that fee would be leaked to the next block.
        //     let proposer = option::extract(&mut collected_fees.proposer);

        //     // Since the block can be produced by the VM itself, we have to make sure we catch
        //     // this case.
        //     if (proposer == @vm_reserved) {
        //         burn_coin_fraction(&mut coin, 100);
        //         coin::destroy_zero(coin);
        //         return
        //     };

        //     burn_coin_fraction(&mut coin, collected_fees.burn_percentage);
        //     stake::add_transaction_fee(proposer, coin);
        //     return
        // };

        // // If checks did not pass, simply burn all collected coins and return none.
        // burn_coin_fraction(&mut coin, 100);
        // coin::destroy_zero(coin)
    }

    /// Burn transaction fees in epilogue.
    public(friend) fun burn_fee(account: address, fee: u64) acquires AptosCoinCapabilities {
        coin::burn_from<AptosCoin>(
            account,
            fee,
            &borrow_global<AptosCoinCapabilities>(@aptos_framework).burn_cap,
        );
    }

    /// Collect transaction fees in epilogue.
    public(friend) fun collect_fee_for_batch(account: address, fee: u64, batch_index: u16) acquires CollectedFeesPerBlockAndBatches {
        let collected_fees = borrow_global_mut<CollectedFeesPerBlockAndBatches>(@aptos_framework);

        // Here, we are always optimistic and always collect fees. If the proposer is not set,
        // or we cannot redistribute fees later for some reason (e.g. account cannot receive AptoCoin)
        // we burn them all at once. This way we avoid having a check for every transaction epilogue.
        let collected_amount = &mut collected_fees.amount;
        // coin::collect_into_aggregatable_coin<AptosCoin>(account, fee, collected_amount);
    }

    /// Only called during genesis.
    public(friend) fun store_aptos_coin_burn_cap(aptos_framework: &signer, burn_cap: BurnCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, AptosCoinCapabilities { burn_cap })
    }

    #[test_only]
    use aptos_framework::aggregator_factory;

    #[test(aptos_framework = @aptos_framework)]
    fun test_initialize_fee_collection_and_distribution(aptos_framework: signer) acquires CollectedFeesPerBlockAndBatches {
        aggregator_factory::initialize_aggregator_factory_for_test(&aptos_framework);
        initialize_fee_collection_and_distributions(&aptos_framework, 10, 65);

        // Check struct has been published.
        assert!(exists<CollectedFeesPerBlockAndBatches>(@aptos_framework), 0);

        // Check that initial balance is 0 and there is no proposer set.
        let collected_fees = borrow_global<CollectedFeesPerBlockAndBatches>(@aptos_framework);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(option::is_none(&collected_fees.proposer), 0);
        assert!(collected_fees.burn_percentage == 25, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_burn_fraction_calculation(aptos_framework: signer) acquires AptosCoinCapabilities {
        use aptos_framework::aptos_coin;
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(&aptos_framework);
        store_aptos_coin_burn_cap(&aptos_framework, burn_cap);

        let c1 = coin::mint<AptosCoin>(100, &mint_cap);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 100, 0);

        // Burning 25%.
        burn_coin_fraction(&mut c1, 25);
        assert!(coin::value(&c1) == 75, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 75, 0);

        // Burning 0%.
        burn_coin_fraction(&mut c1, 0);
        assert!(coin::value(&c1) == 75, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 75, 0);

        // Burning remaining 100%.
        burn_coin_fraction(&mut c1, 100);
        assert!(coin::value(&c1) == 0, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 0, 0);

        coin::destroy_zero(c1);
        let c2 = coin::mint<AptosCoin>(10, &mint_cap);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 10, 0);

        burn_coin_fraction(&mut c2, 5);
        assert!(coin::value(&c2) == 10, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 10, 0);

        burn_coin_fraction(&mut c2, 100);
        coin::destroy_zero(c2);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @aptos_framework, alice = @0xa11ce, bob = @0xb0b, carol = @0xca101)]
    fun test_fees_distribution(
        aptos_framework: signer,
        alice: signer,
        bob: signer,
        carol: signer,
    ) acquires AptosCoinCapabilities, CollectedFeesPerBlockAndBatches {
        use std::signer;
        use aptos_framework::aptos_account;
        use aptos_framework::aptos_coin;

        // Initialization.
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(&aptos_framework);
        store_aptos_coin_burn_cap(&aptos_framework, burn_cap);
        initialize_fee_collection_and_distributions(&aptos_framework, 10, 80);

        // Create dummy accounts.
        let alice_addr = signer::address_of(&alice);
        let bob_addr = signer::address_of(&bob);
        let carol_addr = signer::address_of(&carol);
        aptos_account::create_account(alice_addr);
        aptos_account::create_account(bob_addr);
        aptos_account::create_account(carol_addr);
        coin::deposit(alice_addr, coin::mint(10000, &mint_cap));
        coin::deposit(bob_addr, coin::mint(10000, &mint_cap));
        coin::deposit(carol_addr, coin::mint(10000, &mint_cap));
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 30000, 0);

        // Block 1 starts.
        process_collected_fees();
        register_proposer_for_fee_collection(alice_addr);

        // Check that there was no fees distribution in the first block.
        let collected_fees = borrow_global<CollectedFeesPerBlockAndBatches>(@aptos_framework);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(*option::borrow(&collected_fees.proposer) == alice_addr, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 30000, 0);

        // Simulate transaction fee collection - here we simply collect some fees from Bob.
        collect_fee(bob_addr, 100);
        collect_fee(bob_addr, 500);
        collect_fee(bob_addr, 400);

        // Now Bob must have 1000 less in his account. Alice and Carol have the same amounts.
        assert!(coin::balance<AptosCoin>(alice_addr) == 10000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 9000, 0);
        assert!(coin::balance<AptosCoin>(carol_addr) == 10000, 0);

        // Block 2 starts.
        process_collected_fees();
        register_proposer_for_fee_collection(bob_addr);

        // Collected fees from Bob must have been assigned to Alice.
        assert!(stake::get_validator_fee(alice_addr) == 900, 0);
        assert!(coin::balance<AptosCoin>(alice_addr) == 10000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 9000, 0);
        assert!(coin::balance<AptosCoin>(carol_addr) == 10000, 0);

        // Also, aggregator coin is drained and total supply is slightly changed (10% of 1000 is burnt).
        let collected_fees = borrow_global<CollectedFeesPerBlockAndBatches>(@aptos_framework);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(*option::borrow(&collected_fees.proposer) == bob_addr, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 29900, 0);

        // Simulate transaction fee collection one more time.
        collect_fee(bob_addr, 5000);
        collect_fee(bob_addr, 4000);

        assert!(coin::balance<AptosCoin>(alice_addr) == 10000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 0, 0);
        assert!(coin::balance<AptosCoin>(carol_addr) == 10000, 0);

        // Block 3 starts.
        process_collected_fees();
        register_proposer_for_fee_collection(carol_addr);

        // Collected fees should have been assigned to Bob because he was the peoposer.
        assert!(stake::get_validator_fee(alice_addr) == 900, 0);
        assert!(coin::balance<AptosCoin>(alice_addr) == 10000, 0);
        assert!(stake::get_validator_fee(bob_addr) == 8100, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 0, 0);
        assert!(coin::balance<AptosCoin>(carol_addr) == 10000, 0);

        // Again, aggregator coin is drained and total supply is changed by 10% of 9000.
        let collected_fees = borrow_global<CollectedFeesPerBlockAndBatches>(@aptos_framework);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(*option::borrow(&collected_fees.proposer) == carol_addr, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 29000, 0);

        // Simulate transaction fee collection one last time.
        collect_fee(alice_addr, 1000);
        collect_fee(alice_addr, 1000);

        // Block 4 starts.
        process_collected_fees();
        register_proposer_for_fee_collection(alice_addr);

        // Check that 2000 was collected from Alice.
        assert!(coin::balance<AptosCoin>(alice_addr) == 8000, 0);
        assert!(coin::balance<AptosCoin>(bob_addr) == 0, 0);

        // Carol must have some fees assigned now.
        let collected_fees = borrow_global<CollectedFeesPerBlockAndBatches>(@aptos_framework);
        assert!(stake::get_validator_fee(carol_addr) == 1800, 0);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(*option::borrow(&collected_fees.proposer) == alice_addr, 0);
        assert!(*option::borrow(&coin::supply<AptosCoin>()) == 28800, 0);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }


    // OLD: keeping for backward compatibility.

    /// Stores information about the block proposer and the amount of fees
    /// collected when executing the block.
    struct CollectedFeesPerBlock has key {
        amount: AggregatableCoin<AptosCoin>,
        proposer: Option<address>,
        burn_percentage: u8,
    }

    public fun initialize_fee_collection_and_distribution(aptos_framework: &signer, burn_percentage: u8) {
    }

    public fun upgrade_burn_percentage(
        aptos_framework: &signer,
        new_burn_percentage: u8
    ) {
    }

    public(friend) fun register_proposer_for_fee_collection(proposer_addr: address) {
    }

    public(friend) fun collect_fee(account: address, fee: u64) {
    }
}
