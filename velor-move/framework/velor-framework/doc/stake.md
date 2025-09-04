
<a id="0x1_stake"></a>

# Module `0x1::stake`


Validator lifecycle:
1. Prepare a validator node set up and call stake::initialize_validator
2. Once ready to deposit stake (or have funds assigned by a staking service in exchange for ownership capability),
call stake::add_stake (or *_with_cap versions if called from the staking service)
3. Call stake::join_validator_set (or _with_cap version) to join the active validator set. Changes are effective in
the next epoch.
4. Validate and gain rewards. The stake will automatically be locked up for a fixed duration (set by governance) and
automatically renewed at expiration.
5. At any point, if the validator operator wants to update the consensus key or network/fullnode addresses, they can
call stake::rotate_consensus_key and stake::update_network_and_fullnode_addresses. Similar to changes to stake, the
changes to consensus key/network/fullnode addresses are only effective in the next epoch.
6. Validator can request to unlock their stake at any time. However, their stake will only become withdrawable when
their current lockup expires. This can be at most as long as the fixed lockup duration.
7. After exiting, the validator can either explicitly leave the validator set by calling stake::leave_validator_set
or if their stake drops below the min required, they would get removed at the end of the epoch.
8. Validator can always rejoin the validator set by going through steps 2-3 again.
9. An owner can always switch operators by calling stake::set_operator.
10. An owner can always switch designated voter by calling stake::set_designated_voter.


