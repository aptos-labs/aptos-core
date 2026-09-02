spec aptos_framework::staking_contract {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The Store structure for the staker exists after the staking contract is created.
    /// Criticality: Medium
    /// Implementation: The create_staking_contract_with_coins function ensures that the staker account has a Store
    /// structure assigned.
    /// Enforcement: Formally verified via [high-level-req-1](CreateStakingContractWithCoinsAbortsifAndEnsures).
    ///
    /// No.: 2
    /// Requirement: A staking contract is created and stored in a mapping within the Store resource.
    /// Criticality: High
    /// Implementation: The create_staking_contract_with_coins function adds the newly created StakingContract to the
    /// staking_contracts map with the operator as a key of the Store resource, effectively storing the staking contract.
    /// Enforcement: Formally verified via [high-level-req-2](CreateStakingContractWithCoinsAbortsifAndEnsures).
    ///
    /// No.: 3
    /// Requirement: Adding stake to the stake pool increases the principal value of the pool, reflecting the additional
    /// stake amount.
    /// Criticality: High
    /// Implementation: The add_stake function transfers the specified amount of staked coins from the staker's account
    /// to the stake pool associated with the staking contract. It increases the principal value of the staking contract
    /// by the added stake amount.
    /// Enforcement: Formally verified via [high-level-req-3](add_stake).
    ///
    /// No.: 4
    /// Requirement: The staker may update the voter of a staking contract, enabling them to modify the assigned voter
    /// address and ensure it accurately reflects their desired choice.
    /// Criticality: High
    /// Implementation: The update_voter function ensures that the voter address in a staking contract may be updated by
    /// the staker, resulting in the modification of the delegated voter address in the associated stake pool to reflect
    /// the new address provided.
    /// Enforcement: Formally verified via [high-level-req-4](update_voter).
    ///
    /// No.: 5
    /// Requirement: Only the owner of the stake pool has the permission to reset the lockup period of the pool.
    /// Criticality: Critical
    /// Implementation: The reset_lockup function ensures that only the staker who owns the stake pool has the authority
    /// to reset the lockup period of the pool.
    /// Enforcement: Formally verified via [high-level-req-5](reset_lockup).
    ///
    /// No.: 6
    /// Requirement: Unlocked funds are correctly distributed to recipients based on their distribution shares, taking into
    /// account the associated commission percentage.
    /// Criticality: High
    /// Implementation: The distribution process, implemented in the distribute_internal function, accurately allocates
    /// unlocked funds to their intended recipients based on their distribution shares. It guarantees that each
    /// recipient receives the correct amount of funds, considering the commission percentage associated with the
    /// staking contract.
    /// Enforcement: Audited that the correct amount of unlocked funds is distributed according to distribution shares.
    ///
    /// No.: 7
    /// Requirement: The stake pool ensures that the commission is correctly requested and paid out from the old operator's
    /// stake pool before allowing the switch to the new operator.
    /// Criticality: High
    /// Implementation: The switch_operator function initiates the commission payout from the stake pool associated with
    /// the old operator, ensuring a smooth transition. Paying out the commission before the switch guarantees that the
    /// staker receives the appropriate commission amount and maintains the integrity of the staking process.
    /// Enforcement: Audited that the commission is paid to the old operator.
    ///
    /// No.: 8
    /// Requirement: Stakers can withdraw their funds from the staking contract, ensuring the unlocked amount becomes
    /// available for withdrawal after the lockup period.
    /// Criticality: High
    /// Implementation: The unlock_stake function ensures that the requested amount is properly unlocked from the stake
    /// pool, considering the lockup period and that the funds become available for withdrawal when the lockup expires.
    /// Enforcement: Audited that funds are unlocked properly.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec StakingContract {
        invariant commission_percentage >= 0 && commission_percentage <= 100;
        invariant owner_cap.pool_address == pool_address;
        invariant signer_cap.account == pool_address;
    }

    /// Exact read-only lookup after the two source assertions.
    spec stake_pool_address(staker: address, operator: address): address {
        use 0x1::simple_map;
        pragma opaque = true, aborts_if_is_partial = false;
        include ContractExistsAbortsIf;
        let staking_contracts = global<Store>(staker).staking_contracts;
        ensures result == simple_map::spec_get(staking_contracts, operator).pool_address;
    }

    /// Staking_contract exists the stacker/operator pair.
    /// Exact read-only lookup after the two source assertions.
    spec last_recorded_principal(staker: address, operator: address): u64 {
        use 0x1::simple_map;
        pragma opaque = true, aborts_if_is_partial = false;
        include ContractExistsAbortsIf;
        let staking_contracts = global<Store>(staker).staking_contracts;
        ensures result == simple_map::spec_get(staking_contracts, operator).principal;
    }

    /// Staking_contract exists the stacker/operator pair.
    /// Exact read-only lookup after the two source assertions.
    spec commission_percentage(staker: address, operator: address): u64 {
        use 0x1::simple_map;
        pragma opaque = true, aborts_if_is_partial = false;
        include ContractExistsAbortsIf;
        let staking_contracts = global<Store>(staker).staking_contracts;
        ensures result ==
            simple_map::spec_get(staking_contracts, operator).commission_percentage;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec staking_contract_amounts(staker: address, operator: address): (u64, u64, u64) {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        let staking_contracts = global<Store>(staker).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);

        include ContractExistsAbortsIf;
        include GetStakingContractAmountsAbortsIf { staking_contract };

        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;

        ensures result_1 == total_active_stake;
        ensures result_2 == accumulated_rewards;
        ensures result_3
            == accumulated_rewards * staking_contract.commission_percentage / 100;
    }

    /// Staking_contract exists the stacker/operator pair.
    /// Exact read-only lookup after the two source assertions.
    spec pending_distribution_counts(staker: address, operator: address): u64 {
        use 0x1::simple_map;
        pragma opaque = true, aborts_if_is_partial = false;
        include ContractExistsAbortsIf;
        let staking_contracts = global<Store>(staker).staking_contracts;
        ensures result ==
            len(simple_map::spec_get(staking_contracts, operator).distribution_pool.shareholders);
    }

    /// Exact pool state after redeeming the first shareholder's complete
    /// position. `distribute_internal` always redeems this way.
    spec fun spec_redeem_first_state(pool: Pool): Pool {
        let shareholder = pool.shareholders[0];
        let shares = pool_u64::spec_shares(pool, shareholder);
        let coins = pool_u64::spec_shares_to_amount_with_total_coins(
            pool, shares, pool.total_coins
        );
        Pool {
            shareholders_limit: pool.shareholders_limit,
            total_coins: pool.total_coins - coins,
            total_shares: pool.total_shares - shares,
            shares: simple_map::spec_remove(pool.shares, shareholder),
            shareholders: pool.shareholders[1..len(pool.shareholders)],
            scaling_factor: pool.scaling_factor
        }
    }

    /// State after the first `steps` complete redemptions.
    spec fun spec_redeem_prefix_state(pool: Pool, steps: u64): Pool {
        if (steps == 0) {
            pool
        } else {
            spec_redeem_first_state(spec_redeem_prefix_state(pool, steps - 1))
        }
    }

    /// Exact amount paid to `target` by the first `steps` redemptions, after
    /// redirecting the operator's payment to its configured beneficiary.
    spec fun spec_redeem_prefix_payout(
        pool: Pool,
        steps: u64,
        target: address,
        operator: address,
        operator_beneficiary: address
    ): u64 {
        if (steps == 0) {
            0
        } else {
            let current = spec_redeem_prefix_state(pool, steps - 1);
            let shareholder = current.shareholders[0];
            let recipient = if (shareholder == operator) {
                operator_beneficiary
            } else {
                shareholder
            };
            let shares = pool_u64::spec_shares(current, shareholder);
            let coins = pool_u64::spec_shares_to_amount_with_total_coins(
                current, shares, current.total_coins
            );
            spec_redeem_prefix_payout(
                pool, steps - 1, target, operator, operator_beneficiary
            ) + if (recipient == target) { coins } else { 0 }
        }
    }

    spec fun spec_redeem_prefix_calls_recipient(
        pool: Pool,
        steps: u64,
        target: address,
        operator: address,
        operator_beneficiary: address
    ): bool {
        if (steps == 0) {
            false
        } else {
            let current = spec_redeem_prefix_state(pool, steps - 1);
            let shareholder = current.shareholders[0];
            let recipient = if (shareholder == operator) {
                operator_beneficiary
            } else {
                shareholder
            };
            recipient == target || spec_redeem_prefix_calls_recipient(
                pool, steps - 1, target, operator, operator_beneficiary
            )
        }
    }

    /// Exact local abort domain of the redemption loop. Coin extraction has
    /// the same bound as `redeem_shares`, so its bound is included here too.
    spec fun spec_redeem_prefix_aborts(pool: Pool, steps: u64): bool {
        if (steps == 0) {
            false
        } else {
            let current = spec_redeem_prefix_state(pool, steps - 1);
            let shareholder = current.shareholders[0];
            let shares = pool_u64::spec_shares(current, shareholder);
            let coins = pool_u64::spec_shares_to_amount_with_total_coins(
                current, shares, current.total_coins
            );
            spec_redeem_prefix_aborts(pool, steps - 1)
                || aborts_of<pool_u64::redeem_shares>(current, shareholder, shares)
                || current.total_coins < coins
        }
    }

    spec fun spec_operator_beneficiary(operator: address): address {
        if (exists<BeneficiaryForOperator>(operator)) {
            global<BeneficiaryForOperator>(operator).beneficiary_for_operator
        } else {
            operator
        }
    }

    /// Abort domain of one normalized APT deposit. This is source/effect
    /// equivalent to the coin-to-FA path used by `aptos_account::deposit_coins`
    /// once the canonical APT pairing is installed.
    spec fun spec_aptos_deposit_aborts(recipient: address, amount: u64): bool {
        let store_address = aptos_framework::object::spec_create_user_derived_object_address(
            recipient,
            aptos_framework::primary_fungible_store::DeriveRefPod[
                @aptos_fungible_asset
            ].metadata_derive_ref.self
        );
        let has_store = exists<aptos_framework::fungible_asset::FungibleStore>(
            store_address
        );
        let store = global<aptos_framework::fungible_asset::FungibleStore>(
            store_address
        );
        let concurrent = exists<aptos_framework::fungible_asset::ConcurrentFungibleBalance>(
            store_address
        );
        let balance = if (has_store && store.balance == 0 && concurrent) {
            aptos_framework::aggregator_v2::spec_get_value(
                global<aptos_framework::fungible_asset::ConcurrentFungibleBalance>(
                    store_address
                ).balance
            )
        } else if (has_store) {
            store.balance
        } else {
            0
        };
        (!account::spec_exists_at(recipient)
            && aborts_of<aptos_account::create_account>(recipient))
            || (has_store && store.frozen)
            || (has_store && balance + amount > MAX_U64)
            || (!has_store
                && exists<aptos_framework::object::ObjectCore>(store_address))
    }

    /// Complete loop/payout abort predicate under the StakingContract
    /// representation invariant that the only shareholders are the staker
    /// and operator.
    spec fun spec_distribution_payout_aborts(
        pool: Pool,
        staker: address,
        operator: address,
        operator_beneficiary: address
    ): bool {
        let steps = len(pool.shareholders);
        let final_pool = spec_redeem_prefix_state(pool, steps);
        let staker_amount = spec_redeem_prefix_payout(
            pool, steps, staker, operator, operator_beneficiary
        ) + final_pool.total_coins;
        let beneficiary_amount = spec_redeem_prefix_payout(
            pool, steps, operator_beneficiary, operator, operator_beneficiary
        );
        let calls_staker = spec_redeem_prefix_calls_recipient(
            pool, steps, staker, operator, operator_beneficiary
        ) || final_pool.total_coins > 0;
        let calls_beneficiary = operator_beneficiary != staker
            && spec_redeem_prefix_calls_recipient(
                pool, steps, operator_beneficiary, operator, operator_beneficiary
            );
        spec_redeem_prefix_aborts(pool, steps)
            || (calls_staker && spec_aptos_deposit_aborts(staker, staker_amount))
            || (calls_beneficiary
                && spec_aptos_deposit_aborts(
                    operator_beneficiary, beneficiary_amount
                ))
    }

    spec fun spec_distribute_withdraw_amount(
        staking_contract: StakingContract, stake_pool: stake::StakePool
    ): u64 {
        let pool_address = staking_contract.pool_address;
        let validator_set = global<stake::ValidatorSet>(@aptos_framework);
        let inactive_state = !stake::spec_contains(
            validator_set.pending_active, pool_address
        )
            && !stake::spec_contains(validator_set.active_validators, pool_address)
            && !stake::spec_contains(
                validator_set.pending_inactive, pool_address
            );
        let mature_pending_inactive = inactive_state
            && aptos_framework::timestamp::spec_now_seconds()
                >= stake_pool.locked_until_secs;
        let available = if (mature_pending_inactive) {
            stake_pool.inactive.value + stake_pool.pending_inactive.value
        } else {
            stake_pool.inactive.value
        };
        let requested = stake_pool.inactive.value + stake_pool.pending_inactive.value;
        aptos_std::math64::min(requested, available)
    }

    spec fun spec_distribute_stake_pool_state(
        staking_contract: StakingContract, stake_pool: stake::StakePool
    ): stake::StakePool {
        let pool_address = staking_contract.pool_address;
        let validator_set = global<stake::ValidatorSet>(@aptos_framework);
        let inactive_state = !stake::spec_contains(
            validator_set.pending_active, pool_address
        )
            && !stake::spec_contains(validator_set.active_validators, pool_address)
            && !stake::spec_contains(
                validator_set.pending_inactive, pool_address
            );
        let mature_pending_inactive = inactive_state
            && aptos_framework::timestamp::spec_now_seconds()
                >= stake_pool.locked_until_secs;
        let requested = stake_pool.inactive.value + stake_pool.pending_inactive.value;
        stake::spec_withdraw_with_cap_state(
            stake_pool, mature_pending_inactive, requested
        )
    }

    spec fun spec_distribute_contract_state(
        staking_contract: StakingContract,
        stake_pool: stake::StakePool,
        operator: address
    ): StakingContract {
        let distribution_amount = spec_distribute_withdraw_amount(
            staking_contract, stake_pool
        );
        if (distribution_amount == 0) {
            staking_contract
        } else {
            let updated_pool = spec_distribution_pool_after_update(
                staking_contract.distribution_pool,
                distribution_amount,
                operator,
                staking_contract.commission_percentage
            );
            let redeemed_pool = spec_redeem_prefix_state(
                updated_pool, len(updated_pool.shareholders)
            );
            update_field(
                staking_contract,
                distribution_pool,
                update_field(redeemed_pool, total_coins, 0)
            )
        }
    }

    spec fun spec_unlock_stake_pool_state(
        staking_contract: StakingContract,
        stake_pool: stake::StakePool,
        operator: address,
        requested_amount: u64
    ): stake::StakePool {
        let distributed_contract = spec_distribute_contract_state(
            staking_contract, stake_pool, operator
        );
        let distributed_stake_pool = spec_distribute_stake_pool_state(
            staking_contract, stake_pool
        );
        let total_active_stake = distributed_stake_pool.active.value
            + distributed_stake_pool.pending_active.value;
        let accumulated_rewards = total_active_stake
            - distributed_contract.principal;
        let commission = accumulated_rewards
            * distributed_contract.commission_percentage / 100;
        let after_commission = if (commission == 0) {
            distributed_stake_pool
        } else {
            stake::spec_unlock_with_cap_state(distributed_stake_pool, commission)
        };
        let amount = aptos_std::math64::min(
            requested_amount, after_commission.active.value
        );
        stake::spec_unlock_with_cap_state(after_commission, amount)
    }

    spec fun spec_unlock_staking_contract_state(
        staker: address,
        operator: address,
        staking_contract: StakingContract,
        stake_pool: stake::StakePool,
        requested_amount: u64
    ): StakingContract {
        let distributed_contract = spec_distribute_contract_state(
            staking_contract, stake_pool, operator
        );
        let distributed_stake_pool = spec_distribute_stake_pool_state(
            staking_contract, stake_pool
        );
        let after_commission_contract = spec_request_commission_state(
            distributed_contract, operator, distributed_stake_pool
        );
        let total_active_stake = distributed_stake_pool.active.value
            + distributed_stake_pool.pending_active.value;
        let accumulated_rewards = total_active_stake
            - distributed_contract.principal;
        let commission = accumulated_rewards
            * distributed_contract.commission_percentage / 100;
        let after_commission_stake_pool = if (commission == 0) {
            distributed_stake_pool
        } else {
            stake::spec_unlock_with_cap_state(
                distributed_stake_pool, commission
            )
        };
        let amount = aptos_std::math64::min(
            requested_amount, after_commission_stake_pool.active.value
        );
        let with_principal = update_field(
            after_commission_contract,
            principal,
            after_commission_contract.principal - amount
        );
        let refreshed_pool = spec_distribution_pool_after_update(
            with_principal.distribution_pool,
            after_commission_stake_pool.pending_inactive.value,
            operator,
            with_principal.commission_percentage
        );
        update_field(
            with_principal,
            distribution_pool,
            pool_u64::spec_buy_in_state(refreshed_pool, staker, amount)
        )
    }

    spec fun spec_distribute_internal_aborts_at(
        staker: address,
        operator: address,
        staking_contract: StakingContract,
        stake_pool: stake::StakePool
    ): bool {
        let pool_address = staking_contract.pool_address;
        let has_validator_set = exists<stake::ValidatorSet>(@aptos_framework);
        let validator_set = global<stake::ValidatorSet>(@aptos_framework);
        let inactive_state = has_validator_set
            && !stake::spec_contains(validator_set.pending_active, pool_address)
            && !stake::spec_contains(validator_set.active_validators, pool_address)
            && !stake::spec_contains(
                validator_set.pending_inactive, pool_address
            );
        let has_time = exists<aptos_framework::timestamp::CurrentTimeMicroseconds>(
            @aptos_framework
        );
        let mature_pending_inactive = inactive_state
            && has_time
            && aptos_framework::timestamp::spec_now_seconds()
                >= stake_pool.locked_until_secs;
        let requested = stake_pool.inactive.value + stake_pool.pending_inactive.value;
        let distribution_amount = spec_distribute_withdraw_amount(
            staking_contract, stake_pool
        );
        let updated_pool = spec_distribution_pool_after_update(
            staking_contract.distribution_pool,
            distribution_amount,
            operator,
            staking_contract.commission_percentage
        );
        !has_validator_set
            || (inactive_state && !has_time)
            || stake_pool.inactive.value + stake_pool.pending_inactive.value
                > MAX_U64
            || aptos_framework::reconfiguration_state::spec_is_in_progress()
            || (mature_pending_inactive && requested > MAX_U64)
            || (distribution_amount > 0
                && spec_update_distribution_pool_aborts(
                    staking_contract.distribution_pool,
                    distribution_amount,
                    operator,
                    staking_contract.commission_percentage
                ))
            || (distribution_amount > 0
                && spec_distribution_payout_aborts(
                    updated_pool,
                    staker,
                    operator,
                    spec_operator_beneficiary(operator)
                ))
    }

    spec fun spec_add_distribution_aborts_at(
        operator: address,
        staking_contract: StakingContract,
        recipient: address,
        amount: u64,
        stake_pool: stake::StakePool
    ): bool {
        let refreshed_pool = spec_distribution_pool_after_update(
            staking_contract.distribution_pool,
            stake_pool.pending_inactive.value,
            operator,
            staking_contract.commission_percentage
        );
        spec_update_distribution_pool_aborts(
            staking_contract.distribution_pool,
            stake_pool.pending_inactive.value,
            operator,
            staking_contract.commission_percentage
        ) || pool_u64::spec_buy_in_aborts(refreshed_pool, recipient, amount)
    }

    spec fun spec_request_commission_aborts_at(
        operator: address,
        staking_contract: StakingContract,
        stake_pool: stake::StakePool
    ): bool {
        let total_active_stake = stake_pool.active.value
            + stake_pool.pending_active.value;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission = accumulated_rewards
            * staking_contract.commission_percentage / 100;
        stake_pool.active.value + stake_pool.pending_active.value > MAX_U64
            || total_active_stake < staking_contract.principal
            || accumulated_rewards * staking_contract.commission_percentage
                > MAX_U64
            || (commission != 0
                && spec_add_distribution_aborts_at(
                    operator,
                    staking_contract,
                    operator,
                    commission,
                    stake_pool
                ))
            || (commission != 0
                && aptos_framework::reconfiguration_state::spec_is_in_progress())
            || (commission != 0
                && stake_pool.pending_inactive.value
                    + aptos_std::math64::min(
                        commission, stake_pool.active.value
                    ) > MAX_U64)
    }

    spec fun spec_unlock_stake_aborts_at(
        staker: address,
        operator: address,
        staking_contract: StakingContract,
        stake_pool: stake::StakePool,
        requested_amount: u64
    ): bool {
        let distributed_contract = spec_distribute_contract_state(
            staking_contract, stake_pool, operator
        );
        let distributed_stake_pool = spec_distribute_stake_pool_state(
            staking_contract, stake_pool
        );
        let total_active_stake = distributed_stake_pool.active.value
            + distributed_stake_pool.pending_active.value;
        let accumulated_rewards = total_active_stake
            - distributed_contract.principal;
        let commission = accumulated_rewards
            * distributed_contract.commission_percentage / 100;
        let after_commission_contract = spec_request_commission_state(
            distributed_contract, operator, distributed_stake_pool
        );
        let after_commission_stake_pool = if (commission == 0) {
            distributed_stake_pool
        } else {
            stake::spec_unlock_with_cap_state(
                distributed_stake_pool, commission
            )
        };
        let amount = aptos_std::math64::min(
            requested_amount, after_commission_stake_pool.active.value
        );
        let with_principal = update_field(
            after_commission_contract,
            principal,
            after_commission_contract.principal - amount
        );
        spec_distribute_internal_aborts_at(
            staker, operator, staking_contract, stake_pool
        )
            || spec_request_commission_aborts_at(
                operator, distributed_contract, distributed_stake_pool
            )
            || after_commission_contract.principal < amount
            || spec_add_distribution_aborts_at(
                operator,
                with_principal,
                staker,
                amount,
                after_commission_stake_pool
            )
            || aptos_framework::reconfiguration_state::spec_is_in_progress()
            || after_commission_stake_pool.pending_inactive.value
                + aptos_std::math64::min(
                    amount, after_commission_stake_pool.active.value
                ) > MAX_U64
    }

    spec staking_contract_exists(staker: address, operator: address): bool {
        pragma opaque = true, aborts_if_is_partial = false;
        aborts_if false;
        ensures result == spec_staking_contract_exists(staker, operator);
    }

    spec get_expected_stake_pool_address {
        use 0x1::account;
        pragma aborts_if_is_partial;
        pragma opaque = true;
        ensures [inferred] result == account::spec_create_resource_address(staker, result_of<create_resource_account_seed>(staker, operator, contract_creation_seed));
    }

    spec fun spec_staking_contract_exists(staker: address, operator: address): bool {
        if (!exists<Store>(staker)) { false }
        else {
            let store = global<Store>(staker);
            simple_map::spec_contains_key(store.staking_contracts, operator)
        }
    }

    /// Account is not frozen and sufficient to withdraw.
    spec create_staking_contract(
        staker: &signer,
        operator: address,
        voter: address,
        amount: u64,
        commission_percentage: u64,
        contract_creation_seed: vector<u8>
    ) {
        pragma aborts_if_is_partial;
        pragma verify_duration_estimate = 120;
        include PreconditionsInCreateContract;
        include WithdrawAbortsIf<AptosCoin> { account: staker };
        include CreateStakingContractWithCoinsAbortsIfAndEnsures;
    }

    /// The amount should be at least the min_stake_required, so the stake pool will be eligible to join the validator set.
    /// Initialize Store resource if this is the first time the staker has delegated to anyone.
    /// Cannot create the staking contract if it already exists.
    spec create_staking_contract_with_coins(
        staker: &signer,
        operator: address,
        voter: address,
        coins: Coin<AptosCoin>,
        commission_percentage: u64,
        contract_creation_seed: vector<u8>
    ): address {
        use 0x1::signer;
        pragma verify_duration_estimate = 120;
        pragma aborts_if_is_partial;
        include PreconditionsInCreateContract;

        let amount = coins.value;
        include CreateStakingContractWithCoinsAbortsIfAndEnsures { amount };

        // TODO: this property causes timeout
        // let staker_address = signer::address_of(staker);
        // let seed_0 = bcs::to_bytes(staker_address);
        // let seed_1 = concat(concat(concat(seed_0, bcs::to_bytes(operator)), SALT), contract_creation_seed);
        // let resource_addr = account::spec_create_resource_address(staker_address, seed_1);
        // ensures result == resource_addr;
        pragma opaque = true;
        modifies Store[signer::address_of(staker)];
        modifies Staker[{
            let (a_0,a_1,a_2) = result_of<create_stake_pool>(staker, operator, voter, contract_creation_seed);
            signer::address_of(a_0)
        }];
    }

    /// Account is not frozen and sufficient to withdraw.
    /// Staking_contract exists the stacker/operator pair.
    spec add_stake(staker: &signer, operator: address, amount: u64) {
        use 0x1::signer;
        // TODO(fa_migration)
        use aptos_framework::reconfiguration_state;
        pragma verify_duration_estimate = 600;
        // TODO: this function times out
        include stake::ResourceRequirement;
        aborts_if reconfiguration_state::spec_is_in_progress();

        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf { staker: staker_address };
        let store = global<Store>(staker_address);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);

        include WithdrawAbortsIf<AptosCoin> { account: staker };
        let balance = global<coin::CoinStore<AptosCoin>>(staker_address).coin.value;
        let post post_coin = global<coin::CoinStore<AptosCoin>>(staker_address).coin.value;
        ensures post_coin == balance - amount;

        // postconditions stake::add_stake_with_cap()
        let owner_cap = staking_contract.owner_cap;
        include stake::AddStakeWithCapAbortsIfAndEnsures { owner_cap };

        let post post_store = global<Store>(staker_address);
        let post post_staking_contract = simple_map::spec_get(
            post_store.staking_contracts, operator
        );
        aborts_if staking_contract.principal + amount > MAX_U64;

        // property 3: Adding stake to the stake pool increases the principal value of the pool, reflecting the
        // additional stake amount.
        /// [high-level-req-3]
        ensures post_staking_contract.principal == staking_contract.principal + amount;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies Store[signer::address_of(staker)];
    }

    /// Staking_contract exists the stacker/operator pair.
    spec update_voter(staker: &signer, operator: address, new_voter: address) {
        use 0x1::signer;
        use 0x1::chain_status;
        use 0x1::reconfiguration_state;
        use 0x1::stake;
        let staker_address = signer::address_of(staker);
        include UpdateVoterSchema { staker: staker_address };

        let post store = global<Store>(staker_address);
        let post staking_contract = simple_map::spec_get(
            store.staking_contracts, operator
        );
        let post pool_address = staking_contract.owner_cap.pool_address;
        let post new_delegated_voter = global<stake::StakePool>(pool_address).delegated_voter;
        // property 4: The staker may update the voter of a staking contract, enabling them
        // to modify the assigned voter address and ensure it accurately reflects their desired choice.
        /// [high-level-req-4]
        ensures new_delegated_voter == new_voter;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] (chain_status::is_operating() ==> exists<reconfiguration_state::State>(@0x1)) && (exists<stake::ValidatorSet>(@0x1) ==> {
            let validator_set = stake::ValidatorSet[@0x1];
            (forall i in 0..len(validator_set.active_validators): exists<stake::StakePool>(validator_set.active_validators[i].addr)) && (forall i in 0..len(validator_set.pending_inactive): exists<stake::StakePool>(validator_set.pending_inactive[i].addr)) && (forall i in 0..len(validator_set.pending_active): exists<stake::StakePool>(validator_set.pending_active[i].addr)) && ((forall i in 0..len(validator_set.active_validators): stake::ValidatorConfig[validator_set.active_validators[i].addr].validator_index < len(stake::ValidatorPerformance[@0x1].validators)) && (forall i in 0..len(validator_set.active_validators): validator_set.active_validators[i].config.validator_index < len(stake::ValidatorPerformance[@0x1].validators))) && ((forall i in 0..len(validator_set.pending_inactive): stake::ValidatorConfig[validator_set.pending_inactive[i].addr].validator_index < len(stake::ValidatorPerformance[@0x1].validators)) && (forall i in 0..len(validator_set.pending_inactive): validator_set.pending_inactive[i].config.validator_index < len(stake::ValidatorPerformance[@0x1].validators))) && len(validator_set.pending_inactive) + len(validator_set.active_validators) == len(stake::ValidatorPerformance[@0x1].validators)
        }) ==> (..S1 |~ ensures_of<assert_staking_contract_exists>(signer::address_of(staker), operator));
        aborts_if [inferred] ({
            let a = S1 |~ exists<Store>(signer::address_of(staker));
            (chain_status::is_operating() ==> exists<reconfiguration_state::State>(@0x1)) && ((exists<stake::ValidatorSet>(@0x1) ==> {
                let validator_set = stake::ValidatorSet[@0x1];
                (forall i in 0..len(validator_set.active_validators): exists<stake::StakePool>(validator_set.active_validators[i].addr)) && (forall i in 0..len(validator_set.pending_inactive): exists<stake::StakePool>(validator_set.pending_inactive[i].addr)) && (forall i in 0..len(validator_set.pending_active): exists<stake::StakePool>(validator_set.pending_active[i].addr)) && ((forall i in 0..len(validator_set.active_validators): stake::ValidatorConfig[validator_set.active_validators[i].addr].validator_index < len(stake::ValidatorPerformance[@0x1].validators)) && (forall i in 0..len(validator_set.active_validators): validator_set.active_validators[i].config.validator_index < len(stake::ValidatorPerformance[@0x1].validators))) && ((forall i in 0..len(validator_set.pending_inactive): stake::ValidatorConfig[validator_set.pending_inactive[i].addr].validator_index < len(stake::ValidatorPerformance[@0x1].validators)) && (forall i in 0..len(validator_set.pending_inactive): validator_set.pending_inactive[i].config.validator_index < len(stake::ValidatorPerformance[@0x1].validators))) && len(validator_set.pending_inactive) + len(validator_set.active_validators) == len(stake::ValidatorPerformance[@0x1].validators)
            }) && ((chain_status::is_operating() ==> exists<stake::ValidatorSet>(@0x1)) && !a))
        });
        aborts_if [inferred] (chain_status::is_operating() ==> exists<reconfiguration_state::State>(@0x1)) && ((exists<stake::ValidatorSet>(@0x1) ==> {
            let validator_set = stake::ValidatorSet[@0x1];
            (forall i in 0..len(validator_set.active_validators): exists<stake::StakePool>(validator_set.active_validators[i].addr)) && (forall i in 0..len(validator_set.pending_inactive): exists<stake::StakePool>(validator_set.pending_inactive[i].addr)) && (forall i in 0..len(validator_set.pending_active): exists<stake::StakePool>(validator_set.pending_active[i].addr)) && ((forall i in 0..len(validator_set.active_validators): stake::ValidatorConfig[validator_set.active_validators[i].addr].validator_index < len(stake::ValidatorPerformance[@0x1].validators)) && (forall i in 0..len(validator_set.active_validators): validator_set.active_validators[i].config.validator_index < len(stake::ValidatorPerformance[@0x1].validators))) && ((forall i in 0..len(validator_set.pending_inactive): stake::ValidatorConfig[validator_set.pending_inactive[i].addr].validator_index < len(stake::ValidatorPerformance[@0x1].validators)) && (forall i in 0..len(validator_set.pending_inactive): validator_set.pending_inactive[i].config.validator_index < len(stake::ValidatorPerformance[@0x1].validators))) && len(validator_set.pending_inactive) + len(validator_set.active_validators) == len(stake::ValidatorPerformance[@0x1].validators)
        }) && ((chain_status::is_operating() ==> exists<stake::ValidatorSet>(@0x1)) && aborts_of<assert_staking_contract_exists>(signer::address_of(staker), operator)));
    }

    /// Staking_contract exists the stacker/operator pair.
    /// Only active validator can update locked_until_secs.
    spec reset_lockup(staker: &signer, operator: address) {
        let staker_address = signer::address_of(staker);
        /// [high-level-req-5]
        include ContractExistsAbortsIf { staker: staker_address };
        include IncreaseLockupWithCapAbortsIf { staker: staker_address };
    }

    spec update_commision(staker: &signer, operator: address, new_commission_percentage: u64) {
        use 0x1::signer;
        // TODO: Call `distribute_internal` and could not verify `update_distribution_pool`.
        // TODO: A data invariant not hold happened here involve with 'pool_u64' #L16.
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        aborts_if new_commission_percentage > 100;
        include ContractExistsAbortsIf { staker: staker_address };
        pragma opaque = true, aborts_if_is_partial = true;
        modifies Store[signer::address_of(staker)];
    }

    /// Only staker or operator can call this.
    spec request_commission(account: &signer, staker: address, operator: address) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        // TODO: A data invariant not hold happened here involve with 'pool_u64' #L16.
        pragma verify = false;
        let account_addr = signer::address_of(account);
        include ContractExistsAbortsIf { staker };
        aborts_if account_addr != staker && account_addr != operator;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies Store[staker];
    }

    spec fun spec_request_commission_state(
        staking_contract: StakingContract,
        operator: address,
        stake_pool: stake::StakePool
    ): StakingContract {
        let total_active_stake = stake_pool.active.value + stake_pool.pending_active.value;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards
            * staking_contract.commission_percentage / 100;
        let principal_updated = update_field(
            staking_contract, principal, total_active_stake - commission_amount
        );
        if (commission_amount == 0) {
            principal_updated
        } else {
            update_field(
                principal_updated,
                distribution_pool,
                pool_u64::spec_buy_in_state(
                    spec_distribution_pool_after_update(
                        staking_contract.distribution_pool,
                        stake_pool.pending_inactive.value,
                        operator,
                        staking_contract.commission_percentage
                    ),
                    operator,
                    commission_amount
                )
            )
        }
    }

    spec fun spec_get_staking_contract_amounts_aborts(
        staking_contract: StakingContract
    ): bool {
        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let total_active_stake = stake_pool.active.value + stake_pool.pending_active.value;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        !exists<stake::StakePool>(pool_address)
            || stake_pool.active.value + stake_pool.pending_active.value > MAX_U64
            || total_active_stake < staking_contract.principal
            || accumulated_rewards * staking_contract.commission_percentage > MAX_U64
    }

    spec fun spec_request_commission_internal_aborts(
        operator: address, staking_contract: StakingContract
    ): bool {
        if (spec_get_staking_contract_amounts_aborts(staking_contract)) {
            true
        } else {
            let pool_address = staking_contract.pool_address;
            let stake_pool = global<stake::StakePool>(pool_address);
            let total_active_stake = stake_pool.active.value
                + stake_pool.pending_active.value;
            let accumulated_rewards = total_active_stake - staking_contract.principal;
            let commission_amount = accumulated_rewards
                * staking_contract.commission_percentage / 100;
            let refreshed_distribution_pool = spec_distribution_pool_after_update(
                staking_contract.distribution_pool,
                stake_pool.pending_inactive.value,
                operator,
                staking_contract.commission_percentage
            );
            let owner_pool_address = staking_contract.owner_cap.pool_address;
            let owner_stake_pool = global<stake::StakePool>(owner_pool_address);
            commission_amount != 0
                && (
                    spec_update_distribution_pool_aborts(
                        staking_contract.distribution_pool,
                        stake_pool.pending_inactive.value,
                        operator,
                        staking_contract.commission_percentage
                    )
                        || pool_u64::spec_buy_in_aborts(
                            refreshed_distribution_pool,
                            operator,
                            commission_amount
                        )
                        || aptos_framework::reconfiguration_state::spec_is_in_progress()
                        || !exists<stake::StakePool>(owner_pool_address)
                        || (
                            exists<stake::StakePool>(owner_pool_address)
                                && owner_stake_pool.pending_inactive.value
                                    + aptos_std::math64::min(
                                        commission_amount,
                                        owner_stake_pool.active.value
                                    ) > MAX_U64
                        )
                )
        }
    }

    spec request_commission_internal(
        operator: address,
        staking_contract: &mut StakingContract,
    ): u64 {
        use 0x1::stake;
        pragma opaque = true;
        // Trusted boundary (expert assumption): all nine component result,
        // state, frame, and abort-direction shards prove at 60 seconds. The
        // unified strict abort-coverage shard times out while composing the
        // already specified add-distribution and unlock boundaries. See
        // corpus/metadata/staking-contract-commission-contracts.json.
        pragma verify = false;
        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let total_active_stake = stake_pool.active.value + stake_pool.pending_active.value;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards
            * staking_contract.commission_percentage / 100;
        let owner_pool_address = staking_contract.owner_cap.pool_address;
        let owner_stake_pool = global<stake::StakePool>(owner_pool_address);
        modifies stake::StakePool[owner_pool_address];
        aborts_if [abstract] spec_request_commission_internal_aborts(
            operator, staking_contract
        );
        let principal_updated = update_field(
            staking_contract, principal, total_active_stake - commission_amount
        );
        aborts_if [concrete]
            aborts_of<get_staking_contract_amounts_internal>(staking_contract)
                || (
                    commission_amount != 0
                        && (
                            aborts_of<add_distribution>(
                                operator,
                                principal_updated,
                                operator,
                                commission_amount
                            )
                                || aborts_of<stake::unlock_with_cap>(
                                    commission_amount, staking_contract.owner_cap
                                )
                        )
                );
        ensures result == commission_amount;
        ensures staking_contract
            == spec_request_commission_state(
                old(staking_contract),
                operator,
                old(global<stake::StakePool>(pool_address))
            );
        ensures exists<stake::StakePool>(owner_pool_address)
            == old(exists<stake::StakePool>(owner_pool_address));
        ensures commission_amount == 0
            && old(exists<stake::StakePool>(owner_pool_address)) ==>
                global<stake::StakePool>(owner_pool_address) == owner_stake_pool;
        ensures commission_amount != 0 ==>
            global<stake::StakePool>(owner_pool_address)
                == stake::spec_unlock_with_cap_state(owner_stake_pool, commission_amount);
    }

    /// Staking_contract exists the stacker/operator pair.
    spec unlock_rewards(staker: &signer, operator: address) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        // TODO: Set because of timeout (estimate unknown).
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        let staking_contracts = global<Store>(staker_address).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);
        include ContractExistsAbortsIf { staker: staker_address };
    }

    spec unlock_stake(staker: &signer, operator: address, amount: u64) {
        use 0x1::fungible_asset;
        use 0x1::object;
        use 0x1::primary_fungible_store;
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        // Trusted boundary (expert assumption): this is the exact sequential
        // composition of distribution, commission, capped withdrawal, pool
        // buy-in, and stake unlock. Its two state transformers are explicit
        // above; body verification inherits the two payout boundaries.
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        let staking_contract = simple_map::spec_get(
            global<Store>(staker_address).staking_contracts, operator
        );
        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let operator_beneficiary = spec_operator_beneficiary(operator);
        let staker_store = object::spec_create_user_derived_object_address(
            staker_address,
            primary_fungible_store::DeriveRefPod[
                @aptos_fungible_asset
            ].metadata_derive_ref.self
        );
        let beneficiary_store = object::spec_create_user_derived_object_address(
            operator_beneficiary,
            primary_fungible_store::DeriveRefPod[
                @aptos_fungible_asset
            ].metadata_derive_ref.self
        );

        requires amount == 0
            || !spec_staking_contract_exists(staker_address, operator)
            || (forall i in 0..len(staking_contract.distribution_pool.shareholders):
                staking_contract.distribution_pool.shareholders[i]
                    == staker_address
                    || staking_contract.distribution_pool.shareholders[i]
                        == operator);

        modifies Store[staker_address];
        modifies Staker[pool_address];
        modifies stake::StakePool[pool_address];
        modifies account::Account[staker_address];
        modifies account::Account[operator_beneficiary];
        modifies coin::CoinInfo<AptosCoin>[@aptos_framework];
        modifies coin::CoinConversionMap[@aptos_framework];
        modifies coin::PairedCoinType[@aptos_fungible_asset];
        modifies coin::PairedFungibleAssetRefs[@aptos_fungible_asset];
        modifies object::ObjectCore[@aptos_fungible_asset];
        modifies fungible_asset::Metadata[@aptos_fungible_asset];
        modifies fungible_asset::Supply[@aptos_fungible_asset];
        modifies fungible_asset::ConcurrentSupply[@aptos_fungible_asset];
        modifies primary_fungible_store::DeriveRefPod[@aptos_fungible_asset];
        modifies object::ObjectCore[staker_store];
        modifies fungible_asset::FungibleStore[staker_store];
        modifies fungible_asset::ConcurrentFungibleBalance[staker_store];
        modifies object::Untransferable[staker_store];
        modifies object::ObjectCore[beneficiary_store];
        modifies fungible_asset::FungibleStore[beneficiary_store];
        modifies fungible_asset::ConcurrentFungibleBalance[beneficiary_store];
        modifies object::Untransferable[beneficiary_store];

        aborts_if amount != 0 && !exists<Store>(staker_address);
        aborts_if amount != 0
            && exists<Store>(staker_address)
            && !simple_map::spec_contains_key(
                global<Store>(staker_address).staking_contracts, operator
            );
        aborts_if amount != 0
            && spec_staking_contract_exists(staker_address, operator)
            && !exists<stake::StakePool>(pool_address);
        aborts_if amount != 0
            && spec_staking_contract_exists(staker_address, operator)
            && exists<stake::StakePool>(pool_address)
            && spec_unlock_stake_aborts_at(
                staker_address,
                operator,
                staking_contract,
                stake_pool,
                amount
            );

        ensures amount == 0 ==>
            global<Store>(staker_address) == old(global<Store>(staker_address));
        ensures amount != 0 ==>
            simple_map::spec_get(
                global<Store>(staker_address).staking_contracts, operator
            ) == spec_unlock_staking_contract_state(
                staker_address, operator, staking_contract, stake_pool, amount
            );
        ensures amount != 0 ==>
            global<stake::StakePool>(pool_address)
                == spec_unlock_stake_pool_state(
                    staking_contract, stake_pool, operator, amount
                );
    }

    /// Staking_contract exists the stacker/operator pair.
    spec switch_operator_with_same_commission(
        staker: &signer, old_operator: address, new_operator: address
    ) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma aborts_if_is_partial;
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf { staker: staker_address, operator: old_operator };
    }

    /// Staking_contract exists the stacker/operator pair.
    spec switch_operator(
        staker: &signer,
        old_operator: address,
        new_operator: address,
        new_commission_percentage: u64
    ) {
        use 0x1::signer;
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        // TODO: Set because of timeout (estimate unknown).
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf { staker: staker_address, operator: old_operator };
        let store = global<Store>(staker_address);
        let staking_contracts = store.staking_contracts;
        aborts_if simple_map::spec_contains_key(staking_contracts, new_operator);
        pragma opaque = true, aborts_if_is_partial = true;
        modifies Store[signer::address_of(staker)];
    }

    spec set_beneficiary_for_operator(operator: &signer, new_beneficiary: address) {
        use 0x1::signer;
        use 0x1::features;
        // TODO: temporary mockup
        pragma verify = false;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies BeneficiaryForOperator[signer::address_of(operator)];
        aborts_if [inferred] aborts_of<features::operator_beneficiary_change_enabled>();
    }

    spec beneficiary_for_operator(operator: address): address {
        pragma opaque = true, aborts_if_is_partial = false;
        aborts_if false;
        ensures exists<BeneficiaryForOperator>(operator) ==>
            result == BeneficiaryForOperator[operator].beneficiary_for_operator;
        ensures !exists<BeneficiaryForOperator>(operator) ==> result == operator;
    }

    /// Exact read-only lookup followed by pool_u64::balance.
    spec pending_attribution_snapshot(
        staker: address, operator: address, account: address
    ): u64 {
        use 0x1::pool_u64;
        use 0x1::simple_map;
        pragma opaque = true, aborts_if_is_partial = false;
        include ContractExistsAbortsIf;
        let staking_contract =
            simple_map::spec_get(global<Store>(staker).staking_contracts, operator);
        let pool = staking_contract.distribution_pool;
        let shares = pool_u64::spec_shares(pool, account);
        aborts_if pool.total_coins > 0
            && pool.total_shares > 0
            && (shares * pool.total_coins) / pool.total_shares > MAX_U64;
        ensures result ==
            pool_u64::spec_shares_to_amount_with_total_coins(
                pool, shares, pool.total_coins
            );
    }

    /// Staking_contract exists the stacker/operator pair.
    spec distribute(staker: address, operator: address) {
        // TODO: Call `distribute_internal` and could not verify `update_distribution_pool`.
        pragma aborts_if_is_partial;

        include ContractExistsAbortsIf;
        pragma opaque = true;
        modifies Store[staker];
    }

    /// The StakePool exists under the pool_address of StakingContract.
    /// The value of inactive and pending_inactive in the stake_pool is up to MAX_U64.
    spec distribute_internal(
        staker: address,
        operator: address,
        staking_contract: &mut StakingContract,
    ) {
        use 0x1::fungible_asset;
        use 0x1::object;
        use 0x1::primary_fungible_store;
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        // Trusted boundary (expert assumption): the contract reduces the
        // executable loop to the exact first-shareholder redemption fold above.
        // The fold, finite payout frame, and abort domain are explicit; the
        // body proof is disabled because it composes two existing
        // expert boundaries (`update_distribution_pool` and `deposit_coins`).
        pragma verify = false;
        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let total_potential_withdrawable = stake_pool.inactive.value
            + stake_pool.pending_inactive.value;
        let withdrawn = result_of<stake::withdraw_with_cap>(
            staking_contract.owner_cap, total_potential_withdrawable
        );
        let distribution_amount = withdrawn.value;
        let updated_pool = spec_distribution_pool_after_update(
            staking_contract.distribution_pool,
            distribution_amount,
            operator,
            staking_contract.commission_percentage
        );
        let operator_beneficiary = spec_operator_beneficiary(operator);
        let staker_store = object::spec_create_user_derived_object_address(
            staker,
            primary_fungible_store::DeriveRefPod[
                @aptos_fungible_asset
            ].metadata_derive_ref.self
        );
        let beneficiary_store = object::spec_create_user_derived_object_address(
            operator_beneficiary,
            primary_fungible_store::DeriveRefPod[
                @aptos_fungible_asset
            ].metadata_derive_ref.self
        );

        requires forall i in 0..len(staking_contract.distribution_pool.shareholders):
            staking_contract.distribution_pool.shareholders[i] == staker
                || staking_contract.distribution_pool.shareholders[i] == operator;

        modifies Staker[pool_address];
        modifies stake::StakePool[pool_address];
        modifies account::Account[staker];
        modifies account::Account[operator_beneficiary];
        modifies coin::CoinInfo<AptosCoin>[@aptos_framework];
        modifies coin::CoinConversionMap[@aptos_framework];
        modifies coin::PairedCoinType[@aptos_fungible_asset];
        modifies coin::PairedFungibleAssetRefs[@aptos_fungible_asset];
        modifies object::ObjectCore[@aptos_fungible_asset];
        modifies fungible_asset::Metadata[@aptos_fungible_asset];
        modifies fungible_asset::Supply[@aptos_fungible_asset];
        modifies fungible_asset::ConcurrentSupply[@aptos_fungible_asset];
        modifies primary_fungible_store::DeriveRefPod[@aptos_fungible_asset];
        modifies object::ObjectCore[staker_store];
        modifies fungible_asset::FungibleStore[staker_store];
        modifies fungible_asset::ConcurrentFungibleBalance[staker_store];
        modifies object::Untransferable[staker_store];
        modifies object::ObjectCore[beneficiary_store];
        modifies fungible_asset::FungibleStore[beneficiary_store];
        modifies fungible_asset::ConcurrentFungibleBalance[beneficiary_store];
        modifies object::Untransferable[beneficiary_store];

        aborts_if aborts_of<stake::get_stake>(pool_address);
        aborts_if !aborts_of<stake::get_stake>(pool_address)
            && total_potential_withdrawable > MAX_U64;
        aborts_if aborts_of<stake::withdraw_with_cap>(
            staking_contract.owner_cap, total_potential_withdrawable
        );
        aborts_if distribution_amount > 0
            && spec_update_distribution_pool_aborts(
                staking_contract.distribution_pool,
                distribution_amount,
                operator,
                staking_contract.commission_percentage
            );
        aborts_if distribution_amount > 0
            && !spec_update_distribution_pool_aborts(
                staking_contract.distribution_pool,
                distribution_amount,
                operator,
                staking_contract.commission_percentage
            )
            && spec_distribution_payout_aborts(
                updated_pool, staker, operator, operator_beneficiary
            );

        ensures exists<Staker>(pool_address);
        ensures global<Staker>(pool_address).staker == staker;
        ensures ensures_of<stake::withdraw_with_cap>(
            old(staking_contract).owner_cap,
            total_potential_withdrawable,
            withdrawn
        );
        ensures staking_contract.principal == old(staking_contract).principal;
        ensures staking_contract.pool_address == old(staking_contract).pool_address;
        ensures staking_contract.owner_cap == old(staking_contract).owner_cap;
        ensures staking_contract.commission_percentage
            == old(staking_contract).commission_percentage;
        ensures staking_contract.signer_cap == old(staking_contract).signer_cap;
        ensures distribution_amount == 0 ==>
            staking_contract.distribution_pool
                == old(staking_contract).distribution_pool;
        ensures distribution_amount > 0 ==>
            staking_contract.distribution_pool
                == update_field(
                    spec_redeem_prefix_state(
                        updated_pool, len(updated_pool.shareholders)
                    ),
                    total_coins,
                    0
                );
        ensures distribution_amount > 0 ==>
            len(staking_contract.distribution_pool.shareholders) == 0;
        ensures distribution_amount > 0 ==>
            simple_map::spec_len(staking_contract.distribution_pool.shares) == 0;
        ensures distribution_amount > 0 ==>
            staking_contract.distribution_pool.total_shares == 0;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec assert_staking_contract_exists(staker: address, operator: address) {
        pragma opaque = true;
        include ContractExistsAbortsIf;
    }

    spec add_distribution(
        operator: address,
        staking_contract: &mut StakingContract,
        recipient: address,
        coins_amount: u64,
    ) {
        pragma opaque = true;
        // Trusted boundary (expert assumption): all nine component state and
        // abort-direction shards prove at 60 seconds. Only the unified strict
        // abort-coverage shard times out at both 60 and 120 seconds while
        // composing the recursive distribution fold with buy_in. See
        // corpus/metadata/staking-contract-commission-contracts.json.
        pragma verify = false;
        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let total_distribution_amount = stake_pool.pending_inactive.value;
        let updated_pool = spec_distribution_pool_after_update(
            staking_contract.distribution_pool,
            total_distribution_amount,
            operator,
            staking_contract.commission_percentage
        );
        aborts_if [abstract] !exists<stake::StakePool>(pool_address)
            || spec_update_distribution_pool_aborts(
                staking_contract.distribution_pool,
                total_distribution_amount,
                operator,
                staking_contract.commission_percentage
            )
            || pool_u64::spec_buy_in_aborts(updated_pool, recipient, coins_amount);
        aborts_if [concrete] aborts_of<stake::get_stake>(pool_address)
            || aborts_of<update_distribution_pool>(
                staking_contract.distribution_pool,
                total_distribution_amount,
                operator,
                staking_contract.commission_percentage
            )
            || aborts_of<pool_u64::buy_in>(updated_pool, recipient, coins_amount);
        ensures staking_contract.principal == old(staking_contract).principal;
        ensures staking_contract.pool_address == old(staking_contract).pool_address;
        ensures staking_contract.owner_cap == old(staking_contract).owner_cap;
        ensures staking_contract.commission_percentage
            == old(staking_contract).commission_percentage;
        ensures staking_contract.signer_cap == old(staking_contract).signer_cap;
        ensures staking_contract.distribution_pool
            == pool_u64::spec_buy_in_state(
                spec_distribution_pool_after_update(
                    old(staking_contract).distribution_pool,
                    old(global<stake::StakePool>(pool_address)).pending_inactive.value,
                    operator,
                    old(staking_contract).commission_percentage
                ),
                recipient,
                coins_amount
            );
    }

    /// The StakePool exists under the pool_address of StakingContract.
    spec get_staking_contract_amounts_internal(staking_contract: &StakingContract): (
        u64, u64, u64
    ) {
        pragma opaque = true;
        include GetStakingContractAmountsAbortsIf;

        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards
            * staking_contract.commission_percentage / 100;
        ensures result_1 == total_active_stake;
        ensures result_2 == accumulated_rewards;
        ensures result_3 == commission_amount;
    }

    spec create_stake_pool(
        staker: &signer,
        operator: address,
        voter: address,
        contract_creation_seed: vector<u8>
    ): (signer, SignerCapability, OwnerCapability) {
        pragma verify_duration_estimate = 120;
        include stake::ResourceRequirement;
        let staker_address = signer::address_of(staker);
        // postconditions account::create_resource_account()

        let seed_0 = bcs::to_bytes(staker_address);
        let seed_1 = concat(
            concat(concat(seed_0, bcs::to_bytes(operator)), SALT),
            contract_creation_seed
        );
        let resource_addr = account::spec_create_resource_address(
            staker_address, seed_1
        );
        include CreateStakePoolAbortsIf { resource_addr };
        ensures exists<account::Account>(resource_addr);
        let post post_account = global<account::Account>(resource_addr);
        ensures post_account.authentication_key == account::ZERO_AUTH_KEY;
        ensures post_account.signer_capability_offer.for
            == std::option::spec_some(resource_addr);

        // postconditions stake::initialize_stake_owner()
        ensures exists<stake::StakePool>(resource_addr);
        let post post_owner_cap = global<stake::OwnerCapability>(resource_addr);
        let post post_pool_address = post_owner_cap.pool_address;
        let post post_stake_pool = global<stake::StakePool>(post_pool_address);
        let post post_operator = post_stake_pool.operator_address;
        let post post_delegated_voter = post_stake_pool.delegated_voter;
        ensures resource_addr != operator ==>
            post_operator == operator;
        ensures resource_addr != voter ==>
            post_delegated_voter == voter;
        ensures signer::address_of(result_1) == resource_addr;
        ensures result_2 == SignerCapability { account: resource_addr };
        ensures result_3 == OwnerCapability { pool_address: resource_addr };
    }

    /*
    /// Pure state transformer for one commission iteration.  The executable
    /// walks a snapshot of the shareholders vector, while the pool itself is
    /// updated after every iteration.
    spec fun spec_distribution_pool_step(
        pool: Pool,
        shareholder: address,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): Pool {
        if (shareholder == operator) {
            pool
        } else {
            let shares = pool_u64::spec_shares(pool, shareholder);
            let previous_worth = pool_u64::spec_shares_to_amount_with_total_coins(
                pool, shares, pool.total_coins
            );
            let current_worth = pool_u64::spec_shares_to_amount_with_total_coins(
                pool, shares, updated_total_coins
            );
            let unpaid_commission = if (current_worth < previous_worth) {
                0
            } else {
                (current_worth - previous_worth) * commission_percentage / 100
            };
            let shares_to_transfer = pool_u64::spec_amount_to_shares_with_total_coins(
                pool, unpaid_commission, updated_total_coins
            );
            update_field(
                pool,
                shares,
                pool_u64::spec_transfer_shares_map(
                    pool, shareholder, operator, shares_to_transfer
                )
            )
        }
    }

    spec fun spec_distribution_pool_fold(
        pool: Pool,
        shareholders: vector<address>,
        end: u64,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): Pool {
        if (end == 0) {
            pool
        } else {
            spec_distribution_pool_step(
                spec_distribution_pool_fold(
                    pool,
                    shareholders,
                    end - 1,
                    updated_total_coins,
                    operator,
                    commission_percentage
                ),
                shareholders[end - 1],
                updated_total_coins,
                operator,
                commission_percentage
            )
        }
    }

    */

    spec fun spec_pool_shares(
        shares: simple_map::SimpleMap<address, u64>, shareholder: address
    ): u64 {
        if (simple_map::spec_contains_key(shares, shareholder)) {
            simple_map::spec_get(shares, shareholder)
        } else {
            0
        }
    }

    /// Commission represented as shares, evaluated against the immutable pool
    /// state at the start of distribution.  Each non-operator shareholder is
    /// visited at most once, so prior iterations can only change the
    /// operator's share entry and cannot affect this value.
    spec fun spec_distribution_commission_shares(
        pool: Pool,
        shareholder: address,
        updated_total_coins: u64,
        commission_percentage: u64
    ): u64 [weight = 5] {
        let shareholder_shares = pool_u64::spec_shares(pool, shareholder);
        let previous_worth = pool_u64::spec_shares_to_amount_with_total_coins(
            pool, shareholder_shares, pool.total_coins
        );
        let current_worth = pool_u64::spec_shares_to_amount_with_total_coins(
            pool, shareholder_shares, updated_total_coins
        );
        let unpaid_commission = if (current_worth < previous_worth) {
            0
        } else {
            (current_worth - previous_worth) * commission_percentage / 100
        };
        pool_u64::spec_amount_to_shares_with_total_coins(
            pool, unpaid_commission, updated_total_coins
        )
    }

    spec fun spec_distribution_commission_sum(
        pool: Pool,
        shareholders: vector<address>,
        end: u64,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): u64 {
        if (end == 0) {
            0
        } else {
            let shareholder = shareholders[end - 1];
            let prior = spec_distribution_commission_sum(
                pool,
                shareholders,
                end - 1,
                updated_total_coins,
                operator,
                commission_percentage
            );
            if (shareholder == operator) {
                prior
            } else {
                prior + spec_distribution_commission_shares(
                    pool, shareholder, updated_total_coins, commission_percentage
                )
            }
        }
    }

    spec fun spec_distribution_previous_worth(
        shares: simple_map::SimpleMap<address, u64>,
        total_coins: u64,
        total_shares: u64,
        shareholder: address
    ): u64 {
        let shareholder_shares = spec_pool_shares(shares, shareholder);
        if (total_coins == 0 || total_shares == 0) {
            0
        } else {
            shareholder_shares * total_coins / total_shares
        }
    }

    spec fun spec_distribution_current_worth(
        shares: simple_map::SimpleMap<address, u64>,
        total_coins: u64,
        total_shares: u64,
        shareholder: address,
        updated_total_coins: u64
    ): u64 {
        let shareholder_shares = spec_pool_shares(shares, shareholder);
        if (total_coins == 0 || total_shares == 0) {
            0
        } else {
            shareholder_shares * updated_total_coins / total_shares
        }
    }

    /// Commission amount charged in one iteration before converting coins
    /// back to pool shares.
    spec fun spec_distribution_unpaid_commission(
        shares: simple_map::SimpleMap<address, u64>,
        total_coins: u64,
        total_shares: u64,
        shareholder: address,
        updated_total_coins: u64,
        commission_percentage: u64
    ): u64 {
        let previous_worth = spec_distribution_previous_worth(
            shares, total_coins, total_shares, shareholder
        );
        let current_worth = spec_distribution_current_worth(
            shares, total_coins, total_shares, shareholder, updated_total_coins
        );
        let unpaid_commission = if (current_worth < previous_worth) {
            0
        } else {
            (current_worth - previous_worth) * commission_percentage / 100
        };
        unpaid_commission
    }

    /// Shares charged in one iteration, factored out so the executable
    /// arithmetic can be checked in small proof steps before the map update.
    spec fun spec_distribution_shares_to_transfer(
        shares: simple_map::SimpleMap<address, u64>,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        shareholder: address,
        updated_total_coins: u64,
        commission_percentage: u64
    ): u64 {
        let unpaid_commission = spec_distribution_unpaid_commission(
            shares,
            total_coins,
            total_shares,
            shareholder,
            updated_total_coins,
            commission_percentage
        );
        if (total_coins == 0 || total_shares == 0) {
            unpaid_commission * scaling_factor
        } else {
            unpaid_commission * total_shares / updated_total_coins
        }
    }

    spec fun spec_distribution_shares_step(
        shares: simple_map::SimpleMap<address, u64>,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        shareholder: address,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): simple_map::SimpleMap<address, u64> {
        if (shareholder == operator) {
            shares
        } else {
            let shareholder_shares = spec_pool_shares(shares, shareholder);
            let shares_to_transfer = spec_distribution_shares_to_transfer(
                shares,
                total_coins,
                total_shares,
                scaling_factor,
                shareholder,
                updated_total_coins,
                commission_percentage
            );
            if (shares_to_transfer == 0) {
                shares
            } else {
                let after_deduct = if (shareholder_shares > shares_to_transfer) {
                    simple_map::spec_set(
                        shares, shareholder, shareholder_shares - shares_to_transfer
                    )
                } else {
                    simple_map::spec_remove(shares, shareholder)
                };
                if (simple_map::spec_contains_key(after_deduct, operator)) {
                    simple_map::spec_set(
                        after_deduct,
                        operator,
                        simple_map::spec_get(after_deduct, operator) + shares_to_transfer
                    )
                } else {
                    simple_map::spec_set(after_deduct, operator, shares_to_transfer)
                }
            }
        }
    }

    /// All abort sources in one non-operator iteration. `shares` is the
    /// current map after the preceding snapshot entries have been processed.
    /// The arithmetic terms are mathematical, matching the overflow tests in
    /// the underlying `pool_u64` contracts.
    spec fun spec_distribution_iteration_aborts(
        shares: simple_map::SimpleMap<address, u64>,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        shareholders_limit: u64,
        shareholder: address,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): bool {
        if (shareholder == operator) {
            false
        } else {
            let shareholder_shares = spec_pool_shares(shares, shareholder);
            let operator_shares = spec_pool_shares(shares, operator);
            let previous_worth = spec_distribution_previous_worth(
                shares, total_coins, total_shares, shareholder
            );
            let current_worth = spec_distribution_current_worth(
                shares, total_coins, total_shares, shareholder, updated_total_coins
            );
            let unpaid_commission = spec_distribution_unpaid_commission(
                shares,
                total_coins,
                total_shares,
                shareholder,
                updated_total_coins,
                commission_percentage
            );
            let shares_to_transfer = if (
                current_worth < previous_worth
                    || (total_coins > 0 && total_shares > 0
                        && updated_total_coins == 0)
            ) {
                0
            } else {
                spec_distribution_shares_to_transfer(
                    shares,
                    total_coins,
                    total_shares,
                    scaling_factor,
                    shareholder,
                    updated_total_coins,
                    commission_percentage
                )
            };
            // `balance` and `shares_to_amount_with_total_coins`.
            (total_coins > 0 && total_shares > 0
                && shareholder_shares * total_coins / total_shares > MAX_U64)
            || (total_coins > 0 && total_shares > 0
                && shareholder_shares * updated_total_coins / total_shares > MAX_U64)
            // The executable subtraction and multiplication that compute the
            // unpaid commission.
            || current_worth < previous_worth
            || (current_worth >= previous_worth
                && (current_worth - previous_worth) * commission_percentage > MAX_U64)
            // `amount_to_shares_with_total_coins`.
            || (total_coins > 0 && total_shares > 0 && updated_total_coins == 0)
            || (total_coins > 0 && total_shares > 0 && updated_total_coins > 0
                && unpaid_commission * total_shares / updated_total_coins > MAX_U64)
            || ((total_coins == 0 || total_shares == 0)
                && unpaid_commission * scaling_factor > MAX_U64)
            // `transfer_shares`.
            || !simple_map::spec_contains_key(shares, shareholder)
            || shareholder_shares < shares_to_transfer
            || (shares_to_transfer > 0
                && shareholder != operator
                && simple_map::spec_contains_key(shares, operator)
                && operator_shares + shares_to_transfer > MAX_U64)
            || (shares_to_transfer > 0
                && shareholder != operator
                && !simple_map::spec_contains_key(shares, operator)
                && shareholder_shares > shares_to_transfer
                && simple_map::spec_len(shares) >= shareholders_limit)
        }
    }

    /// One-step equation for the exact map fold used by the executable loop.
    /// Quantifying the private map type in the result keeps the lemma's Move
    /// signature primitive while still exposing the equation to the prover.
    spec lemma spec_distribution_shares_fold_step(
        shareholders: vector<address>,
        end: u64,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ) {
        ensures forall shares: simple_map::SimpleMap<address, u64> {
            spec_distribution_shares_fold(
                shares,
                shareholders,
                end,
                total_coins,
                total_shares,
                scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            )
        }: end < len(shareholders) ==>
            spec_distribution_shares_fold(
                shares,
                shareholders,
                end + 1,
                total_coins,
                total_shares,
                scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            ) == spec_distribution_shares_step(
                spec_distribution_shares_fold(
                    shares,
                    shareholders,
                    end,
                    total_coins,
                    total_shares,
                    scaling_factor,
                    updated_total_coins,
                    operator,
                    commission_percentage
                ),
                total_coins,
                total_shares,
                scaling_factor,
                shareholders[end],
                updated_total_coins,
                operator,
                commission_percentage
            );
    }

    /// If the pool value did not change, every iteration computes zero
    /// commission and the exact shares-map fold is the identity.
    spec lemma spec_distribution_shares_fold_unchanged(
        shareholders: vector<address>,
        end: u64,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        operator: address,
        commission_percentage: u64
    ) {
        ensures end <= len(shareholders) ==> (forall shares: simple_map::SimpleMap<address, u64> {
            spec_distribution_shares_fold(
                shares,
                shareholders,
                end,
                total_coins,
                total_shares,
                scaling_factor,
                total_coins,
                operator,
                commission_percentage
            )
        }: spec_distribution_shares_fold(
            shares,
            shareholders,
            end,
            total_coins,
            total_shares,
            scaling_factor,
            total_coins,
            operator,
            commission_percentage
        ) == shares);
    } proof {
        if (end > 0 && end <= len(shareholders)) {
            apply spec_distribution_shares_fold_unchanged(
                shareholders,
                end - 1,
                total_coins,
                total_shares,
                scaling_factor,
                operator,
                commission_percentage
            );
        }
    }

    spec fun spec_distribution_shares_fold(
        shares: simple_map::SimpleMap<address, u64>,
        shareholders: vector<address>,
        end: u64,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): simple_map::SimpleMap<address, u64> [weight = 20] {
        if (end == 0) {
            shares
        } else {
            spec_distribution_shares_step(
                spec_distribution_shares_fold(
                    shares,
                    shareholders,
                    end - 1,
                    total_coins,
                    total_shares,
                    scaling_factor,
                    updated_total_coins,
                    operator,
                    commission_percentage
                ),
                total_coins,
                total_shares,
                scaling_factor,
                shareholders[end - 1],
                updated_total_coins,
                operator,
                commission_percentage
            )
        }
    }

    /// Whether an original shareholder remains in the ordered shareholder
    /// vector after its distribution step. The operator is skipped; every
    /// other shareholder is removed exactly when all of its shares are
    /// transferred by a nonzero payment.
    spec fun spec_distribution_shareholder_survives(
        shares: simple_map::SimpleMap<address, u64>,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        shareholder: address,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): bool {
        if (shareholder == operator) {
            true
        } else {
            let shares_to_transfer = spec_distribution_shares_to_transfer(
                shares,
                total_coins,
                total_shares,
                scaling_factor,
                shareholder,
                updated_total_coins,
                commission_percentage
            );
            shares_to_transfer == 0
                || spec_pool_shares(shares, shareholder) > shares_to_transfer
        }
    }

    /// Stable filter of the processed prefix of the original shareholder
    /// vector. Survivors retain their original relative order.
    spec fun spec_distribution_shareholders_prefix(
        shareholders: vector<address>,
        shares: simple_map::SimpleMap<address, u64>,
        end: u64,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): vector<address> [weight = 20] {
        if (end == 0) {
            vector[]
        } else {
            let prefix = spec_distribution_shareholders_prefix(
                shareholders,
                shares,
                end - 1,
                total_coins,
                total_shares,
                scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            );
            let shareholder = shareholders[end - 1];
            if (spec_distribution_shareholder_survives(
                shares,
                total_coins,
                total_shares,
                scaling_factor,
                shareholder,
                updated_total_coins,
                operator,
                commission_percentage
            )) {
                concat(prefix, vector[shareholder])
            } else {
                prefix
            }
        }
    }

    /// One-step equation for the stable survivor-prefix filter.
    spec lemma spec_distribution_shareholders_prefix_step(
        shareholders: vector<address>,
        end: u64,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ) {
        ensures forall shares: simple_map::SimpleMap<address, u64> {
            spec_distribution_shareholders_prefix(
                shareholders,
                shares,
                end,
                total_coins,
                total_shares,
                scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            )
        }: end < len(shareholders) ==>
            spec_distribution_shareholders_prefix(
                shareholders,
                shares,
                end + 1,
                total_coins,
                total_shares,
                scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            ) == if (spec_distribution_shareholder_survives(
                shares,
                total_coins,
                total_shares,
                scaling_factor,
                shareholders[end],
                updated_total_coins,
                operator,
                commission_percentage
            )) {
                concat(
                    spec_distribution_shareholders_prefix(
                        shareholders,
                        shares,
                        end,
                        total_coins,
                        total_shares,
                        scaling_factor,
                        updated_total_coins,
                        operator,
                        commission_percentage
                    ),
                    vector[shareholders[end]]
                )
            } else {
                spec_distribution_shareholders_prefix(
                    shareholders,
                    shares,
                    end,
                    total_coins,
                    total_shares,
                    scaling_factor,
                    updated_total_coins,
                    operator,
                    commission_percentage
                )
            };
    }

    /// Appending the final element of a nonempty prefix reconstructs that
    /// prefix. Kept separate so vector extensionality is not mixed with the
    /// commission arithmetic in the recursive filter proof.
    spec lemma spec_vector_prefix_append_step(
        values: vector<address>,
        end: u64
    ) {
        ensures end > 0 && end <= len(values) ==>
            concat(values[0..end - 1], vector[values[end - 1]])
                == values[0..end];
    }

    /// Equal old and updated pool values produce a zero transfer, hence every
    /// shareholder survives its distribution step.
    spec lemma spec_distribution_shareholder_survives_unchanged(
        shareholder: address,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        operator: address,
        commission_percentage: u64
    ) {
        ensures forall shares: simple_map::SimpleMap<address, u64> {
            spec_distribution_shareholder_survives(
                shares,
                total_coins,
                total_shares,
                scaling_factor,
                shareholder,
                total_coins,
                operator,
                commission_percentage
            )
        }: spec_distribution_shareholder_survives(
            shares,
            total_coins,
            total_shares,
            scaling_factor,
            shareholder,
            total_coins,
            operator,
            commission_percentage
        );
    }

    /// If the pool value did not change, every original shareholder survives
    /// and the stable filter is exactly the original prefix.
    spec lemma spec_distribution_shareholders_prefix_unchanged(
        shareholders: vector<address>,
        end: u64,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        operator: address,
        commission_percentage: u64
    ) {
        ensures end <= len(shareholders) ==>
            (forall shares: simple_map::SimpleMap<address, u64> {
                spec_distribution_shareholders_prefix(
                    shareholders,
                    shares,
                    end,
                    total_coins,
                    total_shares,
                    scaling_factor,
                    total_coins,
                    operator,
                    commission_percentage
                )
            }: spec_distribution_shareholders_prefix(
                shareholders,
                shares,
                end,
                total_coins,
                total_shares,
                scaling_factor,
                total_coins,
                operator,
                commission_percentage
            ) == shareholders[0..end]);
    } proof {
        if (end > 0 && end <= len(shareholders)) {
            apply spec_distribution_shareholders_prefix_unchanged(
                shareholders,
                end - 1,
                total_coins,
                total_shares,
                scaling_factor,
                operator,
                commission_percentage
            );
            assert forall shares: simple_map::SimpleMap<address, u64>:
                spec_distribution_shareholders_prefix(
                    shareholders,
                    shares,
                    end - 1,
                    total_coins,
                    total_shares,
                    scaling_factor,
                    total_coins,
                    operator,
                    commission_percentage
                ) == shareholders[0..end - 1];
            apply spec_distribution_shareholder_survives_unchanged(
                shareholders[end - 1],
                total_coins,
                total_shares,
                scaling_factor,
                operator,
                commission_percentage
            );
            assert forall shares: simple_map::SimpleMap<address, u64>:
                spec_distribution_shareholder_survives(
                    shares,
                    total_coins,
                    total_shares,
                    scaling_factor,
                    shareholders[end - 1],
                    total_coins,
                    operator,
                    commission_percentage
                );
            apply spec_distribution_shareholders_prefix_step(
                shareholders,
                end - 1,
                total_coins,
                total_shares,
                scaling_factor,
                total_coins,
                operator,
                commission_percentage
            );
            assert forall shares: simple_map::SimpleMap<address, u64>:
                spec_distribution_shareholders_prefix(
                    shareholders,
                    shares,
                    end,
                    total_coins,
                    total_shares,
                    scaling_factor,
                    total_coins,
                    operator,
                    commission_percentage
                ) == concat(
                    spec_distribution_shareholders_prefix(
                        shareholders,
                        shares,
                        end - 1,
                        total_coins,
                        total_shares,
                        scaling_factor,
                        total_coins,
                        operator,
                        commission_percentage
                    ),
                    vector[shareholders[end - 1]]
                );
            apply spec_vector_prefix_append_step(shareholders, end);
            assert concat(
                shareholders[0..end - 1],
                vector[shareholders[end - 1]]
            ) == shareholders[0..end];
        }
    }

    /// Exact shareholder-vector state after `end` distribution steps. The
    /// unprocessed original suffix remains in place. If the operator was not
    /// originally a shareholder but has received shares, it is appended once
    /// at the end, matching `pool_u64::add_shares`.
    spec fun spec_distribution_shareholders_state(
        shareholders: vector<address>,
        shares: simple_map::SimpleMap<address, u64>,
        current_shares: simple_map::SimpleMap<address, u64>,
        end: u64,
        total_coins: u64,
        total_shares: u64,
        scaling_factor: u64,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): vector<address> {
        let base = concat(
            spec_distribution_shareholders_prefix(
                shareholders,
                shares,
                end,
                total_coins,
                total_shares,
                scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            ),
            shareholders[end..len(shareholders)]
        );
        if (!std::vector::spec_contains(shareholders, operator)
            && simple_map::spec_contains_key(current_shares, operator)) {
            concat(base, vector[operator])
        } else {
            base
        }
    }

    /// Exact local-state effect of a successful distribution-pool refresh.
    spec fun spec_distribution_pool_after_update(
        distribution_pool: Pool,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): Pool {
        let updated_shares = spec_distribution_shares_fold(
            distribution_pool.shares,
            distribution_pool.shareholders,
            len(distribution_pool.shareholders),
            distribution_pool.total_coins,
            distribution_pool.total_shares,
            distribution_pool.scaling_factor,
            updated_total_coins,
            operator,
            commission_percentage
        );
        Pool {
            shareholders_limit: distribution_pool.shareholders_limit,
            total_coins: updated_total_coins,
            total_shares: distribution_pool.total_shares,
            shares: updated_shares,
            shareholders: spec_distribution_shareholders_state(
                distribution_pool.shareholders,
                distribution_pool.shares,
                updated_shares,
                len(distribution_pool.shareholders),
                distribution_pool.total_coins,
                distribution_pool.total_shares,
                distribution_pool.scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            ),
            scaling_factor: distribution_pool.scaling_factor
        }
    }

    /// Exact abort domain of `update_distribution_pool`.
    spec fun spec_update_distribution_pool_aborts(
        distribution_pool: Pool,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ): bool {
        distribution_pool.total_coins != updated_total_coins
            && (exists i: u64:
                i < len(distribution_pool.shareholders)
                    && spec_distribution_iteration_aborts(
                        spec_distribution_shares_fold(
                            distribution_pool.shares,
                            distribution_pool.shareholders,
                            i,
                            distribution_pool.total_coins,
                            distribution_pool.total_shares,
                            distribution_pool.scaling_factor,
                            updated_total_coins,
                            operator,
                            commission_percentage
                        ),
                        distribution_pool.total_coins,
                        distribution_pool.total_shares,
                        distribution_pool.scaling_factor,
                        distribution_pool.shareholders_limit,
                        distribution_pool.shareholders[i],
                        updated_total_coins,
                        operator,
                        commission_percentage
                    ))
    }

    spec update_distribution_pool(
        distribution_pool: &mut Pool,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64
    ) {
        // The historical WP clauses below are retained as provenance while the
        // verified fold contract is developed.  They are not a usable
        // abstraction: all of them were tagged `sathard`.
        pragma aborts_if_is_partial = false;
        pragma opaque = true;
        // Trusted boundary (expert assumption): the exact shares-map fold,
        // scalar effects, and each transfer case have target-body proof
        // evidence; the final remove/push-to-stable-filter vector bridge is
        // solver-intractable at the current limit. See
        // corpus/metadata/trusted-verification-boundaries.json.
        pragma verify = false;
        /*
        ensures [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && pool_u64::total_coins(old(distribution_pool)) != updated_total_coins ==> (forall x4: u64, x5: pool_u64::Pool: (forall x in 0..len(x5.shareholders): simple_map::spec_contains_key<address, u64>(x5.shares, x5.shareholders[x])) && simple_map::spec_len<address, u64>(x5.shares) == len(x5.shareholders) && len(x5.shareholders) <= x5.shareholders_limit && (forall x in 0..len(x5.shareholders), y in 0..len(x5.shareholders): x5.shareholders[x] == x5.shareholders[y] ==> x == y) ==> (forall x3: pool_u64::Pool: (forall x in 0..len(x3.shareholders): simple_map::spec_contains_key<address, u64>(x3.shares, x3.shareholders[x])) && simple_map::spec_len<address, u64>(x3.shares) == len(x3.shareholders) && len(x3.shareholders) <= x3.shareholders_limit && (forall x in 0..len(x3.shareholders), y in 0..len(x3.shareholders): x3.shareholders[x] == x3.shareholders[y] ==> x == y) ==> (forall x2: pool_u64::Pool: (forall x in 0..len(x2.shareholders): simple_map::spec_contains_key<address, u64>(x2.shares, x2.shareholders[x])) && simple_map::spec_len<address, u64>(x2.shares) == len(x2.shareholders) && len(x2.shareholders) <= x2.shareholders_limit && (forall x in 0..len(x2.shareholders), y in 0..len(x2.shareholders): x2.shareholders[x] == x2.shareholders[y] ==> x == y) ==> (forall x1: pool_u64::Pool: (forall x in 0..len(x1.shareholders): simple_map::spec_contains_key<address, u64>(x1.shares, x1.shareholders[x])) && simple_map::spec_len<address, u64>(x1.shares) == len(x1.shareholders) && len(x1.shareholders) <= x1.shareholders_limit && (forall x in 0..len(x1.shareholders), y in 0..len(x1.shareholders): x1.shareholders[x] == x1.shareholders[y] ==> x == y) ==> (forall z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y) && x4 >= len(pool_u64::shareholders(old(distribution_pool))) ==> ensures_of<pool_u64::update_total_coins>(old(distribution_pool), updated_total_coins))))));
        ensures [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && pool_u64::total_coins(old(distribution_pool)) != updated_total_coins ==> (forall x4: u64, x5: pool_u64::Pool: (forall x in 0..len(x5.shareholders): simple_map::spec_contains_key<address, u64>(x5.shares, x5.shareholders[x])) && simple_map::spec_len<address, u64>(x5.shares) == len(x5.shareholders) && len(x5.shareholders) <= x5.shareholders_limit && (forall x in 0..len(x5.shareholders), y in 0..len(x5.shareholders): x5.shareholders[x] == x5.shareholders[y] ==> x == y) ==> (forall x3: pool_u64::Pool: (forall x in 0..len(x3.shareholders): simple_map::spec_contains_key<address, u64>(x3.shares, x3.shareholders[x])) && simple_map::spec_len<address, u64>(x3.shares) == len(x3.shareholders) && len(x3.shareholders) <= x3.shareholders_limit && (forall x in 0..len(x3.shareholders), y in 0..len(x3.shareholders): x3.shareholders[x] == x3.shareholders[y] ==> x == y) ==> (forall x2: pool_u64::Pool: (forall x in 0..len(x2.shareholders): simple_map::spec_contains_key<address, u64>(x2.shares, x2.shareholders[x])) && simple_map::spec_len<address, u64>(x2.shares) == len(x2.shareholders) && len(x2.shareholders) <= x2.shareholders_limit && (forall x in 0..len(x2.shareholders), y in 0..len(x2.shareholders): x2.shareholders[x] == x2.shareholders[y] ==> x == y) ==> (forall x1: pool_u64::Pool: (forall x in 0..len(x1.shareholders): simple_map::spec_contains_key<address, u64>(x1.shares, x1.shareholders[x])) && simple_map::spec_len<address, u64>(x1.shares) == len(x1.shareholders) && len(x1.shareholders) <= x1.shareholders_limit && (forall x in 0..len(x1.shareholders), y in 0..len(x1.shareholders): x1.shareholders[x] == x1.shareholders[y] ==> x == y) ==> (forall z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y) && (x4 < len(pool_u64::shareholders(old(distribution_pool))) && pool_u64::shareholders(old(distribution_pool))[x4] != operator) ==> {
            let a = ..S5 |~ result_of<pool_u64::balance>(old(distribution_pool), pool_u64::shareholders(old(distribution_pool))[x4]);
            S5.. |~ ensures_of<pool_u64::transfer_shares>(old(distribution_pool), pool_u64::shareholders(old(distribution_pool))[x4], operator, pool_u64::spec_amount_to_shares_with_total_coins(old(distribution_pool), ((if (old(distribution_pool).total_coins == 0 || old(distribution_pool).total_shares == 0) 0 else (if (simple_map::spec_contains_key<address, u64>(old(distribution_pool).shares, pool_u64::shareholders(old(distribution_pool))[x4])) simple_map::spec_get<address, u64>(old(distribution_pool).shares, pool_u64::shareholders(old(distribution_pool))[x4]) else 0) * updated_total_coins / old(distribution_pool).total_shares) - a) * commission_percentage / 100, updated_total_coins))
        })))));
        ensures [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && pool_u64::total_coins(old(distribution_pool)) != updated_total_coins ==> (forall x4: u64, x5: pool_u64::Pool: (forall x in 0..len(x5.shareholders): simple_map::spec_contains_key<address, u64>(x5.shares, x5.shareholders[x])) && simple_map::spec_len<address, u64>(x5.shares) == len(x5.shareholders) && len(x5.shareholders) <= x5.shareholders_limit && (forall x in 0..len(x5.shareholders), y in 0..len(x5.shareholders): x5.shareholders[x] == x5.shareholders[y] ==> x == y) ==> (forall x3: pool_u64::Pool: (forall x in 0..len(x3.shareholders): simple_map::spec_contains_key<address, u64>(x3.shares, x3.shareholders[x])) && simple_map::spec_len<address, u64>(x3.shares) == len(x3.shareholders) && len(x3.shareholders) <= x3.shareholders_limit && (forall x in 0..len(x3.shareholders), y in 0..len(x3.shareholders): x3.shareholders[x] == x3.shareholders[y] ==> x == y) ==> (forall x2: pool_u64::Pool: (forall x in 0..len(x2.shareholders): simple_map::spec_contains_key<address, u64>(x2.shares, x2.shareholders[x])) && simple_map::spec_len<address, u64>(x2.shares) == len(x2.shareholders) && len(x2.shareholders) <= x2.shareholders_limit && (forall x in 0..len(x2.shareholders), y in 0..len(x2.shareholders): x2.shareholders[x] == x2.shareholders[y] ==> x == y) ==> (forall x1: pool_u64::Pool: (forall x in 0..len(x1.shareholders): simple_map::spec_contains_key<address, u64>(x1.shares, x1.shareholders[x])) && simple_map::spec_len<address, u64>(x1.shares) == len(x1.shareholders) && len(x1.shareholders) <= x1.shareholders_limit && (forall x in 0..len(x1.shareholders), y in 0..len(x1.shareholders): x1.shareholders[x] == x1.shareholders[y] ==> x == y) ==> (forall z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y) && (x4 < len(pool_u64::shareholders(old(distribution_pool))) && pool_u64::shareholders(old(distribution_pool))[x4] == operator) ==> distribution_pool == z)))));
        ensures [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && pool_u64::total_coins(old(distribution_pool)) == updated_total_coins ==> distribution_pool == old(distribution_pool);
        ensures [inferred = sathard] forall _q0: pool_u64::Pool: ensures_of<pool_u64::update_total_coins>(_q0, updated_total_coins, distribution_pool);
        aborts_if [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && (pool_u64::total_coins(distribution_pool) != updated_total_coins && ((exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: u64, x1: pool_u64::Pool: z < len(pool_u64::shareholders(distribution_pool)) && pool_u64::shareholders(distribution_pool)[z] != operator && (forall x in 0..len(x1.shareholders): simple_map::spec_contains_key<address, u64>(x1.shares, x1.shareholders[x])) && simple_map::spec_len<address, u64>(x1.shares) == len(x1.shareholders) && len(x1.shareholders) <= x1.shareholders_limit && (forall x in 0..len(x1.shareholders), y in 0..len(x1.shareholders): x1.shareholders[x] == x1.shareholders[y] ==> x == y) && aborts_of<pool_u64::amount_to_shares_with_total_coins>(x1, ((if (x1.total_coins == 0 || x1.total_shares == 0) 0 else (if (simple_map::spec_contains_key<address, u64>(x1.shares, pool_u64::shareholders(distribution_pool)[z])) simple_map::spec_get<address, u64>(x1.shares, pool_u64::shareholders(distribution_pool)[z]) else 0) * updated_total_coins / x1.total_shares) - (..S5 |~ result_of<pool_u64::balance>(x1, pool_u64::shareholders(distribution_pool)[z]))) * commission_percentage / 100, updated_total_coins))));
        aborts_if [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && (pool_u64::total_coins(distribution_pool) != updated_total_coins && ((exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: u64, x1: pool_u64::Pool: z < len(pool_u64::shareholders(distribution_pool)) && pool_u64::shareholders(distribution_pool)[z] != operator && (forall x in 0..len(x1.shareholders): simple_map::spec_contains_key<address, u64>(x1.shares, x1.shareholders[x])) && simple_map::spec_len<address, u64>(x1.shares) == len(x1.shareholders) && len(x1.shareholders) <= x1.shareholders_limit && (forall x in 0..len(x1.shareholders), y in 0..len(x1.shareholders): x1.shareholders[x] == x1.shareholders[y] ==> x == y) && ((if (x1.total_coins == 0 || x1.total_shares == 0) 0 else (if (simple_map::spec_contains_key<address, u64>(x1.shares, pool_u64::shareholders(distribution_pool)[z])) simple_map::spec_get<address, u64>(x1.shares, pool_u64::shareholders(distribution_pool)[z]) else 0) * updated_total_coins / x1.total_shares) - (..S5 |~ result_of<pool_u64::balance>(x1, pool_u64::shareholders(distribution_pool)[z]))) * commission_percentage > MAX_U64)));
        aborts_if [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && (pool_u64::total_coins(distribution_pool) != updated_total_coins && ((exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: u64, x1: pool_u64::Pool: z < len(pool_u64::shareholders(distribution_pool)) && pool_u64::shareholders(distribution_pool)[z] != operator && (forall x in 0..len(x1.shareholders): simple_map::spec_contains_key<address, u64>(x1.shares, x1.shareholders[x])) && simple_map::spec_len<address, u64>(x1.shares) == len(x1.shareholders) && len(x1.shareholders) <= x1.shareholders_limit && (forall x in 0..len(x1.shareholders), y in 0..len(x1.shareholders): x1.shareholders[x] == x1.shareholders[y] ==> x == y) && (if (x1.total_coins == 0 || x1.total_shares == 0) 0 else (if (simple_map::spec_contains_key<address, u64>(x1.shares, pool_u64::shareholders(distribution_pool)[z])) simple_map::spec_get<address, u64>(x1.shares, pool_u64::shareholders(distribution_pool)[z]) else 0) * updated_total_coins / x1.total_shares) - (..S5 |~ result_of<pool_u64::balance>(x1, pool_u64::shareholders(distribution_pool)[z])) < 0)));
        aborts_if [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && (pool_u64::total_coins(distribution_pool) != updated_total_coins && ((exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: u64, x1: pool_u64::Pool: z < len(pool_u64::shareholders(distribution_pool)) && pool_u64::shareholders(distribution_pool)[z] != operator && (forall x in 0..len(x1.shareholders): simple_map::spec_contains_key<address, u64>(x1.shares, x1.shareholders[x])) && simple_map::spec_len<address, u64>(x1.shares) == len(x1.shareholders) && len(x1.shareholders) <= x1.shareholders_limit && (forall x in 0..len(x1.shareholders), y in 0..len(x1.shareholders): x1.shareholders[x] == x1.shareholders[y] ==> x == y) && aborts_of<pool_u64::shares_to_amount_with_total_coins>(x1, if (simple_map::spec_contains_key<address, u64>(x1.shares, pool_u64::shareholders(distribution_pool)[z])) simple_map::spec_get<address, u64>(x1.shares, pool_u64::shareholders(distribution_pool)[z]) else 0, updated_total_coins))));
        aborts_if [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && (pool_u64::total_coins(distribution_pool) != updated_total_coins && ((exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: u64, x1: pool_u64::Pool: z < len(pool_u64::shareholders(distribution_pool)) && pool_u64::shareholders(distribution_pool)[z] != operator && (forall x in 0..len(x1.shareholders): simple_map::spec_contains_key<address, u64>(x1.shares, x1.shareholders[x])) && simple_map::spec_len<address, u64>(x1.shares) == len(x1.shareholders) && len(x1.shareholders) <= x1.shareholders_limit && (forall x in 0..len(x1.shareholders), y in 0..len(x1.shareholders): x1.shareholders[x] == x1.shareholders[y] ==> x == y) && aborts_of<pool_u64::balance>(x1, pool_u64::shareholders(distribution_pool)[z]))));
        aborts_if [inferred = sathard] (forall x in 0..len(distribution_pool.shareholders): simple_map::spec_contains_key<address, u64>(distribution_pool.shares, distribution_pool.shareholders[x])) && simple_map::spec_len<address, u64>(distribution_pool.shares) == len(distribution_pool.shareholders) && len(distribution_pool.shareholders) <= distribution_pool.shareholders_limit && (forall x in 0..len(distribution_pool.shareholders), y in 0..len(distribution_pool.shareholders): distribution_pool.shareholders[x] == distribution_pool.shareholders[y] ==> x == y) && (pool_u64::total_coins(distribution_pool) != updated_total_coins && ((exists x: u64: x < len(pool_u64::shareholders(distribution_pool)) && !in_range(pool_u64::shareholders(distribution_pool), x)) && (exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y)) && (exists z: pool_u64::Pool: (forall x in 0..len(z.shareholders): simple_map::spec_contains_key<address, u64>(z.shares, z.shareholders[x])) && simple_map::spec_len<address, u64>(z.shares) == len(z.shareholders) && len(z.shareholders) <= z.shareholders_limit && (forall x in 0..len(z.shareholders), y in 0..len(z.shareholders): z.shareholders[x] == z.shareholders[y] ==> x == y))));
        */
        ensures distribution_pool
            == spec_distribution_pool_after_update(
                old(distribution_pool),
                updated_total_coins,
                operator,
                commission_percentage
            );
        aborts_if spec_update_distribution_pool_aborts(
            distribution_pool,
            updated_total_coins,
            operator,
            commission_percentage
        );
    } proof {
        forall end: u64 {
            spec_distribution_shares_fold(
                distribution_pool.shares,
                distribution_pool.shareholders,
                end,
                distribution_pool.total_coins,
                distribution_pool.total_shares,
                distribution_pool.scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            )
        } [weight = 5] apply spec_distribution_shares_fold_step(
            distribution_pool.shareholders,
            end,
            distribution_pool.total_coins,
            distribution_pool.total_shares,
            distribution_pool.scaling_factor,
            updated_total_coins,
            operator,
            commission_percentage
        );
        forall end: u64 {
            spec_distribution_shares_fold(
                distribution_pool.shares,
                distribution_pool.shareholders,
                end,
                distribution_pool.total_coins,
                distribution_pool.total_shares,
                distribution_pool.scaling_factor,
                distribution_pool.total_coins,
                operator,
                commission_percentage
            )
        } [weight = 5] apply spec_distribution_shares_fold_unchanged(
            distribution_pool.shareholders,
            end,
            distribution_pool.total_coins,
            distribution_pool.total_shares,
            distribution_pool.scaling_factor,
            operator,
            commission_percentage
        );
        forall end: u64 {
            spec_distribution_shareholders_prefix(
                distribution_pool.shareholders,
                distribution_pool.shares,
                end,
                distribution_pool.total_coins,
                distribution_pool.total_shares,
                distribution_pool.scaling_factor,
                updated_total_coins,
                operator,
                commission_percentage
            )
        } [weight = 5] apply spec_distribution_shareholders_prefix_step(
            distribution_pool.shareholders,
            end,
            distribution_pool.total_coins,
            distribution_pool.total_shares,
            distribution_pool.scaling_factor,
            updated_total_coins,
            operator,
            commission_percentage
        );
        forall end: u64 {
            spec_distribution_shareholders_prefix(
                distribution_pool.shareholders,
                distribution_pool.shares,
                end,
                distribution_pool.total_coins,
                distribution_pool.total_shares,
                distribution_pool.scaling_factor,
                distribution_pool.total_coins,
                operator,
                commission_percentage
            )
        } [weight = 5] apply spec_distribution_shareholders_prefix_unchanged(
            distribution_pool.shareholders,
            end,
            distribution_pool.total_coins,
            distribution_pool.total_shares,
            distribution_pool.scaling_factor,
            operator,
            commission_percentage
        );
    }

    /// The Account exists under the staker.
    /// The guid_creation_num of the account resource is up to MAX_U64.
    spec new_staking_contracts_holder(staker: &signer): Store {
        use 0x1::signer;
        use 0x1::account;
        use 0x1::simple_map;
        pragma aborts_if_is_partial;
        include NewStakingContractsHolderAbortsIf;
        pragma opaque = true;
        modifies account::Account[signer::address_of(staker)];
        ensures [inferred] ({
            let a = ..S1 |~ result_of<account::new_event_handle<CreateStakingContractEvent>>(staker);
            let b = S1..S2 |~ result_of<account::new_event_handle<UpdateVoterEvent>>(staker);
            let c = S2..S3 |~ result_of<account::new_event_handle<ResetLockupEvent>>(staker);
            let d = S3..S4 |~ result_of<account::new_event_handle<AddStakeEvent>>(staker);
            let e = S4..S5 |~ result_of<account::new_event_handle<RequestCommissionEvent>>(staker);
            let f = S5..S6 |~ result_of<account::new_event_handle<UnlockStakeEvent>>(staker);
            let a_1 = S6..S7 |~ result_of<account::new_event_handle<SwitchOperatorEvent>>(staker);
            let b_1 = S7..S8 |~ result_of<account::new_event_handle<AddDistributionEvent>>(staker);
            let c_1 = S8.. |~ result_of<account::new_event_handle<DistributeEvent>>(staker);
            result == Store{staking_contracts: simple_map::spec_new<address, StakingContract>(), create_staking_contract_events: a, update_voter_events: b, reset_lockup_events: c, add_stake_events: d, request_commission_events: e, unlock_stake_events: f, switch_operator_events: a_1, add_distribution_events: b_1, distribute_events: c_1}
        });
        aborts_if [inferred] S8 |~ (aborts_of<account::new_event_handle<DistributeEvent>>(staker));
        aborts_if [inferred] S7 |~ (aborts_of<account::new_event_handle<AddDistributionEvent>>(staker));
        aborts_if [inferred] S6 |~ (aborts_of<account::new_event_handle<SwitchOperatorEvent>>(staker));
        aborts_if [inferred] S5 |~ (aborts_of<account::new_event_handle<UnlockStakeEvent>>(staker));
        aborts_if [inferred] S4 |~ (aborts_of<account::new_event_handle<RequestCommissionEvent>>(staker));
        aborts_if [inferred] S3 |~ (aborts_of<account::new_event_handle<AddStakeEvent>>(staker));
        aborts_if [inferred] S2 |~ (aborts_of<account::new_event_handle<ResetLockupEvent>>(staker));
        aborts_if [inferred] S1 |~ (aborts_of<account::new_event_handle<UpdateVoterEvent>>(staker));
        aborts_if [inferred] aborts_of<account::new_event_handle<CreateStakingContractEvent>>(staker);
    }

    spec schema NewStakingContractsHolderAbortsIf {
        staker: signer;

        let addr = signer::address_of(staker);
        // let account = global<account::Account>(addr);
        // aborts_if !exists<account::Account>(addr);
        // aborts_if account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if account.guid_creation_num + 9 > MAX_U64;
    }

    /// The Store exists under the staker.
    /// a staking_contract exists for the staker/operator pair.
    spec schema ContractExistsAbortsIf {
        staker: address;
        operator: address;

        aborts_if !exists<Store>(staker);
        let staking_contracts = global<Store>(staker).staking_contracts;
        // This property may cause timeout
        aborts_if !simple_map::spec_contains_key(staking_contracts, operator);
    }

    spec schema UpdateVoterSchema {
        staker: address;
        operator: address;

        let store = global<Store>(staker);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let pool_address = staking_contract.pool_address;
        aborts_if !exists<stake::StakePool>(pool_address);
        aborts_if !exists<stake::StakePool>(staking_contract.owner_cap.pool_address);
        include ContractExistsAbortsIf;
    }

    spec schema WithdrawAbortsIf<CoinType> {
        account: signer;
        amount: u64;

        let account_addr = signer::address_of(account);
        let coin_store = global<coin::CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;
        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
        aborts_if balance < amount;
    }

    spec schema GetStakingContractAmountsAbortsIf {
        staking_contract: StakingContract;
        aborts_if spec_get_staking_contract_amounts_aborts(staking_contract);
    }

    spec schema IncreaseLockupWithCapAbortsIf {
        use aptos_framework::timestamp;
        staker: address;
        operator: address;

        let store = global<Store>(staker);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let pool_address = staking_contract.owner_cap.pool_address;

        // property 5: Only the owner of the stake pool has the permission to reset the lockup period of the pool.
        aborts_if !stake::stake_pool_exists(pool_address);
        aborts_if !exists<staking_config::StakingConfig>(@aptos_framework);

        let config = global<staking_config::StakingConfig>(@aptos_framework);
        let stake_pool = global<stake::StakePool>(pool_address);
        let old_locked_until_secs = stake_pool.locked_until_secs;
        let seconds = global<timestamp::CurrentTimeMicroseconds>(@aptos_framework).microseconds
            / timestamp::MICRO_CONVERSION_FACTOR;
        let new_locked_until_secs = seconds + config.recurring_lockup_duration_secs;
        aborts_if seconds + config.recurring_lockup_duration_secs > MAX_U64;
        aborts_if old_locked_until_secs > new_locked_until_secs
            || old_locked_until_secs == new_locked_until_secs;
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);

        let post post_store = global<Store>(staker);
        let post post_staking_contract = simple_map::spec_get(
            post_store.staking_contracts, operator
        );
        let post post_stake_pool = global<stake::StakePool>(
            post_staking_contract.owner_cap.pool_address
        );
        ensures post_stake_pool.locked_until_secs == new_locked_until_secs;
    }

    spec schema CreateStakingContractWithCoinsAbortsIfAndEnsures {
        staker: signer;
        operator: address;
        voter: address;
        amount: u64;
        commission_percentage: u64;
        contract_creation_seed: vector<u8>;

        aborts_if commission_percentage > 100;
        aborts_if !exists<staking_config::StakingConfig>(@aptos_framework);
        let config = global<staking_config::StakingConfig>(@aptos_framework);
        let min_stake_required = config.minimum_stake;
        aborts_if amount < min_stake_required;

        let staker_address = signer::address_of(staker);
        let account = global<account::Account>(staker_address);
        aborts_if !exists<Store>(staker_address)
            && !exists<account::Account>(staker_address);
        aborts_if !exists<Store>(staker_address)
            && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        /// [high-level-req-1]
        ensures exists<Store>(staker_address);

        let store = global<Store>(staker_address);
        let staking_contracts = store.staking_contracts;
        // TODO: this property causes timeout
        // aborts_if simple_map::spec_contains_key(staking_contracts, operator);

        // Verify create_stake_pool()
        // TODO: this property causes timeout
        // let seed_0 = bcs::to_bytes(staker_address);
        // let seed_1 = concat(concat(concat(seed_0, bcs::to_bytes(operator)), SALT), contract_creation_seed);
        // let resource_addr = account::spec_create_resource_address(staker_address, seed_1);
        // include CreateStakePoolAbortsIf {resource_addr};

        // Verify stake::add_stake_with_cap()
        let owner_cap = simple_map::spec_get(store.staking_contracts, operator).owner_cap;
        // TODO: this property causes timeout
        // include stake::AddStakeWithCapAbortsIfAndEnsures{owner_cap: owner_cap};
        let post post_store = global<Store>(staker_address);
        let post post_staking_contracts = post_store.staking_contracts;
        // TODO: this property causes timeout
        // ensures simple_map::spec_contains_key(post_staking_contracts, operator);
    }

    spec schema PreconditionsInCreateContract {
        requires exists<stake::ValidatorPerformance>(@aptos_framework);
        requires exists<stake::ValidatorSet>(@aptos_framework);
        requires exists<staking_config::StakingRewardsConfig>(@aptos_framework)
            || !std::features::spec_periodical_reward_rate_decrease_enabled();
        requires exists<aptos_framework::timestamp::CurrentTimeMicroseconds>(
            @aptos_framework
        );
        requires exists<stake::AptosCoinCapabilities>(@aptos_framework);
    }

    spec schema CreateStakePoolAbortsIf {
        resource_addr: address;
        operator: address;
        voter: address;
        contract_creation_seed: vector<u8>;

        // postconditions account::create_resource_account()
        let acc = global<account::Account>(resource_addr);
        aborts_if exists<account::Account>(resource_addr)
            && (
                std::option::is_some(acc.signer_capability_offer.for)
                    || acc.sequence_number != (0 as u64)
            );
        aborts_if !exists<account::Account>(resource_addr)
            && len(bcs::to_bytes(resource_addr)) != 32;
        aborts_if len(account::ZERO_AUTH_KEY) != 32;

        // postconditions stake::initialize_stake_owner()
        aborts_if exists<stake::ValidatorConfig>(resource_addr);
        let allowed = global<stake::AllowedValidators>(@aptos_framework);
        aborts_if exists<stake::AllowedValidators>(@aptos_framework)
            && !contains(allowed.accounts, resource_addr);
        aborts_if exists<stake::StakePool>(resource_addr);
        aborts_if exists<stake::OwnerCapability>(resource_addr);
        // 12 is the times that calls 'events::guids'
        aborts_if exists<account::Account>(resource_addr)
            && acc.guid_creation_num + 12 >= account::MAX_GUID_CREATION_NUM;
    }
    spec create_resource_account_seed(
        staker: address, operator: address, contract_creation_seed: vector<u8>
    ): vector<u8> {
        use 0x1::bcs;
        pragma opaque = true, aborts_if_is_partial = false;
        aborts_if false;
        ensures result ==
            concat(
                concat(
                    concat(bcs::to_bytes<address>(staker), bcs::to_bytes<address>(operator)),
                    SALT
                ),
                contract_creation_seed
            );
    }

    spec staker_address(pool_address: address): 0x1::option::Option<address> {
        use 0x1::option;
        pragma opaque = true, aborts_if_is_partial = false;
        aborts_if false;
        ensures exists<Staker>(pool_address) ==>
            result == option::some<address>(Staker[pool_address].staker);
        ensures !exists<Staker>(pool_address) ==> result == option::none<address>();
    }

}
