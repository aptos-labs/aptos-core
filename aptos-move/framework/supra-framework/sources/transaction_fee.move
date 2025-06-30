/// This module provides an interface to burn or collect and redistribute transaction fees.
module supra_framework::transaction_fee {
    use supra_framework::coin::{Self, AggregatableCoin, BurnCapability, Coin, MintCapability};
    use supra_framework::supra_account;
    use supra_framework::supra_coin::SupraCoin;
    use supra_framework::stake;
    use supra_framework::fungible_asset::BurnRef;
    use supra_framework::system_addresses;
    use std::error;
    use std::features;
    use std::option::{Self, Option};
    use std::signer;
    use supra_framework::event;

    friend supra_framework::block;
    friend supra_framework::genesis;
    friend supra_framework::reconfiguration;
    friend supra_framework::transaction_validation;

    /// Gas fees are already being collected and the struct holding
    /// information about collected amounts is already published.
    const EALREADY_COLLECTING_FEES: u64 = 1;

    /// The burn percentage is out of range [0, 100].
    const EINVALID_BURN_PERCENTAGE: u64 = 3;

    /// No longer supported.
    const ENO_LONGER_SUPPORTED: u64 = 4;

    const EFA_GAS_CHARGING_NOT_ENABLED: u64 = 5;

    /// Stores burn capability to burn the gas fees.
    struct SupraCoinCapabilities has key {
        burn_cap: BurnCapability<SupraCoin>,
    }

    /// Stores burn capability to burn the gas fees.
    struct SupraFABurnCapabilities has key {
        burn_ref: BurnRef,
    }

    /// Stores mint capability to mint the refunds.
    struct SupraCoinMintCapability has key {
        mint_cap: MintCapability<SupraCoin>,
    }

    /// Stores information about the block proposer and the amount of fees
    /// collected when executing the block.
    struct CollectedFeesPerBlock has key {
        amount: AggregatableCoin<SupraCoin>,
        proposer: Option<address>,
        burn_percentage: u8,
    }

    #[event]
    /// Breakdown of fee charge and refund for a transaction.
    /// The structure is:
    ///
    /// - Net charge or refund (not in the statement)
    ///    - total charge: total_charge_gas_units, matches `gas_used` in the on-chain `TransactionInfo`.
    ///      This is the sum of the sub-items below. Notice that there's potential precision loss when
    ///      the conversion between internal and external gas units and between native token and gas
    ///      units, so it's possible that the numbers don't add up exactly. -- This number is the final
    ///      charge, while the break down is merely informational.
    ///        - gas charge for execution (CPU time): `execution_gas_units`
    ///        - gas charge for IO (storage random access): `io_gas_units`
    ///        - storage fee charge (storage space): `storage_fee_quants`, to be included in
    ///          `total_charge_gas_unit`, this number is converted to gas units according to the user
    ///          specified `gas_unit_price` on the transaction.
    ///    - storage deletion refund: `storage_fee_refund_quants`, this is not included in `gas_used` or
    ///      `total_charge_gas_units`, the net charge / refund is calculated by
    ///      `total_charge_gas_units` * `gas_unit_price` - `storage_fee_refund_quants`.
    ///
    /// This is meant to emitted as a module event.
    struct FeeStatement has drop, store {
        /// Total gas charge.
        total_charge_gas_units: u64,
        /// Execution gas charge.
        execution_gas_units: u64,
        /// IO gas charge.
        io_gas_units: u64,
        /// Storage fee charge.
        storage_fee_quants: u64,
        /// Storage fee refund.
        storage_fee_refund_quants: u64,
    }