-  [Resource `OwnerCapability`](#0x1_stake_OwnerCapability)
-  [Resource `StakePool`](#0x1_stake_StakePool)
-  [Resource `ValidatorConfig`](#0x1_stake_ValidatorConfig)
-  [Struct `ValidatorInfo`](#0x1_stake_ValidatorInfo)
-  [Resource `ValidatorSet`](#0x1_stake_ValidatorSet)
-  [Resource `PendingTransactionFee`](#0x1_stake_PendingTransactionFee)
-  [Enum Resource `TransactionFeeConfig`](#0x1_stake_TransactionFeeConfig)
-  [Struct `DistributeTransactionFee`](#0x1_stake_DistributeTransactionFee)
-  [Resource `VelorCoinCapabilities`](#0x1_stake_VelorCoinCapabilities)
-  [Struct `IndividualValidatorPerformance`](#0x1_stake_IndividualValidatorPerformance)
-  [Resource `ValidatorPerformance`](#0x1_stake_ValidatorPerformance)
-  [Struct `RegisterValidatorCandidateEvent`](#0x1_stake_RegisterValidatorCandidateEvent)
-  [Struct `StakeManagementPermission`](#0x1_stake_StakeManagementPermission)
-  [Struct `RegisterValidatorCandidate`](#0x1_stake_RegisterValidatorCandidate)
-  [Struct `SetOperatorEvent`](#0x1_stake_SetOperatorEvent)
-  [Struct `SetOperator`](#0x1_stake_SetOperator)
-  [Struct `AddStakeEvent`](#0x1_stake_AddStakeEvent)
-  [Struct `AddStake`](#0x1_stake_AddStake)
-  [Struct `ReactivateStakeEvent`](#0x1_stake_ReactivateStakeEvent)
-  [Struct `ReactivateStake`](#0x1_stake_ReactivateStake)
-  [Struct `RotateConsensusKeyEvent`](#0x1_stake_RotateConsensusKeyEvent)
-  [Struct `RotateConsensusKey`](#0x1_stake_RotateConsensusKey)
-  [Struct `UpdateNetworkAndFullnodeAddressesEvent`](#0x1_stake_UpdateNetworkAndFullnodeAddressesEvent)
-  [Struct `UpdateNetworkAndFullnodeAddresses`](#0x1_stake_UpdateNetworkAndFullnodeAddresses)
-  [Struct `IncreaseLockupEvent`](#0x1_stake_IncreaseLockupEvent)
-  [Struct `IncreaseLockup`](#0x1_stake_IncreaseLockup)
-  [Struct `JoinValidatorSetEvent`](#0x1_stake_JoinValidatorSetEvent)
-  [Struct `JoinValidatorSet`](#0x1_stake_JoinValidatorSet)
-  [Struct `DistributeRewardsEvent`](#0x1_stake_DistributeRewardsEvent)
-  [Struct `DistributeRewards`](#0x1_stake_DistributeRewards)
-  [Struct `UnlockStakeEvent`](#0x1_stake_UnlockStakeEvent)
-  [Struct `UnlockStake`](#0x1_stake_UnlockStake)
-  [Struct `WithdrawStakeEvent`](#0x1_stake_WithdrawStakeEvent)
-  [Struct `WithdrawStake`](#0x1_stake_WithdrawStake)
-  [Struct `LeaveValidatorSetEvent`](#0x1_stake_LeaveValidatorSetEvent)
-  [Struct `LeaveValidatorSet`](#0x1_stake_LeaveValidatorSet)
-  [Resource `ValidatorFees`](#0x1_stake_ValidatorFees)
-  [Resource `AllowedValidators`](#0x1_stake_AllowedValidators)
-  [Resource `Ghost$ghost_valid_perf`](#0x1_stake_Ghost$ghost_valid_perf)
-  [Resource `Ghost$ghost_proposer_idx`](#0x1_stake_Ghost$ghost_proposer_idx)
-  [Resource `Ghost$ghost_active_num`](#0x1_stake_Ghost$ghost_active_num)
-  [Resource `Ghost$ghost_pending_inactive_num`](#0x1_stake_Ghost$ghost_pending_inactive_num)
-  [Constants](#@Constants_0)
-  [Function `check_stake_permission`](#0x1_stake_check_stake_permission)
-  [Function `grant_permission`](#0x1_stake_grant_permission)
-  [Function `get_lockup_secs`](#0x1_stake_get_lockup_secs)
-  [Function `get_remaining_lockup_secs`](#0x1_stake_get_remaining_lockup_secs)
-  [Function `get_stake`](#0x1_stake_get_stake)
-  [Function `get_validator_state`](#0x1_stake_get_validator_state)
-  [Function `get_current_epoch_voting_power`](#0x1_stake_get_current_epoch_voting_power)
-  [Function `get_delegated_voter`](#0x1_stake_get_delegated_voter)
-  [Function `get_operator`](#0x1_stake_get_operator)
-  [Function `get_owned_pool_address`](#0x1_stake_get_owned_pool_address)
-  [Function `get_validator_index`](#0x1_stake_get_validator_index)
-  [Function `get_current_epoch_proposal_counts`](#0x1_stake_get_current_epoch_proposal_counts)
-  [Function `get_validator_config`](#0x1_stake_get_validator_config)
-  [Function `stake_pool_exists`](#0x1_stake_stake_pool_exists)
-  [Function `get_pending_transaction_fee`](#0x1_stake_get_pending_transaction_fee)
-  [Function `initialize`](#0x1_stake_initialize)
-  [Function `store_velor_coin_mint_cap`](#0x1_stake_store_velor_coin_mint_cap)
-  [Function `remove_validators`](#0x1_stake_remove_validators)
-  [Function `initialize_pending_transaction_fee`](#0x1_stake_initialize_pending_transaction_fee)
-  [Function `set_transaction_fee_config`](#0x1_stake_set_transaction_fee_config)
-  [Function `record_fee`](#0x1_stake_record_fee)
-  [Function `initialize_stake_owner`](#0x1_stake_initialize_stake_owner)
-  [Function `initialize_validator`](#0x1_stake_initialize_validator)
-  [Function `initialize_owner`](#0x1_stake_initialize_owner)
-  [Function `extract_owner_cap`](#0x1_stake_extract_owner_cap)
-  [Function `deposit_owner_cap`](#0x1_stake_deposit_owner_cap)
-  [Function `destroy_owner_cap`](#0x1_stake_destroy_owner_cap)
-  [Function `set_operator`](#0x1_stake_set_operator)
-  [Function `set_operator_with_cap`](#0x1_stake_set_operator_with_cap)
-  [Function `set_delegated_voter`](#0x1_stake_set_delegated_voter)
-  [Function `set_delegated_voter_with_cap`](#0x1_stake_set_delegated_voter_with_cap)
-  [Function `add_stake`](#0x1_stake_add_stake)
-  [Function `add_stake_with_cap`](#0x1_stake_add_stake_with_cap)
-  [Function `reactivate_stake`](#0x1_stake_reactivate_stake)
-  [Function `reactivate_stake_with_cap`](#0x1_stake_reactivate_stake_with_cap)
-  [Function `rotate_consensus_key`](#0x1_stake_rotate_consensus_key)
-  [Function `update_network_and_fullnode_addresses`](#0x1_stake_update_network_and_fullnode_addresses)
-  [Function `increase_lockup`](#0x1_stake_increase_lockup)
-  [Function `increase_lockup_with_cap`](#0x1_stake_increase_lockup_with_cap)
-  [Function `join_validator_set`](#0x1_stake_join_validator_set)
-  [Function `join_validator_set_internal`](#0x1_stake_join_validator_set_internal)
-  [Function `unlock`](#0x1_stake_unlock)
-  [Function `unlock_with_cap`](#0x1_stake_unlock_with_cap)
-  [Function `withdraw`](#0x1_stake_withdraw)
-  [Function `withdraw_with_cap`](#0x1_stake_withdraw_with_cap)
-  [Function `leave_validator_set`](#0x1_stake_leave_validator_set)
-  [Function `is_current_epoch_validator`](#0x1_stake_is_current_epoch_validator)
-  [Function `update_performance_statistics`](#0x1_stake_update_performance_statistics)
-  [Function `on_new_epoch`](#0x1_stake_on_new_epoch)
-  [Function `cur_validator_consensus_infos`](#0x1_stake_cur_validator_consensus_infos)
-  [Function `next_validator_consensus_infos`](#0x1_stake_next_validator_consensus_infos)
-  [Function `validator_consensus_infos_from_validator_set`](#0x1_stake_validator_consensus_infos_from_validator_set)
-  [Function `addresses_from_validator_infos`](#0x1_stake_addresses_from_validator_infos)
-  [Function `update_stake_pool`](#0x1_stake_update_stake_pool)
-  [Function `get_reconfig_start_time_secs`](#0x1_stake_get_reconfig_start_time_secs)
-  [Function `calculate_rewards_amount`](#0x1_stake_calculate_rewards_amount)
-  [Function `distribute_rewards`](#0x1_stake_distribute_rewards)
-  [Function `append`](#0x1_stake_append)
-  [Function `find_validator`](#0x1_stake_find_validator)
-  [Function `generate_validator_info`](#0x1_stake_generate_validator_info)
-  [Function `get_next_epoch_voting_power`](#0x1_stake_get_next_epoch_voting_power)
-  [Function `update_voting_power_increase`](#0x1_stake_update_voting_power_increase)
-  [Function `assert_stake_pool_exists`](#0x1_stake_assert_stake_pool_exists)
-  [Function `configure_allowed_validators`](#0x1_stake_configure_allowed_validators)
-  [Function `is_allowed`](#0x1_stake_is_allowed)
-  [Function `assert_owner_cap_exists`](#0x1_stake_assert_owner_cap_exists)
-  [Function `assert_reconfig_not_in_progress`](#0x1_stake_assert_reconfig_not_in_progress)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Resource `ValidatorSet`](#@Specification_1_ValidatorSet)
    -  [Function `get_validator_state`](#@Specification_1_get_validator_state)
    -  [Function `get_pending_transaction_fee`](#@Specification_1_get_pending_transaction_fee)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `remove_validators`](#@Specification_1_remove_validators)
    -  [Function `initialize_stake_owner`](#@Specification_1_initialize_stake_owner)
    -  [Function `initialize_validator`](#@Specification_1_initialize_validator)
    -  [Function `extract_owner_cap`](#@Specification_1_extract_owner_cap)
    -  [Function `deposit_owner_cap`](#@Specification_1_deposit_owner_cap)
    -  [Function `set_operator_with_cap`](#@Specification_1_set_operator_with_cap)
    -  [Function `set_delegated_voter_with_cap`](#@Specification_1_set_delegated_voter_with_cap)
    -  [Function `add_stake`](#@Specification_1_add_stake)
    -  [Function `add_stake_with_cap`](#@Specification_1_add_stake_with_cap)
    -  [Function `reactivate_stake_with_cap`](#@Specification_1_reactivate_stake_with_cap)
    -  [Function `rotate_consensus_key`](#@Specification_1_rotate_consensus_key)
    -  [Function `update_network_and_fullnode_addresses`](#@Specification_1_update_network_and_fullnode_addresses)
    -  [Function `increase_lockup_with_cap`](#@Specification_1_increase_lockup_with_cap)
    -  [Function `join_validator_set`](#@Specification_1_join_validator_set)
    -  [Function `unlock_with_cap`](#@Specification_1_unlock_with_cap)
    -  [Function `withdraw`](#@Specification_1_withdraw)
    -  [Function `leave_validator_set`](#@Specification_1_leave_validator_set)
    -  [Function `is_current_epoch_validator`](#@Specification_1_is_current_epoch_validator)
    -  [Function `update_performance_statistics`](#@Specification_1_update_performance_statistics)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)
    -  [Function `next_validator_consensus_infos`](#@Specification_1_next_validator_consensus_infos)
    -  [Function `validator_consensus_infos_from_validator_set`](#@Specification_1_validator_consensus_infos_from_validator_set)
    -  [Function `update_stake_pool`](#@Specification_1_update_stake_pool)
    -  [Function `get_reconfig_start_time_secs`](#@Specification_1_get_reconfig_start_time_secs)
    -  [Function `calculate_rewards_amount`](#@Specification_1_calculate_rewards_amount)
    -  [Function `distribute_rewards`](#@Specification_1_distribute_rewards)
    -  [Function `append`](#@Specification_1_append)
    -  [Function `find_validator`](#@Specification_1_find_validator)
    -  [Function `update_voting_power_increase`](#@Specification_1_update_voting_power_increase)
    -  [Function `assert_stake_pool_exists`](#@Specification_1_assert_stake_pool_exists)
    -  [Function `configure_allowed_validators`](#@Specification_1_configure_allowed_validators)
    -  [Function `assert_owner_cap_exists`](#@Specification_1_assert_owner_cap_exists)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aggregator_v2.md#0x1_aggregator_v2">0x1::aggregator_v2</a>;
<b>use</b> <a href="velor_coin.md#0x1_velor_coin">0x1::velor_coin</a>;
<b>use</b> <a href="big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../velor-stdlib/doc/bls12381.md#0x1_bls12381">0x1::bls12381</a>;
<b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
<b>use</b> <a href="../../velor-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state">0x1::reconfiguration_state</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../velor-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_stake_OwnerCapability"></a>

## Resource `OwnerCapability`

Capability that represents ownership and can be used to control the validator and the associated stake pool.
Having this be separate from the signer for the account that the validator resources are hosted at allows
modules to have control over a validator.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_StakePool"></a>

## Resource `StakePool`

Each validator has a separate StakePool resource and can provide a stake.
Changes in stake for an active validator:
1. If a validator calls add_stake, the newly added stake is moved to pending_active.
2. If validator calls unlock, their stake is moved to pending_inactive.
2. When the next epoch starts, any pending_inactive stake is moved to inactive and can be withdrawn.
Any pending_active stake is moved to active and adds to the validator's voting power.

Changes in stake for an inactive validator:
1. If a validator calls add_stake, the newly added stake is moved directly to active.
2. If validator calls unlock, their stake is moved directly to inactive.
3. When the next epoch starts, the validator can be activated if their active stake is more than the minimum.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inactive: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_until_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>operator_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>delegated_voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>initialize_validator_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_RegisterValidatorCandidateEvent">stake::RegisterValidatorCandidateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>set_operator_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_SetOperatorEvent">stake::SetOperatorEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_AddStakeEvent">stake::AddStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reactivate_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_ReactivateStakeEvent">stake::ReactivateStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>rotate_consensus_key_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_RotateConsensusKeyEvent">stake::RotateConsensusKeyEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_network_and_fullnode_addresses_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddressesEvent">stake::UpdateNetworkAndFullnodeAddressesEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>increase_lockup_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_IncreaseLockupEvent">stake::IncreaseLockupEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>join_validator_set_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_JoinValidatorSetEvent">stake::JoinValidatorSetEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>distribute_rewards_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_DistributeRewardsEvent">stake::DistributeRewardsEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_UnlockStakeEvent">stake::UnlockStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_WithdrawStakeEvent">stake::WithdrawStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>leave_validator_set_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="stake.md#0x1_stake_LeaveValidatorSetEvent">stake::LeaveValidatorSetEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ValidatorConfig"></a>

## Resource `ValidatorConfig`

Validator info stored in validator address.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>validator_index: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ValidatorInfo"></a>

## Struct `ValidatorInfo`

Consensus information per validator, stored in ValidatorSet.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>voting_power: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>config: <a href="stake.md#0x1_stake_ValidatorConfig">stake::ValidatorConfig</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ValidatorSet"></a>

## Resource `ValidatorSet`

Full ValidatorSet, stored in @velor_framework.
1. join_validator_set adds to pending_active queue.
2. leave_valdiator_set moves from active to pending_inactive queue.
3. on_new_epoch processes two pending queues and refresh ValidatorInfo from the owner's address.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>consensus_scheme: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>active_validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>total_voting_power: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>total_joining_power: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_PendingTransactionFee"></a>

## Resource `PendingTransactionFee`

Transaction fee that is collected in current epoch, indexed by validator_index.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pending_fee_by_validator: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;u64, <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;u64&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_TransactionFeeConfig"></a>

## Enum Resource `TransactionFeeConfig`



<pre><code>enum <a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a> <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V0</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>max_fee_octa_allowed_per_epoch_per_pool: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_stake_DistributeTransactionFee"></a>

## Struct `DistributeTransactionFee`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_DistributeTransactionFee">DistributeTransactionFee</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee_amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_VelorCoinCapabilities"></a>

## Resource `VelorCoinCapabilities`

VelorCoin capabilities, set during genesis and stored in @CoreResource account.
This allows the Stake module to mint rewards to stakers.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_IndividualValidatorPerformance"></a>

## Struct `IndividualValidatorPerformance`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_IndividualValidatorPerformance">IndividualValidatorPerformance</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>successful_proposals: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>failed_proposals: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ValidatorPerformance"></a>

## Resource `ValidatorPerformance`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_IndividualValidatorPerformance">stake::IndividualValidatorPerformance</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RegisterValidatorCandidateEvent"></a>

## Struct `RegisterValidatorCandidateEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_RegisterValidatorCandidateEvent">RegisterValidatorCandidateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_StakeManagementPermission"></a>

## Struct `StakeManagementPermission`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_StakeManagementPermission">StakeManagementPermission</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RegisterValidatorCandidate"></a>

## Struct `RegisterValidatorCandidate`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_RegisterValidatorCandidate">RegisterValidatorCandidate</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_SetOperatorEvent"></a>

## Struct `SetOperatorEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_SetOperatorEvent">SetOperatorEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_operator: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_SetOperator"></a>

## Struct `SetOperator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_SetOperator">SetOperator</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_operator: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_AddStakeEvent"></a>

## Struct `AddStakeEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_AddStakeEvent">AddStakeEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_added: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_AddStake"></a>

## Struct `AddStake`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_AddStake">AddStake</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_added: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ReactivateStakeEvent"></a>

## Struct `ReactivateStakeEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ReactivateStakeEvent">ReactivateStakeEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ReactivateStake"></a>

## Struct `ReactivateStake`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_ReactivateStake">ReactivateStake</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RotateConsensusKeyEvent"></a>

## Struct `RotateConsensusKeyEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_RotateConsensusKeyEvent">RotateConsensusKeyEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RotateConsensusKey"></a>

## Struct `RotateConsensusKey`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_RotateConsensusKey">RotateConsensusKey</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_UpdateNetworkAndFullnodeAddressesEvent"></a>

## Struct `UpdateNetworkAndFullnodeAddressesEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddressesEvent">UpdateNetworkAndFullnodeAddressesEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_UpdateNetworkAndFullnodeAddresses"></a>

## Struct `UpdateNetworkAndFullnodeAddresses`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddresses">UpdateNetworkAndFullnodeAddresses</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_IncreaseLockupEvent"></a>

## Struct `IncreaseLockupEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_IncreaseLockupEvent">IncreaseLockupEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_locked_until_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_locked_until_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_IncreaseLockup"></a>

## Struct `IncreaseLockup`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_IncreaseLockup">IncreaseLockup</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_locked_until_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_locked_until_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_JoinValidatorSetEvent"></a>

## Struct `JoinValidatorSetEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_JoinValidatorSetEvent">JoinValidatorSetEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_JoinValidatorSet"></a>

## Struct `JoinValidatorSet`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_JoinValidatorSet">JoinValidatorSet</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_DistributeRewardsEvent"></a>

## Struct `DistributeRewardsEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_DistributeRewardsEvent">DistributeRewardsEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_DistributeRewards"></a>

## Struct `DistributeRewards`

The amount includes transaction fee and staking rewards.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_DistributeRewards">DistributeRewards</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_UnlockStakeEvent"></a>

## Struct `UnlockStakeEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_UnlockStakeEvent">UnlockStakeEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_unlocked: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_UnlockStake"></a>

## Struct `UnlockStake`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_UnlockStake">UnlockStake</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_unlocked: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_WithdrawStakeEvent"></a>

## Struct `WithdrawStakeEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_WithdrawStakeEvent">WithdrawStakeEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_withdrawn: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_WithdrawStake"></a>

## Struct `WithdrawStake`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_WithdrawStake">WithdrawStake</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_withdrawn: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_LeaveValidatorSetEvent"></a>

## Struct `LeaveValidatorSetEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_LeaveValidatorSetEvent">LeaveValidatorSetEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_LeaveValidatorSet"></a>

## Struct `LeaveValidatorSet`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="stake.md#0x1_stake_LeaveValidatorSet">LeaveValidatorSet</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ValidatorFees"></a>

## Resource `ValidatorFees`

DEPRECATED


<pre><code>#[deprecated]
<b>struct</b> <a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>fees_table: <a href="../../velor-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<b>address</b>, <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_AllowedValidators"></a>

## Resource `AllowedValidators`

This provides an ACL for Testnet purposes. In testnet, everyone is a whale, a whale can be a validator.
This allows a testnet to bring additional entities into the validator set without compromising the
security of the testnet. This will NOT be enabled in Mainnet.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>accounts: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_valid_perf"></a>

## Resource `Ghost$ghost_valid_perf`



<pre><code><b>struct</b> Ghost$<a href="stake.md#0x1_stake_ghost_valid_perf">ghost_valid_perf</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: <a href="stake.md#0x1_stake_ValidatorPerformance">stake::ValidatorPerformance</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_proposer_idx"></a>

## Resource `Ghost$ghost_proposer_idx`



<pre><code><b>struct</b> Ghost$<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_active_num"></a>

## Resource `Ghost$ghost_active_num`



<pre><code><b>struct</b> Ghost$<a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_pending_inactive_num"></a>

## Resource `Ghost$ghost_pending_inactive_num`



<pre><code><b>struct</b> Ghost$<a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_stake_MAX_U64"></a>



<pre><code><b>const</b> <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_stake_EALREADY_REGISTERED"></a>

Account is already registered as a validator candidate.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EALREADY_REGISTERED">EALREADY_REGISTERED</a>: u64 = 8;
</code></pre>



<a id="0x1_stake_MAX_REWARDS_RATE"></a>

Limit the maximum value of <code>rewards_rate</code> in order to avoid any arithmetic overflow.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>: u64 = 1000000;
</code></pre>



<a id="0x1_stake_EALREADY_ACTIVE_VALIDATOR"></a>

Account is already a validator or pending validator.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EALREADY_ACTIVE_VALIDATOR">EALREADY_ACTIVE_VALIDATOR</a>: u64 = 4;
</code></pre>



<a id="0x1_stake_EFEES_TABLE_ALREADY_EXISTS"></a>

Table to store collected transaction fees for each validator already exists.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EFEES_TABLE_ALREADY_EXISTS">EFEES_TABLE_ALREADY_EXISTS</a>: u64 = 19;
</code></pre>



<a id="0x1_stake_EINELIGIBLE_VALIDATOR"></a>

Validator is not defined in the ACL of entities allowed to be validators


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EINELIGIBLE_VALIDATOR">EINELIGIBLE_VALIDATOR</a>: u64 = 17;
</code></pre>



<a id="0x1_stake_EINVALID_LOCKUP"></a>

Cannot update stake pool's lockup to earlier than current lockup.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EINVALID_LOCKUP">EINVALID_LOCKUP</a>: u64 = 18;
</code></pre>



<a id="0x1_stake_EINVALID_PUBLIC_KEY"></a>

Invalid consensus public key


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>: u64 = 11;
</code></pre>



<a id="0x1_stake_ELAST_VALIDATOR"></a>

Can't remove last validator.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ELAST_VALIDATOR">ELAST_VALIDATOR</a>: u64 = 6;
</code></pre>



<a id="0x1_stake_ENOT_OPERATOR"></a>

Account does not have the right operator capability.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>: u64 = 9;
</code></pre>



<a id="0x1_stake_ENOT_VALIDATOR"></a>

Account is not a validator.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ENOT_VALIDATOR">ENOT_VALIDATOR</a>: u64 = 5;
</code></pre>



<a id="0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED"></a>

Validators cannot join or leave post genesis on this test network.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED">ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED</a>: u64 = 10;
</code></pre>



<a id="0x1_stake_ENO_STAKE_PERMISSION"></a>

Signer does not have permission to perform stake logic.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ENO_STAKE_PERMISSION">ENO_STAKE_PERMISSION</a>: u64 = 28;
</code></pre>



<a id="0x1_stake_EOWNER_CAP_ALREADY_EXISTS"></a>

An account cannot own more than one owner capability.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>: u64 = 16;
</code></pre>



<a id="0x1_stake_EOWNER_CAP_NOT_FOUND"></a>

Owner capability does not exist at the provided account.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>: u64 = 15;
</code></pre>



<a id="0x1_stake_ERECONFIGURATION_IN_PROGRESS"></a>

Validator set change temporarily disabled because of in-progress reconfiguration. Please retry after 1 minute.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ERECONFIGURATION_IN_PROGRESS">ERECONFIGURATION_IN_PROGRESS</a>: u64 = 20;
</code></pre>



<a id="0x1_stake_ESTAKE_EXCEEDS_MAX"></a>

Total stake exceeds maximum allowed.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ESTAKE_EXCEEDS_MAX">ESTAKE_EXCEEDS_MAX</a>: u64 = 7;
</code></pre>



<a id="0x1_stake_ESTAKE_POOL_DOES_NOT_EXIST"></a>

Stake pool does not exist at the provided pool address.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ESTAKE_POOL_DOES_NOT_EXIST">ESTAKE_POOL_DOES_NOT_EXIST</a>: u64 = 14;
</code></pre>



<a id="0x1_stake_ESTAKE_TOO_HIGH"></a>

Too much stake to join validator set.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ESTAKE_TOO_HIGH">ESTAKE_TOO_HIGH</a>: u64 = 3;
</code></pre>



<a id="0x1_stake_ESTAKE_TOO_LOW"></a>

Not enough stake to join validator set.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ESTAKE_TOO_LOW">ESTAKE_TOO_LOW</a>: u64 = 2;
</code></pre>



<a id="0x1_stake_ETRANSACTION_FEE_NOT_FULLY_DISTRIBUTED"></a>

Transaction fee is not fully distributed at epoch ending.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ETRANSACTION_FEE_NOT_FULLY_DISTRIBUTED">ETRANSACTION_FEE_NOT_FULLY_DISTRIBUTED</a>: u64 = 29;
</code></pre>



<a id="0x1_stake_EVALIDATOR_CONFIG"></a>

Validator Config not published.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EVALIDATOR_CONFIG">EVALIDATOR_CONFIG</a>: u64 = 1;
</code></pre>



<a id="0x1_stake_EVALIDATOR_SET_TOO_LARGE"></a>

Validator set exceeds the limit


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EVALIDATOR_SET_TOO_LARGE">EVALIDATOR_SET_TOO_LARGE</a>: u64 = 12;
</code></pre>



<a id="0x1_stake_EVOTING_POWER_INCREASE_EXCEEDS_LIMIT"></a>

Voting power increase has exceeded the limit for this current epoch.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EVOTING_POWER_INCREASE_EXCEEDS_LIMIT">EVOTING_POWER_INCREASE_EXCEEDS_LIMIT</a>: u64 = 13;
</code></pre>



<a id="0x1_stake_MAX_VALIDATOR_SET_SIZE"></a>

Limit the maximum size to u16::max, it's the current limit of the bitvec
https://github.com/velor-chain/velor-core/blob/main/crates/velor-bitvec/src/lib.rs#L20


<pre><code><b>const</b> <a href="stake.md#0x1_stake_MAX_VALIDATOR_SET_SIZE">MAX_VALIDATOR_SET_SIZE</a>: u64 = 65536;
</code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_ACTIVE"></a>



<pre><code><b>const</b> <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a>: u64 = 2;
</code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code><b>const</b> <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a>: u64 = 4;
</code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE"></a>

Validator status enum. We can switch to proper enum later once Move supports it.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE">VALIDATOR_STATUS_PENDING_ACTIVE</a>: u64 = 1;
</code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE"></a>



<pre><code><b>const</b> <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a>: u64 = 3;
</code></pre>



<a id="0x1_stake_check_stake_permission"></a>

## Function `check_stake_permission`

Permissions


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(s: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(s: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">permissioned_signer::check_permission_exists</a>(s, <a href="stake.md#0x1_stake_StakeManagementPermission">StakeManagementPermission</a> {}),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="stake.md#0x1_stake_ENO_STAKE_PERMISSION">ENO_STAKE_PERMISSION</a>),
    );
}
</code></pre>



</details>

<a id="0x1_stake_grant_permission"></a>

## Function `grant_permission`

Grant permission to mutate staking on behalf of the master signer.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_grant_permission">grant_permission</a>(master: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_grant_permission">grant_permission</a>(master: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">permissioned_signer::authorize_unlimited</a>(master, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>, <a href="stake.md#0x1_stake_StakeManagementPermission">StakeManagementPermission</a> {})
}
</code></pre>



</details>

<a id="0x1_stake_get_lockup_secs"></a>

## Function `get_lockup_secs`

Return the lockup expiration of the stake pool at <code>pool_address</code>.
This will throw an error if there's no stake pool at <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_lockup_secs">get_lockup_secs</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_lockup_secs">get_lockup_secs</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).locked_until_secs
}
</code></pre>



</details>

<a id="0x1_stake_get_remaining_lockup_secs"></a>

## Function `get_remaining_lockup_secs`

Return the remaining lockup of the stake pool at <code>pool_address</code>.
This will throw an error if there's no stake pool at <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_remaining_lockup_secs">get_remaining_lockup_secs</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_remaining_lockup_secs">get_remaining_lockup_secs</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> lockup_time = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).locked_until_secs;
    <b>if</b> (lockup_time &lt;= <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()) {
        0
    } <b>else</b> {
        lockup_time - <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()
    }
}
</code></pre>



</details>

<a id="0x1_stake_get_stake"></a>

## Function `get_stake`

Return the different stake amounts for <code>pool_address</code> (whether the validator is active or not).
The returned amounts are for (active, inactive, pending_active, pending_inactive) stake respectively.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_stake">get_stake</a>(pool_address: <b>address</b>): (u64, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_stake">get_stake</a>(pool_address: <b>address</b>): (u64, u64, u64, u64) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> stake_pool = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    (
        <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.active),
        <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.inactive),
        <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.pending_active),
        <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.pending_inactive),
    )
}
</code></pre>



</details>

<a id="0x1_stake_get_validator_state"></a>

## Function `get_validator_state`

Returns the validator's state.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <b>let</b> validator_set = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&validator_set.pending_active, pool_address))) {
        <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE">VALIDATOR_STATUS_PENDING_ACTIVE</a>
    } <b>else</b> <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&validator_set.active_validators, pool_address))) {
        <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a>
    } <b>else</b> <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&validator_set.pending_inactive, pool_address))) {
        <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a>
    } <b>else</b> {
        <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a>
    }
}
</code></pre>



</details>

<a id="0x1_stake_get_current_epoch_voting_power"></a>

## Function `get_current_epoch_voting_power`

Return the voting power of the validator in the current epoch.
This is the same as the validator's total active and pending_inactive stake.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_current_epoch_voting_power">get_current_epoch_voting_power</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_current_epoch_voting_power">get_current_epoch_voting_power</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> validator_state = <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address);
    // Both active and pending inactive validators can still vote in the current epoch.
    <b>if</b> (validator_state == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a> || validator_state == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a>) {
        <b>let</b> active_stake = <a href="coin.md#0x1_coin_value">coin::value</a>(&<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).active);
        <b>let</b> pending_inactive_stake = <a href="coin.md#0x1_coin_value">coin::value</a>(&<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).pending_inactive);
        active_stake + pending_inactive_stake
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x1_stake_get_delegated_voter"></a>

## Function `get_delegated_voter`

Return the delegated voter of the validator at <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_delegated_voter">get_delegated_voter</a>(pool_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_delegated_voter">get_delegated_voter</a>(pool_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).delegated_voter
}
</code></pre>



</details>

<a id="0x1_stake_get_operator"></a>

## Function `get_operator`

Return the operator of the validator at <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_operator">get_operator</a>(pool_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_operator">get_operator</a>(pool_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).operator_address
}
</code></pre>



</details>

<a id="0x1_stake_get_owned_pool_address"></a>

## Function `get_owned_pool_address`

Return the pool address in <code>owner_cap</code>.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_owned_pool_address">get_owned_pool_address</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_owned_pool_address">get_owned_pool_address</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>): <b>address</b> {
    owner_cap.pool_address
}
</code></pre>



</details>

<a id="0x1_stake_get_validator_index"></a>

## Function `get_validator_index`

Return the validator index for <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_index">get_validator_index</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_index">get_validator_index</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address).validator_index
}
</code></pre>



</details>

<a id="0x1_stake_get_current_epoch_proposal_counts"></a>

## Function `get_current_epoch_proposal_counts`

Return the number of successful and failed proposals for the proposal at the given validator index.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_current_epoch_proposal_counts">get_current_epoch_proposal_counts</a>(validator_index: u64): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_current_epoch_proposal_counts">get_current_epoch_proposal_counts</a>(validator_index: u64): (u64, u64) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> {
    <b>let</b> validator_performances = &<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework).validators;
    <b>let</b> validator_performance = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(validator_performances, validator_index);
    (validator_performance.successful_proposals, validator_performance.failed_proposals)
}
</code></pre>



</details>

<a id="0x1_stake_get_validator_config"></a>

## Function `get_validator_config`

Return the validator's config.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_config">get_validator_config</a>(pool_address: <b>address</b>): (<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_config">get_validator_config</a>(
    pool_address: <b>address</b>
): (<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> validator_config = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
    (validator_config.consensus_pubkey, validator_config.network_addresses, validator_config.fullnode_addresses)
}
</code></pre>



</details>

<a id="0x1_stake_stake_pool_exists"></a>

## Function `stake_pool_exists`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_stake_get_pending_transaction_fee"></a>

## Function `get_pending_transaction_fee`

Returns the pending transaction fee that is accumulated in current epoch.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_pending_transaction_fee">get_pending_transaction_fee</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_pending_transaction_fee">get_pending_transaction_fee</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; <b>acquires</b> <a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a> {
    <b>let</b> result = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> fee_table = &<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>&gt;(@velor_framework).pending_fee_by_validator;
    <b>let</b> num_validators = fee_table.compute_length();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_validators) {
        result.push_back(fee_table.borrow(&i).read());
        i = i + 1;
    };

    result
}
</code></pre>



</details>

<a id="0x1_stake_initialize"></a>

## Function `initialize`

Initialize validator set to the core resource account.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize">initialize</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize">initialize</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);

    <b>move_to</b>(velor_framework, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
        consensus_scheme: 0,
        active_validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        pending_active: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        pending_inactive: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        total_voting_power: 0,
        total_joining_power: 0,
    });

    <b>move_to</b>(velor_framework, <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> {
        validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
    });
}
</code></pre>



</details>

<a id="0x1_stake_store_velor_coin_mint_cap"></a>

## Function `store_velor_coin_mint_cap`

This is only called during Genesis, which is where MintCapability<VelorCoin> can be created.
Beyond genesis, no one can create VelorCoin mint/burn capabilities.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_store_velor_coin_mint_cap">store_velor_coin_mint_cap</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_store_velor_coin_mint_cap">store_velor_coin_mint_cap</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: MintCapability&lt;VelorCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <b>move_to</b>(velor_framework, <a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a> { mint_cap })
}
</code></pre>



</details>

<a id="0x1_stake_remove_validators"></a>

## Function `remove_validators`

Allow on chain governance to remove validators from the validator set.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_remove_validators">remove_validators</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_remove_validators">remove_validators</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    validators: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <b>let</b> validator_set = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <b>let</b> active_validators = &<b>mut</b> validator_set.active_validators;
    <b>let</b> pending_inactive = &<b>mut</b> validator_set.pending_inactive;
    <b>spec</b> {
        <b>update</b> <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> = len(active_validators);
        <b>update</b> <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> = len(pending_inactive);
    };
    <b>let</b> len_validators = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validators);
    <b>let</b> i = 0;
    // Remove each validator from the validator set.
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> i &lt;= len_validators;
            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(active_validators);
            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(active_validators);
            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(pending_inactive);
            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(pending_inactive);
            <b>invariant</b> <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> + <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> == len(active_validators) + len(pending_inactive);
        };
        i &lt; len_validators
    }) {
        <b>let</b> validator = *<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(validators, i);
        <b>let</b> validator_index = <a href="stake.md#0x1_stake_find_validator">find_validator</a>(active_validators, validator);
        <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&validator_index)) {
            <b>let</b> validator_info = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(active_validators, *<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&validator_index));
            <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(pending_inactive, validator_info);
            <b>spec</b> {
                <b>update</b> <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> = <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> - 1;
                <b>update</b> <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> = <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> + 1;
            };
        };
        i = i + 1;
    };
}
</code></pre>



</details>

<a id="0x1_stake_initialize_pending_transaction_fee"></a>

## Function `initialize_pending_transaction_fee`



<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_initialize_pending_transaction_fee">initialize_pending_transaction_fee</a>(framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_initialize_pending_transaction_fee">initialize_pending_transaction_fee</a>(framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(framework);

    <b>if</b> (!<b>exists</b>&lt;<a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>&gt;(@velor_framework)) {
        <b>move_to</b>(framework, <a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a> {
            // The max leaf order is set <b>to</b> 10 because there is a existing limitation that a
            // resource can only have 10 aggregators at max.
            pending_fee_by_validator: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_config">big_ordered_map::new_with_config</a>(5, 10, <b>true</b>),
        });
    }
}
</code></pre>



</details>

<a id="0x1_stake_set_transaction_fee_config"></a>

## Function `set_transaction_fee_config`



<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_transaction_fee_config">set_transaction_fee_config</a>(framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="stake.md#0x1_stake_TransactionFeeConfig">stake::TransactionFeeConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_transaction_fee_config">set_transaction_fee_config</a>(framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a>) <b>acquires</b> <a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(framework);

    <b>if</b> (<b>exists</b>&lt;<a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a>&gt;(@velor_framework)) {
        *<b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a>&gt;(@velor_framework) = config;
    } <b>else</b> {
        <b>move_to</b>(framework, config);
    }
}
</code></pre>



</details>

<a id="0x1_stake_record_fee"></a>

## Function `record_fee`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_record_fee">record_fee</a>(vm: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fee_distribution_validator_indices: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, fee_amounts_octa: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_record_fee">record_fee</a>(
    vm: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    fee_distribution_validator_indices: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    fee_amounts_octa: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
) <b>acquires</b> <a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a> {
    // Operational constraint: can only be invoked by the VM.
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);

    <b>assert</b>!(fee_distribution_validator_indices.length() == fee_amounts_octa.length());

    <b>let</b> num_validators_to_distribute = fee_distribution_validator_indices.length();
    <b>let</b> pending_fee = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>&gt;(@velor_framework);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_validators_to_distribute) {
        <b>let</b> validator_index = fee_distribution_validator_indices[i];
        <b>let</b> fee_octa = fee_amounts_octa[i];
        pending_fee.pending_fee_by_validator.borrow_mut(&validator_index).add(fee_octa);
        i = i + 1;
    }
}
</code></pre>



</details>

<a id="0x1_stake_initialize_stake_owner"></a>

## Function `initialize_stake_owner`

Initialize the validator account and give ownership to the signing account
except it leaves the ValidatorConfig to be set by another entity.
Note: this triggers setting the operator and owner, set it to the account's address
to set later.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_stake_owner">initialize_stake_owner</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_stake_amount: u64, operator: <b>address</b>, voter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_stake_owner">initialize_stake_owner</a>(
    owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    initial_stake_amount: u64,
    operator: <b>address</b>,
    voter: <b>address</b>,
) <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>, <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <a href="stake.md#0x1_stake_initialize_owner">initialize_owner</a>(owner);
    <b>move_to</b>(owner, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
        consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        validator_index: 0,
    });

    <b>if</b> (initial_stake_amount &gt; 0) {
        <a href="stake.md#0x1_stake_add_stake">add_stake</a>(owner, initial_stake_amount);
    };

    <b>let</b> account_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <b>if</b> (account_address != operator) {
        <a href="stake.md#0x1_stake_set_operator">set_operator</a>(owner, operator)
    };
    <b>if</b> (account_address != voter) {
        <a href="stake.md#0x1_stake_set_delegated_voter">set_delegated_voter</a>(owner, voter)
    };
}
</code></pre>



</details>

<a id="0x1_stake_initialize_validator"></a>

## Function `initialize_validator`

Initialize the validator account and give ownership to the signing account.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator">initialize_validator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_of_possession: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator">initialize_validator</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof_of_possession: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(<a href="account.md#0x1_account">account</a>);
    // Checks the <b>public</b> key <b>has</b> a valid proof-of-possession <b>to</b> prevent rogue-key attacks.
    <b>let</b> pubkey_from_pop = &<a href="../../velor-stdlib/doc/bls12381.md#0x1_bls12381_public_key_from_bytes_with_pop">bls12381::public_key_from_bytes_with_pop</a>(
        consensus_pubkey,
        &proof_of_possession_from_bytes(proof_of_possession)
    );
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(pubkey_from_pop), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>));

    <a href="stake.md#0x1_stake_initialize_owner">initialize_owner</a>(<a href="account.md#0x1_account">account</a>);
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
        consensus_pubkey,
        network_addresses,
        fullnode_addresses,
        validator_index: 0,
    });
}
</code></pre>



</details>

<a id="0x1_stake_initialize_owner"></a>

## Function `initialize_owner`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_initialize_owner">initialize_owner</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_initialize_owner">initialize_owner</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <b>assert</b>!(<a href="stake.md#0x1_stake_is_allowed">is_allowed</a>(owner_address), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EINELIGIBLE_VALIDATOR">EINELIGIBLE_VALIDATOR</a>));
    <b>assert</b>!(!<a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(owner_address), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="stake.md#0x1_stake_EALREADY_REGISTERED">EALREADY_REGISTERED</a>));

    <b>move_to</b>(owner, <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
        active: <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;VelorCoin&gt;(),
        pending_active: <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;VelorCoin&gt;(),
        pending_inactive: <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;VelorCoin&gt;(),
        inactive: <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;VelorCoin&gt;(),
        locked_until_secs: 0,
        operator_address: owner_address,
        delegated_voter: owner_address,
        // Events.
        initialize_validator_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_RegisterValidatorCandidateEvent">RegisterValidatorCandidateEvent</a>&gt;(owner),
        set_operator_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_SetOperatorEvent">SetOperatorEvent</a>&gt;(owner),
        add_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_AddStakeEvent">AddStakeEvent</a>&gt;(owner),
        reactivate_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_ReactivateStakeEvent">ReactivateStakeEvent</a>&gt;(owner),
        rotate_consensus_key_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_RotateConsensusKeyEvent">RotateConsensusKeyEvent</a>&gt;(owner),
        update_network_and_fullnode_addresses_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddressesEvent">UpdateNetworkAndFullnodeAddressesEvent</a>&gt;(
            owner
        ),
        increase_lockup_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_IncreaseLockupEvent">IncreaseLockupEvent</a>&gt;(owner),
        join_validator_set_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_JoinValidatorSetEvent">JoinValidatorSetEvent</a>&gt;(owner),
        distribute_rewards_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_DistributeRewardsEvent">DistributeRewardsEvent</a>&gt;(owner),
        unlock_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_UnlockStakeEvent">UnlockStakeEvent</a>&gt;(owner),
        withdraw_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_WithdrawStakeEvent">WithdrawStakeEvent</a>&gt;(owner),
        leave_validator_set_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_LeaveValidatorSetEvent">LeaveValidatorSetEvent</a>&gt;(owner),
    });

    <b>move_to</b>(owner, <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> { pool_address: owner_address });
}
</code></pre>



</details>

<a id="0x1_stake_extract_owner_cap"></a>

## Function `extract_owner_cap`

Extract and return owner capability from the signing account.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_extract_owner_cap">extract_owner_cap</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_extract_owner_cap">extract_owner_cap</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);
    <b>move_from</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address)
}
</code></pre>



</details>

<a id="0x1_stake_deposit_owner_cap"></a>

## Function `deposit_owner_cap`

Deposit <code>owner_cap</code> into <code><a href="account.md#0x1_account">account</a></code>. This requires <code><a href="account.md#0x1_account">account</a></code> to not already have ownership of another
staking pool.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_deposit_owner_cap">deposit_owner_cap</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_deposit_owner_cap">deposit_owner_cap</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>) {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <b>assert</b>!(!<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner)), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>));
    <b>move_to</b>(owner, owner_cap);
}
</code></pre>



</details>

<a id="0x1_stake_destroy_owner_cap"></a>

## Function `destroy_owner_cap`

Destroy <code>owner_cap</code>.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_destroy_owner_cap">destroy_owner_cap</a>(owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_destroy_owner_cap">destroy_owner_cap</a>(owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>) {
    <b>let</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> { pool_address: _ } = owner_cap;
}
</code></pre>



</details>

<a id="0x1_stake_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_set_operator">set_operator</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_set_operator">set_operator</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);
    <b>let</b> ownership_cap = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <a href="stake.md#0x1_stake_set_operator_with_cap">set_operator_with_cap</a>(ownership_cap, new_operator);
}
</code></pre>



</details>

<a id="0x1_stake_set_operator_with_cap"></a>

## Function `set_operator_with_cap`

Allows an account with ownership capability to change the operator of the stake pool.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_operator_with_cap">set_operator_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, new_operator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_operator_with_cap">set_operator_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, new_operator: <b>address</b>) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <b>let</b> pool_address = owner_cap.pool_address;
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>let</b> old_operator = stake_pool.operator_address;
    stake_pool.operator_address = new_operator;

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="stake.md#0x1_stake_SetOperator">SetOperator</a> {
                pool_address,
                old_operator,
                new_operator,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.set_operator_events,
            <a href="stake.md#0x1_stake_SetOperatorEvent">SetOperatorEvent</a> {
                pool_address,
                old_operator,
                new_operator,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_stake_set_delegated_voter"></a>

## Function `set_delegated_voter`

Allows an owner to change the delegated voter of the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter">set_delegated_voter</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter">set_delegated_voter</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);
    <b>let</b> ownership_cap = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">set_delegated_voter_with_cap</a>(ownership_cap, new_voter);
}
</code></pre>



</details>

<a id="0x1_stake_set_delegated_voter_with_cap"></a>

## Function `set_delegated_voter_with_cap`

Allows an owner to change the delegated voter of the stake pool.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">set_delegated_voter_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, new_voter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">set_delegated_voter_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, new_voter: <b>address</b>) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <b>let</b> pool_address = owner_cap.pool_address;
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    stake_pool.delegated_voter = new_voter;
}
</code></pre>



</details>

<a id="0x1_stake_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins from the <code><a href="account.md#0x1_account">account</a></code> owning the StakePool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_add_stake">add_stake</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_add_stake">add_stake</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);
    <b>let</b> ownership_cap = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <a href="stake.md#0x1_stake_add_stake_with_cap">add_stake_with_cap</a>(ownership_cap, <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;VelorCoin&gt;(owner, amount));
}
</code></pre>



</details>

<a id="0x1_stake_add_stake_with_cap"></a>

## Function `add_stake_with_cap`

Add <code>coins</code> into <code>pool_address</code>. this requires the corresponding <code>owner_cap</code> to be passed in.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_add_stake_with_cap">add_stake_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_add_stake_with_cap">add_stake_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, coins: Coin&lt;VelorCoin&gt;) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <b>let</b> pool_address = owner_cap.pool_address;
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);

    <b>let</b> amount = <a href="coin.md#0x1_coin_value">coin::value</a>(&coins);
    <b>if</b> (amount == 0) {
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);
        <b>return</b>
    };

    // Only track and validate <a href="voting.md#0x1_voting">voting</a> power increase for active and pending_active validator.
    // Pending_inactive validator will be removed from the validator set in the next epoch.
    // Inactive validator's total <a href="stake.md#0x1_stake">stake</a> will be tracked when they join the validator set.
    <b>let</b> validator_set = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    // Search directly rather using get_validator_state <b>to</b> save on unnecessary loops.
    <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&validator_set.active_validators, pool_address)) ||
        <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&validator_set.pending_active, pool_address))) {
        <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(amount);
    };

    // Add <b>to</b> pending_active <b>if</b> it's a current validator because the <a href="stake.md#0x1_stake">stake</a> is not counted until the next epoch.
    // Otherwise, the delegation can be added <b>to</b> active directly <b>as</b> the validator is also activated in the epoch.
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>if</b> (<a href="stake.md#0x1_stake_is_current_epoch_validator">is_current_epoch_validator</a>(pool_address)) {
        <a href="coin.md#0x1_coin_merge">coin::merge</a>&lt;VelorCoin&gt;(&<b>mut</b> stake_pool.pending_active, coins);
    } <b>else</b> {
        <a href="coin.md#0x1_coin_merge">coin::merge</a>&lt;VelorCoin&gt;(&<b>mut</b> stake_pool.active, coins);
    };

    <b>let</b> (_, maximum_stake) = <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());
    <b>let</b> voting_power = <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool);
    <b>assert</b>!(voting_power &lt;= maximum_stake, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ESTAKE_EXCEEDS_MAX">ESTAKE_EXCEEDS_MAX</a>));

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="stake.md#0x1_stake_AddStake">AddStake</a> {
                pool_address,
                amount_added: amount,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.add_stake_events,
            <a href="stake.md#0x1_stake_AddStakeEvent">AddStakeEvent</a> {
                pool_address,
                amount_added: amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_stake_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake">reactivate_stake</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake">reactivate_stake</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);
    <b>let</b> ownership_cap = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <a href="stake.md#0x1_stake_reactivate_stake_with_cap">reactivate_stake_with_cap</a>(ownership_cap, amount);
}
</code></pre>



</details>

<a id="0x1_stake_reactivate_stake_with_cap"></a>

## Function `reactivate_stake_with_cap`



<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake_with_cap">reactivate_stake_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake_with_cap">reactivate_stake_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <b>let</b> pool_address = owner_cap.pool_address;
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);

    // Cap the amount <b>to</b> reactivate by the amount in pending_inactive.
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>let</b> total_pending_inactive = <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.pending_inactive);
    amount = <b>min</b>(amount, total_pending_inactive);

    // Since this does not count <b>as</b> a <a href="voting.md#0x1_voting">voting</a> power change (pending inactive still counts <b>as</b> <a href="voting.md#0x1_voting">voting</a> power in the
    // current epoch), <a href="stake.md#0x1_stake">stake</a> can be immediately moved from pending inactive <b>to</b> active.
    // We also don't need <b>to</b> check <a href="voting.md#0x1_voting">voting</a> power increase <b>as</b> there's none.
    <b>let</b> reactivated_coins = <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> stake_pool.pending_inactive, amount);
    <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> stake_pool.active, reactivated_coins);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="stake.md#0x1_stake_ReactivateStake">ReactivateStake</a> {
                pool_address,
                amount,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.reactivate_stake_events,
            <a href="stake.md#0x1_stake_ReactivateStakeEvent">ReactivateStakeEvent</a> {
                pool_address,
                amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_stake_rotate_consensus_key"></a>

## Function `rotate_consensus_key`

Rotate the consensus key of the validator, it'll take effect in next epoch.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_rotate_consensus_key">rotate_consensus_key</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_of_possession: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_rotate_consensus_key">rotate_consensus_key</a>(
    operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    new_consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof_of_possession: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(operator);
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);

    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) == stake_pool.operator_address, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>));

    <b>assert</b>!(<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EVALIDATOR_CONFIG">EVALIDATOR_CONFIG</a>));
    <b>let</b> validator_info = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
    <b>let</b> old_consensus_pubkey = validator_info.consensus_pubkey;
    // Checks the <b>public</b> key <b>has</b> a valid proof-of-possession <b>to</b> prevent rogue-key attacks.
    <b>let</b> pubkey_from_pop = &<a href="../../velor-stdlib/doc/bls12381.md#0x1_bls12381_public_key_from_bytes_with_pop">bls12381::public_key_from_bytes_with_pop</a>(
        new_consensus_pubkey,
        &proof_of_possession_from_bytes(proof_of_possession)
    );
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(pubkey_from_pop), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>));
    validator_info.consensus_pubkey = new_consensus_pubkey;

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="stake.md#0x1_stake_RotateConsensusKey">RotateConsensusKey</a> {
                pool_address,
                old_consensus_pubkey,
                new_consensus_pubkey,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.rotate_consensus_key_events,
            <a href="stake.md#0x1_stake_RotateConsensusKeyEvent">RotateConsensusKeyEvent</a> {
                pool_address,
                old_consensus_pubkey,
                new_consensus_pubkey,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_stake_update_network_and_fullnode_addresses"></a>

## Function `update_network_and_fullnode_addresses`

Update the network and full node addresses of the validator. This only takes effect in the next epoch.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_update_network_and_fullnode_addresses">update_network_and_fullnode_addresses</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, new_fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_update_network_and_fullnode_addresses">update_network_and_fullnode_addresses</a>(
    operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    new_network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    new_fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(operator);
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) == stake_pool.operator_address, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>));
    <b>assert</b>!(<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EVALIDATOR_CONFIG">EVALIDATOR_CONFIG</a>));
    <b>let</b> validator_info = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
    <b>let</b> old_network_addresses = validator_info.network_addresses;
    validator_info.network_addresses = new_network_addresses;
    <b>let</b> old_fullnode_addresses = validator_info.fullnode_addresses;
    validator_info.fullnode_addresses = new_fullnode_addresses;

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddresses">UpdateNetworkAndFullnodeAddresses</a> {
                pool_address,
                old_network_addresses,
                new_network_addresses,
                old_fullnode_addresses,
                new_fullnode_addresses,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.update_network_and_fullnode_addresses_events,
            <a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddressesEvent">UpdateNetworkAndFullnodeAddressesEvent</a> {
                pool_address,
                old_network_addresses,
                new_network_addresses,
                old_fullnode_addresses,
                new_fullnode_addresses,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_stake_increase_lockup"></a>

## Function `increase_lockup`

Similar to increase_lockup_with_cap but will use ownership capability from the signing account.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup">increase_lockup</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup">increase_lockup</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);
    <b>let</b> ownership_cap = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <a href="stake.md#0x1_stake_increase_lockup_with_cap">increase_lockup_with_cap</a>(ownership_cap);
}
</code></pre>



</details>

<a id="0x1_stake_increase_lockup_with_cap"></a>

## Function `increase_lockup_with_cap`

Unlock from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
directly inactive if it's not from an active validator.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup_with_cap">increase_lockup_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup_with_cap">increase_lockup_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <b>let</b> pool_address = owner_cap.pool_address;
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> config = <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();

    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>let</b> old_locked_until_secs = stake_pool.locked_until_secs;
    <b>let</b> new_locked_until_secs = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() + <a href="staking_config.md#0x1_staking_config_get_recurring_lockup_duration">staking_config::get_recurring_lockup_duration</a>(&config);
    <b>assert</b>!(old_locked_until_secs &lt; new_locked_until_secs, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EINVALID_LOCKUP">EINVALID_LOCKUP</a>));
    stake_pool.locked_until_secs = new_locked_until_secs;

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="stake.md#0x1_stake_IncreaseLockup">IncreaseLockup</a> {
                pool_address,
                old_locked_until_secs,
                new_locked_until_secs,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.increase_lockup_events,
            <a href="stake.md#0x1_stake_IncreaseLockupEvent">IncreaseLockupEvent</a> {
                pool_address,
                old_locked_until_secs,
                new_locked_until_secs,
            },
        );
    }
}
</code></pre>



</details>

<a id="0x1_stake_join_validator_set"></a>

## Function `join_validator_set`

This can only called by the operator of the validator/staking pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set">join_validator_set</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set">join_validator_set</a>(
    operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>
) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(operator);
    <b>assert</b>!(
        <a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(&<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>()),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED">ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED</a>),
    );

    <a href="stake.md#0x1_stake_join_validator_set_internal">join_validator_set_internal</a>(operator, pool_address);
}
</code></pre>



</details>

<a id="0x1_stake_join_validator_set_internal"></a>

## Function `join_validator_set_internal`

Request to have <code>pool_address</code> join the validator set. Can only be called after calling <code>initialize_validator</code>.
If the validator has the required stake (more than minimum and less than maximum allowed), they will be
added to the pending_active queue. All validators in this queue will be added to the active set when the next
epoch starts (eligibility will be rechecked).

This internal version can only be called by the Genesis module during Genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set_internal">join_validator_set_internal</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set_internal">join_validator_set_internal</a>(
    operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>
) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) == stake_pool.operator_address, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>));
    <b>assert</b>!(
        <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address) == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a>,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="stake.md#0x1_stake_EALREADY_ACTIVE_VALIDATOR">EALREADY_ACTIVE_VALIDATOR</a>),
    );

    <b>let</b> config = <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();
    <b>let</b> (minimum_stake, maximum_stake) = <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&config);
    <b>let</b> voting_power = <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool);
    <b>assert</b>!(voting_power &gt;= minimum_stake, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ESTAKE_TOO_LOW">ESTAKE_TOO_LOW</a>));
    <b>assert</b>!(voting_power &lt;= maximum_stake, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ESTAKE_TOO_HIGH">ESTAKE_TOO_HIGH</a>));

    // Track and validate <a href="voting.md#0x1_voting">voting</a> power increase.
    <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(voting_power);

    // Add validator <b>to</b> pending_active, <b>to</b> be activated in the next epoch.
    <b>let</b> validator_config = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
    <b>assert</b>!(!<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&validator_config.consensus_pubkey), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>));

    // Validate the current validator set size <b>has</b> not exceeded the limit.
    <b>let</b> validator_set = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(
        &<b>mut</b> validator_set.pending_active,
        <a href="stake.md#0x1_stake_generate_validator_info">generate_validator_info</a>(pool_address, stake_pool, *validator_config)
    );
    <b>let</b> validator_set_size = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validator_set.active_validators) + <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(
        &validator_set.pending_active
    );
    <b>assert</b>!(validator_set_size &lt;= <a href="stake.md#0x1_stake_MAX_VALIDATOR_SET_SIZE">MAX_VALIDATOR_SET_SIZE</a>, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EVALIDATOR_SET_TOO_LARGE">EVALIDATOR_SET_TOO_LARGE</a>));

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="stake.md#0x1_stake_JoinValidatorSet">JoinValidatorSet</a> { pool_address });
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.join_validator_set_events,
            <a href="stake.md#0x1_stake_JoinValidatorSetEvent">JoinValidatorSetEvent</a> { pool_address },
        );
    }
}
</code></pre>



</details>

<a id="0x1_stake_unlock"></a>

## Function `unlock`

Similar to unlock_with_cap but will use ownership capability from the signing account.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_unlock">unlock</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_unlock">unlock</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);
    <b>let</b> ownership_cap = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <a href="stake.md#0x1_stake_unlock_with_cap">unlock_with_cap</a>(amount, ownership_cap);
}
</code></pre>



</details>

<a id="0x1_stake_unlock_with_cap"></a>

## Function `unlock_with_cap`

Unlock <code>amount</code> from the active stake. Only possible if the lockup has expired.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_unlock_with_cap">unlock_with_cap</a>(amount: u64, owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_unlock_with_cap">unlock_with_cap</a>(amount: u64, owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> {
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    // Short-circuit <b>if</b> amount <b>to</b> unlock is 0 so we don't emit events.
    <b>if</b> (amount == 0) {
        <b>return</b>
    };

    // Unlocked coins are moved <b>to</b> pending_inactive. When the current lockup cycle expires, they will be moved into
    // inactive in the earliest possible epoch transition.
    <b>let</b> pool_address = owner_cap.pool_address;
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    // Cap amount <b>to</b> unlock by maximum active <a href="stake.md#0x1_stake">stake</a>.
    <b>let</b> amount = <b>min</b>(amount, <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.active));
    <b>let</b> unlocked_stake = <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> stake_pool.active, amount);
    <a href="coin.md#0x1_coin_merge">coin::merge</a>&lt;VelorCoin&gt;(&<b>mut</b> stake_pool.pending_inactive, unlocked_stake);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="stake.md#0x1_stake_UnlockStake">UnlockStake</a> {
                pool_address,
                amount_unlocked: amount,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.unlock_stake_events,
            <a href="stake.md#0x1_stake_UnlockStakeEvent">UnlockStakeEvent</a> {
                pool_address,
                amount_unlocked: amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_stake_withdraw"></a>

## Function `withdraw`

Withdraw from <code><a href="account.md#0x1_account">account</a></code>'s inactive stake.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_withdraw">withdraw</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, withdraw_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_withdraw">withdraw</a>(
    owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    withdraw_amount: u64
) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(owner);
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);
    <b>let</b> ownership_cap = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <b>let</b> coins = <a href="stake.md#0x1_stake_withdraw_with_cap">withdraw_with_cap</a>(ownership_cap, withdraw_amount);
    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>&lt;VelorCoin&gt;(owner_address, coins);
}
</code></pre>



</details>

<a id="0x1_stake_withdraw_with_cap"></a>

## Function `withdraw_with_cap`

Withdraw from <code>pool_address</code>'s inactive stake with the corresponding <code>owner_cap</code>.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_withdraw_with_cap">withdraw_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, withdraw_amount: u64): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_withdraw_with_cap">withdraw_with_cap</a>(
    owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>,
    withdraw_amount: u64
): Coin&lt;VelorCoin&gt; <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <b>let</b> pool_address = owner_cap.pool_address;
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    // There's an edge case <b>where</b> a validator unlocks their <a href="stake.md#0x1_stake">stake</a> and leaves the validator set before
    // the <a href="stake.md#0x1_stake">stake</a> is fully unlocked (the current lockup cycle <b>has</b> not expired yet).
    // This can leave their <a href="stake.md#0x1_stake">stake</a> stuck in pending_inactive even after the current lockup cycle expires.
    <b>if</b> (<a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address) == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a> &&
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt;= stake_pool.locked_until_secs) {
        <b>let</b> pending_inactive_stake = <a href="coin.md#0x1_coin_extract_all">coin::extract_all</a>(&<b>mut</b> stake_pool.pending_inactive);
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> stake_pool.inactive, pending_inactive_stake);
    };

    // Cap withdraw amount by total inactive coins.
    withdraw_amount = <b>min</b>(withdraw_amount, <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.inactive));
    <b>if</b> (withdraw_amount == 0) <b>return</b> <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;VelorCoin&gt;();

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="stake.md#0x1_stake_WithdrawStake">WithdrawStake</a> {
                pool_address,
                amount_withdrawn: withdraw_amount,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.withdraw_stake_events,
            <a href="stake.md#0x1_stake_WithdrawStakeEvent">WithdrawStakeEvent</a> {
                pool_address,
                amount_withdrawn: withdraw_amount,
            },
        );
    };

    <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> stake_pool.inactive, withdraw_amount)
}
</code></pre>



</details>

<a id="0x1_stake_leave_validator_set"></a>

## Function `leave_validator_set`

Request to have <code>pool_address</code> leave the validator set. The validator is only actually removed from the set when
the next epoch starts.
The last validator in the set cannot leave. This is an edge case that should never happen as long as the network
is still operational.

Can only be called by the operator of the validator/staking pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_leave_validator_set">leave_validator_set</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_leave_validator_set">leave_validator_set</a>(
    operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>
) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_check_stake_permission">check_stake_permission</a>(operator);
    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();
    <b>let</b> config = <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();
    <b>assert</b>!(
        <a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(&config),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED">ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED</a>),
    );

    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    // Account <b>has</b> <b>to</b> be the operator.
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) == stake_pool.operator_address, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>));

    <b>let</b> validator_set = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    // If the validator is still pending_active, directly kick the validator out.
    <b>let</b> maybe_pending_active_index = <a href="stake.md#0x1_stake_find_validator">find_validator</a>(&validator_set.pending_active, pool_address);
    <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&maybe_pending_active_index)) {
        <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(
            &<b>mut</b> validator_set.pending_active, <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> maybe_pending_active_index));

        // Decrease the <a href="voting.md#0x1_voting">voting</a> power increase <b>as</b> the pending validator's <a href="voting.md#0x1_voting">voting</a> power was added when they requested
        // <b>to</b> join. Now that they changed their mind, their <a href="voting.md#0x1_voting">voting</a> power should not affect the joining limit of this
        // epoch.
        <b>let</b> validator_stake = (<a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool) <b>as</b> u128);
        // total_joining_power should be larger than validator_stake but just in case there <b>has</b> been a small
        // rounding <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> somewhere that can lead <b>to</b> an underflow, we still want <b>to</b> allow this transaction <b>to</b>
        // succeed.
        <b>if</b> (validator_set.total_joining_power &gt; validator_stake) {
            validator_set.total_joining_power = validator_set.total_joining_power - validator_stake;
        } <b>else</b> {
            validator_set.total_joining_power = 0;
        };
    } <b>else</b> {
        // Validate that the validator is already part of the validator set.
        <b>let</b> maybe_active_index = <a href="stake.md#0x1_stake_find_validator">find_validator</a>(&validator_set.active_validators, pool_address);
        <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&maybe_active_index), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="stake.md#0x1_stake_ENOT_VALIDATOR">ENOT_VALIDATOR</a>));
        <b>let</b> validator_info = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(
            &<b>mut</b> validator_set.active_validators, <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> maybe_active_index));
        <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validator_set.active_validators) &gt; 0, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="stake.md#0x1_stake_ELAST_VALIDATOR">ELAST_VALIDATOR</a>));
        <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> validator_set.pending_inactive, validator_info);

        <b>if</b> (std::features::module_event_migration_enabled()) {
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="stake.md#0x1_stake_LeaveValidatorSet">LeaveValidatorSet</a> { pool_address });
        } <b>else</b> {
            <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
                &<b>mut</b> stake_pool.leave_validator_set_events,
                <a href="stake.md#0x1_stake_LeaveValidatorSetEvent">LeaveValidatorSetEvent</a> {
                    pool_address,
                },
            );
        };
    };
}
</code></pre>



</details>

<a id="0x1_stake_is_current_epoch_validator"></a>

## Function `is_current_epoch_validator`

Returns true if the current validator can still vote in the current epoch.
This includes validators that requested to leave but are still in the pending_inactive queue and will be removed
when the epoch starts.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_is_current_epoch_validator">is_current_epoch_validator</a>(pool_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_is_current_epoch_validator">is_current_epoch_validator</a>(pool_address: <b>address</b>): bool <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);
    <b>let</b> validator_state = <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address);
    validator_state == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a> || validator_state == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a>
}
</code></pre>



</details>

<a id="0x1_stake_update_performance_statistics"></a>

## Function `update_performance_statistics`

Update the validator performance (proposal statistics). This is only called by block::prologue().
This function cannot abort.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_update_performance_statistics">update_performance_statistics</a>(proposer_index: <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, failed_proposer_indices: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_update_performance_statistics">update_performance_statistics</a>(
    proposer_index: Option&lt;u64&gt;,
    failed_proposer_indices: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> {
    // Validator set cannot change until the end of the epoch, so the validator index in arguments should
    // match <b>with</b> those of the validators in <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> resource.
    <b>let</b> validator_perf = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework);
    <b>let</b> validator_len = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validator_perf.validators);

    <b>spec</b> {
        <b>update</b> <a href="stake.md#0x1_stake_ghost_valid_perf">ghost_valid_perf</a> = validator_perf;
        <b>update</b> <a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a> = proposer_index;
    };
    // proposer_index is an <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option">option</a> because it can be missing (for NilBlocks)
    <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&proposer_index)) {
        <b>let</b> cur_proposer_index = <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> proposer_index);
        // Here, and in all other <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>, skip <a href="../../velor-stdlib/doc/any.md#0x1_any">any</a> validator indices that are out of bounds,
        // this <b>ensures</b> that this function doesn't <b>abort</b> <b>if</b> there are out of bounds errors.
        <b>if</b> (cur_proposer_index &lt; validator_len) {
            <b>let</b> validator = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> validator_perf.validators, cur_proposer_index);
            <b>spec</b> {
                <b>assume</b> validator.successful_proposals + 1 &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
            };
            validator.successful_proposals = validator.successful_proposals + 1;
        };
    };

    <b>let</b> f = 0;
    <b>let</b> f_len = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&failed_proposer_indices);
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> len(validator_perf.validators) == validator_len;
            <b>invariant</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>) && <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(
                <a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>
            ) &lt; validator_len) ==&gt;
                (validator_perf.validators[<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>)].successful_proposals ==
                    <a href="stake.md#0x1_stake_ghost_valid_perf">ghost_valid_perf</a>.validators[<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>)].successful_proposals + 1);
        };
        f &lt; f_len
    }) {
        <b>let</b> validator_index = *<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&failed_proposer_indices, f);
        <b>if</b> (validator_index &lt; validator_len) {
            <b>let</b> validator = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> validator_perf.validators, validator_index);
            <b>spec</b> {
                <b>assume</b> validator.failed_proposals + 1 &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
            };
            validator.failed_proposals = validator.failed_proposals + 1;
        };
        f = f + 1;
    };
}
</code></pre>



</details>

<a id="0x1_stake_on_new_epoch"></a>

## Function `on_new_epoch`

Triggered during a reconfiguration. This function shouldn't abort.

1. Distribute transaction fees and rewards to stake pools of active and pending inactive validators (requested
to leave but not yet removed).
2. Officially move pending active stake to active and move pending inactive stake to inactive.
The staking pool's voting power in this new epoch will be updated to the total active stake.
3. Add pending active validators to the active set if they satisfy requirements so they can vote and remove
pending inactive validators so they no longer can vote.
4. The validator's voting power in the validator set is updated to be the corresponding staking pool's voting
power.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_on_new_epoch">on_new_epoch</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_on_new_epoch">on_new_epoch</a>(
) <b>acquires</b> <a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a>, <a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>, <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <b>let</b> validator_set = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <b>let</b> config = <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();
    <b>let</b> validator_perf = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework);

    // Process pending <a href="stake.md#0x1_stake">stake</a> and distribute transaction fees and rewards for each currently active validator.
    <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&validator_set.active_validators, |validator| {
        <b>let</b> validator: &<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> = validator;
        <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(validator_perf, validator.addr, &config);
    });

    // Process pending <a href="stake.md#0x1_stake">stake</a> and distribute transaction fees and rewards for each currently pending_inactive validator
    // (requested <b>to</b> leave but not removed yet).
    <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&validator_set.pending_inactive, |validator| {
        <b>let</b> validator: &<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> = validator;
        <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(validator_perf, validator.addr, &config);
    });

    // Activate currently pending_active validators.
    <a href="stake.md#0x1_stake_append">append</a>(&<b>mut</b> validator_set.active_validators, &<b>mut</b> validator_set.pending_active);

    // Officially deactivate all pending_inactive validators. They will now no longer receive rewards.
    validator_set.pending_inactive = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    // Update active validator set so that network <b>address</b>/<b>public</b> key change takes effect.
    // Moreover, recalculate the total <a href="voting.md#0x1_voting">voting</a> power, and deactivate the validator whose
    // <a href="voting.md#0x1_voting">voting</a> power is less than the minimum required <a href="stake.md#0x1_stake">stake</a>.
    <b>let</b> next_epoch_validators = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> (minimum_stake, _) = <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&config);
    <b>let</b> vlen = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validator_set.active_validators);
    <b>let</b> total_voting_power = 0;
    <b>let</b> i = 0;
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(next_epoch_validators);
            <b>invariant</b> i &lt;= vlen;
        };
        i &lt; vlen
    }) {
        <b>let</b> old_validator_info = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> validator_set.active_validators, i);
        <b>let</b> pool_address = old_validator_info.addr;
        <b>let</b> validator_config = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
        <b>let</b> stake_pool = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
        <b>let</b> new_validator_info = <a href="stake.md#0x1_stake_generate_validator_info">generate_validator_info</a>(pool_address, stake_pool, *validator_config);

        // A validator needs at least the <b>min</b> <a href="stake.md#0x1_stake">stake</a> required <b>to</b> join the validator set.
        <b>if</b> (new_validator_info.voting_power &gt;= minimum_stake) {
            <b>spec</b> {
                <b>assume</b> total_voting_power + new_validator_info.voting_power &lt;= MAX_U128;
            };
            total_voting_power = total_voting_power + (new_validator_info.voting_power <b>as</b> u128);
            <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> next_epoch_validators, new_validator_info);
        };
        i = i + 1;
    };

    validator_set.active_validators = next_epoch_validators;
    validator_set.total_voting_power = total_voting_power;
    validator_set.total_joining_power = 0;

    // Update validator indices, reset performance scores, and renew lockups.
    validator_perf.validators = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> recurring_lockup_duration_secs = <a href="staking_config.md#0x1_staking_config_get_recurring_lockup_duration">staking_config::get_recurring_lockup_duration</a>(&config);
    <b>let</b> vlen = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validator_set.active_validators);
    <b>let</b> validator_index = 0;
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validator_set.active_validators);
            <b>invariant</b> len(validator_set.pending_active) == 0;
            <b>invariant</b> len(validator_set.pending_inactive) == 0;
            <b>invariant</b> 0 &lt;= validator_index && validator_index &lt;= vlen;
            <b>invariant</b> vlen == len(validator_set.active_validators);
            <b>invariant</b> <b>forall</b> i in 0..validator_index:
                <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(validator_set.active_validators[i].addr).validator_index &lt; validator_index;
            <b>invariant</b> <b>forall</b> i in 0..validator_index:
                validator_set.active_validators[i].config.validator_index &lt; validator_index;
            <b>invariant</b> len(validator_perf.validators) == validator_index;
        };
        validator_index &lt; vlen
    }) {
        <b>let</b> validator_info = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> validator_set.active_validators, validator_index);
        validator_info.config.validator_index = validator_index;
        <b>let</b> validator_config = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(validator_info.addr);
        validator_config.validator_index = validator_index;

        <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> validator_perf.validators, <a href="stake.md#0x1_stake_IndividualValidatorPerformance">IndividualValidatorPerformance</a> {
            successful_proposals: 0,
            failed_proposals: 0,
        });

        // Automatically renew a validator's lockup for validators that will still be in the validator set in the
        // next epoch.
        <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(validator_info.addr);
        <b>let</b> now_secs = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
        <b>let</b> reconfig_start_secs = <b>if</b> (<a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>()) {
            <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>()
        } <b>else</b> {
            now_secs
        };
        <b>if</b> (stake_pool.locked_until_secs &lt;= reconfig_start_secs) {
            <b>spec</b> {
                <b>assume</b> now_secs + recurring_lockup_duration_secs &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
            };
            stake_pool.locked_until_secs = now_secs + recurring_lockup_duration_secs;
        };

        validator_index = validator_index + 1;
    };

    <b>if</b> (<b>exists</b>&lt;<a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>&gt;(@velor_framework)) {
        <b>let</b> pending_fee_by_validator = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>&gt;(@velor_framework).pending_fee_by_validator;
        <b>assert</b>!(pending_fee_by_validator.is_empty(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="stake.md#0x1_stake_ETRANSACTION_FEE_NOT_FULLY_DISTRIBUTED">ETRANSACTION_FEE_NOT_FULLY_DISTRIBUTED</a>));
        validator_set.active_validators.for_each_ref(|v| pending_fee_by_validator.add(v.config.validator_index, <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>&lt;u64&gt;()));
    };

    <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_periodical_reward_rate_decrease_enabled">features::periodical_reward_rate_decrease_enabled</a>()) {
        // Update rewards rate after reward distribution.
        <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_epoch_rewards_rate">staking_config::calculate_and_save_latest_epoch_rewards_rate</a>();
    };
}
</code></pre>



