/// This module provides an interface to burn or collect and redistribute transaction fees.
module aptos_framework::transaction_fee {
    use aptos_framework::coin::{Self, AggregatableCoin, BurnCapability, Coin};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::system_addresses;
    use std::error;
    use std::option::{Self, Option};

    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;

    /// When gas fees are already being collected and the struct holding
    /// information about collected amounts is already published.
    const EALREADY_COLLECTING_FEES: u64 = 1;

    /// When the burn percentage is out of range [0, 100].
    const EINVALID_BURN_PERCENTAGE: u64 = 2;

    /// Stores burn capability to burn the gas fees.
    struct AptosCoinCapabilities has key {
        burn_cap: BurnCapability<AptosCoin>,
    }

    /// Stores information about the block proposer and the amount of fees
    /// collected when executing the block.
    struct CollectedFeesPerBlock has key {
        amount: AggregatableCoin<AptosCoin>,
        proposer: Option<address>,
        burn_percentage: u8,
    }

    /// Initializes the resource storing information about gas fees collection and
    /// distribution. Should be called by on-chain governance.
    public fun initialize_fee_collection_and_distribution(aptos_framework: &signer, burn_percentage: u8) {
        // Sanity checks.
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<CollectedFeesPerBlock>(@aptos_framework),
            error::already_exists(EALREADY_COLLECTING_FEES)
        );
        assert!(burn_percentage <= 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

        // Initially, no fees are collected and the block proposer is not set.
        let zero = coin::initialize_aggregator_coin(aptos_framework);
        let info = CollectedFeesPerBlock {
            amount: zero,
            proposer: option::none(),
            burn_percentage,
        };
        move_to(aptos_framework, info);
    }

    /// Registers the proposer of the block for gas fees collection. This function
    /// can only be called by the VM.
    public fun register_proposer_for_fee_collection(vm: &signer, proposer_addr: address) acquires CollectedFeesPerBlock {
        system_addresses::assert_vm(vm);
        let collected_fees = borrow_global_mut<CollectedFeesPerBlock>(@aptos_framework);
        let _ = option::swap_or_fill(&mut collected_fees.proposer, proposer_addr);
    }

    /// Burns a specified fraction of the coin.
    fun burn_coin_fraction(coin: &mut Coin<AptosCoin>, burn_percentage: u8) acquires AptosCoinCapabilities {
        assert!(burn_percentage <= 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

        let collected_amount = coin::value(coin);
        let amount_to_burn = (burn_percentage as u64) * collected_amount / 100;
        let coin_to_burn = coin::extract(coin, amount_to_burn);
        burn_collected_fee(coin_to_burn);
    }

    /// Calculates the fee which should be distributed to the block proposer at the
    /// end of an epoch, and records it in the system. This function can only be
    /// called by the VM and should be called at the beginning of the block.
    public fun assign_or_burn_collected_fee(vm: &signer) acquires AptosCoinCapabilities, CollectedFeesPerBlock {
        system_addresses::assert_vm(vm);
        let collected_fees = borrow_global_mut<CollectedFeesPerBlock>(@aptos_framework);

        // If there are no collected fees, do nothing.
        if (coin::is_zero(&collected_fees.amount)) {
            return
        };

        // Otherwise get the collected fee, and check if it can distributed later.
        let coin = coin::drain(&mut collected_fees.amount);
        if (option::is_some(&collected_fees.proposer)) {
            let proposer_addr = *option::borrow(&collected_fees.proposer);
            if (coin::is_account_registered<AptosCoin>(proposer_addr)) {
                burn_coin_fraction(&mut coin, collected_fees.burn_percentage);
                // TODO: change with stake::add_fee()
                coin::deposit(proposer_addr, coin);
                return
            };
        };

        // If checks did not pass, simply burn the collected coins and return none.
        burn_collected_fee(coin)
    }

    /// Burn transaction fees in epilogue.
    public(friend) fun burn_fee(account: address, fee: u64) acquires AptosCoinCapabilities {
        coin::burn_from<AptosCoin>(
            account,
            fee,
            &borrow_global<AptosCoinCapabilities>(@aptos_framework).burn_cap,
        );
    }

    /// Burn a collected transaction fee.
    public(friend) fun burn_collected_fee(coin: Coin<AptosCoin>) acquires AptosCoinCapabilities {
        if (coin::value(&coin) == 0) {
            coin::destroy_zero(coin)
        } else {
            coin::burn(
                coin,
                &borrow_global<AptosCoinCapabilities>(@aptos_framework).burn_cap,
            )
        };
    }

    /// Collect transaction fees in epilogue.
    public(friend) fun collect_fee(account: address, fee: u64) acquires CollectedFeesPerBlock {
        let collected_fees = borrow_global_mut<CollectedFeesPerBlock>(@aptos_framework);

        // Here, we are always optimistic and always collect fees. If the proposer is not set,
        // or we cannot redistribute fees later for some reason (e.g. account cannot receive AptoCoin)
        // we burn them all at once. This way we avoid having a check for every transaction epilogue.
        let collected_amount = &mut collected_fees.amount;
        coin::collect_from<AptosCoin>(account, fee, collected_amount);
    }

    /// Only called during genesis.
    public(friend) fun store_aptos_coin_burn_cap(aptos_framework: &signer, burn_cap: BurnCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, AptosCoinCapabilities { burn_cap })
    }

    #[test_only]
    use aptos_framework::aggregator_factory;

    #[test(aptos_framework = @aptos_framework)]
    fun test_initialize_fee_collection_and_distribution(aptos_framework: signer) acquires CollectedFeesPerBlock {
        aggregator_factory::initialize_aggregator_factory_for_test(&aptos_framework);
        initialize_fee_collection_and_distribution(&aptos_framework, 25);

        // Check struct has been published.
        assert!(exists<CollectedFeesPerBlock>(@aptos_framework), 0);

        // Check that initial balance is 0 and there is no proposer set.
        let collected_fees = borrow_global<CollectedFeesPerBlock>(@aptos_framework);
        assert!(coin::is_zero(&collected_fees.amount), 0);
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

        burn_collected_fee(c2);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
}
