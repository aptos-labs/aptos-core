/// Module that offers convenient simple delegations where a single owner account can delegate
/// to multiple operators. This does not offer general purpose delegations where funds from multiple
/// delegators can be pooled and staked with a single operator.
module aptos_framework::simple_delegations {
    use std::bcs;
    use std::error;
    use std::signer;
    use std::vector;

    use aptos_std::event::{EventHandle, emit_event};
    use aptos_std::simple_map::{Self, SimpleMap};

    use aptos_framework::account::{Self, SignerCapability};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::stake::{Self, OwnerCapability};
    use aptos_framework::staking_config;

    friend aptos_framework::genesis;

    const STAKE_POOL_ACCOUNT_SALT: vector<u8> = b"aptos_framework::simple_delegations";

    /// Delegations amount must be at least the min stake required for a stake pool to join the validator set.
    const EINSUFFICIENT_DELEGATION_AMOUNT: u64 = 1;
    /// Commission percentage has to be between 0 and 100.
    const EINVALID_COMMISSION_PERCENTAGE: u64 = 2;
    /// No delegations found to any operator.
    const ENO_DELEGATIONS_FOUND: u64 = 3;
    /// No delegation from the delegator to the specified operator found.
    const EDELEGATION_NOT_FOUND: u64 = 4;
    /// Can't merge two stake pools from 2 existing operators.
    const ECANT_MERGE_TWO_EXISTING_STAKE_POOLS: u64 = 5;
    /// Cannot claim any commissions as commission rate is zero.
    const EZERO_COMMISSION_RATE: u64 = 6;
    /// Cannot change an existing delegation.
    const ECANNOT_CHANGE_EXISTING_DELEGATION: u64 = 7;
    /// Cannot request commission from an inactive delegation. Any unpaid commission will be settled when the delegator
    /// withdraws funds.
    const EINACTIVE_DELEGATION: u64 = 8;
    /// Cannot end a delegation that has already been ended.
    const EDELEGATION_ALREADY_ENDED: u64 = 9;
    /// Cannot end a delegation while the original delegated stake is still pending_inactive.
    /// This can result in lost funds. Delegator needs to wait until the next epoch before they can end the delegation.
    const EPENDING_ACTIVE_FOUND: u64 = 10;
    /// Cannot withdraw stake from a still active delegation.
    const EDELEGATION_STILL_ACTIVE: u64 = 11;
    /// Cannot withdraw stake as it's still being unlocked in the stake pool.
    const EFUNDS_STILL_BEING_UNLOCKED: u64 = 12;

    /// Validator status enum copied from aptos_framework::stake.
    const VALIDATOR_STATUS_PENDING_ACTIVE: u64 = 1;
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    const VALIDATOR_STATUS_PENDING_INACTIVE: u64 = 3;
    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    struct CommissionDebt has copy, drop, store {
        creditor: address,
        amount: u64,
    }

    struct Delegation has store {
        total_stake: u64,
        pool_address: address,
        owner_cap: OwnerCapability,
        commission_percentage: u64,
        is_active: bool,
        // Potential debts to previous operators if operator is switched before the previous operator
        // has withdrawn their commission.
        debts: vector<CommissionDebt>,
        // Just in case we need the SignerCap for stake pool account in the future.
        signer_cap: SignerCapability,
    }

    struct Delegations has key {
        delegations: SimpleMap<address, Delegation>,

        // Events.
        delegate_events: EventHandle<DelegateEvent>,
        update_voter_events: EventHandle<UpdateVoterEvent>,
        request_commission_events: EventHandle<RequestCommissionEvent>,
        withdraw_commission_events: EventHandle<WithdrawCommissionEvent>,
        switch_operator_events: EventHandle<SwitchOperatorEvent>,
        end_delegation_events: EventHandle<EndDelegationEvent>,
        withdraw_delegation_events: EventHandle<WithdrawDelegationEvent>,
    }

    struct DelegateEvent has drop, store {
        operator: address,
        voter: address,
        pool_address: address,
        amount: u64,
        commission_percentage: u64,
    }

    struct UpdateVoterEvent has drop, store {
        operator: address,
        pool_address: address,
        old_voter: address,
        new_voter: address,
    }

    struct RequestCommissionEvent has drop, store {
        operator: address,
        pool_address: address,
        accumulated_rewards: u64,
        commission_amount: u64,
        previous_total_stake: u64,
        updated_total_stake: u64,
    }

    struct WithdrawCommissionEvent has drop, store {
        operator: address,
        pool_address: address,
        total_withdrawable_amount: u64,
        amount_paid_to_current_operator: u64,
    }