</details>

<a id="0x1_stake_cur_validator_consensus_infos"></a>

## Function `cur_validator_consensus_infos`

Return the <code>ValidatorConsensusInfo</code> of each current validator, sorted by current validator index.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_cur_validator_consensus_infos">cur_validator_consensus_infos</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_cur_validator_consensus_infos">cur_validator_consensus_infos</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt; <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <b>let</b> validator_set = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(validator_set)
}
</code></pre>



</details>

<a id="0x1_stake_next_validator_consensus_infos"></a>

## Function `next_validator_consensus_infos`



<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_next_validator_consensus_infos">next_validator_consensus_infos</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_next_validator_consensus_infos">next_validator_consensus_infos</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt; <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>, <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
    // Init.
    <b>let</b> cur_validator_set = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> = <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();
    <b>let</b> validator_perf = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework);
    <b>let</b> (minimum_stake, _) = <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&<a href="staking_config.md#0x1_staking_config">staking_config</a>);
    <b>let</b> (rewards_rate, rewards_rate_denominator) = <a href="staking_config.md#0x1_staking_config_get_reward_rate">staking_config::get_reward_rate</a>(&<a href="staking_config.md#0x1_staking_config">staking_config</a>);

    // Compute new validator set.
    <b>let</b> new_active_validators = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> num_new_actives = 0;
    <b>let</b> candidate_idx = 0;
    <b>let</b> new_total_power = 0;
    <b>let</b> num_cur_actives = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&cur_validator_set.active_validators);
    <b>let</b> num_cur_pending_actives = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&cur_validator_set.pending_active);
    <b>spec</b> {
        <b>assume</b> num_cur_actives + num_cur_pending_actives &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
    };
    <b>let</b> num_candidates = num_cur_actives + num_cur_pending_actives;
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> candidate_idx &lt;= num_candidates;
            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(new_active_validators);
            <b>invariant</b> len(new_active_validators) == num_new_actives;
            <b>invariant</b> <b>forall</b> i in 0..len(new_active_validators):
                new_active_validators[i].config.validator_index == i;
            <b>invariant</b> num_new_actives &lt;= candidate_idx;
            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(new_active_validators);
        };
        candidate_idx &lt; num_candidates
    }) {
        <b>let</b> candidate_in_current_validator_set = candidate_idx &lt; num_cur_actives;
        <b>let</b> candidate = <b>if</b> (candidate_idx &lt; num_cur_actives) {
            <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&cur_validator_set.active_validators, candidate_idx)
        } <b>else</b> {
            <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&cur_validator_set.pending_active, candidate_idx - num_cur_actives)
        };
        <b>let</b> stake_pool = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(candidate.addr);
        <b>let</b> cur_active = <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.active);
        <b>let</b> cur_pending_active = <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.pending_active);
        <b>let</b> cur_pending_inactive = <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.pending_inactive);

        <b>let</b> cur_reward = <b>if</b> (candidate_in_current_validator_set && cur_active &gt; 0) {
            <b>spec</b> {
                <b>assert</b> candidate.config.validator_index &lt; len(validator_perf.validators);
            };
            <b>let</b> cur_perf = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&validator_perf.validators, candidate.config.validator_index);
            <b>spec</b> {
                <b>assume</b> cur_perf.successful_proposals + cur_perf.failed_proposals &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
            };
            <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(cur_active, cur_perf.successful_proposals, cur_perf.successful_proposals + cur_perf.failed_proposals, rewards_rate, rewards_rate_denominator)
        } <b>else</b> {
            0
        };

        <b>let</b> lockup_expired = <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>() &gt;= stake_pool.locked_until_secs;
        <b>spec</b> {
            <b>assume</b> cur_active + cur_pending_active + cur_reward &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
            <b>assume</b> cur_active + cur_pending_inactive + cur_pending_active + cur_reward &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
        };
        <b>let</b> new_voting_power =
            cur_active
            + <b>if</b> (lockup_expired) { 0 } <b>else</b> { cur_pending_inactive }
            + cur_pending_active
            + cur_reward;

        <b>if</b> (new_voting_power &gt;= minimum_stake) {
            <b>let</b> config = *<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(candidate.addr);
            config.validator_index = num_new_actives;
            <b>let</b> new_validator_info = <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> {
                addr: candidate.addr,
                voting_power: new_voting_power,
                config,
            };

            // Update <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>.
            <b>spec</b> {
                <b>assume</b> new_total_power + new_voting_power &lt;= MAX_U128;
            };
            new_total_power = new_total_power + (new_voting_power <b>as</b> u128);
            <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> new_active_validators, new_validator_info);
            num_new_actives = num_new_actives + 1;

        };
        candidate_idx = candidate_idx + 1;
    };

    <b>let</b> new_validator_set = <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
        consensus_scheme: cur_validator_set.consensus_scheme,
        active_validators: new_active_validators,
        pending_inactive: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        pending_active: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        total_voting_power: new_total_power,
        total_joining_power: 0,
    };

    <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(&new_validator_set)
}
</code></pre>