    /// Initializes the resource storing information about gas fees collection and
    /// distribution. Should be called by on-chain governance.
    public fun initialize_fee_collection_and_distribution(supra_framework: &signer, burn_percentage: u8) {
        system_addresses::assert_supra_framework(supra_framework);
        assert!(
            !exists<CollectedFeesPerBlock>(@supra_framework),
            error::already_exists(EALREADY_COLLECTING_FEES)
        );
        assert!(burn_percentage <= 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

        // Make sure stakng module is aware of transaction fees collection.
        stake::initialize_validator_fees(supra_framework);

        // Initially, no fees are collected and the block proposer is not set.
        let collected_fees = CollectedFeesPerBlock {
            amount: coin::initialize_aggregatable_coin(supra_framework),
            proposer: option::none(),
            burn_percentage,
        };
        move_to(supra_framework, collected_fees);
    }

    fun is_fees_collection_enabled(): bool {
        exists<CollectedFeesPerBlock>(@supra_framework)
    }

    /// Sets the burn percentage for collected fees to a new value. Should be called by on-chain governance.
    public fun upgrade_burn_percentage(
        supra_framework: &signer,
        new_burn_percentage: u8
    ) acquires SupraCoinCapabilities, CollectedFeesPerBlock {
        system_addresses::assert_supra_framework(supra_framework);
        assert!(new_burn_percentage <= 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

        // Prior to upgrading the burn percentage, make sure to process collected
        // fees. Otherwise we would use the new (incorrect) burn_percentage when
        // processing fees later!
        process_collected_fees();

        if (is_fees_collection_enabled()) {
            // Upgrade has no effect unless fees are being collected.
            let burn_percentage = &mut borrow_global_mut<CollectedFeesPerBlock>(@supra_framework).burn_percentage;
            *burn_percentage = new_burn_percentage
        }
    }

    /// Registers the proposer of the block for gas fees collection. This function
    /// can only be called at the beginning of the block.
    public(friend) fun register_proposer_for_fee_collection(proposer_addr: address) acquires CollectedFeesPerBlock {
        if (is_fees_collection_enabled()) {
            let collected_fees = borrow_global_mut<CollectedFeesPerBlock>(@supra_framework);
            let _ = option::swap_or_fill(&mut collected_fees.proposer, proposer_addr);
        }
    }

    /// Burns a specified fraction of the coin.
    fun burn_coin_fraction(coin: &mut Coin<SupraCoin>, burn_percentage: u8) acquires SupraCoinCapabilities {
        assert!(burn_percentage <= 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

        let collected_amount = coin::value(coin);
        spec {
            // We assume that `burn_percentage * collected_amount` does not overflow.
            assume burn_percentage * collected_amount <= MAX_U64;
        };
        let amount_to_burn = (burn_percentage as u64) * collected_amount / 100;
        if (amount_to_burn != 0) {
            let coin_to_burn = coin::extract(coin, amount_to_burn);
            coin::burn(
                coin_to_burn,
                &borrow_global<SupraCoinCapabilities>(@supra_framework).burn_cap,
            );
        }
    }

    /// Calculates the fee which should be distributed to the block proposer at the
    /// end of an epoch, and records it in the system. This function can only be called
    /// at the beginning of the block or during reconfiguration.
    public(friend) fun process_collected_fees() acquires SupraCoinCapabilities, CollectedFeesPerBlock {
        if (!is_fees_collection_enabled()) {
            return
        };
        let collected_fees = borrow_global_mut<CollectedFeesPerBlock>(@supra_framework);

        // If there are no collected fees, only unset the proposer. See the rationale for
        // setting proposer to option::none() below.
        if (coin::is_aggregatable_coin_zero(&collected_fees.amount)) {
            if (option::is_some(&collected_fees.proposer)) {
                let _ = option::extract(&mut collected_fees.proposer);
            };
            return
        };

        // Otherwise get the collected fee, and check if it can distributed later.
        let coin = coin::drain_aggregatable_coin(&mut collected_fees.amount);
        if (option::is_some(&collected_fees.proposer)) {
            // Extract the address of proposer here and reset it to option::none(). This
            // is particularly useful to avoid any undesired side-effects where coins are
            // collected but never distributed or distributed to the wrong account.
            // With this design, processing collected fees enforces that all fees will be burnt
            // unless the proposer is specified in the block prologue. When we have a governance
            // proposal that triggers reconfiguration, we distribute pending fees and burn the
            // fee for the proposal. Otherwise, that fee would be leaked to the next block.
            let proposer = option::extract(&mut collected_fees.proposer);

            // Since the block can be produced by the VM itself, we have to make sure we catch
            // this case.
            if (proposer == @vm_reserved) {
                burn_coin_fraction(&mut coin, 100);
                coin::destroy_zero(coin);
                return
            };

            burn_coin_fraction(&mut coin, collected_fees.burn_percentage);
            stake::add_transaction_fee(proposer, coin);
            return
        };

        // If checks did not pass, simply burn all collected coins and return none.
        burn_coin_fraction(&mut coin, 100);
        coin::destroy_zero(coin)
    }

    /// Burn transaction fees in epilogue.
    public(friend) fun burn_fee(account: address, fee: u64) acquires SupraFABurnCapabilities, SupraCoinCapabilities {
        if (exists<SupraFABurnCapabilities>(@supra_framework)) {
            let burn_ref = &borrow_global<SupraFABurnCapabilities>(@supra_framework).burn_ref;
            supra_account::burn_from_fungible_store(burn_ref, account, fee);
        } else {
            let burn_cap = &borrow_global<SupraCoinCapabilities>(@supra_framework).burn_cap;
            if (features::operations_default_to_fa_supra_store_enabled()) {
                let (burn_ref, burn_receipt) = coin::get_paired_burn_ref(burn_cap);
                supra_account::burn_from_fungible_store(&burn_ref, account, fee);
                coin::return_paired_burn_ref(burn_ref, burn_receipt);
            } else {
                coin::burn_from<SupraCoin>(
                    account,
                    fee,
                    burn_cap,
                );
            };
        };
    }

    /// Mint refund in epilogue.
    public(friend) fun mint_and_refund(account: address, refund: u64) acquires SupraCoinMintCapability {
        let mint_cap = &borrow_global<SupraCoinMintCapability>(@supra_framework).mint_cap;
        let refund_coin = coin::mint(refund, mint_cap);
        coin::force_deposit(account, refund_coin);
    }

    /// Collect transaction fees in epilogue.
    public(friend) fun collect_fee(account: address, fee: u64) acquires CollectedFeesPerBlock {
        let collected_fees = borrow_global_mut<CollectedFeesPerBlock>(@supra_framework);

        // Here, we are always optimistic and always collect fees. If the proposer is not set,
        // or we cannot redistribute fees later for some reason (e.g. account cannot receive AptoCoin)
        // we burn them all at once. This way we avoid having a check for every transaction epilogue.
        let collected_amount = &mut collected_fees.amount;
        coin::collect_into_aggregatable_coin<SupraCoin>(account, fee, collected_amount);
    }

    /// Only called during genesis.
    public(friend) fun store_supra_coin_burn_cap(supra_framework: &signer, burn_cap: BurnCapability<SupraCoin>) {
        system_addresses::assert_supra_framework(supra_framework);

        if (features::operations_default_to_fa_supra_store_enabled()) {
            let burn_ref = coin::convert_and_take_paired_burn_ref(burn_cap);
            move_to(supra_framework, SupraFABurnCapabilities { burn_ref });
        } else {
            move_to(supra_framework, SupraCoinCapabilities { burn_cap })
        }
    }

    public entry fun convert_to_aptos_fa_burn_ref(supra_framework: &signer) acquires SupraCoinCapabilities {
        assert!(features::operations_default_to_fa_supra_store_enabled(), EFA_GAS_CHARGING_NOT_ENABLED);
        system_addresses::assert_supra_framework(supra_framework);
        let SupraCoinCapabilities {
            burn_cap,
        } = move_from<SupraCoinCapabilities>(signer::address_of(supra_framework));
        let burn_ref = coin::convert_and_take_paired_burn_ref(burn_cap);
        move_to(supra_framework, SupraFABurnCapabilities { burn_ref });
    }

    /// Only called during genesis.
    public(friend) fun store_supra_coin_mint_cap(supra_framework: &signer, mint_cap: MintCapability<SupraCoin>) {
        system_addresses::assert_supra_framework(supra_framework);
        move_to(supra_framework, SupraCoinMintCapability { mint_cap })
    }

    #[deprecated]
    public fun initialize_storage_refund(_: &signer) {
        abort error::not_implemented(ENO_LONGER_SUPPORTED)
    }

    // Called by the VM after epilogue.
    fun emit_fee_statement(fee_statement: FeeStatement) {
        event::emit(fee_statement)
    }

    #[test_only]
    use supra_framework::aggregator_factory;
    #[test_only]
    use supra_framework::object;

    #[test(supra_framework = @supra_framework)]
    fun test_initialize_fee_collection_and_distribution(supra_framework: signer) acquires CollectedFeesPerBlock {
        aggregator_factory::initialize_aggregator_factory_for_test(&supra_framework);
        initialize_fee_collection_and_distribution(&supra_framework, 25);

        // Check struct has been published.
        assert!(exists<CollectedFeesPerBlock>(@supra_framework), 0);

        // Check that initial balance is 0 and there is no proposer set.
        let collected_fees = borrow_global<CollectedFeesPerBlock>(@supra_framework);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(option::is_none(&collected_fees.proposer), 0);
        assert!(collected_fees.burn_percentage == 25, 0);
    }

    #[test(supra_framework = @supra_framework)]
    fun test_burn_fraction_calculation(supra_framework: signer) acquires SupraCoinCapabilities {
        use supra_framework::supra_coin;
        let (burn_cap, mint_cap) = supra_coin::initialize_for_test(&supra_framework);
        store_supra_coin_burn_cap(&supra_framework, burn_cap);

        let c1 = coin::mint<SupraCoin>(100, &mint_cap);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 100, 0);

        // Burning 25%.
        burn_coin_fraction(&mut c1, 25);
        assert!(coin::value(&c1) == 75, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 75, 0);

        // Burning 0%.
        burn_coin_fraction(&mut c1, 0);
        assert!(coin::value(&c1) == 75, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 75, 0);

        // Burning remaining 100%.
        burn_coin_fraction(&mut c1, 100);
        assert!(coin::value(&c1) == 0, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 0, 0);

        coin::destroy_zero(c1);
        let c2 = coin::mint<SupraCoin>(10, &mint_cap);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 10, 0);

        burn_coin_fraction(&mut c2, 5);
        assert!(coin::value(&c2) == 10, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 10, 0);

        burn_coin_fraction(&mut c2, 100);
        coin::destroy_zero(c2);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(supra_framework = @supra_framework, alice = @0xa11ce, bob = @0xb0b, carol = @0xca101)]
    fun test_fees_distribution(
        supra_framework: signer,
        alice: signer,
        bob: signer,
        carol: signer,
    ) acquires SupraCoinCapabilities, CollectedFeesPerBlock {
        use std::signer;
        use supra_framework::supra_account;
        use supra_framework::supra_coin;

        // Initialization.
        let (burn_cap, mint_cap) = supra_coin::initialize_for_test(&supra_framework);
        store_supra_coin_burn_cap(&supra_framework, burn_cap);
        initialize_fee_collection_and_distribution(&supra_framework, 10);

        // Create dummy accounts.
        let alice_addr = signer::address_of(&alice);
        let bob_addr = signer::address_of(&bob);
        let carol_addr = signer::address_of(&carol);
        supra_account::create_account(alice_addr);
        supra_account::create_account(bob_addr);
        supra_account::create_account(carol_addr);
        assert!(object::object_address(&coin::ensure_paired_metadata<SupraCoin>()) == @supra_fungible_asset, 0);
        coin::deposit(alice_addr, coin::mint(10000, &mint_cap));
        coin::deposit(bob_addr, coin::mint(10000, &mint_cap));
        coin::deposit(carol_addr, coin::mint(10000, &mint_cap));
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 30000, 0);

        // Block 1 starts.
        process_collected_fees();
        register_proposer_for_fee_collection(alice_addr);

        // Check that there was no fees distribution in the first block.
        let collected_fees = borrow_global<CollectedFeesPerBlock>(@supra_framework);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(*option::borrow(&collected_fees.proposer) == alice_addr, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 30000, 0);

        // Simulate transaction fee collection - here we simply collect some fees from Bob.
        collect_fee(bob_addr, 100);
        collect_fee(bob_addr, 500);
        collect_fee(bob_addr, 400);

        // Now Bob must have 1000 less in his account. Alice and Carol have the same amounts.
        assert!(coin::balance<SupraCoin>(alice_addr) == 10000, 0);
        assert!(coin::balance<SupraCoin>(bob_addr) == 9000, 0);
        assert!(coin::balance<SupraCoin>(carol_addr) == 10000, 0);

        // Block 2 starts.
        process_collected_fees();
        register_proposer_for_fee_collection(bob_addr);

        // Collected fees from Bob must have been assigned to Alice.
        assert!(stake::get_validator_fee(alice_addr) == 900, 0);
        assert!(coin::balance<SupraCoin>(alice_addr) == 10000, 0);
        assert!(coin::balance<SupraCoin>(bob_addr) == 9000, 0);
        assert!(coin::balance<SupraCoin>(carol_addr) == 10000, 0);

        // Also, aggregator coin is drained and total supply is slightly changed (10% of 1000 is burnt).
        let collected_fees = borrow_global<CollectedFeesPerBlock>(@supra_framework);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(*option::borrow(&collected_fees.proposer) == bob_addr, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 29900, 0);

        // Simulate transaction fee collection one more time.
        collect_fee(bob_addr, 5000);
        collect_fee(bob_addr, 4000);

        assert!(coin::balance<SupraCoin>(alice_addr) == 10000, 0);
        assert!(coin::balance<SupraCoin>(bob_addr) == 0, 0);
        assert!(coin::balance<SupraCoin>(carol_addr) == 10000, 0);

        // Block 3 starts.
        process_collected_fees();
        register_proposer_for_fee_collection(carol_addr);

        // Collected fees should have been assigned to Bob because he was the peoposer.
        assert!(stake::get_validator_fee(alice_addr) == 900, 0);
        assert!(coin::balance<SupraCoin>(alice_addr) == 10000, 0);
        assert!(stake::get_validator_fee(bob_addr) == 8100, 0);
        assert!(coin::balance<SupraCoin>(bob_addr) == 0, 0);
        assert!(coin::balance<SupraCoin>(carol_addr) == 10000, 0);

        // Again, aggregator coin is drained and total supply is changed by 10% of 9000.
        let collected_fees = borrow_global<CollectedFeesPerBlock>(@supra_framework);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(*option::borrow(&collected_fees.proposer) == carol_addr, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 29000, 0);

        // Simulate transaction fee collection one last time.
        collect_fee(alice_addr, 1000);
        collect_fee(alice_addr, 1000);

        // Block 4 starts.
        process_collected_fees();
        register_proposer_for_fee_collection(alice_addr);

        // Check that 2000 was collected from Alice.
        assert!(coin::balance<SupraCoin>(alice_addr) == 8000, 0);
        assert!(coin::balance<SupraCoin>(bob_addr) == 0, 0);

        // Carol must have some fees assigned now.
        let collected_fees = borrow_global<CollectedFeesPerBlock>(@supra_framework);
        assert!(stake::get_validator_fee(carol_addr) == 1800, 0);
        assert!(coin::is_aggregatable_coin_zero(&collected_fees.amount), 0);
        assert!(*option::borrow(&collected_fees.proposer) == alice_addr, 0);
        assert!(*option::borrow(&coin::supply<SupraCoin>()) == 28800, 0);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
}