    struct SwitchOperatorEvent has drop, store {
        pool_address: address,
        old_operator: address,
        new_operator: address,
        unpaid_commission_amount: u64,
        updated_total_stake: u64,
    }

    struct EndDelegationEvent has drop, store {
        pool_address: address,
        operator: address,
        unpaid_commission_amount: u64,
    }

    struct WithdrawDelegationEvent has drop, store {
        operator: address,
        pool_address: address,
        total_withdrawable_amount: u64,
        amount_withdrawn: u64,
    }

    public fun get_stake_pool_address(delegator: address, operator: address): address acquires Delegations {
        let delegations = &borrow_global<Delegations>(delegator).delegations;
        simple_map::borrow(delegations, &operator).pool_address
    }

    public fun get_last_recorded_total_stake(delegator: address, operator: address): u64 acquires Delegations {
        let delegations = &borrow_global<Delegations>(delegator).delegations;
        simple_map::borrow(delegations, &operator).total_stake
    }

    public fun get_commission_percentage(delegator: address, operator: address): u64 acquires Delegations {
        let delegations = &borrow_global<Delegations>(delegator).delegations;
        simple_map::borrow(delegations, &operator).commission_percentage
    }

    public fun is_delegation_active(delegator: address, operator: address): bool acquires Delegations {
        let delegations = &borrow_global<Delegations>(delegator).delegations;
        simple_map::borrow(delegations, &operator).is_active
    }

    /// Delegator can call this function to define a simple delegation contract with a specified operator.
    ///
    /// Can only delegate to a specific operator once. Afterward, delegator cannot update amount or commission.
    public entry fun delegate(
        delegator: &signer,
        operator: address,
        voter: address,
        amount: u64,
        commission_percentage: u64,
    ) acquires Delegations {
        assert!(
            commission_percentage >= 0 && commission_percentage <= 100,
            error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE),
        );
        // The delegated stake should be at least the min_stake_required, so the stake pool will be eligible to join the
        // validator set.
        let (min_stake_required, _) = staking_config::get_required_stake(&staking_config::get());
        assert!(amount > min_stake_required, error::invalid_argument(EINSUFFICIENT_DELEGATION_AMOUNT));

        // Initialize Delegations resource if this is the first the delegator has delegated to anyone.
        let delegator_address = signer::address_of(delegator);
        if (!exists<Delegations>(delegator_address)) {
            move_to(delegator, new_delegations_resource(delegator));
        };

        // Only allow delegating to the same operator once.
        let delegations = borrow_global_mut<Delegations>(delegator_address);
        let delegations_map = &mut delegations.delegations;
        assert!(
            !simple_map::contains_key(delegations_map, &operator),
            error::invalid_argument(ECANNOT_CHANGE_EXISTING_DELEGATION)
        );

        // Initialize the stake pool in a new resource account. This allows the same delegator to delegate to multiple
        // different operators.
        let seed = create_seed(delegator_address, operator);
        let (stake_pool_signer, stake_pool_signer_cap) = account::create_resource_account(delegator, seed);
        // Initialize stake pool with amount = 0 as the coins need to come from the delegator, not the stake pool
        // resource account.
        stake::initialize_stake_owner(&stake_pool_signer, 0, operator, voter);

        // Extract owner_cap from the StakePool, so we have control over it in the delegations flow.
        // This is stored as part of the delegation. Thus, the delegator would not have direct control over it without
        // going through well-defined functions in this module.
        let owner_cap = stake::extract_owner_cap(&stake_pool_signer);

        // Add the delegation to the stake pool.
        let delegated_coins = coin::withdraw<AptosCoin>(delegator, amount);
        stake::add_stake_with_cap(&owner_cap, delegated_coins);

        // Record the delegation.
        let pool_address = signer::address_of(&stake_pool_signer);
        simple_map::add(delegations_map, operator, Delegation {
            is_active: true,
            total_stake: amount,
            pool_address,
            owner_cap,
            commission_percentage,
            debts: vector::empty<CommissionDebt>(),
            signer_cap: stake_pool_signer_cap,
        });