</details>

<a id="0x1_stake_validator_consensus_infos_from_validator_set"></a>

## Function `validator_consensus_infos_from_validator_set`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(validator_set: &<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(validator_set: &<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt; {
    <b>let</b> validator_consensus_infos = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    <b>let</b> num_active = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validator_set.active_validators);
    <b>let</b> num_pending_inactive = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validator_set.pending_inactive);
    <b>spec</b> {
        <b>assume</b> num_active + num_pending_inactive &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
    };
    <b>let</b> total = num_active + num_pending_inactive;

    // Pre-fill the <b>return</b> value <b>with</b> dummy values.
    <b>let</b> idx = 0;
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> idx &lt;= len(validator_set.active_validators) + len(validator_set.pending_inactive);
            <b>invariant</b> len(validator_consensus_infos) == idx;
            <b>invariant</b> len(validator_consensus_infos) &lt;= len(validator_set.active_validators) + len(validator_set.pending_inactive);
        };
        idx &lt; total
    }) {
        <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> validator_consensus_infos, <a href="validator_consensus_info.md#0x1_validator_consensus_info_default">validator_consensus_info::default</a>());
        idx = idx + 1;
    };
    <b>spec</b> {
        <b>assert</b> len(validator_consensus_infos) == len(validator_set.active_validators) + len(validator_set.pending_inactive);
        <b>assert</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validator_set.active_validators,
            len(validator_set.active_validators) + len(validator_set.pending_inactive));
    };

    <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&validator_set.active_validators, |obj| {
        <b>let</b> vi: &<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> = obj;
        <b>spec</b> {
            <b>assume</b> len(validator_consensus_infos) == len(validator_set.active_validators) + len(validator_set.pending_inactive);
            <b>assert</b> vi.config.validator_index &lt; len(validator_consensus_infos);
        };
        <b>let</b> vci = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> validator_consensus_infos, vi.config.validator_index);
        *vci = <a href="validator_consensus_info.md#0x1_validator_consensus_info_new">validator_consensus_info::new</a>(
            vi.addr,
            vi.config.consensus_pubkey,
            vi.voting_power
        );
        <b>spec</b> {
            <b>assert</b> len(validator_consensus_infos) == len(validator_set.active_validators) + len(validator_set.pending_inactive);
        };
    });

    <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&validator_set.pending_inactive, |obj| {
        <b>let</b> vi: &<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> = obj;
        <b>spec</b> {
            <b>assume</b> len(validator_consensus_infos) == len(validator_set.active_validators) + len(validator_set.pending_inactive);
            <b>assert</b> vi.config.validator_index &lt; len(validator_consensus_infos);
        };
        <b>let</b> vci = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> validator_consensus_infos, vi.config.validator_index);
        *vci = <a href="validator_consensus_info.md#0x1_validator_consensus_info_new">validator_consensus_info::new</a>(
            vi.addr,
            vi.config.consensus_pubkey,
            vi.voting_power
        );
        <b>spec</b> {
            <b>assert</b> len(validator_consensus_infos) == len(validator_set.active_validators) + len(validator_set.pending_inactive);
        };
    });

    validator_consensus_infos
}
</code></pre>



</details>

<a id="0x1_stake_addresses_from_validator_infos"></a>

## Function `addresses_from_validator_infos`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_addresses_from_validator_infos">addresses_from_validator_infos</a>(infos: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_addresses_from_validator_infos">addresses_from_validator_infos</a>(infos: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; {
    <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(infos, |obj| {
        <b>let</b> info: &<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> = obj;
        info.addr
    })
}
</code></pre>



</details>

<a id="0x1_stake_update_stake_pool"></a>

## Function `update_stake_pool`

Calculate the stake amount of a stake pool for the next epoch.
Update individual validator's stake pool if <code>commit == <b>true</b></code>.

1. distribute transaction fees to active/pending_inactive delegations
2. distribute rewards to active/pending_inactive delegations
3. process pending_active, pending_inactive correspondingly
This function shouldn't abort.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(validator_perf: &<a href="stake.md#0x1_stake_ValidatorPerformance">stake::ValidatorPerformance</a>, pool_address: <b>address</b>, <a href="staking_config.md#0x1_staking_config">staking_config</a>: &<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(
    validator_perf: &<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>,
    pool_address: <b>address</b>,
    <a href="staking_config.md#0x1_staking_config">staking_config</a>: &StakingConfig,
) <b>acquires</b> <a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a>, <a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
    <b>let</b> stake_pool = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>let</b> validator_config = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
    <b>let</b> validator_index = validator_config.validator_index;
    <b>let</b> cur_validator_perf = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&validator_perf.validators, validator_index);
    <b>let</b> num_successful_proposals = cur_validator_perf.successful_proposals;

    <b>let</b> fee_pending_inactive = 0;
    <b>let</b> fee_active = 0;
    <b>let</b> fee_limit = <b>if</b> (<b>exists</b>&lt;<a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a>&gt;(@velor_framework)) {
        <b>let</b> TransactionFeeConfig::V0 { max_fee_octa_allowed_per_epoch_per_pool } = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_TransactionFeeConfig">TransactionFeeConfig</a>&gt;(@velor_framework);
        *max_fee_octa_allowed_per_epoch_per_pool
    } <b>else</b> {
        <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a> <b>as</b> u64
    };

    <b>if</b> (<b>exists</b>&lt;<a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>&gt;(@velor_framework)) {
        <b>let</b> pending_fee_by_validator = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_PendingTransactionFee">PendingTransactionFee</a>&gt;(@velor_framework).pending_fee_by_validator;
        <b>if</b> (pending_fee_by_validator.contains(&validator_index)) {
            <b>let</b> fee_octa = pending_fee_by_validator.remove(&validator_index).read();
            <b>if</b> (fee_octa &gt; fee_limit) {
                fee_octa = fee_limit;
            };
            <b>let</b> stake_active = (<a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.active) <b>as</b> u128);
            <b>let</b> stake_pending_inactive = (<a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.pending_inactive) <b>as</b> u128);
            fee_pending_inactive = (((fee_octa <b>as</b> u128) * stake_pending_inactive / (stake_active + stake_pending_inactive)) <b>as</b> u64);
            fee_active = fee_octa - fee_pending_inactive;
        }
    };

    <b>spec</b> {
        // The following addition should not overflow because `num_total_proposals` cannot be larger than 86400,
        // the maximum number of proposals in a day (1 proposal per second).
        <b>assume</b> cur_validator_perf.successful_proposals + cur_validator_perf.failed_proposals &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
    };
    <b>let</b> num_total_proposals = cur_validator_perf.successful_proposals + cur_validator_perf.failed_proposals;
    <b>let</b> (rewards_rate, rewards_rate_denominator) = <a href="staking_config.md#0x1_staking_config_get_reward_rate">staking_config::get_reward_rate</a>(<a href="staking_config.md#0x1_staking_config">staking_config</a>);
    <b>let</b> rewards_active = <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(
        &<b>mut</b> stake_pool.active,
        num_successful_proposals,
        num_total_proposals,
        rewards_rate,
        rewards_rate_denominator
    );
    <b>let</b> rewards_pending_inactive = <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(
        &<b>mut</b> stake_pool.pending_inactive,
        num_successful_proposals,
        num_total_proposals,
        rewards_rate,
        rewards_rate_denominator
    );
    <b>spec</b> {
        <b>assume</b> rewards_active + rewards_pending_inactive &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
    };

    <b>if</b> (std::features::is_distribute_transaction_fee_enabled()) {
        <b>let</b> mint_cap = &<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a>&gt;(@velor_framework).mint_cap;
        <b>if</b> (fee_active &gt; 0) {
            <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> stake_pool.active, <a href="coin.md#0x1_coin_mint">coin::mint</a>(fee_active, mint_cap));
        };
        <b>if</b> (fee_pending_inactive &gt; 0) {
            <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> stake_pool.pending_inactive, <a href="coin.md#0x1_coin_mint">coin::mint</a>(fee_pending_inactive, mint_cap));
        };
        <b>let</b> fee_amount = fee_active + fee_pending_inactive;
        <b>if</b> (fee_amount &gt; 0) {
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="stake.md#0x1_stake_DistributeTransactionFee">DistributeTransactionFee</a> { pool_address, fee_amount });
        };
    };

    <b>let</b> rewards_amount = rewards_active + rewards_pending_inactive;
    // Pending active <a href="stake.md#0x1_stake">stake</a> can now be active.
    <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> stake_pool.active, <a href="coin.md#0x1_coin_extract_all">coin::extract_all</a>(&<b>mut</b> stake_pool.pending_active));

    // Pending inactive <a href="stake.md#0x1_stake">stake</a> is only fully unlocked and moved into inactive <b>if</b> the current lockup cycle <b>has</b> expired
    <b>let</b> current_lockup_expiration = stake_pool.locked_until_secs;
    <b>if</b> (<a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>() &gt;= current_lockup_expiration) {
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(
            &<b>mut</b> stake_pool.inactive,
            <a href="coin.md#0x1_coin_extract_all">coin::extract_all</a>(&<b>mut</b> stake_pool.pending_inactive),
        );
    };

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="stake.md#0x1_stake_DistributeRewards">DistributeRewards</a> { pool_address, rewards_amount });
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> stake_pool.distribute_rewards_events,
            <a href="stake.md#0x1_stake_DistributeRewardsEvent">DistributeRewardsEvent</a> {
                pool_address,
                rewards_amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_stake_get_reconfig_start_time_secs"></a>

## Function `get_reconfig_start_time_secs`

Assuming we are in a middle of a reconfiguration (no matter it is immediate or async), get its start time.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>(): u64 {
    <b>if</b> (<a href="reconfiguration_state.md#0x1_reconfiguration_state_is_initialized">reconfiguration_state::is_initialized</a>()) {
        <a href="reconfiguration_state.md#0x1_reconfiguration_state_start_time_secs">reconfiguration_state::start_time_secs</a>()
    } <b>else</b> {
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()
    }
}
</code></pre>



</details>

<a id="0x1_stake_calculate_rewards_amount"></a>

## Function `calculate_rewards_amount`

Calculate the rewards amount.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(stake_amount: u64, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(
    stake_amount: u64,
    num_successful_proposals: u64,
    num_total_proposals: u64,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
): u64 {
    <b>spec</b> {
        // The following condition must hold because
        // (1) num_successful_proposals &lt;= num_total_proposals, and
        // (2) `num_total_proposals` cannot be larger than 86400, the maximum number of proposals
        //     in a day (1 proposal per second), and `num_total_proposals` is reset <b>to</b> 0 every epoch.
        <b>assume</b> num_successful_proposals * <a href="stake.md#0x1_stake_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a> &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
    };
    // The rewards amount is equal <b>to</b> (<a href="stake.md#0x1_stake">stake</a> amount * rewards rate * performance multiplier).
    // We do multiplication in u128 before division <b>to</b> avoid the overflow and minimize the rounding <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a>.
    <b>let</b> rewards_numerator = (stake_amount <b>as</b> u128) * (rewards_rate <b>as</b> u128) * (num_successful_proposals <b>as</b> u128);
    <b>let</b> rewards_denominator = (rewards_rate_denominator <b>as</b> u128) * (num_total_proposals <b>as</b> u128);
    <b>if</b> (rewards_denominator &gt; 0) {
        ((rewards_numerator / rewards_denominator) <b>as</b> u64)
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x1_stake_distribute_rewards"></a>

## Function `distribute_rewards`

Mint rewards corresponding to current epoch's <code><a href="stake.md#0x1_stake">stake</a></code> and <code>num_successful_votes</code>.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(<a href="stake.md#0x1_stake">stake</a>: &<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(
    <a href="stake.md#0x1_stake">stake</a>: &<b>mut</b> Coin&lt;VelorCoin&gt;,
    num_successful_proposals: u64,
    num_total_proposals: u64,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
): u64 <b>acquires</b> <a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a> {
    <b>let</b> stake_amount = <a href="coin.md#0x1_coin_value">coin::value</a>(<a href="stake.md#0x1_stake">stake</a>);
    <b>let</b> rewards_amount = <b>if</b> (stake_amount &gt; 0) {
        <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(
            stake_amount,
            num_successful_proposals,
            num_total_proposals,
            rewards_rate,
            rewards_rate_denominator
        )
    } <b>else</b> {
        0
    };
    <b>if</b> (rewards_amount &gt; 0) {
        <b>let</b> mint_cap = &<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a>&gt;(@velor_framework).mint_cap;
        <b>let</b> rewards = <a href="coin.md#0x1_coin_mint">coin::mint</a>(rewards_amount, mint_cap);
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(<a href="stake.md#0x1_stake">stake</a>, rewards);
    };
    rewards_amount
}
</code></pre>



</details>

<a id="0x1_stake_append"></a>

## Function `append`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_append">append</a>&lt;T&gt;(v1: &<b>mut</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, v2: &<b>mut</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_append">append</a>&lt;T&gt;(v1: &<b>mut</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, v2: &<b>mut</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;) {
    <b>while</b> (!<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(v2)) {
        <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(v1, <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(v2));
    }
}
</code></pre>



</details>

<a id="0x1_stake_find_validator"></a>

## Function `find_validator`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_find_validator">find_validator</a>(v: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;, addr: <b>address</b>): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_find_validator">find_validator</a>(v: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, addr: <b>address</b>): Option&lt;u64&gt; {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(v);
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> !(<b>exists</b> j in 0..i: v[j].addr == addr);
        };
        i &lt; len
    }) {
        <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(v, i).addr == addr) {
            <b>return</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(i)
        };
        i = i + 1;
    };
    <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a id="0x1_stake_generate_validator_info"></a>