        emit_event<DelegateEvent>(
            &mut delegations.delegate_events,
            DelegateEvent { operator, voter, pool_address, amount, commission_percentage },
        );
    }

    /// Convenient function to allow the delegator to update the voter address in a delegation they made.
    public entry fun update_voter(delegator: &signer, operator: address, new_voter: address) acquires Delegations {
        let delegator_address = signer::address_of(delegator);
        assert_delegation_exists(delegator_address, operator);

        let delegations = borrow_global_mut<Delegations>(delegator_address);
        let delegations_map = &mut delegations.delegations;
        let delegation = simple_map::borrow_mut(delegations_map, &operator);
        let pool_address = delegation.pool_address;
        let old_voter = stake::get_delegated_voter(pool_address);
        stake::set_delegated_voter_with_cap(&delegation.owner_cap, new_voter);

        emit_event<UpdateVoterEvent>(
            &mut delegations.update_voter_events,
            UpdateVoterEvent { operator, pool_address, old_voter, new_voter },
        );
    }

    /// Unlock commission amount from the stake pool. Operator needs to wait for the amount to become withdrawable
    /// at the end of the stake pool's lockup period before they can actually can withdraw_commission.
    public entry fun request_commission(operator_account: &signer, delegator: address) acquires Delegations {
        let operator = signer::address_of(operator_account);
        assert_delegation_exists(delegator, operator);

        let delegations = borrow_global_mut<Delegations>(delegator);
        let delegations_map = &mut delegations.delegations;
        let delegation = simple_map::borrow_mut(delegations_map, &operator);
        assert!(delegation.commission_percentage > 0, error::invalid_argument(EZERO_COMMISSION_RATE));
        assert!(delegation.is_active, error::invalid_argument(EINACTIVE_DELEGATION));

        // Unlock just the commission portion from the stake pool.
        let previous_total_stake = delegation.total_stake;
        let (total_active_stake, accumulated_rewards, commission_amount) = get_accumulated_rewards(delegation);
        stake::unlock_with_cap(commission_amount, &delegation.owner_cap);

        // Update new total stake amount to reflect the paid commission amount.
        let updated_total_stake = total_active_stake - commission_amount;
        delegation.total_stake = updated_total_stake;

        let pool_address = delegation.pool_address;
        emit_event<RequestCommissionEvent>(
            &mut delegations.request_commission_events,
            RequestCommissionEvent {
                operator,
                pool_address,
                accumulated_rewards,
                commission_amount,
                previous_total_stake,
                updated_total_stake,
            },
        );
    }

    /// Allow the operator to withdraw any unlocked commissions.
    public entry fun withdraw_commission(operator_account: &signer, delegator: address) acquires Delegations {
        let operator = signer::address_of(operator_account);
        assert_delegation_exists(delegator, operator);

        let delegations = borrow_global_mut<Delegations>(delegator);
        let delegations_map = &mut delegations.delegations;
        let delegation = simple_map::borrow_mut(delegations_map, &operator);
        assert!(delegation.is_active, error::invalid_argument(EINACTIVE_DELEGATION));

        let pool_address = delegation.pool_address;
        // Invariant: All withdrawable coins are commission before the delegator ends the delegation or switch operators
        let (_, total_withdrawable_amount, _, _) = stake::get_stake(pool_address);
        let coins = stake::withdraw_with_cap(&delegation.owner_cap, total_withdrawable_amount);

        // Pay off any outstanding debts to previous operators first before paying commission to this current operator.
        pay_debts(delegation, &mut coins);
        let amount_paid_to_current_operator = coin::value(&coins);
        coin::deposit(operator, coins);

        emit_event<WithdrawCommissionEvent>(
            &mut delegations.withdraw_commission_events,
            WithdrawCommissionEvent {
                operator,
                pool_address,
                total_withdrawable_amount,
                amount_paid_to_current_operator,
            }
        );
    }

    /// Allows delegator to switch operator without going through the lenghty process to unstake.
    ///
    /// This cannot be called if the delegation is no longer active. Delegators would need to wait until they can
    /// withdraw their delegation and re-delegate to the new operator.
    public entry fun switch_operator(
        delegator: &signer,
        old_operator: address,
        new_operator: address,
        new_commission_percentage: u64,
    ) acquires Delegations {
        let delegator_address = signer::address_of(delegator);
        assert_delegation_exists(delegator_address, old_operator);

        // Merging two existing delegations are too complex as we'd need to merge two separate stake pools.
        let delegations = borrow_global_mut<Delegations>(delegator_address);
        let delegations_map = &mut delegations.delegations;
        assert!(
            !simple_map::contains_key(delegations_map, &new_operator),
            error::not_found(ECANT_MERGE_TWO_EXISTING_STAKE_POOLS),
        );

        let (_, delegation) = simple_map::remove(delegations_map, &old_operator);
        assert!(delegation.is_active, error::invalid_argument(EINACTIVE_DELEGATION));
        let (total_active_stake, unpaid_commission_amount, new_commission) =
            record_unpaid_commission(&mut delegation, old_operator);
        let updated_total_stake = total_active_stake - new_commission;

        // Request to unlock the new commission portion that has not been requested.
        stake::unlock_with_cap(new_commission, &delegation.owner_cap);

        // Update stake pool and delegation with the new total stake (minus unlocked commission) and the new operator.
        stake::set_operator_with_cap(&delegation.owner_cap, new_operator);
        delegation.commission_percentage = new_commission_percentage;
        let pool_address = delegation.pool_address;
        delegation.total_stake = updated_total_stake;
        simple_map::add(delegations_map, new_operator, delegation);

        emit_event<SwitchOperatorEvent>(
            &mut delegations.switch_operator_events,
            SwitchOperatorEvent {
                pool_address,
                old_operator,
                new_operator,
                unpaid_commission_amount,
                updated_total_stake,
            }
        );
    }

    /// Delegator can call this to end an active delegation. This would record any outstanding unpaid commissions as
    /// debts that will later need to be paid during withdrawal.
    ///
    /// This cannot be called twice and will fail if the delegation is no longer active.
    public entry fun end_delegation(delegator: &signer, operator: address) acquires Delegations {
        let delegator_address = signer::address_of(delegator);
        assert_delegation_exists(delegator_address, operator);

        let delegations = borrow_global_mut<Delegations>(delegator_address);
        let delegations_map = &mut delegations.delegations;
        let delegation = simple_map::borrow_mut(delegations_map, &operator);
        assert!(delegation.is_active, error::invalid_argument(EDELEGATION_ALREADY_ENDED));
        delegation.is_active = false;

        // Record any outstanding commissions not yet paid.
        let (_, unpaid_commission_amount, _) = record_unpaid_commission(delegation, operator);

        // Update the operator to be the delegator, so we can safely make sure the validator node is removed
        // from the validator set.
        // This also ensures that from now on, the operator no longer has any power over the stake pool.
        stake::set_operator_with_cap(&delegation.owner_cap, delegator_address);

        // Safely remove the validator from the validator set.
        let validator_state = stake::get_validator_state(delegation.pool_address);
        if (validator_state == VALIDATOR_STATUS_PENDING_ACTIVE || validator_state == VALIDATOR_STATUS_ACTIVE) {
            stake::leave_validator_set(delegator, delegation.pool_address);
        };

        // Unlock all active stake. There should be no pending active stake. Otherwise, the delegator should wait until
        // the next epoch to undelegate as there's currnetly no way to convert pending_active stake directly to inactive
        let (active, _, pending_active, _) = stake::get_stake(delegation.pool_address);
        assert!(pending_active == 0, error::invalid_state(EPENDING_ACTIVE_FOUND));
        stake::unlock_with_cap(active, &delegation.owner_cap);

        emit_event<EndDelegationEvent>(
            &mut delegations.end_delegation_events,
            EndDelegationEvent {
                pool_address: delegation.pool_address,
                operator,
                unpaid_commission_amount,
            }
        );
    }

    /// Delegators can call this to withdraw fully unlocked delegation. This would pay off any commission debts first
    /// before sending the remaining amount to the delegator.
    ///
    /// This cannot be called if the delegation is still active (i.e. delegator never called end_delegation).
    public entry fun withdraw_delegation(delegator: &signer, operator: address) acquires Delegations {
        let delegator_address = signer::address_of(delegator);
        assert_delegation_exists(delegator_address, operator);

        let delegations = borrow_global_mut<Delegations>(delegator_address);
        let delegations_map = &mut delegations.delegations;
        let (_, delegation) = simple_map::remove(delegations_map, &operator);
        assert!(!delegation.is_active, error::invalid_argument(EDELEGATION_STILL_ACTIVE));

        let (_, total_withdrawable_amount, _, pending_inactive) = stake::get_stake(delegation.pool_address);
        // Ensure all pending_inactive funds have been converted to active to avoid any stuck/lost funds.
        assert!(pending_inactive == 0, error::invalid_state(EFUNDS_STILL_BEING_UNLOCKED));
        let coins = stake::withdraw_with_cap(&delegation.owner_cap, total_withdrawable_amount);

        // Pay off any outstanding debts to previous operators first.
        pay_debts(&mut delegation, &mut coins);
        let amount_withdrawn = coin::value(&coins);
        coin::deposit(delegator_address, coins);

        // Destroy the delegation.
        let Delegation {
            total_stake: _,
            pool_address,
            owner_cap,
            commission_percentage: _,
            is_active: _,
            debts: _,
            signer_cap: _,
        } = delegation;
        stake::destroy_owner_cap(owner_cap);

        emit_event<WithdrawDelegationEvent>(
            &mut delegations.withdraw_delegation_events,
            WithdrawDelegationEvent { pool_address, operator, total_withdrawable_amount, amount_withdrawn }
        );
    }

    // Assert that a delegation exists for the delegator/operator pair.
    fun assert_delegation_exists(delegator: address, operator: address) acquires Delegations {
        assert!(
            exists<Delegations>(delegator),
            error::not_found(ENO_DELEGATIONS_FOUND),
        );
        let delegations = &mut borrow_global_mut<Delegations>(delegator).delegations;
        assert!(
            simple_map::contains_key(delegations, &operator),
            error::not_found(EDELEGATION_NOT_FOUND),
        );
    }

    // Create a salt for generating the resource accounts that will be holding the StakePool for a given delegation.
    // This address should be deterministic for the same delegator and operator.
    fun create_seed(delegator: address, operator: address): vector<u8> {
        let seed = bcs::to_bytes(&delegator);
        vector::append(&mut seed, bcs::to_bytes(&operator));
        // Include a salt to avoid conflicts with any other modules out there that might also generate
        // deterministic resource accounts for the same delegator + operator addresses.
        vector::append(&mut seed, STAKE_POOL_ACCOUNT_SALT);
        seed
    }

    // Record any commission debts. This includes:
    // 1. Any unlocked commission not yet withdrawn (e.g. still locked up in the stake pool).
    // 2. Any unrequested commission from accumulated rewards since last commission request.
    fun record_unpaid_commission(delegation: &mut Delegation, creditor: address): (u64, u64, u64) {
        // Record any outstanding commission debt that has not been paid to the old operator.
        // This includes any commision from outstanding rewards + requested commissions not yet paid out.
        let (_, inactive, _, pending_inactive) = stake::get_stake(delegation.pool_address);
        let unpaid_commission = pending_inactive + inactive;
        let (total_active_stake, _, new_commission) = get_accumulated_rewards(delegation);
        let total_unpaid_commission = unpaid_commission + new_commission;
        vector::push_back(&mut delegation.debts, CommissionDebt {
            creditor,
            amount: total_unpaid_commission,
        });

        (total_active_stake, total_unpaid_commission, new_commission)
    }

    // Extract from `coins` to pay any oustanding commission debts in the current delegation.
    fun pay_debts(delegation: &mut Delegation, coins: &mut Coin<AptosCoin>) {
        let debts = &mut delegation.debts;
        while (vector::length(debts) > 0 && coin::value(coins) > 0) {
            let debt = vector::remove(debts, 0);
            let coin_amount = coin::value(coins);
            // Pay as much debt as possible.
            let amount_to_pay = if (coin_amount >= debt.amount) {
                debt.amount
            } else {
                vector::push_back(debts, CommissionDebt {
                    creditor: debt.creditor,
                    amount: debt.amount - coin_amount,
                });
                coin_amount
            };
            let coins_to_pay = coin::extract(coins, amount_to_pay);
            coin::deposit(debt.creditor, coins_to_pay);
        }
    }

    // Calculate commission since last update and update the total stake amount.
    fun get_accumulated_rewards(delegation: &mut Delegation): (u64, u64, u64) {
        // Any outgoing flows of funds before the delegator withdraws all delegations can only come from operator
        // withdrawing commissions.
        // So to calculate rewards, we only care about current active + pending_active - last recorded total stake.
        let (active, _, pending_active, _) = stake::get_stake(delegation.pool_address);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - delegation.total_stake;
        let commission_amount = accumulated_rewards * delegation.commission_percentage / 100;

        (total_active_stake, accumulated_rewards, commission_amount)
    }

    // Create a new delegations resource.
    fun new_delegations_resource(delegator: &signer): Delegations {
        Delegations {
            delegations: simple_map::create<address, Delegation>(),

            // Events.
            delegate_events: account::new_event_handle<DelegateEvent>(delegator),
            update_voter_events: account::new_event_handle<UpdateVoterEvent>(delegator),
            request_commission_events: account::new_event_handle<RequestCommissionEvent>(delegator),
            withdraw_commission_events: account::new_event_handle<WithdrawCommissionEvent>(delegator),
            switch_operator_events: account::new_event_handle<SwitchOperatorEvent>(delegator),
            end_delegation_events: account::new_event_handle<EndDelegationEvent>(delegator),
            withdraw_delegation_events: account::new_event_handle<WithdrawDelegationEvent>(delegator),
        }
    }

    #[test_only]
    use aptos_framework::timestamp;

    #[test_only]
    const INITIAL_BALANCE: u64 = 1000;

    #[test_only]
    public fun setup(aptos_framework: &signer, delegator: &signer, operator: &signer) {
        // 100% per-epoch rewards for testing purposes.
        stake::initialize_for_test_custom(aptos_framework, 100, 10000, 1, true, 100, 100, 1000000);

        account::create_account_for_test(signer::address_of(delegator));
        account::create_account_for_test(signer::address_of(operator));
        stake::mint(delegator, INITIAL_BALANCE);
        stake::mint(operator, INITIAL_BALANCE);

        // Let a random account join the validator set so the tested stake pool can leave the validator set later.
        let random_validator = &account::create_account_for_test(@0x42);
        stake::initialize_test_validator(random_validator, INITIAL_BALANCE, true, true);
    }

    #[test_only]
    const CONSENSUS_KEY: vector<u8> = x"8a54b92288d4ba5073d3a52e80cc00ae9fbbc1cc5b433b46089b7804c38a76f00fc64746c7685ee628fc2d0b929c2294";
    #[test_only]
    const CONSENSUS_POP: vector<u8> = x"a9d6c1f1270f2d1454c89a83a4099f813a56dc7db55591d46aa4e6ccae7898b234029ba7052f18755e6fa5e6b73e235f14efc4e2eb402ca2b8f56bad69f965fc11b7b25eb1c95a06f83ddfd023eac4559b6582696cfea97b227f4ce5bdfdfed0";

    #[test_only]
    public fun join_validator_set(operator: &signer, pool_address: address) {
        stake::rotate_consensus_key(operator, pool_address, CONSENSUS_KEY, CONSENSUS_POP);
        stake::join_validator_set(operator, pool_address);
        stake::end_epoch();
    }

    #[test_only]
    public fun fast_forward_to_unlock() {
        timestamp::fast_forward_seconds(1);
        stake::end_epoch();
    }

    #[test_only]
    public fun setup_delegation(
        aptos_framework: &signer,
        delegator: &signer,
        operator: &signer,
        amount: u64,
        commission: u64,
    ) acquires Delegations {
        setup(aptos_framework, delegator, operator);
        let operator_address = signer::address_of(operator);

        // Voter is initially set to operator but then updated to be delegator.
        delegate(delegator, operator_address, operator_address, amount, commission);
    }

    #[test(aptos_framework = @0x1, delegator = @0x123, operator = @0x234)]
    public entry fun test_end_to_end(
        aptos_framework: &signer,
        delegator: &signer,
        operator: &signer,
    ) acquires Delegations {
        setup_delegation(aptos_framework, delegator, operator, INITIAL_BALANCE, 10);
        let delegator_address = signer::address_of(delegator);
        let operator_address = signer::address_of(operator);
        assert_delegation_exists(delegator_address, operator_address);
        assert_delegation(delegator_address, operator_address, INITIAL_BALANCE, 10);

        // Verify that the stake pool has been set up properly.
        let pool_address = get_stake_pool_address(delegator_address, operator_address);
        stake::assert_stake_pool(pool_address, INITIAL_BALANCE, 0, 0, 0);
        assert!(get_last_recorded_total_stake(delegator_address, operator_address) == INITIAL_BALANCE, 0);

        // Operator joins the validator set.
        join_validator_set(operator, pool_address);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 1);

        // Fast forward to generate rewards.
        stake::end_epoch();
        let new_balance = INITIAL_BALANCE * 2;
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, 0);
        // Operator claims 10% of rewards so far as commissions
        let expected_commission_1 = (new_balance - get_last_recorded_total_stake(delegator_address, operator_address)) / 10;
        new_balance = new_balance - expected_commission_1;
        request_commission(operator, delegator_address);
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, expected_commission_1);
        assert!(get_last_recorded_total_stake(delegator_address, operator_address) == new_balance, 0);
        fast_forward_to_unlock();
        // Both original stake and operator commissions have doubled from 100% in rewards.
        expected_commission_1 = expected_commission_1 * 2;
        new_balance = new_balance * 2;
        stake::assert_stake_pool(pool_address, new_balance, expected_commission_1, 0, 0);
        withdraw_commission(operator, delegator_address);
        let operator_balance = coin::balance<AptosCoin>(operator_address);
        let expected_operator_balance = INITIAL_BALANCE + expected_commission_1;
        assert!(operator_balance == expected_operator_balance, operator_balance);
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, 0);

        // Second round of commission request/withdrawal.
        let expected_commission_2 = (new_balance - get_last_recorded_total_stake(delegator_address, operator_address)) / 10;
        new_balance = new_balance - expected_commission_2;
        request_commission(operator, delegator_address);
        assert!(get_last_recorded_total_stake(delegator_address, operator_address) == new_balance, 0);
        fast_forward_to_unlock();
        expected_commission_2 = expected_commission_2 * 2;
        new_balance = new_balance * 2;
        withdraw_commission(operator, delegator_address);
        operator_balance = coin::balance<AptosCoin>(operator_address);
        expected_operator_balance = expected_operator_balance + expected_commission_2;
        assert!(operator_balance == expected_operator_balance, operator_balance);

        // New rounds of rewards.
        fast_forward_to_unlock();
        new_balance = new_balance * 2;

        // Delegator ends the delegation, which should record unpaid commission as debt.
        let unpaid_commission = (new_balance - get_last_recorded_total_stake(delegator_address, operator_address)) / 10;
        end_delegation(delegator, operator_address);
        stake::assert_stake_pool(pool_address, 0, 0, 0, new_balance);
        assert!(stake::get_operator(pool_address) == delegator_address, 2);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_PENDING_INACTIVE, 3);
        assert_debt(delegator_address, operator_address, operator_address, unpaid_commission);

        // Withdraw delegation.
        fast_forward_to_unlock();
        // Since we ended the epoch while the stake pool is still pending_inactive, the stake pool still received
        // rewards.
        new_balance = new_balance * 2;
        stake::assert_stake_pool(pool_address, 0, new_balance, 0, 0);
        withdraw_delegation(delegator, operator_address);
        operator_balance = coin::balance<AptosCoin>(operator_address);
        assert!(operator_balance == expected_operator_balance + unpaid_commission, operator_balance);
        let delegator_balance = coin::balance<AptosCoin>(delegator_address);
        assert!(delegator_balance == new_balance - unpaid_commission, delegator_balance);
    }

    #[test(aptos_framework = @0x1, delegator = @0x123, operator = @0x234)]
    #[expected_failure(abort_code = 0x10007)]
    public entry fun test_delegator_cannot_delegate_to_same_operator_multiple_times(
        aptos_framework: &signer,
        delegator: &signer,
        operator: &signer,
    ) acquires Delegations {
        setup_delegation(aptos_framework, delegator, operator, INITIAL_BALANCE, 10);
        let operator_address = signer::address_of(operator);
        delegate(delegator, operator_address, operator_address, 1000, 10);
    }

    #[test(aptos_framework = @0x1, delegator = @0x123, operator = @0x234)]
    #[expected_failure(abort_code = 0x10002)]
    public entry fun test_delegator_cannot_delegate_with_invalid_commission(
        aptos_framework: &signer,
        delegator: &signer,
        operator: &signer,
    ) acquires Delegations {
        setup_delegation(aptos_framework, delegator, operator, INITIAL_BALANCE, 101);
    }

    #[test(aptos_framework = @0x1, delegator = @0x123, operator = @0x234)]
    #[expected_failure(abort_code = 0x10001)]
    public entry fun test_delegator_cannot_delegate_less_than_min_stake_required(
        aptos_framework: &signer,
        delegator: &signer,
        operator: &signer,
    ) acquires Delegations {
        setup_delegation(aptos_framework, delegator, operator, 50, 100);
    }

    #[test(aptos_framework = @0x1, delegator = @0x123, operator = @0x234)]
    public entry fun test_update_voter(
        aptos_framework: &signer,
        delegator: &signer,
        operator: &signer,
    ) acquires Delegations {
        setup_delegation(aptos_framework, delegator, operator, INITIAL_BALANCE, 10);
        let delegator_address = signer::address_of(delegator);
        let operator_address = signer::address_of(operator);

        // Voter is initially set to operator but then updated to be delegator.
        let pool_address = get_stake_pool_address(delegator_address, operator_address);
        assert!(stake::get_delegated_voter(pool_address) == operator_address, 0);
        update_voter(delegator, operator_address, delegator_address);
        assert!(stake::get_delegated_voter(pool_address) == delegator_address, 1);
    }

    #[test(aptos_framework = @0x1, delegator = @0x123, operator = @0x234)]
    #[expected_failure(abort_code = 0x10008)]
    public entry fun test_operator_cannot_request_commission_from_inactive_delegation(
        aptos_framework: &signer,
        delegator: &signer,
        operator: &signer,
    ) acquires Delegations {
        setup_delegation(aptos_framework, delegator, operator, INITIAL_BALANCE, 10);

        // Delegator ends delegation, which sets the state to inactive.
        let operator_address = signer::address_of(operator);
        end_delegation(delegator, operator_address);

        // Requesting commission after the delegation becomes inactive should error out.
        request_commission(operator, signer::address_of(delegator));
    }

    #[test(aptos_framework = @0x1, delegator = @0x123, operator = @0x234)]
    #[expected_failure(abort_code = 0x10008)]
    public entry fun test_operator_cannot_withdraw_commission_from_inactive_delegation(
        aptos_framework: &signer,
        delegator: &signer,
        operator: &signer,
    ) acquires Delegations {
        setup_delegation(aptos_framework, delegator, operator, INITIAL_BALANCE, 10);

        // Requesting commission before the delegation becomes inactive should work.
        let operator_address = signer::address_of(operator);
        let delegator_address = signer::address_of(delegator);
        request_commission(operator, delegator_address);

        // Delegator ends delegation, which sets the state to inactive.
        end_delegation(delegator, operator_address);

        // Withdrawing commission after the delegation becomes inactive should error out.
        withdraw_commission(operator, delegator_address);
    }

    #[test(aptos_framework = @0x1, delegator = @0x123, operator_1 = @0x234, operator_2 = @0x345)]
    public entry fun test_delegator_can_switch_operator(
        aptos_framework: &signer,
        delegator: &signer,
        operator_1: &signer,
        operator_2: &signer,
    ) acquires Delegations {
        setup_delegation(aptos_framework, delegator, operator_1, INITIAL_BALANCE, 10);
        account::create_account_for_test(signer::address_of(operator_2));
        stake::mint(operator_2, INITIAL_BALANCE);
        let delegator_address = signer::address_of(delegator);
        let operator_1_address = signer::address_of(operator_1);
        let operator_2_address = signer::address_of(operator_2);

        // Join validator set and earn some rewards.
        let pool_address = get_stake_pool_address(delegator_address, operator_1_address);
        join_validator_set(operator_1, pool_address);
        stake::end_epoch();
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 0);

        // Switch operators.
        switch_operator(delegator, operator_1_address, operator_2_address, 20);
        // The delegation is now associated with operator 2 but there's a commission debt to operator 1.
        let stake_balance = INITIAL_BALANCE * 2; // 100% rewards.
        let unpaid_commission = (stake_balance - INITIAL_BALANCE) / 10;
        assert_debt(delegator_address, operator_2_address, operator_1_address, unpaid_commission);
        // Unpaid commission should be unlocked from the stake pool.
        let expected_total_active_stake = stake_balance - unpaid_commission;
        stake::assert_stake_pool(pool_address, expected_total_active_stake, 0, 0, unpaid_commission);
        assert!(get_last_recorded_total_stake(delegator_address, operator_2_address) == expected_total_active_stake, 0);

        // The stake pool's validator should not have left the validator set.
        assert!(get_stake_pool_address(delegator_address, operator_2_address) == pool_address, 1);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 2);

        // End epoch to get more rewards.
        fast_forward_to_unlock();

        // Verify that when commissions are withdrawn, previous debt are first paid to operator 1. Then new commission
        // is paid to operator 2.
        let current_total_stake_with_rewards = expected_total_active_stake * 2;
        // Commission rate was updated so operator 2 should receive 20% rewards instead of 10%.
        let commission_for_operator_2 = (current_total_stake_with_rewards - expected_total_active_stake) / 5;
        request_commission(operator_2, delegator_address);
        expected_total_active_stake = current_total_stake_with_rewards - commission_for_operator_2;
        stake::assert_stake_pool(
            pool_address,
            expected_total_active_stake,
            // Includes rewards.
            unpaid_commission * 2,
            0,
            commission_for_operator_2,
        );
        assert!(get_last_recorded_total_stake(delegator_address, operator_2_address) == expected_total_active_stake, 0);
        fast_forward_to_unlock();
        withdraw_commission(operator_2, delegator_address);
        let operator_1_balance = coin::balance<AptosCoin>(operator_1_address);
        let operator_2_balance = coin::balance<AptosCoin>(operator_2_address);
        assert!(operator_1_balance == INITIAL_BALANCE + unpaid_commission, operator_1_balance);
        // Unfortunately the rewards of the recorded commission debt to operator 1 still goes to operator 2 as it'd be
        // too complex to track which portion of the debt still receives rewards vs not.
        commission_for_operator_2 = commission_for_operator_2 * 2; // With 100% rewards.
        assert!(
            operator_2_balance == INITIAL_BALANCE + commission_for_operator_2 + unpaid_commission, operator_2_balance);
        stake::assert_stake_pool(
            pool_address,
            expected_total_active_stake * 2, // With 100% rewards.
            0,
            0,
            0,
        );
    }

    #[test_only]
    public fun assert_delegation(
        delegator: address, operator: address, total_stake: u64, commission_percentage: u64) acquires Delegations {
        let delegations = &borrow_global<Delegations>(delegator).delegations;
        let delegation = simple_map::borrow(delegations, &operator);
        assert!(delegation.total_stake == total_stake, delegation.total_stake);
        assert!(delegation.commission_percentage == commission_percentage, delegation.commission_percentage);
    }

    #[test_only]
    public fun assert_debt(delegator: address, operator: address, creditor: address, amount: u64) acquires Delegations {
        let delegations = &borrow_global<Delegations>(delegator).delegations;
        let delegation = simple_map::borrow(delegations, &operator);
        let debt= vector::borrow(&delegation.debts, 0);
        assert!(debt.creditor == creditor, 0);
        assert!(debt.amount == amount, debt.amount);
    }
}