## Function `generate_validator_info`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_generate_validator_info">generate_validator_info</a>(addr: <b>address</b>, stake_pool: &<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>, config: <a href="stake.md#0x1_stake_ValidatorConfig">stake::ValidatorConfig</a>): <a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_generate_validator_info">generate_validator_info</a>(addr: <b>address</b>, stake_pool: &<a href="stake.md#0x1_stake_StakePool">StakePool</a>, config: <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>): <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> {
    <b>let</b> voting_power = <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool);
    <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> {
        addr,
        voting_power,
        config,
    }
}
</code></pre>



</details>

<a id="0x1_stake_get_next_epoch_voting_power"></a>

## Function `get_next_epoch_voting_power`

Returns validator's next epoch voting power, including pending_active, active, and pending_inactive stake.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool: &<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool: &<a href="stake.md#0x1_stake_StakePool">StakePool</a>): u64 {
    <b>let</b> value_pending_active = <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.pending_active);
    <b>let</b> value_active = <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.active);
    <b>let</b> value_pending_inactive = <a href="coin.md#0x1_coin_value">coin::value</a>(&stake_pool.pending_inactive);
    <b>spec</b> {
        <b>assume</b> value_pending_active + value_active + value_pending_inactive &lt;= <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
    };
    value_pending_active + value_active + value_pending_inactive
}
</code></pre>



</details>

<a id="0x1_stake_update_voting_power_increase"></a>

## Function `update_voting_power_increase`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(increase_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(increase_amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> {
    <b>let</b> validator_set = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <b>let</b> voting_power_increase_limit =
        (<a href="staking_config.md#0x1_staking_config_get_voting_power_increase_limit">staking_config::get_voting_power_increase_limit</a>(&<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>()) <b>as</b> u128);
    validator_set.total_joining_power = validator_set.total_joining_power + (increase_amount <b>as</b> u128);

    // Only validator <a href="voting.md#0x1_voting">voting</a> power increase <b>if</b> the current validator set's <a href="voting.md#0x1_voting">voting</a> power &gt; 0.
    <b>if</b> (validator_set.total_voting_power &gt; 0) {
        <b>assert</b>!(
            validator_set.total_joining_power &lt;= validator_set.total_voting_power * voting_power_increase_limit / 100,
            <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EVOTING_POWER_INCREASE_EXCEEDS_LIMIT">EVOTING_POWER_INCREASE_EXCEEDS_LIMIT</a>),
        );
    }
}
</code></pre>



</details>

<a id="0x1_stake_assert_stake_pool_exists"></a>

## Function `assert_stake_pool_exists`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address: <b>address</b>) {
    <b>assert</b>!(<a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(pool_address), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ESTAKE_POOL_DOES_NOT_EXIST">ESTAKE_POOL_DOES_NOT_EXIST</a>));
}
</code></pre>



</details>

<a id="0x1_stake_configure_allowed_validators"></a>

## Function `configure_allowed_validators`



<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_configure_allowed_validators">configure_allowed_validators</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, accounts: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_configure_allowed_validators">configure_allowed_validators</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    accounts: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
) <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> {
    <b>let</b> velor_framework_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(velor_framework_address)) {
        <b>move_to</b>(velor_framework, <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> { accounts });
    } <b>else</b> {
        <b>let</b> allowed = <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(velor_framework_address);
        allowed.accounts = accounts;
    }
}
</code></pre>



</details>

<a id="0x1_stake_is_allowed"></a>

## Function `is_allowed`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_is_allowed">is_allowed</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_is_allowed">is_allowed</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@velor_framework)) {
        <b>true</b>
    } <b>else</b> {
        <b>let</b> allowed = <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@velor_framework);
        <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&allowed.accounts, &<a href="account.md#0x1_account">account</a>)
    }
}
</code></pre>



</details>

<a id="0x1_stake_assert_owner_cap_exists"></a>

## Function `assert_owner_cap_exists`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>) {
    <b>assert</b>!(<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>));
}
</code></pre>



</details>

<a id="0x1_stake_assert_reconfig_not_in_progress"></a>

## Function `assert_reconfig_not_in_progress`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>() {
    <b>assert</b>!(!<a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">reconfiguration_state::is_in_progress</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="stake.md#0x1_stake_ERECONFIGURATION_IN_PROGRESS">ERECONFIGURATION_IN_PROGRESS</a>));
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>The validator set resource stores consensus information for each validator. The consensus scheme remains consistent across all validators within the set.</td>
<td>Low</td>
<td>The consensus_scheme attribute within ValidatorSet initializes with the value zero during the module's initialization and its value remains unchanged afterward.</td>
<td>Formally verified by the data invariant of <a href="#high-level-req-1">ValidatorSet</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The owner of a validator is immutable.</td>
<td>Low</td>
<td>During the initialization of a validator, the owner attribute becomes the signer's address. This assignment establishes the signer as the owner and controller of the validator entity. Subsequently, the owner attribute remains unchanged throughout the validator's lifespan, maintaining its assigned value without any modifications.</td>
<td>Formally verified in the schema <a href="#high-level-req-2">ValidatorOwnerNoChange</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The total staked value in the stake pool should remain constant, excluding operations related to adding and withdrawing.</td>
<td>Low</td>
<td>The total staked value (VelorCoin) of a stake pool is grouped by: active, inactive, pending_active, and pending_inactive. The stake value remains constant except during the execution of the add_stake_with_cap or withdraw_with_cap functions or on_new_epoch (which distributes the reward).</td>
<td>Formally specified in the schema <a href="#high-level-req-3">StakedValueNoChange</a>.</td>
</tr>

<tr>
<td>4</td>
<td>During each epoch, the following operations should be consistently performed without aborting: rewards distribution, validator activation/deactivation, updates to validator sets and voting power, and renewal of lockups.</td>
<td>Low</td>
<td>The on_new_epoch function is triggered at each epoch boundary to perform distribution of the transaction fee, updates to active/inactive stakes, updates to pending active/inactive validators and adjusts voting power of the validators without aborting.</td>
<td>Formally verified via <a href="#high-level-req-4">on_new_epoch</a>. This also requires a manual review to verify the state updates of the stake pool.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial;
<b>invariant</b> [suspendable] <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework) ==&gt; <a href="stake.md#0x1_stake_validator_set_is_valid">validator_set_is_valid</a>();
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a>&gt;(@velor_framework);
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework);
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>apply</b> <a href="stake.md#0x1_stake_ValidatorOwnerNoChange">ValidatorOwnerNoChange</a> <b>to</b> *;
<b>apply</b> <a href="stake.md#0x1_stake_ValidatorNotChangeDuringReconfig">ValidatorNotChangeDuringReconfig</a> <b>to</b> * <b>except</b> on_new_epoch;
<b>apply</b> <a href="stake.md#0x1_stake_StakePoolNotChangeDuringReconfig">StakePoolNotChangeDuringReconfig</a> <b>to</b> * <b>except</b> on_new_epoch, update_stake_pool;
<a id="0x1_stake_ghost_valid_perf"></a>
<b>global</b> <a href="stake.md#0x1_stake_ghost_valid_perf">ghost_valid_perf</a>: <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>;
<a id="0x1_stake_ghost_proposer_idx"></a>
<b>global</b> <a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>: Option&lt;u64&gt;;
<a id="0x1_stake_ghost_active_num"></a>
<b>global</b> <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a>: u64;
<a id="0x1_stake_ghost_pending_inactive_num"></a>
<b>global</b> <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a>: u64;
</code></pre>



<a id="@Specification_1_ValidatorSet"></a>

### Resource `ValidatorSet`


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<dl>
<dt>
<code>consensus_scheme: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>active_validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>total_voting_power: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>total_joining_power: u128</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>invariant</b> consensus_scheme == 0;
</code></pre>




<a id="0x1_stake_ValidatorNotChangeDuringReconfig"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_ValidatorNotChangeDuringReconfig">ValidatorNotChangeDuringReconfig</a> {
    <b>ensures</b> (<a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>() && <b>old</b>(<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework))) ==&gt;
        <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework)) == <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
}
</code></pre>




<a id="0x1_stake_StakePoolNotChangeDuringReconfig"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_StakePoolNotChangeDuringReconfig">StakePoolNotChangeDuringReconfig</a> {
    <b>ensures</b> <b>forall</b> a: <b>address</b> <b>where</b> <b>old</b>(<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a)): <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>() ==&gt;
        (<b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).pending_inactive) == <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).pending_inactive &&
        <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).pending_active) == <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).pending_active &&
        <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).inactive) == <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).inactive &&
        <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).active) == <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).active);
}
</code></pre>




<a id="0x1_stake_ValidatorOwnerNoChange"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_ValidatorOwnerNoChange">ValidatorOwnerNoChange</a> {
    // This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
    <b>ensures</b> <b>forall</b> addr: <b>address</b> <b>where</b> <b>old</b>(<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr)):
        <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr)).pool_address == <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr).pool_address;
}
</code></pre>




<a id="0x1_stake_StakedValueNochange"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a> {
    pool_address: <b>address</b>;
    <b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>let</b> <b>post</b> post_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    // This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    <b>ensures</b> stake_pool.active.value + stake_pool.inactive.value + stake_pool.pending_active.value + stake_pool.pending_inactive.value ==
        post_stake_pool.active.value + post_stake_pool.inactive.value + post_stake_pool.pending_active.value + post_stake_pool.pending_inactive.value;
}
</code></pre>




<a id="0x1_stake_validator_set_is_valid"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_set_is_valid">validator_set_is_valid</a>(): bool {
   <b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
   <a href="stake.md#0x1_stake_validator_set_is_valid_impl">validator_set_is_valid_impl</a>(validator_set)
}
</code></pre>




<a id="0x1_stake_validator_set_is_valid_impl"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_set_is_valid_impl">validator_set_is_valid_impl</a>(validator_set: <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>): bool {
   <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validator_set.active_validators) &&
       <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validator_set.pending_inactive) &&
       <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validator_set.pending_active) &&
       <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(validator_set.active_validators) &&
       <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(validator_set.pending_inactive)
       && <a href="stake.md#0x1_stake_spec_validator_indices_active_pending_inactive">spec_validator_indices_active_pending_inactive</a>(validator_set)
}
</code></pre>



<a id="@Specification_1_get_validator_state"></a>

### Function `get_validator_state`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>ensures</b> result == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE">VALIDATOR_STATUS_PENDING_ACTIVE</a> ==&gt; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_active, pool_address);
<b>ensures</b> result == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a> ==&gt; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.active_validators, pool_address);
<b>ensures</b> result == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a> ==&gt; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_inactive, pool_address);
<b>ensures</b> result == <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a> ==&gt; (
    !<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_active, pool_address)
        && !<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.active_validators, pool_address)
        && !<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_inactive, pool_address)
);
</code></pre>



<a id="@Specification_1_get_pending_transaction_fee"></a>

### Function `get_pending_transaction_fee`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_pending_transaction_fee">get_pending_transaction_fee</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize">initialize</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>pragma</b> disable_invariants_in_body;
<b>let</b> velor_addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_velor_framework_address">system_addresses::is_velor_framework_address</a>(velor_addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(velor_addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(velor_addr);
<b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(velor_addr);
<b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(velor_addr).consensus_scheme == 0;
<b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(velor_addr);
</code></pre>



<a id="@Specification_1_remove_validators"></a>

### Function `remove_validators`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_remove_validators">remove_validators</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> <b>post</b> post_validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> active_validators = validator_set.active_validators;
<b>let</b> <b>post</b> post_active_validators = post_validator_set.active_validators;
<b>let</b> pending_inactive_validators = validator_set.pending_inactive;
<b>let</b> <b>post</b> post_pending_inactive_validators = post_validator_set.pending_inactive;
<b>invariant</b> len(active_validators) &gt; 0;
<b>ensures</b> len(active_validators) + len(pending_inactive_validators) == len(post_active_validators)
    + len(post_pending_inactive_validators);
</code></pre>



<a id="@Specification_1_initialize_stake_owner"></a>

### Function `initialize_stake_owner`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_stake_owner">initialize_stake_owner</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_stake_amount: u64, operator: <b>address</b>, voter: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>pragma</b> verify = <b>false</b>;
<b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: owner
};
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;
<b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
<b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(addr) == <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
    consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
    network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
    fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
    validator_index: 0,
};
<b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr) == <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> { pool_address: addr };
<b>let</b> <b>post</b> stakepool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(addr);
<b>let</b> <b>post</b> active = stakepool.active.value;
<b>let</b> <b>post</b> pending_active = stakepool.pending_active.value;
<b>ensures</b> <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(addr) ==&gt;
    pending_active == initial_stake_amount;
<b>ensures</b> !<a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(addr) ==&gt;
    active == initial_stake_amount;
</code></pre>



<a id="@Specification_1_initialize_validator"></a>

### Function `initialize_validator`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator">initialize_validator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_of_possession: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: <a href="account.md#0x1_account">account</a>
};
<b>let</b> pubkey_from_pop = <a href="../../velor-stdlib/doc/bls12381.md#0x1_bls12381_spec_public_key_from_bytes_with_pop">bls12381::spec_public_key_from_bytes_with_pop</a>(
    consensus_pubkey,
    proof_of_possession_from_bytes(proof_of_possession)
);
<b>aborts_if</b> !<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(pubkey_from_pop);
<b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>let</b> post_addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>let</b> allowed = <b>global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@velor_framework);
<b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@velor_framework) && !<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(allowed.accounts, addr);
<b>aborts_if</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>aborts_if</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr).guid_creation_num + 12 &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
<b>aborts_if</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr).guid_creation_num + 12 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
<b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(post_addr);
<b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(post_addr) == <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> { pool_address: post_addr };
<b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(post_addr) == <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> {
    consensus_pubkey,
    network_addresses,
    fullnode_addresses,
    validator_index: 0,
};
</code></pre>



<a id="@Specification_1_extract_owner_cap"></a>

### Function `extract_owner_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_extract_owner_cap">extract_owner_cap</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: owner
};
<b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
<b>ensures</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
</code></pre>



<a id="@Specification_1_deposit_owner_cap"></a>

### Function `deposit_owner_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_deposit_owner_cap">deposit_owner_cap</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)
</code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: owner
};
<b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
<b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
<b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
<b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address) == owner_cap;
</code></pre>



<a id="@Specification_1_set_operator_with_cap"></a>

### Function `set_operator_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_operator_with_cap">set_operator_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, new_operator: <b>address</b>)
</code></pre>




<pre><code><b>let</b> pool_address = owner_cap.pool_address;
<b>let</b> <b>post</b> post_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;
<b>ensures</b> post_stake_pool.operator_address == new_operator;
</code></pre>



<a id="@Specification_1_set_delegated_voter_with_cap"></a>

### Function `set_delegated_voter_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">set_delegated_voter_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, new_voter: <b>address</b>)
</code></pre>




<pre><code><b>let</b> pool_address = owner_cap.pool_address;
<b>let</b> <b>post</b> post_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>ensures</b> post_stake_pool.delegated_voter == new_voter;
</code></pre>



<a id="@Specification_1_add_stake"></a>

### Function `add_stake`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_add_stake">add_stake</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: owner
};
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_AddStakeAbortsIfAndEnsures">AddStakeAbortsIfAndEnsures</a>;
</code></pre>



<a id="@Specification_1_add_stake_with_cap"></a>

### Function `add_stake_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_add_stake_with_cap">add_stake_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;)
</code></pre>




<pre><code><b>pragma</b> disable_invariants_in_body;
<b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;
<b>let</b> amount = coins.value;
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>include</b> <a href="stake.md#0x1_stake_AddStakeWithCapAbortsIfAndEnsures">AddStakeWithCapAbortsIfAndEnsures</a> { amount };
</code></pre>



<a id="@Specification_1_reactivate_stake_with_cap"></a>

### Function `reactivate_stake_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake_with_cap">reactivate_stake_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, amount: u64)
</code></pre>




<pre><code><b>let</b> pool_address = owner_cap.pool_address;
<b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>aborts_if</b> !<a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(pool_address);
<b>let</b> pre_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> <b>post</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> min_amount = velor_std::math64::min(amount, pre_stake_pool.pending_inactive.value);
<b>ensures</b> stake_pool.pending_inactive.value == pre_stake_pool.pending_inactive.value - min_amount;
<b>ensures</b> stake_pool.active.value == pre_stake_pool.active.value + min_amount;
</code></pre>



<a id="@Specification_1_rotate_consensus_key"></a>

### Function `rotate_consensus_key`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_rotate_consensus_key">rotate_consensus_key</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_consensus_pubkey: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_of_possession: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: operator
};
<b>let</b> pre_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> <b>post</b> validator_info = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) != pre_stake_pool.operator_address;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>let</b> pubkey_from_pop = <a href="../../velor-stdlib/doc/bls12381.md#0x1_bls12381_spec_public_key_from_bytes_with_pop">bls12381::spec_public_key_from_bytes_with_pop</a>(
    new_consensus_pubkey,
    proof_of_possession_from_bytes(proof_of_possession)
);
<b>aborts_if</b> !<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(pubkey_from_pop);
<b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;
<b>ensures</b> validator_info.consensus_pubkey == new_consensus_pubkey;
</code></pre>



<a id="@Specification_1_update_network_and_fullnode_addresses"></a>

### Function `update_network_and_fullnode_addresses`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_update_network_and_fullnode_addresses">update_network_and_fullnode_addresses</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_network_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, new_fullnode_addresses: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: operator
};
<b>let</b> pre_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> <b>post</b> validator_info = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>aborts_if</b> <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) != pre_stake_pool.operator_address;
<b>ensures</b> validator_info.network_addresses == new_network_addresses;
<b>ensures</b> validator_info.fullnode_addresses == new_fullnode_addresses;
</code></pre>



<a id="@Specification_1_increase_lockup_with_cap"></a>

### Function `increase_lockup_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup_with_cap">increase_lockup_with_cap</a>(owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)
</code></pre>




<pre><code><b>let</b> config = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@velor_framework);
<b>let</b> pool_address = owner_cap.pool_address;
<b>let</b> pre_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> <b>post</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> now_seconds = <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();
<b>let</b> lockup = config.recurring_lockup_duration_secs;
<b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> pre_stake_pool.locked_until_secs &gt;= lockup + now_seconds;
<b>aborts_if</b> lockup + now_seconds &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@velor_framework);
<b>ensures</b> stake_pool.locked_until_secs == lockup + now_seconds;
</code></pre>



<a id="@Specification_1_join_validator_set"></a>

### Function `join_validator_set`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set">join_validator_set</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 60;
<b>pragma</b> disable_invariants_in_body;
<b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: operator
};
<b>aborts_if</b> !<a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>aborts_if</b> !<b>exists</b>&lt;StakingConfig&gt;(@velor_framework);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> <b>post</b> p_validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>aborts_if</b> <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) != stake_pool.operator_address;
<b>aborts_if</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.active_validators, pool_address)) ||
            <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_inactive, pool_address)) ||
                <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_active, pool_address));
<b>let</b> config = <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();
<b>let</b> voting_power = <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool);
<b>let</b> minimum_stake = config.minimum_stake;
<b>let</b> maximum_stake = config.maximum_stake;
<b>aborts_if</b> voting_power &lt; minimum_stake;
<b>aborts_if</b> voting_power &gt;maximum_stake;
<b>let</b> validator_config = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>aborts_if</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(validator_config.consensus_pubkey);
<b>let</b> validator_set_size = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.active_validators) + <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.pending_active) + 1;
<b>aborts_if</b> validator_set_size &gt; <a href="stake.md#0x1_stake_MAX_VALIDATOR_SET_SIZE">MAX_VALIDATOR_SET_SIZE</a>;
<b>let</b> voting_power_increase_limit = (<a href="staking_config.md#0x1_staking_config_get_voting_power_increase_limit">staking_config::get_voting_power_increase_limit</a>(config) <b>as</b> u128);
<b>aborts_if</b> (validator_set.total_joining_power + (voting_power <b>as</b> u128)) &gt; MAX_U128;
<b>aborts_if</b> validator_set.total_voting_power * voting_power_increase_limit &gt; MAX_U128;
<b>aborts_if</b> validator_set.total_voting_power &gt; 0 &&
    (validator_set.total_joining_power + (voting_power <b>as</b> u128)) * 100 &gt; validator_set.total_voting_power * voting_power_increase_limit;
<b>let</b> <b>post</b> p_validator_info = <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> {
    addr: pool_address,
    voting_power,
    config: validator_config,
};
<b>ensures</b> validator_set.total_joining_power + voting_power == p_validator_set.total_joining_power;
<b>ensures</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(p_validator_set.pending_active, p_validator_info);
</code></pre>



<a id="@Specification_1_unlock_with_cap"></a>

### Function `unlock_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_unlock_with_cap">unlock_with_cap</a>(amount: u64, owner_cap: &<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>let</b> pool_address = owner_cap.pool_address;
<b>let</b> pre_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> <b>post</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>aborts_if</b> amount != 0 && !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;
<b>let</b> min_amount = velor_std::math64::min(amount,pre_stake_pool.active.value);
<b>ensures</b> stake_pool.active.value == pre_stake_pool.active.value - min_amount;
<b>ensures</b> stake_pool.pending_inactive.value == pre_stake_pool.pending_inactive.value + min_amount;
</code></pre>



<a id="@Specification_1_withdraw"></a>

### Function `withdraw`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_withdraw">withdraw</a>(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, withdraw_amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: owner
};
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
<b>let</b> ownership_cap = <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr);
<b>let</b> pool_address = ownership_cap.pool_address;
<b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> bool_find_validator = !<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.active_validators, pool_address)) &&
            !<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_inactive, pool_address)) &&
                !<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_active, pool_address));
<b>aborts_if</b> bool_find_validator && !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework);
<b>let</b> new_withdraw_amount_1 = <b>min</b>(withdraw_amount, stake_pool.inactive.value + stake_pool.pending_inactive.value);
<b>let</b> new_withdraw_amount_2 = <b>min</b>(withdraw_amount, stake_pool.inactive.value);
<b>aborts_if</b> bool_find_validator && <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; stake_pool.locked_until_secs &&
            new_withdraw_amount_1 &gt; 0 && stake_pool.inactive.value + stake_pool.pending_inactive.value &lt; new_withdraw_amount_1;
<b>aborts_if</b> !(bool_find_validator && <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework)) &&
            new_withdraw_amount_2 &gt; 0 && stake_pool.inactive.value &lt; new_withdraw_amount_2;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;VelorCoin&gt;&gt;(addr);
<b>include</b> <a href="coin.md#0x1_coin_DepositAbortsIf">coin::DepositAbortsIf</a>&lt;VelorCoin&gt;{account_addr: addr};
<b>let</b> coin_store = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;VelorCoin&gt;&gt;(addr);
<b>let</b> <b>post</b> p_coin_store = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;VelorCoin&gt;&gt;(addr);
<b>ensures</b> bool_find_validator && <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; stake_pool.locked_until_secs
            && <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr) && <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;VelorCoin&gt;&gt;(addr) ==&gt;
                coin_store.<a href="coin.md#0x1_coin">coin</a>.value + new_withdraw_amount_1 == p_coin_store.<a href="coin.md#0x1_coin">coin</a>.value;
<b>ensures</b> !(bool_find_validator && <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework))
            && <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr) && <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;VelorCoin&gt;&gt;(addr) ==&gt;
                coin_store.<a href="coin.md#0x1_coin">coin</a>.value + new_withdraw_amount_2 == p_coin_store.<a href="coin.md#0x1_coin">coin</a>.value;
</code></pre>



<a id="@Specification_1_leave_validator_set"></a>

### Function `leave_validator_set`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_leave_validator_set">leave_validator_set</a>(operator: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> disable_invariants_in_body;
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: operator
};
<b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>let</b> config = <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();
<b>aborts_if</b> !<a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(config);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@velor_framework);
<b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>aborts_if</b> <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) != stake_pool.operator_address;
<b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> validator_find_bool = <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_active, pool_address));
<b>let</b> active_validators = validator_set.active_validators;
<b>let</b> pending_active = validator_set.pending_active;
<b>let</b> <b>post</b> post_validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> <b>post</b> post_active_validators = post_validator_set.active_validators;
<b>let</b> pending_inactive_validators = validator_set.pending_inactive;
<b>let</b> <b>post</b> post_pending_inactive_validators = post_validator_set.pending_inactive;
<b>ensures</b> len(active_validators) + len(pending_inactive_validators) == len(post_active_validators)
    + len(post_pending_inactive_validators);
<b>aborts_if</b> !validator_find_bool && !<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(active_validators, pool_address));
<b>aborts_if</b> !validator_find_bool && <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.active_validators) &lt;= <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(active_validators, pool_address));
<b>aborts_if</b> !validator_find_bool && <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.active_validators) &lt; 2;
<b>aborts_if</b> validator_find_bool && <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.pending_active) &lt;= <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(pending_active, pool_address));
<b>let</b> <b>post</b> p_validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>let</b> validator_stake = (<a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool) <b>as</b> u128);
<b>ensures</b> validator_find_bool && validator_set.total_joining_power &gt; validator_stake ==&gt;
            p_validator_set.total_joining_power == validator_set.total_joining_power - validator_stake;
<b>ensures</b> !validator_find_bool ==&gt; !<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(p_validator_set.pending_active, pool_address));
</code></pre>



<a id="@Specification_1_is_current_epoch_validator"></a>

### Function `is_current_epoch_validator`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_is_current_epoch_validator">is_current_epoch_validator</a>(pool_address: <b>address</b>): bool
</code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;
<b>aborts_if</b> !<a href="stake.md#0x1_stake_spec_has_stake_pool">spec_has_stake_pool</a>(pool_address);
<b>ensures</b> result == <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(pool_address);
</code></pre>



<a id="@Specification_1_update_performance_statistics"></a>

### Function `update_performance_statistics`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_update_performance_statistics">update_performance_statistics</a>(proposer_index: <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, failed_proposer_indices: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>aborts_if</b> <b>false</b>;
<b>let</b> validator_perf = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework);
<b>let</b> <b>post</b> post_validator_perf = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework);
<b>let</b> validator_len = len(validator_perf.validators);
<b>ensures</b> (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>) && <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>) &lt; validator_len) ==&gt;
    (post_validator_perf.validators[<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>)].successful_proposals ==
        validator_perf.validators[<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>)].successful_proposals + 1);
</code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_on_new_epoch">on_new_epoch</a>()
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> disable_invariants_in_body;
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a>;
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;
<b>include</b> velor_framework::velor_coin::ExistsVelorCoin;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_next_validator_consensus_infos"></a>

### Function `next_validator_consensus_infos`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_next_validator_consensus_infos">next_validator_consensus_infos</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>aborts_if</b> <b>false</b>;
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a>;
<b>include</b> <a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>() ==&gt; <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">staking_config::StakingRewardsConfigEnabledRequirement</a>;
</code></pre>



<a id="@Specification_1_validator_consensus_infos_from_validator_set"></a>

### Function `validator_consensus_infos_from_validator_set`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(validator_set: &<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>invariant</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validator_set.active_validators,
    len(validator_set.active_validators) + len(validator_set.pending_inactive));
<b>invariant</b> len(validator_set.pending_inactive) == 0 ||
    <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validator_set.pending_inactive,
        len(validator_set.active_validators) + len(validator_set.pending_inactive));
</code></pre>




<a id="0x1_stake_AddStakeWithCapAbortsIfAndEnsures"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_AddStakeWithCapAbortsIfAndEnsures">AddStakeWithCapAbortsIfAndEnsures</a> {
    owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>;
    amount: u64;
    <b>let</b> pool_address = owner_cap.pool_address;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>let</b> config = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@velor_framework);
    <b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <b>let</b> voting_power_increase_limit = config.voting_power_increase_limit;
    <b>let</b> <b>post</b> post_validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <b>let</b> update_voting_power_increase = amount != 0 && (<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.active_validators, pool_address)
                                                       || <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_active, pool_address));
    <b>aborts_if</b> update_voting_power_increase && validator_set.total_joining_power + amount &gt; MAX_U128;
    <b>ensures</b> update_voting_power_increase ==&gt; post_validator_set.total_joining_power == validator_set.total_joining_power + amount;
    <b>aborts_if</b> update_voting_power_increase && validator_set.total_voting_power &gt; 0
            && validator_set.total_voting_power * voting_power_increase_limit &gt; MAX_U128;
    <b>aborts_if</b> update_voting_power_increase && validator_set.total_voting_power &gt; 0
            && validator_set.total_joining_power + amount &gt; validator_set.total_voting_power * voting_power_increase_limit / 100;
    <b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>let</b> <b>post</b> post_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>let</b> value_pending_active = stake_pool.pending_active.value;
    <b>let</b> value_active = stake_pool.active.value;
    <b>ensures</b> amount != 0 && <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(pool_address) ==&gt; post_stake_pool.pending_active.value == value_pending_active + amount;
    <b>ensures</b> amount != 0 && !<a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(pool_address) ==&gt; post_stake_pool.active.value == value_active + amount;
    <b>let</b> maximum_stake = config.maximum_stake;
    <b>let</b> value_pending_inactive = stake_pool.pending_inactive.value;
    <b>let</b> next_epoch_voting_power = value_pending_active + value_active + value_pending_inactive;
    <b>let</b> voting_power = next_epoch_voting_power + amount;
    <b>aborts_if</b> amount != 0 && voting_power &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;
    <b>aborts_if</b> amount != 0 && voting_power &gt; maximum_stake;
}
</code></pre>




<a id="0x1_stake_AddStakeAbortsIfAndEnsures"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_AddStakeAbortsIfAndEnsures">AddStakeAbortsIfAndEnsures</a> {
    owner: <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    amount: u64;
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <b>let</b> owner_cap = <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);
    <b>include</b> <a href="stake.md#0x1_stake_AddStakeWithCapAbortsIfAndEnsures">AddStakeWithCapAbortsIfAndEnsures</a> { owner_cap };
}
</code></pre>




<a id="0x1_stake_spec_is_allowed"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_is_allowed">spec_is_allowed</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool {
   <b>if</b> (!<b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@velor_framework)) {
       <b>true</b>
   } <b>else</b> {
       <b>let</b> allowed = <b>global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@velor_framework);
       contains(allowed.accounts, <a href="account.md#0x1_account">account</a>)
   }
}
</code></pre>




<a id="0x1_stake_spec_find_validator"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(v: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, addr: <b>address</b>): Option&lt;u64&gt;;
</code></pre>




<a id="0x1_stake_spec_validators_are_initialized"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;): bool {
   <b>forall</b> i in 0..len(validators):
       <a href="stake.md#0x1_stake_spec_has_stake_pool">spec_has_stake_pool</a>(validators[i].addr) &&
           <a href="stake.md#0x1_stake_spec_has_validator_config">spec_has_validator_config</a>(validators[i].addr)
}
</code></pre>




<a id="0x1_stake_spec_validators_are_initialized_addrs"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized_addrs">spec_validators_are_initialized_addrs</a>(addrs: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;): bool {
   <b>forall</b> i in 0..len(addrs):
       <a href="stake.md#0x1_stake_spec_has_stake_pool">spec_has_stake_pool</a>(addrs[i]) &&
           <a href="stake.md#0x1_stake_spec_has_validator_config">spec_has_validator_config</a>(addrs[i])
}
</code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;): bool {
   <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_addr">spec_validator_indices_are_valid_addr</a>(validators, <a href="stake.md#0x1_stake_spec_validator_index_upper_bound">spec_validator_index_upper_bound</a>()) &&
       <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validators, <a href="stake.md#0x1_stake_spec_validator_index_upper_bound">spec_validator_index_upper_bound</a>())
}
</code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid_addr"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_addr">spec_validator_indices_are_valid_addr</a>(validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, upper_bound: u64): bool {
   <b>forall</b> i in 0..len(validators):
       <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(validators[i].addr).validator_index &lt; upper_bound
}
</code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid_config"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, upper_bound: u64): bool {
   <b>forall</b> i in 0..len(validators):
       validators[i].config.validator_index &lt; upper_bound
}
</code></pre>




<a id="0x1_stake_spec_validator_indices_active_pending_inactive"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_indices_active_pending_inactive">spec_validator_indices_active_pending_inactive</a>(validator_set: <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>): bool {
   len(validator_set.pending_inactive) + len(validator_set.active_validators) == <a href="stake.md#0x1_stake_spec_validator_index_upper_bound">spec_validator_index_upper_bound</a>()
}
</code></pre>




<a id="0x1_stake_spec_validator_index_upper_bound"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_index_upper_bound">spec_validator_index_upper_bound</a>(): u64 {
   len(<b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework).validators)
}
</code></pre>




<a id="0x1_stake_spec_has_stake_pool"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_has_stake_pool">spec_has_stake_pool</a>(a: <b>address</b>): bool {
   <b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a)
}
</code></pre>




<a id="0x1_stake_spec_has_validator_config"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_has_validator_config">spec_has_validator_config</a>(a: <b>address</b>): bool {
   <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(a)
}
</code></pre>




<a id="0x1_stake_spec_rewards_amount"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(
   stake_amount: u64,
   num_successful_proposals: u64,
   num_total_proposals: u64,
   rewards_rate: u64,
   rewards_rate_denominator: u64,
): u64;
</code></pre>




<a id="0x1_stake_spec_contains"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validators: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, addr: <b>address</b>): bool {
   <b>exists</b> i in 0..len(validators): validators[i].addr == addr
}
</code></pre>




<a id="0x1_stake_spec_is_current_epoch_validator"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(pool_address: <b>address</b>): bool {
   <b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
   !<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_active, pool_address)
       && (<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.active_validators, pool_address)
       || <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_inactive, pool_address))
}
</code></pre>




<a id="0x1_stake_ResourceRequirement"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a> {
    <b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_VelorCoinCapabilities">VelorCoinCapabilities</a>&gt;(@velor_framework);
    <b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@velor_framework);
    <b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
    <b>requires</b> <b>exists</b>&lt;StakingConfig&gt;(@velor_framework);
    <b>requires</b> <b>exists</b>&lt;StakingRewardsConfig&gt;(@velor_framework) || !<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>();
    <b>requires</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework);
}
</code></pre>




<a id="0x1_stake_spec_get_reward_rate_1"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_get_reward_rate_1">spec_get_reward_rate_1</a>(config: StakingConfig): num {
   <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>()) {
       <b>let</b> epoch_rewards_rate = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@velor_framework).rewards_rate;
       <b>if</b> (epoch_rewards_rate.value == 0) {
           0
       } <b>else</b> {
           <b>let</b> denominator_0 = velor_std::fixed_point64::spec_divide_u128(<a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">staking_config::MAX_REWARDS_RATE</a>, epoch_rewards_rate);
           <b>let</b> denominator = <b>if</b> (denominator_0 &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>) {
               <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>
           } <b>else</b> {
               denominator_0
           };
           <b>let</b> nominator = velor_std::fixed_point64::spec_multiply_u128(denominator, epoch_rewards_rate);
           nominator
       }
   } <b>else</b> {
           config.rewards_rate
   }
}
</code></pre>




<a id="0x1_stake_spec_get_reward_rate_2"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_get_reward_rate_2">spec_get_reward_rate_2</a>(config: StakingConfig): num {
   <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>()) {
       <b>let</b> epoch_rewards_rate = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@velor_framework).rewards_rate;
       <b>if</b> (epoch_rewards_rate.value == 0) {
           1
       } <b>else</b> {
           <b>let</b> denominator_0 = velor_std::fixed_point64::spec_divide_u128(<a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">staking_config::MAX_REWARDS_RATE</a>, epoch_rewards_rate);
           <b>let</b> denominator = <b>if</b> (denominator_0 &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>) {
               <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>
           } <b>else</b> {
               denominator_0
           };
           denominator
       }
   } <b>else</b> {
           config.rewards_rate_denominator
   }
}
</code></pre>



<a id="@Specification_1_update_stake_pool"></a>

### Function `update_stake_pool`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(validator_perf: &<a href="stake.md#0x1_stake_ValidatorPerformance">stake::ValidatorPerformance</a>, pool_address: <b>address</b>, <a href="staking_config.md#0x1_staking_config">staking_config</a>: &<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a>;
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_UpdateStakePoolAbortsIf">UpdateStakePoolAbortsIf</a>;
<b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> validator_config = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
<b>let</b> cur_validator_perf = validator_perf.validators[validator_config.validator_index];
<b>let</b> num_successful_proposals = cur_validator_perf.successful_proposals;
<b>let</b> num_total_proposals = cur_validator_perf.successful_proposals + cur_validator_perf.failed_proposals;
<b>let</b> rewards_rate = <a href="stake.md#0x1_stake_spec_get_reward_rate_1">spec_get_reward_rate_1</a>(<a href="staking_config.md#0x1_staking_config">staking_config</a>);
<b>let</b> rewards_rate_denominator = <a href="stake.md#0x1_stake_spec_get_reward_rate_2">spec_get_reward_rate_2</a>(<a href="staking_config.md#0x1_staking_config">staking_config</a>);
<b>let</b> rewards_amount_1 = <b>if</b> (stake_pool.active.value &gt; 0) {
    <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(stake_pool.active.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
} <b>else</b> {
    0
};
<b>let</b> rewards_amount_2 = <b>if</b> (stake_pool.pending_inactive.value &gt; 0) {
    <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(stake_pool.pending_inactive.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
} <b>else</b> {
    0
};
<b>let</b> <b>post</b> post_stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
<b>let</b> <b>post</b> post_active_value = post_stake_pool.active.value;
<b>let</b> <b>post</b> post_pending_inactive_value = post_stake_pool.pending_inactive.value;
<b>let</b> <b>post</b> post_inactive_value = post_stake_pool.inactive.value;
<b>ensures</b> post_stake_pool.pending_active.value == 0;
<b>ensures</b> post_active_value == stake_pool.active.value + rewards_amount_1 + stake_pool.pending_active.value;
<b>ensures</b> <b>if</b> (<a href="stake.md#0x1_stake_spec_get_reconfig_start_time_secs">spec_get_reconfig_start_time_secs</a>() &gt;= stake_pool.locked_until_secs) {
    post_pending_inactive_value == 0 &&
    post_inactive_value == stake_pool.inactive.value + stake_pool.pending_inactive.value + rewards_amount_2
} <b>else</b> {
    post_pending_inactive_value == stake_pool.pending_inactive.value + rewards_amount_2
};
</code></pre>




<a id="0x1_stake_AbortsIfSignerPermissionStake"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_AbortsIfSignerPermissionStake">AbortsIfSignerPermissionStake</a> {
    s: <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>let</b> perm = <a href="stake.md#0x1_stake_StakeManagementPermission">StakeManagementPermission</a> {};
    <b>aborts_if</b> !<a href="permissioned_signer.md#0x1_permissioned_signer_spec_check_permission_exists">permissioned_signer::spec_check_permission_exists</a>(s, perm);
}
</code></pre>




<a id="0x1_stake_UpdateStakePoolAbortsIf"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_UpdateStakePoolAbortsIf">UpdateStakePoolAbortsIf</a> {
    pool_address: <b>address</b>;
    validator_perf: <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);
    <b>aborts_if</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address).validator_index &gt;= len(validator_perf.validators);
    <b>let</b> velor_addr = <a href="../../velor-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;VelorCoin&gt;().account_address;
    <b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);
    <b>include</b> <a href="stake.md#0x1_stake_DistributeRewardsAbortsIf">DistributeRewardsAbortsIf</a> {<a href="stake.md#0x1_stake">stake</a>: stake_pool.active};
    <b>include</b> <a href="stake.md#0x1_stake_DistributeRewardsAbortsIf">DistributeRewardsAbortsIf</a> {<a href="stake.md#0x1_stake">stake</a>: stake_pool.pending_inactive};
}
</code></pre>



<a id="@Specification_1_get_reconfig_start_time_secs"></a>

### Function `get_reconfig_start_time_secs`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>(): u64
</code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a>;
</code></pre>




<a id="0x1_stake_GetReconfigStartTimeRequirement"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a> {
    <b>requires</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework);
    <b>include</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StartTimeSecsRequirement">reconfiguration_state::StartTimeSecsRequirement</a>;
}
</code></pre>




<a id="0x1_stake_spec_get_reconfig_start_time_secs"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_get_reconfig_start_time_secs">spec_get_reconfig_start_time_secs</a>(): u64 {
   <b>if</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">reconfiguration_state::State</a>&gt;(@velor_framework)) {
       <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_start_time_secs">reconfiguration_state::spec_start_time_secs</a>()
   } <b>else</b> {
       <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>()
   }
}
</code></pre>




<a id="0x1_stake_spec_get_lockup_secs"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_get_lockup_secs">spec_get_lockup_secs</a>(pool_address: <b>address</b>): u64 {
   <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).locked_until_secs
}
</code></pre>



<a id="@Specification_1_calculate_rewards_amount"></a>

### Function `calculate_rewards_amount`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(stake_amount: u64, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify_duration_estimate = 300;
<b>pragma</b> verify = <b>false</b>;
<b>requires</b> rewards_rate &lt;= <a href="stake.md#0x1_stake_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>;
<b>requires</b> rewards_rate_denominator &gt; 0;
<b>requires</b> rewards_rate &lt;= rewards_rate_denominator;
<b>requires</b> num_successful_proposals &lt;= num_total_proposals;
<b>ensures</b> [concrete] (rewards_rate_denominator * num_total_proposals == 0) ==&gt; result == 0;
<b>ensures</b> [concrete] (rewards_rate_denominator * num_total_proposals &gt; 0) ==&gt; {
    <b>let</b> amount = ((stake_amount * rewards_rate * num_successful_proposals) /
        (rewards_rate_denominator * num_total_proposals));
    result == amount
};
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> [abstract] result == <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(
    stake_amount,
    num_successful_proposals,
    num_total_proposals,
    rewards_rate,
    rewards_rate_denominator);
</code></pre>



<a id="@Specification_1_distribute_rewards"></a>

### Function `distribute_rewards`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(<a href="stake.md#0x1_stake">stake</a>: &<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="velor_coin.md#0x1_velor_coin_VelorCoin">velor_coin::VelorCoin</a>&gt;, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;
<b>requires</b> rewards_rate &lt;= <a href="stake.md#0x1_stake_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>;
<b>requires</b> rewards_rate_denominator &gt; 0;
<b>requires</b> rewards_rate &lt;= rewards_rate_denominator;
<b>requires</b> num_successful_proposals &lt;= num_total_proposals;
<b>include</b> <a href="stake.md#0x1_stake_DistributeRewardsAbortsIf">DistributeRewardsAbortsIf</a>;
<b>ensures</b> <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) &gt; 0 ==&gt;
    result == <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(
        <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value),
        num_successful_proposals,
        num_total_proposals,
        rewards_rate,
        rewards_rate_denominator);
<b>ensures</b> <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) &gt; 0 ==&gt;
    <a href="stake.md#0x1_stake">stake</a>.value == <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) + <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(
        <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value),
        num_successful_proposals,
        num_total_proposals,
        rewards_rate,
        rewards_rate_denominator);
<b>ensures</b> <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) == 0 ==&gt; result == 0;
<b>ensures</b> <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) == 0 ==&gt; <a href="stake.md#0x1_stake">stake</a>.value == <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value);
</code></pre>




<a id="0x1_stake_DistributeRewardsAbortsIf"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_DistributeRewardsAbortsIf">DistributeRewardsAbortsIf</a> {
    <a href="stake.md#0x1_stake">stake</a>: Coin&lt;VelorCoin&gt;;
    num_successful_proposals: num;
    num_total_proposals: num;
    rewards_rate: num;
    rewards_rate_denominator: num;
    <b>let</b> stake_amount = <a href="coin.md#0x1_coin_value">coin::value</a>(<a href="stake.md#0x1_stake">stake</a>);
    <b>let</b> rewards_amount = <b>if</b> (stake_amount &gt; 0) {
        <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(stake_amount, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
    } <b>else</b> {
        0
    };
    <b>let</b> amount = rewards_amount;
    <b>let</b> addr = <a href="../../velor-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;VelorCoin&gt;().account_address;
    <b>aborts_if</b> (rewards_amount &gt; 0) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;VelorCoin&gt;&gt;(addr);
    <b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;VelorCoin&gt;&gt;(addr);
    <b>include</b> (rewards_amount &gt; 0) ==&gt; <a href="coin.md#0x1_coin_CoinAddAbortsIf">coin::CoinAddAbortsIf</a>&lt;VelorCoin&gt; { amount: amount };
}
</code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_append">append</a>&lt;T&gt;(v1: &<b>mut</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, v2: &<b>mut</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;)
</code></pre>




<pre><code><b>pragma</b> opaque, verify = <b>false</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> len(v1) == <b>old</b>(len(v1) + len(v2));
<b>ensures</b> len(v2) == 0;
<b>ensures</b> (<b>forall</b> i in 0..<b>old</b>(len(v1)): v1[i] == <b>old</b>(v1[i]));
<b>ensures</b> (<b>forall</b> i in <b>old</b>(len(v1))..len(v1): v1[i] == <b>old</b>(v2[len(v2) - (i - len(v1)) - 1]));
</code></pre>



<a id="@Specification_1_find_validator"></a>

### Function `find_validator`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_find_validator">find_validator</a>(v: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;, addr: <b>address</b>): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(result) ==&gt; (<b>forall</b> i in 0..len(v): v[i].addr != addr);
<b>ensures</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(result) ==&gt; v[<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(result)].addr == addr;
<b>ensures</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(result) ==&gt; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(v, addr);
<b>ensures</b> [abstract] result == <a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(v,addr);
</code></pre>



<a id="@Specification_1_update_voting_power_increase"></a>

### Function `update_voting_power_increase`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(increase_amount: u64)
</code></pre>




<pre><code><b>requires</b> !<a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@velor_framework);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@velor_framework);
<b>let</b> velor = @velor_framework;
<b>let</b> pre_validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(velor);
<b>let</b> <b>post</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(velor);
<b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(velor);
<b>let</b> voting_power_increase_limit = <a href="staking_config.md#0x1_staking_config">staking_config</a>.voting_power_increase_limit;
<b>aborts_if</b> pre_validator_set.total_joining_power + increase_amount &gt; MAX_U128;
<b>aborts_if</b> pre_validator_set.total_voting_power &gt; 0 && pre_validator_set.total_voting_power * voting_power_increase_limit &gt; MAX_U128;
<b>aborts_if</b> pre_validator_set.total_voting_power &gt; 0 &&
    pre_validator_set.total_joining_power + increase_amount &gt; pre_validator_set.total_voting_power * voting_power_increase_limit / 100;
<b>ensures</b> validator_set.total_voting_power &gt; 0 ==&gt;
    validator_set.total_joining_power &lt;= validator_set.total_voting_power * voting_power_increase_limit / 100;
<b>ensures</b> validator_set.total_joining_power == pre_validator_set.total_joining_power + increase_amount;
</code></pre>



<a id="@Specification_1_assert_stake_pool_exists"></a>

### Function `assert_stake_pool_exists`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(pool_address);
</code></pre>



<a id="@Specification_1_configure_allowed_validators"></a>

### Function `configure_allowed_validators`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_configure_allowed_validators">configure_allowed_validators</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, accounts: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>




<pre><code><b>let</b> velor_framework_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_velor_framework_address">system_addresses::is_velor_framework_address</a>(velor_framework_address);
<b>let</b> <b>post</b> allowed = <b>global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(velor_framework_address);
<b>ensures</b> allowed.accounts == accounts;
</code></pre>



<a id="@Specification_1_assert_owner_cap_exists"></a>

### Function `assert_owner_cap_exists`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner);
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
