
<a id="0x1_stake"></a>

# Module `0x1::stake`


Validator lifecycle:
1. Prepare a validator node set up and call stake::initialize_validator
2. Once ready to deposit stake (or have funds assigned by a staking service in exchange for ownership capability),
call stake::add_stake (or &#42;_with_cap versions if called from the staking service)
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
8. Validator can always rejoin the validator set by going through steps 2&#45;3 again.
9. An owner can always switch operators by calling stake::set_operator.
10. An owner can always switch designated voter by calling stake::set_designated_voter.


-  [Resource `OwnerCapability`](#0x1_stake_OwnerCapability)
-  [Resource `StakePool`](#0x1_stake_StakePool)
-  [Resource `ValidatorConfig`](#0x1_stake_ValidatorConfig)
-  [Struct `ValidatorInfo`](#0x1_stake_ValidatorInfo)
-  [Resource `ValidatorSet`](#0x1_stake_ValidatorSet)
-  [Resource `AptosCoinCapabilities`](#0x1_stake_AptosCoinCapabilities)
-  [Struct `IndividualValidatorPerformance`](#0x1_stake_IndividualValidatorPerformance)
-  [Resource `ValidatorPerformance`](#0x1_stake_ValidatorPerformance)
-  [Struct `RegisterValidatorCandidateEvent`](#0x1_stake_RegisterValidatorCandidateEvent)
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
-  [Function `initialize_validator_fees`](#0x1_stake_initialize_validator_fees)
-  [Function `add_transaction_fee`](#0x1_stake_add_transaction_fee)
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
-  [Function `initialize`](#0x1_stake_initialize)
-  [Function `store_aptos_coin_mint_cap`](#0x1_stake_store_aptos_coin_mint_cap)
-  [Function `remove_validators`](#0x1_stake_remove_validators)
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
    -  [Function `initialize_validator_fees`](#@Specification_1_initialize_validator_fees)
    -  [Function `add_transaction_fee`](#@Specification_1_add_transaction_fee)
    -  [Function `get_validator_state`](#@Specification_1_get_validator_state)
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/bls12381.md#0x1_bls12381">0x1::bls12381</a>;<br /><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state">0x1::reconfiguration_state</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_stake_OwnerCapability"></a>

## Resource `OwnerCapability`

Capability that represents ownership and can be used to control the validator and the associated stake pool.
Having this be separate from the signer for the account that the validator resources are hosted at allows
modules to have control over a validator.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> <b>has</b> store, key<br /></code></pre>



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
Any pending_active stake is moved to active and adds to the validator&apos;s voting power.

Changes in stake for an inactive validator:
1. If a validator calls add_stake, the newly added stake is moved directly to active.
2. If validator calls unlock, their stake is moved directly to inactive.
3. When the next epoch starts, the validator can be activated if their active stake is more than the minimum.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inactive: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
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


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

Full ValidatorSet, stored in @aptos_framework.
1. join_validator_set adds to pending_active queue.
2. leave_valdiator_set moves from active to pending_inactive queue.
3. on_new_epoch processes two pending queues and refresh ValidatorInfo from the owner&apos;s address.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>consensus_scheme: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>active_validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
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

<a id="0x1_stake_AptosCoinCapabilities"></a>

## Resource `AptosCoinCapabilities`

AptosCoin capabilities, set during genesis and stored in @CoreResource account.
This allows the Stake module to mint rewards to stakers.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_AptosCoinCapabilities">AptosCoinCapabilities</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_IndividualValidatorPerformance"></a>

## Struct `IndividualValidatorPerformance`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_IndividualValidatorPerformance">IndividualValidatorPerformance</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_IndividualValidatorPerformance">stake::IndividualValidatorPerformance</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RegisterValidatorCandidateEvent"></a>

## Struct `RegisterValidatorCandidateEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_RegisterValidatorCandidateEvent">RegisterValidatorCandidateEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_stake_RegisterValidatorCandidate"></a>

## Struct `RegisterValidatorCandidate`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_RegisterValidatorCandidate">RegisterValidatorCandidate</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_SetOperatorEvent">SetOperatorEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_SetOperator">SetOperator</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_AddStakeEvent">AddStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_AddStake">AddStake</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ReactivateStakeEvent">ReactivateStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_ReactivateStake">ReactivateStake</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_RotateConsensusKeyEvent">RotateConsensusKeyEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RotateConsensusKey"></a>

## Struct `RotateConsensusKey`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_RotateConsensusKey">RotateConsensusKey</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_UpdateNetworkAndFullnodeAddressesEvent"></a>

## Struct `UpdateNetworkAndFullnodeAddressesEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddressesEvent">UpdateNetworkAndFullnodeAddressesEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_UpdateNetworkAndFullnodeAddresses"></a>

## Struct `UpdateNetworkAndFullnodeAddresses`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddresses">UpdateNetworkAndFullnodeAddresses</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_IncreaseLockupEvent"></a>

## Struct `IncreaseLockupEvent`



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_IncreaseLockupEvent">IncreaseLockupEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_IncreaseLockup">IncreaseLockup</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_JoinValidatorSetEvent">JoinValidatorSetEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_JoinValidatorSet">JoinValidatorSet</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_DistributeRewardsEvent">DistributeRewardsEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_DistributeRewards">DistributeRewards</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_UnlockStakeEvent">UnlockStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_UnlockStake">UnlockStake</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_WithdrawStakeEvent">WithdrawStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_WithdrawStake">WithdrawStake</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="stake.md#0x1_stake_LeaveValidatorSetEvent">LeaveValidatorSetEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="stake.md#0x1_stake_LeaveValidatorSet">LeaveValidatorSet</a> <b>has</b> drop, store<br /></code></pre>



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

Stores transaction fees assigned to validators. All fees are distributed to validators
at the end of the epoch.


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>fees_table: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<b>address</b>, <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;&gt;</code>
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


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_valid_perf"></a>

## Resource `Ghost$ghost_valid_perf`



<pre><code><b>struct</b> Ghost$<a href="stake.md#0x1_stake_ghost_valid_perf">ghost_valid_perf</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



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



<pre><code><b>struct</b> Ghost$<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_active_num"></a>

## Resource `Ghost$ghost_active_num`



<pre><code><b>struct</b> Ghost$<a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



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



<pre><code><b>struct</b> Ghost$<a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



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



<pre><code><b>const</b> <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>: u128 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_stake_EALREADY_REGISTERED"></a>

Account is already registered as a validator candidate.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EALREADY_REGISTERED">EALREADY_REGISTERED</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_stake_MAX_REWARDS_RATE"></a>

Limit the maximum value of <code>rewards_rate</code> in order to avoid any arithmetic overflow.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>: u64 &#61; 1000000;<br /></code></pre>



<a id="0x1_stake_EALREADY_ACTIVE_VALIDATOR"></a>

Account is already a validator or pending validator.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EALREADY_ACTIVE_VALIDATOR">EALREADY_ACTIVE_VALIDATOR</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_stake_EFEES_TABLE_ALREADY_EXISTS"></a>

Table to store collected transaction fees for each validator already exists.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EFEES_TABLE_ALREADY_EXISTS">EFEES_TABLE_ALREADY_EXISTS</a>: u64 &#61; 19;<br /></code></pre>



<a id="0x1_stake_EINELIGIBLE_VALIDATOR"></a>

Validator is not defined in the ACL of entities allowed to be validators


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EINELIGIBLE_VALIDATOR">EINELIGIBLE_VALIDATOR</a>: u64 &#61; 17;<br /></code></pre>



<a id="0x1_stake_EINVALID_LOCKUP"></a>

Cannot update stake pool&apos;s lockup to earlier than current lockup.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EINVALID_LOCKUP">EINVALID_LOCKUP</a>: u64 &#61; 18;<br /></code></pre>



<a id="0x1_stake_EINVALID_PUBLIC_KEY"></a>

Invalid consensus public key


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x1_stake_ELAST_VALIDATOR"></a>

Can&apos;t remove last validator.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ELAST_VALIDATOR">ELAST_VALIDATOR</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_stake_ENOT_OPERATOR"></a>

Account does not have the right operator capability.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_stake_ENOT_VALIDATOR"></a>

Account is not a validator.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ENOT_VALIDATOR">ENOT_VALIDATOR</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED"></a>

Validators cannot join or leave post genesis on this test network.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED">ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_stake_EOWNER_CAP_ALREADY_EXISTS"></a>

An account cannot own more than one owner capability.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>: u64 &#61; 16;<br /></code></pre>



<a id="0x1_stake_EOWNER_CAP_NOT_FOUND"></a>

Owner capability does not exist at the provided account.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>: u64 &#61; 15;<br /></code></pre>



<a id="0x1_stake_ERECONFIGURATION_IN_PROGRESS"></a>

Validator set change temporarily disabled because of in&#45;progress reconfiguration.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ERECONFIGURATION_IN_PROGRESS">ERECONFIGURATION_IN_PROGRESS</a>: u64 &#61; 20;<br /></code></pre>



<a id="0x1_stake_ESTAKE_EXCEEDS_MAX"></a>

Total stake exceeds maximum allowed.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ESTAKE_EXCEEDS_MAX">ESTAKE_EXCEEDS_MAX</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_stake_ESTAKE_POOL_DOES_NOT_EXIST"></a>

Stake pool does not exist at the provided pool address.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ESTAKE_POOL_DOES_NOT_EXIST">ESTAKE_POOL_DOES_NOT_EXIST</a>: u64 &#61; 14;<br /></code></pre>



<a id="0x1_stake_ESTAKE_TOO_HIGH"></a>

Too much stake to join validator set.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ESTAKE_TOO_HIGH">ESTAKE_TOO_HIGH</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_stake_ESTAKE_TOO_LOW"></a>

Not enough stake to join validator set.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_ESTAKE_TOO_LOW">ESTAKE_TOO_LOW</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_stake_EVALIDATOR_CONFIG"></a>

Validator Config not published.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EVALIDATOR_CONFIG">EVALIDATOR_CONFIG</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_stake_EVALIDATOR_SET_TOO_LARGE"></a>

Validator set exceeds the limit


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EVALIDATOR_SET_TOO_LARGE">EVALIDATOR_SET_TOO_LARGE</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x1_stake_EVOTING_POWER_INCREASE_EXCEEDS_LIMIT"></a>

Voting power increase has exceeded the limit for this current epoch.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_EVOTING_POWER_INCREASE_EXCEEDS_LIMIT">EVOTING_POWER_INCREASE_EXCEEDS_LIMIT</a>: u64 &#61; 13;<br /></code></pre>



<a id="0x1_stake_MAX_VALIDATOR_SET_SIZE"></a>

Limit the maximum size to u16::max, it&apos;s the current limit of the bitvec
https://github.com/aptos&#45;labs/aptos&#45;core/blob/main/crates/aptos&#45;bitvec/src/lib.rs#L20


<pre><code><b>const</b> <a href="stake.md#0x1_stake_MAX_VALIDATOR_SET_SIZE">MAX_VALIDATOR_SET_SIZE</a>: u64 &#61; 65536;<br /></code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_ACTIVE"></a>



<pre><code><b>const</b> <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code><b>const</b> <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE"></a>

Validator status enum. We can switch to proper enum later once Move supports it.


<pre><code><b>const</b> <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE">VALIDATOR_STATUS_PENDING_ACTIVE</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE"></a>



<pre><code><b>const</b> <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_stake_initialize_validator_fees"></a>

## Function `initialize_validator_fees`

Initializes the resource storing information about collected transaction fees per validator.
Used by <code><a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a>.<b>move</b></code> to initialize fee collection and distribution.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator_fees">initialize_validator_fees</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator_fees">initialize_validator_fees</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>assert</b>!(<br />        !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="stake.md#0x1_stake_EFEES_TABLE_ALREADY_EXISTS">EFEES_TABLE_ALREADY_EXISTS</a>)<br />    );<br />    <b>move_to</b>(aptos_framework, <a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a> &#123; fees_table: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>() &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_add_transaction_fee"></a>

## Function `add_transaction_fee`

Stores the transaction fee collected to the specified validator address.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_add_transaction_fee">add_transaction_fee</a>(validator_addr: <b>address</b>, fee: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_add_transaction_fee">add_transaction_fee</a>(validator_addr: <b>address</b>, fee: Coin&lt;AptosCoin&gt;) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a> &#123;<br />    <b>let</b> fees_table &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework).fees_table;<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(fees_table, validator_addr)) &#123;<br />        <b>let</b> collected_fee &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(fees_table, validator_addr);<br />        <a href="coin.md#0x1_coin_merge">coin::merge</a>(collected_fee, fee);<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(fees_table, validator_addr, fee);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_lockup_secs"></a>

## Function `get_lockup_secs`

Return the lockup expiration of the stake pool at <code>pool_address</code>.
This will throw an error if there&apos;s no stake pool at <code>pool_address</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_lockup_secs">get_lockup_secs</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_lockup_secs">get_lockup_secs</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).locked_until_secs<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_remaining_lockup_secs"></a>

## Function `get_remaining_lockup_secs`

Return the remaining lockup of the stake pool at <code>pool_address</code>.
This will throw an error if there&apos;s no stake pool at <code>pool_address</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_remaining_lockup_secs">get_remaining_lockup_secs</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_remaining_lockup_secs">get_remaining_lockup_secs</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> lockup_time &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).locked_until_secs;<br />    <b>if</b> (lockup_time &lt;&#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()) &#123;<br />        0<br />    &#125; <b>else</b> &#123;<br />        lockup_time &#45; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_stake"></a>

## Function `get_stake`

Return the different stake amounts for <code>pool_address</code> (whether the validator is active or not).
The returned amounts are for (active, inactive, pending_active, pending_inactive) stake respectively.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_stake">get_stake</a>(pool_address: <b>address</b>): (u64, u64, u64, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_stake">get_stake</a>(pool_address: <b>address</b>): (u64, u64, u64, u64) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> stake_pool &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    (<br />        <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.active),<br />        <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.inactive),<br />        <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.pending_active),<br />        <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.pending_inactive),<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_validator_state"></a>

## Function `get_validator_state`

Returns the validator&apos;s state.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <b>let</b> validator_set &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&amp;validator_set.pending_active, pool_address))) &#123;<br />        <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE">VALIDATOR_STATUS_PENDING_ACTIVE</a><br />    &#125; <b>else</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&amp;validator_set.active_validators, pool_address))) &#123;<br />        <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a><br />    &#125; <b>else</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&amp;validator_set.pending_inactive, pool_address))) &#123;<br />        <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a><br />    &#125; <b>else</b> &#123;<br />        <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a><br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_current_epoch_voting_power"></a>

## Function `get_current_epoch_voting_power`

Return the voting power of the validator in the current epoch.
This is the same as the validator&apos;s total active and pending_inactive stake.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_current_epoch_voting_power">get_current_epoch_voting_power</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_current_epoch_voting_power">get_current_epoch_voting_power</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> validator_state &#61; <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address);<br />    // Both active and pending inactive validators can still vote in the current epoch.<br />    <b>if</b> (validator_state &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a> &#124;&#124; validator_state &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a>) &#123;<br />        <b>let</b> active_stake &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).active);<br />        <b>let</b> pending_inactive_stake &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).pending_inactive);<br />        active_stake &#43; pending_inactive_stake<br />    &#125; <b>else</b> &#123;<br />        0<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_delegated_voter"></a>

## Function `get_delegated_voter`

Return the delegated voter of the validator at <code>pool_address</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_delegated_voter">get_delegated_voter</a>(pool_address: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_delegated_voter">get_delegated_voter</a>(pool_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).delegated_voter<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_operator"></a>

## Function `get_operator`

Return the operator of the validator at <code>pool_address</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_operator">get_operator</a>(pool_address: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_operator">get_operator</a>(pool_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address).operator_address<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_owned_pool_address"></a>

## Function `get_owned_pool_address`

Return the pool address in <code>owner_cap</code>.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_owned_pool_address">get_owned_pool_address</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_owned_pool_address">get_owned_pool_address</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>): <b>address</b> &#123;<br />    owner_cap.pool_address<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_validator_index"></a>

## Function `get_validator_index`

Return the validator index for <code>pool_address</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_index">get_validator_index</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_index">get_validator_index</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address).validator_index<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_current_epoch_proposal_counts"></a>

## Function `get_current_epoch_proposal_counts`

Return the number of successful and failed proposals for the proposal at the given validator index.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_current_epoch_proposal_counts">get_current_epoch_proposal_counts</a>(validator_index: u64): (u64, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_current_epoch_proposal_counts">get_current_epoch_proposal_counts</a>(validator_index: u64): (u64, u64) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> &#123;<br />    <b>let</b> validator_performances &#61; &amp;<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework).validators;<br />    <b>let</b> validator_performance &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(validator_performances, validator_index);<br />    (validator_performance.successful_proposals, validator_performance.failed_proposals)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_validator_config"></a>

## Function `get_validator_config`

Return the validator&apos;s config.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_config">get_validator_config</a>(pool_address: <b>address</b>): (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_config">get_validator_config</a>(<br />    pool_address: <b>address</b><br />): (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> validator_config &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br />    (validator_config.consensus_pubkey, validator_config.network_addresses, validator_config.fullnode_addresses)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_stake_pool_exists"></a>

## Function `stake_pool_exists`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(addr: <b>address</b>): bool &#123;<br />    <b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_initialize"></a>

## Function `initialize`

Initialize validator set to the core resource account.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    <b>move_to</b>(aptos_framework, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />        consensus_scheme: 0,<br />        active_validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />        pending_active: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />        pending_inactive: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />        total_voting_power: 0,<br />        total_joining_power: 0,<br />    &#125;);<br /><br />    <b>move_to</b>(aptos_framework, <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> &#123;<br />        validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

This is only called during Genesis, which is where MintCapability&lt;AptosCoin&gt; can be created.
Beyond genesis, no one can create AptosCoin mint/burn capabilities.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: MintCapability&lt;AptosCoin&gt;) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>move_to</b>(aptos_framework, <a href="stake.md#0x1_stake_AptosCoinCapabilities">AptosCoinCapabilities</a> &#123; mint_cap &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_remove_validators"></a>

## Function `remove_validators`

Allow on chain governance to remove validators from the validator set.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_remove_validators">remove_validators</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_remove_validators">remove_validators</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    validators: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,<br />) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>let</b> validator_set &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    <b>let</b> active_validators &#61; &amp;<b>mut</b> validator_set.active_validators;<br />    <b>let</b> pending_inactive &#61; &amp;<b>mut</b> validator_set.pending_inactive;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> &#61; len(active_validators);<br />        <b>update</b> <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> &#61; len(pending_inactive);<br />    &#125;;<br />    <b>let</b> len_validators &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validators);<br />    <b>let</b> i &#61; 0;<br />    // Remove each validator from the validator set.<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> i &lt;&#61; len_validators;<br />            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(active_validators);<br />            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(active_validators);<br />            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(pending_inactive);<br />            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(pending_inactive);<br />            <b>invariant</b> <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> &#43; <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> &#61;&#61; len(active_validators) &#43; len(pending_inactive);<br />        &#125;;<br />        i &lt; len_validators<br />    &#125;) &#123;<br />        <b>let</b> validator &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(validators, i);<br />        <b>let</b> validator_index &#61; <a href="stake.md#0x1_stake_find_validator">find_validator</a>(active_validators, validator);<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;validator_index)) &#123;<br />            <b>let</b> validator_info &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(active_validators, &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;validator_index));<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(pending_inactive, validator_info);<br />            <b>spec</b> &#123;<br />                <b>update</b> <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> &#61; <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a> &#45; 1;<br />                <b>update</b> <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> &#61; <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a> &#43; 1;<br />            &#125;;<br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_initialize_stake_owner"></a>

## Function `initialize_stake_owner`

Initialize the validator account and give ownership to the signing account
except it leaves the ValidatorConfig to be set by another entity.
Note: this triggers setting the operator and owner, set it to the account&apos;s address
to set later.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_stake_owner">initialize_stake_owner</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_stake_amount: u64, operator: <b>address</b>, voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_stake_owner">initialize_stake_owner</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    initial_stake_amount: u64,<br />    operator: <b>address</b>,<br />    voter: <b>address</b>,<br />) <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>, <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <a href="stake.md#0x1_stake_initialize_owner">initialize_owner</a>(owner);<br />    <b>move_to</b>(owner, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />        consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />        network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />        fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />        validator_index: 0,<br />    &#125;);<br /><br />    <b>if</b> (initial_stake_amount &gt; 0) &#123;<br />        <a href="stake.md#0x1_stake_add_stake">add_stake</a>(owner, initial_stake_amount);<br />    &#125;;<br /><br />    <b>let</b> account_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>if</b> (account_address !&#61; operator) &#123;<br />        <a href="stake.md#0x1_stake_set_operator">set_operator</a>(owner, operator)<br />    &#125;;<br />    <b>if</b> (account_address !&#61; voter) &#123;<br />        <a href="stake.md#0x1_stake_set_delegated_voter">set_delegated_voter</a>(owner, voter)<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_initialize_validator"></a>

## Function `initialize_validator`

Initialize the validator account and give ownership to the signing account.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator">initialize_validator</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_of_possession: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator">initialize_validator</a>(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    proof_of_possession: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> &#123;<br />    // Checks the <b>public</b> key <b>has</b> a valid proof&#45;of&#45;possession <b>to</b> prevent rogue&#45;key attacks.<br />    <b>let</b> pubkey_from_pop &#61; &amp;<b>mut</b> <a href="../../aptos-stdlib/doc/bls12381.md#0x1_bls12381_public_key_from_bytes_with_pop">bls12381::public_key_from_bytes_with_pop</a>(<br />        consensus_pubkey,<br />        &amp;proof_of_possession_from_bytes(proof_of_possession)<br />    );<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(pubkey_from_pop), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>));<br /><br />    <a href="stake.md#0x1_stake_initialize_owner">initialize_owner</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />        consensus_pubkey,<br />        network_addresses,<br />        fullnode_addresses,<br />        validator_index: 0,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_initialize_owner"></a>

## Function `initialize_owner`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_initialize_owner">initialize_owner</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_initialize_owner">initialize_owner</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>assert</b>!(<a href="stake.md#0x1_stake_is_allowed">is_allowed</a>(owner_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EINELIGIBLE_VALIDATOR">EINELIGIBLE_VALIDATOR</a>));<br />    <b>assert</b>!(!<a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(owner_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="stake.md#0x1_stake_EALREADY_REGISTERED">EALREADY_REGISTERED</a>));<br /><br />    <b>move_to</b>(owner, <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />        active: <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;AptosCoin&gt;(),<br />        pending_active: <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;AptosCoin&gt;(),<br />        pending_inactive: <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;AptosCoin&gt;(),<br />        inactive: <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;AptosCoin&gt;(),<br />        locked_until_secs: 0,<br />        operator_address: owner_address,<br />        delegated_voter: owner_address,<br />        // Events.<br />        initialize_validator_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_RegisterValidatorCandidateEvent">RegisterValidatorCandidateEvent</a>&gt;(owner),<br />        set_operator_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_SetOperatorEvent">SetOperatorEvent</a>&gt;(owner),<br />        add_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_AddStakeEvent">AddStakeEvent</a>&gt;(owner),<br />        reactivate_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_ReactivateStakeEvent">ReactivateStakeEvent</a>&gt;(owner),<br />        rotate_consensus_key_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_RotateConsensusKeyEvent">RotateConsensusKeyEvent</a>&gt;(owner),<br />        update_network_and_fullnode_addresses_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddressesEvent">UpdateNetworkAndFullnodeAddressesEvent</a>&gt;(<br />            owner<br />        ),<br />        increase_lockup_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_IncreaseLockupEvent">IncreaseLockupEvent</a>&gt;(owner),<br />        join_validator_set_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_JoinValidatorSetEvent">JoinValidatorSetEvent</a>&gt;(owner),<br />        distribute_rewards_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_DistributeRewardsEvent">DistributeRewardsEvent</a>&gt;(owner),<br />        unlock_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_UnlockStakeEvent">UnlockStakeEvent</a>&gt;(owner),<br />        withdraw_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_WithdrawStakeEvent">WithdrawStakeEvent</a>&gt;(owner),<br />        leave_validator_set_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="stake.md#0x1_stake_LeaveValidatorSetEvent">LeaveValidatorSetEvent</a>&gt;(owner),<br />    &#125;);<br /><br />    <b>move_to</b>(owner, <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> &#123; pool_address: owner_address &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_extract_owner_cap"></a>

## Function `extract_owner_cap`

Extract and return owner capability from the signing account.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_extract_owner_cap">extract_owner_cap</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_extract_owner_cap">extract_owner_cap</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);<br />    <b>move_from</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_deposit_owner_cap"></a>

## Function `deposit_owner_cap`

Deposit <code>owner_cap</code> into <code><a href="account.md#0x1_account">account</a></code>. This requires <code><a href="account.md#0x1_account">account</a></code> to not already have ownership of another
staking pool.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_deposit_owner_cap">deposit_owner_cap</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_deposit_owner_cap">deposit_owner_cap</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>) &#123;<br />    <b>assert</b>!(!<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>));<br />    <b>move_to</b>(owner, owner_cap);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_destroy_owner_cap"></a>

## Function `destroy_owner_cap`

Destroy <code>owner_cap</code>.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_destroy_owner_cap">destroy_owner_cap</a>(owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_destroy_owner_cap">destroy_owner_cap</a>(owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>) &#123;<br />    <b>let</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> &#123; pool_address: _ &#125; &#61; owner_cap;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_set_operator">set_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_set_operator">set_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);<br />    <b>let</b> ownership_cap &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br />    <a href="stake.md#0x1_stake_set_operator_with_cap">set_operator_with_cap</a>(ownership_cap, new_operator);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_set_operator_with_cap"></a>

## Function `set_operator_with_cap`

Allows an account with ownership capability to change the operator of the stake pool.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_operator_with_cap">set_operator_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_operator_with_cap">set_operator_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, new_operator: <b>address</b>) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <b>let</b> pool_address &#61; owner_cap.pool_address;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    <b>let</b> old_operator &#61; stake_pool.operator_address;<br />    stake_pool.operator_address &#61; new_operator;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="stake.md#0x1_stake_SetOperator">SetOperator</a> &#123;<br />                pool_address,<br />                old_operator,<br />                new_operator,<br />            &#125;,<br />        );<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.set_operator_events,<br />        <a href="stake.md#0x1_stake_SetOperatorEvent">SetOperatorEvent</a> &#123;<br />            pool_address,<br />            old_operator,<br />            new_operator,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_set_delegated_voter"></a>

## Function `set_delegated_voter`

Allows an owner to change the delegated voter of the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter">set_delegated_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter">set_delegated_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);<br />    <b>let</b> ownership_cap &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br />    <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">set_delegated_voter_with_cap</a>(ownership_cap, new_voter);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_set_delegated_voter_with_cap"></a>

## Function `set_delegated_voter_with_cap`

Allows an owner to change the delegated voter of the stake pool.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">set_delegated_voter_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">set_delegated_voter_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, new_voter: <b>address</b>) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <b>let</b> pool_address &#61; owner_cap.pool_address;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    stake_pool.delegated_voter &#61; new_voter;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins from the <code><a href="account.md#0x1_account">account</a></code> owning the StakePool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_add_stake">add_stake</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_add_stake">add_stake</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);<br />    <b>let</b> ownership_cap &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br />    <a href="stake.md#0x1_stake_add_stake_with_cap">add_stake_with_cap</a>(ownership_cap, <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;AptosCoin&gt;(owner, amount));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_add_stake_with_cap"></a>

## Function `add_stake_with_cap`

Add <code>coins</code> into <code>pool_address</code>. this requires the corresponding <code>owner_cap</code> to be passed in.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_add_stake_with_cap">add_stake_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_add_stake_with_cap">add_stake_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, coins: Coin&lt;AptosCoin&gt;) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <b>let</b> pool_address &#61; owner_cap.pool_address;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br /><br />    <b>let</b> amount &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;coins);<br />    <b>if</b> (amount &#61;&#61; 0) &#123;<br />        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);<br />        <b>return</b><br />    &#125;;<br /><br />    // Only track and validate <a href="voting.md#0x1_voting">voting</a> power increase for active and pending_active validator.<br />    // Pending_inactive validator will be removed from the validator set in the next epoch.<br />    // Inactive validator&apos;s total <a href="stake.md#0x1_stake">stake</a> will be tracked when they join the validator set.<br />    <b>let</b> validator_set &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    // Search directly rather using get_validator_state <b>to</b> save on unnecessary loops.<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&amp;validator_set.active_validators, pool_address)) &#124;&#124;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="stake.md#0x1_stake_find_validator">find_validator</a>(&amp;validator_set.pending_active, pool_address))) &#123;<br />        <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(amount);<br />    &#125;;<br /><br />    // Add <b>to</b> pending_active <b>if</b> it&apos;s a current validator because the <a href="stake.md#0x1_stake">stake</a> is not counted until the next epoch.<br />    // Otherwise, the delegation can be added <b>to</b> active directly <b>as</b> the validator is also activated in the epoch.<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    <b>if</b> (<a href="stake.md#0x1_stake_is_current_epoch_validator">is_current_epoch_validator</a>(pool_address)) &#123;<br />        <a href="coin.md#0x1_coin_merge">coin::merge</a>&lt;AptosCoin&gt;(&amp;<b>mut</b> stake_pool.pending_active, coins);<br />    &#125; <b>else</b> &#123;<br />        <a href="coin.md#0x1_coin_merge">coin::merge</a>&lt;AptosCoin&gt;(&amp;<b>mut</b> stake_pool.active, coins);<br />    &#125;;<br /><br />    <b>let</b> (_, maximum_stake) &#61; <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&amp;<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());<br />    <b>let</b> voting_power &#61; <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool);<br />    <b>assert</b>!(voting_power &lt;&#61; maximum_stake, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ESTAKE_EXCEEDS_MAX">ESTAKE_EXCEEDS_MAX</a>));<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="stake.md#0x1_stake_AddStake">AddStake</a> &#123;<br />                pool_address,<br />                amount_added: amount,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.add_stake_events,<br />        <a href="stake.md#0x1_stake_AddStakeEvent">AddStakeEvent</a> &#123;<br />            pool_address,<br />            amount_added: amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake">reactivate_stake</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake">reactivate_stake</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);<br />    <b>let</b> ownership_cap &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br />    <a href="stake.md#0x1_stake_reactivate_stake_with_cap">reactivate_stake_with_cap</a>(ownership_cap, amount);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_reactivate_stake_with_cap"></a>

## Function `reactivate_stake_with_cap`



<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake_with_cap">reactivate_stake_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake_with_cap">reactivate_stake_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <b>let</b> pool_address &#61; owner_cap.pool_address;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br /><br />    // Cap the amount <b>to</b> reactivate by the amount in pending_inactive.<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    <b>let</b> total_pending_inactive &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.pending_inactive);<br />    amount &#61; <b>min</b>(amount, total_pending_inactive);<br /><br />    // Since this does not count <b>as</b> a <a href="voting.md#0x1_voting">voting</a> power change (pending inactive still counts <b>as</b> <a href="voting.md#0x1_voting">voting</a> power in the<br />    // current epoch), <a href="stake.md#0x1_stake">stake</a> can be immediately moved from pending inactive <b>to</b> active.<br />    // We also don&apos;t need <b>to</b> check <a href="voting.md#0x1_voting">voting</a> power increase <b>as</b> there&apos;s none.<br />    <b>let</b> reactivated_coins &#61; <a href="coin.md#0x1_coin_extract">coin::extract</a>(&amp;<b>mut</b> stake_pool.pending_inactive, amount);<br />    <a href="coin.md#0x1_coin_merge">coin::merge</a>(&amp;<b>mut</b> stake_pool.active, reactivated_coins);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="stake.md#0x1_stake_ReactivateStake">ReactivateStake</a> &#123;<br />                pool_address,<br />                amount,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.reactivate_stake_events,<br />        <a href="stake.md#0x1_stake_ReactivateStakeEvent">ReactivateStakeEvent</a> &#123;<br />            pool_address,<br />            amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_rotate_consensus_key"></a>

## Function `rotate_consensus_key`

Rotate the consensus key of the validator, it&apos;ll take effect in next epoch.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_rotate_consensus_key">rotate_consensus_key</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_of_possession: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_rotate_consensus_key">rotate_consensus_key</a>(<br />    operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    new_consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    proof_of_possession: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br /><br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) &#61;&#61; stake_pool.operator_address, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>));<br /><br />    <b>assert</b>!(<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EVALIDATOR_CONFIG">EVALIDATOR_CONFIG</a>));<br />    <b>let</b> validator_info &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br />    <b>let</b> old_consensus_pubkey &#61; validator_info.consensus_pubkey;<br />    // Checks the <b>public</b> key <b>has</b> a valid proof&#45;of&#45;possession <b>to</b> prevent rogue&#45;key attacks.<br />    <b>let</b> pubkey_from_pop &#61; &amp;<b>mut</b> <a href="../../aptos-stdlib/doc/bls12381.md#0x1_bls12381_public_key_from_bytes_with_pop">bls12381::public_key_from_bytes_with_pop</a>(<br />        new_consensus_pubkey,<br />        &amp;proof_of_possession_from_bytes(proof_of_possession)<br />    );<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(pubkey_from_pop), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>));<br />    validator_info.consensus_pubkey &#61; new_consensus_pubkey;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="stake.md#0x1_stake_RotateConsensusKey">RotateConsensusKey</a> &#123;<br />                pool_address,<br />                old_consensus_pubkey,<br />                new_consensus_pubkey,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.rotate_consensus_key_events,<br />        <a href="stake.md#0x1_stake_RotateConsensusKeyEvent">RotateConsensusKeyEvent</a> &#123;<br />            pool_address,<br />            old_consensus_pubkey,<br />            new_consensus_pubkey,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_update_network_and_fullnode_addresses"></a>

## Function `update_network_and_fullnode_addresses`

Update the network and full node addresses of the validator. This only takes effect in the next epoch.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_update_network_and_fullnode_addresses">update_network_and_fullnode_addresses</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, new_fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_update_network_and_fullnode_addresses">update_network_and_fullnode_addresses</a>(<br />    operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    new_network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    new_fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) &#61;&#61; stake_pool.operator_address, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>));<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EVALIDATOR_CONFIG">EVALIDATOR_CONFIG</a>));<br />    <b>let</b> validator_info &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br />    <b>let</b> old_network_addresses &#61; validator_info.network_addresses;<br />    validator_info.network_addresses &#61; new_network_addresses;<br />    <b>let</b> old_fullnode_addresses &#61; validator_info.fullnode_addresses;<br />    validator_info.fullnode_addresses &#61; new_fullnode_addresses;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddresses">UpdateNetworkAndFullnodeAddresses</a> &#123;<br />                pool_address,<br />                old_network_addresses,<br />                new_network_addresses,<br />                old_fullnode_addresses,<br />                new_fullnode_addresses,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.update_network_and_fullnode_addresses_events,<br />        <a href="stake.md#0x1_stake_UpdateNetworkAndFullnodeAddressesEvent">UpdateNetworkAndFullnodeAddressesEvent</a> &#123;<br />            pool_address,<br />            old_network_addresses,<br />            new_network_addresses,<br />            old_fullnode_addresses,<br />            new_fullnode_addresses,<br />        &#125;,<br />    );<br /><br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_increase_lockup"></a>

## Function `increase_lockup`

Similar to increase_lockup_with_cap but will use ownership capability from the signing account.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup">increase_lockup</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup">increase_lockup</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);<br />    <b>let</b> ownership_cap &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br />    <a href="stake.md#0x1_stake_increase_lockup_with_cap">increase_lockup_with_cap</a>(ownership_cap);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_increase_lockup_with_cap"></a>

## Function `increase_lockup_with_cap`

Unlock from active delegation, it&apos;s moved to pending_inactive if locked_until_secs &lt; current_time or
directly inactive if it&apos;s not from an active validator.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup_with_cap">increase_lockup_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup_with_cap">increase_lockup_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <b>let</b> pool_address &#61; owner_cap.pool_address;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> config &#61; <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();<br /><br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    <b>let</b> old_locked_until_secs &#61; stake_pool.locked_until_secs;<br />    <b>let</b> new_locked_until_secs &#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &#43; <a href="staking_config.md#0x1_staking_config_get_recurring_lockup_duration">staking_config::get_recurring_lockup_duration</a>(&amp;config);<br />    <b>assert</b>!(old_locked_until_secs &lt; new_locked_until_secs, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EINVALID_LOCKUP">EINVALID_LOCKUP</a>));<br />    stake_pool.locked_until_secs &#61; new_locked_until_secs;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="stake.md#0x1_stake_IncreaseLockup">IncreaseLockup</a> &#123;<br />                pool_address,<br />                old_locked_until_secs,<br />                new_locked_until_secs,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.increase_lockup_events,<br />        <a href="stake.md#0x1_stake_IncreaseLockupEvent">IncreaseLockupEvent</a> &#123;<br />            pool_address,<br />            old_locked_until_secs,<br />            new_locked_until_secs,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_join_validator_set"></a>

## Function `join_validator_set`

This can only called by the operator of the validator/staking pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set">join_validator_set</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set">join_validator_set</a>(<br />    operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b><br />) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <b>assert</b>!(<br />        <a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(&amp;<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>()),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED">ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED</a>),<br />    );<br /><br />    <a href="stake.md#0x1_stake_join_validator_set_internal">join_validator_set_internal</a>(operator, pool_address);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_join_validator_set_internal"></a>

## Function `join_validator_set_internal`

Request to have <code>pool_address</code> join the validator set. Can only be called after calling <code>initialize_validator</code>.
If the validator has the required stake (more than minimum and less than maximum allowed), they will be
added to the pending_active queue. All validators in this queue will be added to the active set when the next
epoch starts (eligibility will be rechecked).

This internal version can only be called by the Genesis module during Genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set_internal">join_validator_set_internal</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set_internal">join_validator_set_internal</a>(<br />    operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b><br />) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) &#61;&#61; stake_pool.operator_address, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>));<br />    <b>assert</b>!(<br />        <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address) &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a>,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="stake.md#0x1_stake_EALREADY_ACTIVE_VALIDATOR">EALREADY_ACTIVE_VALIDATOR</a>),<br />    );<br /><br />    <b>let</b> config &#61; <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();<br />    <b>let</b> (minimum_stake, maximum_stake) &#61; <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&amp;config);<br />    <b>let</b> voting_power &#61; <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool);<br />    <b>assert</b>!(voting_power &gt;&#61; minimum_stake, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ESTAKE_TOO_LOW">ESTAKE_TOO_LOW</a>));<br />    <b>assert</b>!(voting_power &lt;&#61; maximum_stake, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ESTAKE_TOO_HIGH">ESTAKE_TOO_HIGH</a>));<br /><br />    // Track and validate <a href="voting.md#0x1_voting">voting</a> power increase.<br />    <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(voting_power);<br /><br />    // Add validator <b>to</b> pending_active, <b>to</b> be activated in the next epoch.<br />    <b>let</b> validator_config &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br />    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;validator_config.consensus_pubkey), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>));<br /><br />    // Validate the current validator set size <b>has</b> not exceeded the limit.<br />    <b>let</b> validator_set &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(<br />        &amp;<b>mut</b> validator_set.pending_active,<br />        <a href="stake.md#0x1_stake_generate_validator_info">generate_validator_info</a>(pool_address, stake_pool, &#42;validator_config)<br />    );<br />    <b>let</b> validator_set_size &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;validator_set.active_validators) &#43; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(<br />        &amp;validator_set.pending_active<br />    );<br />    <b>assert</b>!(validator_set_size &lt;&#61; <a href="stake.md#0x1_stake_MAX_VALIDATOR_SET_SIZE">MAX_VALIDATOR_SET_SIZE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EVALIDATOR_SET_TOO_LARGE">EVALIDATOR_SET_TOO_LARGE</a>));<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="stake.md#0x1_stake_JoinValidatorSet">JoinValidatorSet</a> &#123; pool_address &#125;);<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.join_validator_set_events,<br />        <a href="stake.md#0x1_stake_JoinValidatorSetEvent">JoinValidatorSetEvent</a> &#123; pool_address &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_unlock"></a>

## Function `unlock`

Similar to unlock_with_cap but will use ownership capability from the signing account.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_unlock">unlock</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_unlock">unlock</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);<br />    <b>let</b> ownership_cap &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br />    <a href="stake.md#0x1_stake_unlock_with_cap">unlock_with_cap</a>(amount, ownership_cap);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_unlock_with_cap"></a>

## Function `unlock_with_cap`

Unlock <code>amount</code> from the active stake. Only possible if the lockup has expired.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_unlock_with_cap">unlock_with_cap</a>(amount: u64, owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_unlock_with_cap">unlock_with_cap</a>(amount: u64, owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    // Short&#45;circuit <b>if</b> amount <b>to</b> unlock is 0 so we don&apos;t emit events.<br />    <b>if</b> (amount &#61;&#61; 0) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    // Unlocked coins are moved <b>to</b> pending_inactive. When the current lockup cycle expires, they will be moved into<br />    // inactive in the earliest possible epoch transition.<br />    <b>let</b> pool_address &#61; owner_cap.pool_address;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    // Cap amount <b>to</b> unlock by maximum active <a href="stake.md#0x1_stake">stake</a>.<br />    <b>let</b> amount &#61; <b>min</b>(amount, <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.active));<br />    <b>let</b> unlocked_stake &#61; <a href="coin.md#0x1_coin_extract">coin::extract</a>(&amp;<b>mut</b> stake_pool.active, amount);<br />    <a href="coin.md#0x1_coin_merge">coin::merge</a>&lt;AptosCoin&gt;(&amp;<b>mut</b> stake_pool.pending_inactive, unlocked_stake);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="stake.md#0x1_stake_UnlockStake">UnlockStake</a> &#123;<br />                pool_address,<br />                amount_unlocked: amount,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.unlock_stake_events,<br />        <a href="stake.md#0x1_stake_UnlockStakeEvent">UnlockStakeEvent</a> &#123;<br />            pool_address,<br />            amount_unlocked: amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_withdraw"></a>

## Function `withdraw`

Withdraw from <code><a href="account.md#0x1_account">account</a></code>&apos;s inactive stake.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_withdraw">withdraw</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, withdraw_amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_withdraw">withdraw</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    withdraw_amount: u64<br />) <b>acquires</b> <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner_address);<br />    <b>let</b> ownership_cap &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br />    <b>let</b> coins &#61; <a href="stake.md#0x1_stake_withdraw_with_cap">withdraw_with_cap</a>(ownership_cap, withdraw_amount);<br />    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>&lt;AptosCoin&gt;(owner_address, coins);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_withdraw_with_cap"></a>

## Function `withdraw_with_cap`

Withdraw from <code>pool_address</code>&apos;s inactive stake with the corresponding <code>owner_cap</code>.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_withdraw_with_cap">withdraw_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, withdraw_amount: u64): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_withdraw_with_cap">withdraw_with_cap</a>(<br />    owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>,<br />    withdraw_amount: u64<br />): Coin&lt;AptosCoin&gt; <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <b>let</b> pool_address &#61; owner_cap.pool_address;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    // There&apos;s an edge case <b>where</b> a validator unlocks their <a href="stake.md#0x1_stake">stake</a> and leaves the validator set before<br />    // the <a href="stake.md#0x1_stake">stake</a> is fully unlocked (the current lockup cycle <b>has</b> not expired yet).<br />    // This can leave their <a href="stake.md#0x1_stake">stake</a> stuck in pending_inactive even after the current lockup cycle expires.<br />    <b>if</b> (<a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address) &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a> &amp;&amp;<br />        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt;&#61; stake_pool.locked_until_secs) &#123;<br />        <b>let</b> pending_inactive_stake &#61; <a href="coin.md#0x1_coin_extract_all">coin::extract_all</a>(&amp;<b>mut</b> stake_pool.pending_inactive);<br />        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&amp;<b>mut</b> stake_pool.inactive, pending_inactive_stake);<br />    &#125;;<br /><br />    // Cap withdraw amount by total inactive coins.<br />    withdraw_amount &#61; <b>min</b>(withdraw_amount, <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.inactive));<br />    <b>if</b> (withdraw_amount &#61;&#61; 0) <b>return</b> <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;AptosCoin&gt;();<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="stake.md#0x1_stake_WithdrawStake">WithdrawStake</a> &#123;<br />                pool_address,<br />                amount_withdrawn: withdraw_amount,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.withdraw_stake_events,<br />        <a href="stake.md#0x1_stake_WithdrawStakeEvent">WithdrawStakeEvent</a> &#123;<br />            pool_address,<br />            amount_withdrawn: withdraw_amount,<br />        &#125;,<br />    );<br /><br />    <a href="coin.md#0x1_coin_extract">coin::extract</a>(&amp;<b>mut</b> stake_pool.inactive, withdraw_amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_leave_validator_set"></a>

## Function `leave_validator_set`

Request to have <code>pool_address</code> leave the validator set. The validator is only actually removed from the set when
the next epoch starts.
The last validator in the set cannot leave. This is an edge case that should never happen as long as the network
is still operational.

Can only be called by the operator of the validator/staking pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_leave_validator_set">leave_validator_set</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_leave_validator_set">leave_validator_set</a>(<br />    operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b><br />) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>();<br />    <b>let</b> config &#61; <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();<br />    <b>assert</b>!(<br />        <a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(&amp;config),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED">ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED</a>),<br />    );<br /><br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    // Account <b>has</b> <b>to</b> be the operator.<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) &#61;&#61; stake_pool.operator_address, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="stake.md#0x1_stake_ENOT_OPERATOR">ENOT_OPERATOR</a>));<br /><br />    <b>let</b> validator_set &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    // If the validator is still pending_active, directly kick the validator out.<br />    <b>let</b> maybe_pending_active_index &#61; <a href="stake.md#0x1_stake_find_validator">find_validator</a>(&amp;validator_set.pending_active, pool_address);<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;maybe_pending_active_index)) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(<br />            &amp;<b>mut</b> validator_set.pending_active, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> maybe_pending_active_index));<br /><br />        // Decrease the <a href="voting.md#0x1_voting">voting</a> power increase <b>as</b> the pending validator&apos;s <a href="voting.md#0x1_voting">voting</a> power was added when they requested<br />        // <b>to</b> join. Now that they changed their mind, their <a href="voting.md#0x1_voting">voting</a> power should not affect the joining limit of this<br />        // epoch.<br />        <b>let</b> validator_stake &#61; (<a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool) <b>as</b> u128);<br />        // total_joining_power should be larger than validator_stake but just in case there <b>has</b> been a small<br />        // rounding <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> somewhere that can lead <b>to</b> an underflow, we still want <b>to</b> allow this transaction <b>to</b><br />        // succeed.<br />        <b>if</b> (validator_set.total_joining_power &gt; validator_stake) &#123;<br />            validator_set.total_joining_power &#61; validator_set.total_joining_power &#45; validator_stake;<br />        &#125; <b>else</b> &#123;<br />            validator_set.total_joining_power &#61; 0;<br />        &#125;;<br />    &#125; <b>else</b> &#123;<br />        // Validate that the validator is already part of the validator set.<br />        <b>let</b> maybe_active_index &#61; <a href="stake.md#0x1_stake_find_validator">find_validator</a>(&amp;validator_set.active_validators, pool_address);<br />        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;maybe_active_index), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="stake.md#0x1_stake_ENOT_VALIDATOR">ENOT_VALIDATOR</a>));<br />        <b>let</b> validator_info &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(<br />            &amp;<b>mut</b> validator_set.active_validators, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> maybe_active_index));<br />        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;validator_set.active_validators) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="stake.md#0x1_stake_ELAST_VALIDATOR">ELAST_VALIDATOR</a>));<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> validator_set.pending_inactive, validator_info);<br /><br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="stake.md#0x1_stake_LeaveValidatorSet">LeaveValidatorSet</a> &#123; pool_address &#125;);<br />        &#125;;<br />        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />            &amp;<b>mut</b> stake_pool.leave_validator_set_events,<br />            <a href="stake.md#0x1_stake_LeaveValidatorSetEvent">LeaveValidatorSetEvent</a> &#123;<br />                pool_address,<br />            &#125;,<br />        );<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_is_current_epoch_validator"></a>

## Function `is_current_epoch_validator`

Returns true if the current validator can still vote in the current epoch.
This includes validators that requested to leave but are still in the pending_inactive queue and will be removed
when the epoch starts.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_is_current_epoch_validator">is_current_epoch_validator</a>(pool_address: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_is_current_epoch_validator">is_current_epoch_validator</a>(pool_address: <b>address</b>): bool <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address);<br />    <b>let</b> validator_state &#61; <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address);<br />    validator_state &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a> &#124;&#124; validator_state &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a><br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_update_performance_statistics"></a>

## Function `update_performance_statistics`

Update the validator performance (proposal statistics). This is only called by block::prologue().
This function cannot abort.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_update_performance_statistics">update_performance_statistics</a>(proposer_index: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_update_performance_statistics">update_performance_statistics</a>(<br />    proposer_index: Option&lt;u64&gt;,<br />    failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;<br />) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> &#123;<br />    // Validator set cannot change until the end of the epoch, so the validator index in arguments should<br />    // match <b>with</b> those of the validators in <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a> resource.<br />    <b>let</b> validator_perf &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework);<br />    <b>let</b> validator_len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;validator_perf.validators);<br /><br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="stake.md#0x1_stake_ghost_valid_perf">ghost_valid_perf</a> &#61; validator_perf;<br />        <b>update</b> <a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a> &#61; proposer_index;<br />    &#125;;<br />    // proposer_index is an <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">option</a> because it can be missing (for NilBlocks)<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;proposer_index)) &#123;<br />        <b>let</b> cur_proposer_index &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> proposer_index);<br />        // Here, and in all other <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>, skip <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> validator indices that are out of bounds,<br />        // this <b>ensures</b> that this function doesn&apos;t <b>abort</b> <b>if</b> there are out of bounds errors.<br />        <b>if</b> (cur_proposer_index &lt; validator_len) &#123;<br />            <b>let</b> validator &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> validator_perf.validators, cur_proposer_index);<br />            <b>spec</b> &#123;<br />                <b>assume</b> validator.successful_proposals &#43; 1 &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />            &#125;;<br />            validator.successful_proposals &#61; validator.successful_proposals &#43; 1;<br />        &#125;;<br />    &#125;;<br /><br />    <b>let</b> f &#61; 0;<br />    <b>let</b> f_len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;failed_proposer_indices);<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> len(validator_perf.validators) &#61;&#61; validator_len;<br />            <b>invariant</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>) &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<br />                <a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a><br />            ) &lt; validator_len) &#61;&#61;&gt;<br />                (validator_perf.validators[<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>)].successful_proposals &#61;&#61;<br />                    <a href="stake.md#0x1_stake_ghost_valid_perf">ghost_valid_perf</a>.validators[<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>)].successful_proposals &#43; 1);<br />        &#125;;<br />        f &lt; f_len<br />    &#125;) &#123;<br />        <b>let</b> validator_index &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;failed_proposer_indices, f);<br />        <b>if</b> (validator_index &lt; validator_len) &#123;<br />            <b>let</b> validator &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> validator_perf.validators, validator_index);<br />            <b>spec</b> &#123;<br />                <b>assume</b> validator.failed_proposals &#43; 1 &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />            &#125;;<br />            validator.failed_proposals &#61; validator.failed_proposals &#43; 1;<br />        &#125;;<br />        f &#61; f &#43; 1;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_on_new_epoch"></a>

## Function `on_new_epoch`

Triggered during a reconfiguration. This function shouldn&apos;t abort.

1. Distribute transaction fees and rewards to stake pools of active and pending inactive validators (requested
to leave but not yet removed).
2. Officially move pending active stake to active and move pending inactive stake to inactive.
The staking pool&apos;s voting power in this new epoch will be updated to the total active stake.
3. Add pending active validators to the active set if they satisfy requirements so they can vote and remove
pending inactive validators so they no longer can vote.
4. The validator&apos;s voting power in the validator set is updated to be the corresponding staking pool&apos;s voting
power.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_on_new_epoch">on_new_epoch</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_on_new_epoch">on_new_epoch</a>(<br />) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_AptosCoinCapabilities">AptosCoinCapabilities</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>, <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>, <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>, <a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a> &#123;<br />    <b>let</b> validator_set &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    <b>let</b> config &#61; <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();<br />    <b>let</b> validator_perf &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework);<br /><br />    // Process pending <a href="stake.md#0x1_stake">stake</a> and distribute transaction fees and rewards for each currently active validator.<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;validator_set.active_validators, &#124;validator&#124; &#123;<br />        <b>let</b> validator: &amp;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#61; validator;<br />        <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(validator_perf, validator.addr, &amp;config);<br />    &#125;);<br /><br />    // Process pending <a href="stake.md#0x1_stake">stake</a> and distribute transaction fees and rewards for each currently pending_inactive validator<br />    // (requested <b>to</b> leave but not removed yet).<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;validator_set.pending_inactive, &#124;validator&#124; &#123;<br />        <b>let</b> validator: &amp;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#61; validator;<br />        <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(validator_perf, validator.addr, &amp;config);<br />    &#125;);<br /><br />    // Activate currently pending_active validators.<br />    <a href="stake.md#0x1_stake_append">append</a>(&amp;<b>mut</b> validator_set.active_validators, &amp;<b>mut</b> validator_set.pending_active);<br /><br />    // Officially deactivate all pending_inactive validators. They will now no longer receive rewards.<br />    validator_set.pending_inactive &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br /><br />    // Update active validator set so that network <b>address</b>/<b>public</b> key change takes effect.<br />    // Moreover, recalculate the total <a href="voting.md#0x1_voting">voting</a> power, and deactivate the validator whose<br />    // <a href="voting.md#0x1_voting">voting</a> power is less than the minimum required <a href="stake.md#0x1_stake">stake</a>.<br />    <b>let</b> next_epoch_validators &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <b>let</b> (minimum_stake, _) &#61; <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&amp;config);<br />    <b>let</b> vlen &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;validator_set.active_validators);<br />    <b>let</b> total_voting_power &#61; 0;<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(next_epoch_validators);<br />            <b>invariant</b> i &lt;&#61; vlen;<br />        &#125;;<br />        i &lt; vlen<br />    &#125;) &#123;<br />        <b>let</b> old_validator_info &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> validator_set.active_validators, i);<br />        <b>let</b> pool_address &#61; old_validator_info.addr;<br />        <b>let</b> validator_config &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br />        <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />        <b>let</b> new_validator_info &#61; <a href="stake.md#0x1_stake_generate_validator_info">generate_validator_info</a>(pool_address, stake_pool, &#42;validator_config);<br /><br />        // A validator needs at least the <b>min</b> <a href="stake.md#0x1_stake">stake</a> required <b>to</b> join the validator set.<br />        <b>if</b> (new_validator_info.voting_power &gt;&#61; minimum_stake) &#123;<br />            <b>spec</b> &#123;<br />                <b>assume</b> total_voting_power &#43; new_validator_info.voting_power &lt;&#61; MAX_U128;<br />            &#125;;<br />            total_voting_power &#61; total_voting_power &#43; (new_validator_info.voting_power <b>as</b> u128);<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> next_epoch_validators, new_validator_info);<br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br /><br />    validator_set.active_validators &#61; next_epoch_validators;<br />    validator_set.total_voting_power &#61; total_voting_power;<br />    validator_set.total_joining_power &#61; 0;<br /><br />    // Update validator indices, reset performance scores, and renew lockups.<br />    validator_perf.validators &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <b>let</b> recurring_lockup_duration_secs &#61; <a href="staking_config.md#0x1_staking_config_get_recurring_lockup_duration">staking_config::get_recurring_lockup_duration</a>(&amp;config);<br />    <b>let</b> vlen &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;validator_set.active_validators);<br />    <b>let</b> validator_index &#61; 0;<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validator_set.active_validators);<br />            <b>invariant</b> len(validator_set.pending_active) &#61;&#61; 0;<br />            <b>invariant</b> len(validator_set.pending_inactive) &#61;&#61; 0;<br />            <b>invariant</b> 0 &lt;&#61; validator_index &amp;&amp; validator_index &lt;&#61; vlen;<br />            <b>invariant</b> vlen &#61;&#61; len(validator_set.active_validators);<br />            <b>invariant</b> <b>forall</b> i in 0..validator_index:<br />                <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(validator_set.active_validators[i].addr).validator_index &lt; validator_index;<br />            <b>invariant</b> <b>forall</b> i in 0..validator_index:<br />                validator_set.active_validators[i].config.validator_index &lt; validator_index;<br />            <b>invariant</b> len(validator_perf.validators) &#61;&#61; validator_index;<br />        &#125;;<br />        validator_index &lt; vlen<br />    &#125;) &#123;<br />        <b>let</b> validator_info &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> validator_set.active_validators, validator_index);<br />        validator_info.config.validator_index &#61; validator_index;<br />        <b>let</b> validator_config &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(validator_info.addr);<br />        validator_config.validator_index &#61; validator_index;<br /><br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> validator_perf.validators, <a href="stake.md#0x1_stake_IndividualValidatorPerformance">IndividualValidatorPerformance</a> &#123;<br />            successful_proposals: 0,<br />            failed_proposals: 0,<br />        &#125;);<br /><br />        // Automatically renew a validator&apos;s lockup for validators that will still be in the validator set in the<br />        // next epoch.<br />        <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(validator_info.addr);<br />        <b>let</b> now_secs &#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();<br />        <b>let</b> reconfig_start_secs &#61; <b>if</b> (<a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>()) &#123;<br />            <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>()<br />        &#125; <b>else</b> &#123;<br />            now_secs<br />        &#125;;<br />        <b>if</b> (stake_pool.locked_until_secs &lt;&#61; reconfig_start_secs) &#123;<br />            <b>spec</b> &#123;<br />                <b>assume</b> now_secs &#43; recurring_lockup_duration_secs &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />            &#125;;<br />            stake_pool.locked_until_secs &#61; now_secs &#43; recurring_lockup_duration_secs;<br />        &#125;;<br /><br />        validator_index &#61; validator_index &#43; 1;<br />    &#125;;<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_periodical_reward_rate_decrease_enabled">features::periodical_reward_rate_decrease_enabled</a>()) &#123;<br />        // Update rewards rate after reward distribution.<br />        <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_epoch_rewards_rate">staking_config::calculate_and_save_latest_epoch_rewards_rate</a>();<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_cur_validator_consensus_infos"></a>

## Function `cur_validator_consensus_infos`

Return the <code>ValidatorConsensusInfo</code> of each current validator, sorted by current validator index.


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_cur_validator_consensus_infos">cur_validator_consensus_infos</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_cur_validator_consensus_infos">cur_validator_consensus_infos</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt; <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <b>let</b> validator_set &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(validator_set)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_next_validator_consensus_infos"></a>

## Function `next_validator_consensus_infos`



<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_next_validator_consensus_infos">next_validator_consensus_infos</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_next_validator_consensus_infos">next_validator_consensus_infos</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt; <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>, <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>, <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />    // Init.<br />    <b>let</b> cur_validator_set &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    <b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> &#61; <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();<br />    <b>let</b> validator_perf &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework);<br />    <b>let</b> (minimum_stake, _) &#61; <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&amp;<a href="staking_config.md#0x1_staking_config">staking_config</a>);<br />    <b>let</b> (rewards_rate, rewards_rate_denominator) &#61; <a href="staking_config.md#0x1_staking_config_get_reward_rate">staking_config::get_reward_rate</a>(&amp;<a href="staking_config.md#0x1_staking_config">staking_config</a>);<br /><br />    // Compute new validator set.<br />    <b>let</b> new_active_validators &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    <b>let</b> num_new_actives &#61; 0;<br />    <b>let</b> candidate_idx &#61; 0;<br />    <b>let</b> new_total_power &#61; 0;<br />    <b>let</b> num_cur_actives &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;cur_validator_set.active_validators);<br />    <b>let</b> num_cur_pending_actives &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;cur_validator_set.pending_active);<br />    <b>spec</b> &#123;<br />        <b>assume</b> num_cur_actives &#43; num_cur_pending_actives &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    <b>let</b> num_candidates &#61; num_cur_actives &#43; num_cur_pending_actives;<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> candidate_idx &lt;&#61; num_candidates;<br />            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(new_active_validators);<br />            <b>invariant</b> len(new_active_validators) &#61;&#61; num_new_actives;<br />            <b>invariant</b> <b>forall</b> i in 0..len(new_active_validators):<br />                new_active_validators[i].config.validator_index &#61;&#61; i;<br />            <b>invariant</b> num_new_actives &lt;&#61; candidate_idx;<br />            <b>invariant</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(new_active_validators);<br />        &#125;;<br />        candidate_idx &lt; num_candidates<br />    &#125;) &#123;<br />        <b>let</b> candidate_in_current_validator_set &#61; candidate_idx &lt; num_cur_actives;<br />        <b>let</b> candidate &#61; <b>if</b> (candidate_idx &lt; num_cur_actives) &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;cur_validator_set.active_validators, candidate_idx)<br />        &#125; <b>else</b> &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;cur_validator_set.pending_active, candidate_idx &#45; num_cur_actives)<br />        &#125;;<br />        <b>let</b> stake_pool &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(candidate.addr);<br />        <b>let</b> cur_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.active);<br />        <b>let</b> cur_pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.pending_active);<br />        <b>let</b> cur_pending_inactive &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.pending_inactive);<br /><br />        <b>let</b> cur_reward &#61; <b>if</b> (candidate_in_current_validator_set &amp;&amp; cur_active &gt; 0) &#123;<br />            <b>spec</b> &#123;<br />                <b>assert</b> candidate.config.validator_index &lt; len(validator_perf.validators);<br />            &#125;;<br />            <b>let</b> cur_perf &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;validator_perf.validators, candidate.config.validator_index);<br />            <b>spec</b> &#123;<br />                <b>assume</b> cur_perf.successful_proposals &#43; cur_perf.failed_proposals &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />            &#125;;<br />            <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(cur_active, cur_perf.successful_proposals, cur_perf.successful_proposals &#43; cur_perf.failed_proposals, rewards_rate, rewards_rate_denominator)<br />        &#125; <b>else</b> &#123;<br />            0<br />        &#125;;<br /><br />        <b>let</b> cur_fee &#61; 0;<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_collect_and_distribute_gas_fees">features::collect_and_distribute_gas_fees</a>()) &#123;<br />            <b>let</b> fees_table &#61; &amp;<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework).fees_table;<br />            <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(fees_table, candidate.addr)) &#123;<br />                <b>let</b> fee_coin &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(fees_table, candidate.addr);<br />                cur_fee &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(fee_coin);<br />            &#125;<br />        &#125;;<br /><br />        <b>let</b> lockup_expired &#61; <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>() &gt;&#61; stake_pool.locked_until_secs;<br />        <b>spec</b> &#123;<br />            <b>assume</b> cur_active &#43; cur_pending_active &#43; cur_reward &#43; cur_fee &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />            <b>assume</b> cur_active &#43; cur_pending_inactive &#43; cur_pending_active &#43; cur_reward &#43; cur_fee &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />        &#125;;<br />        <b>let</b> new_voting_power &#61;<br />            cur_active<br />            &#43; <b>if</b> (lockup_expired) &#123; 0 &#125; <b>else</b> &#123; cur_pending_inactive &#125;<br />            &#43; cur_pending_active<br />            &#43; cur_reward &#43; cur_fee;<br /><br />        <b>if</b> (new_voting_power &gt;&#61; minimum_stake) &#123;<br />            <b>let</b> config &#61; &#42;<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(candidate.addr);<br />            config.validator_index &#61; num_new_actives;<br />            <b>let</b> new_validator_info &#61; <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#123;<br />                addr: candidate.addr,<br />                voting_power: new_voting_power,<br />                config,<br />            &#125;;<br /><br />            // Update <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>.<br />            <b>spec</b> &#123;<br />                <b>assume</b> new_total_power &#43; new_voting_power &lt;&#61; MAX_U128;<br />            &#125;;<br />            new_total_power &#61; new_total_power &#43; (new_voting_power <b>as</b> u128);<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> new_active_validators, new_validator_info);<br />            num_new_actives &#61; num_new_actives &#43; 1;<br /><br />        &#125;;<br />        candidate_idx &#61; candidate_idx &#43; 1;<br />    &#125;;<br /><br />    <b>let</b> new_validator_set &#61; <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />        consensus_scheme: cur_validator_set.consensus_scheme,<br />        active_validators: new_active_validators,<br />        pending_inactive: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],<br />        pending_active: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],<br />        total_voting_power: new_total_power,<br />        total_joining_power: 0,<br />    &#125;;<br /><br />    <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(&amp;new_validator_set)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_validator_consensus_infos_from_validator_set"></a>

## Function `validator_consensus_infos_from_validator_set`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(validator_set: &amp;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(validator_set: &amp;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt; &#123;<br />    <b>let</b> validator_consensus_infos &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br /><br />    <b>let</b> num_active &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;validator_set.active_validators);<br />    <b>let</b> num_pending_inactive &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;validator_set.pending_inactive);<br />    <b>spec</b> &#123;<br />        <b>assume</b> num_active &#43; num_pending_inactive &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    <b>let</b> total &#61; num_active &#43; num_pending_inactive;<br /><br />    // Pre&#45;fill the <b>return</b> value <b>with</b> dummy values.<br />    <b>let</b> idx &#61; 0;<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> idx &lt;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br />            <b>invariant</b> len(validator_consensus_infos) &#61;&#61; idx;<br />            <b>invariant</b> len(validator_consensus_infos) &lt;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br />        &#125;;<br />        idx &lt; total<br />    &#125;) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> validator_consensus_infos, <a href="validator_consensus_info.md#0x1_validator_consensus_info_default">validator_consensus_info::default</a>());<br />        idx &#61; idx &#43; 1;<br />    &#125;;<br />    <b>spec</b> &#123;<br />        <b>assert</b> len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br />        <b>assert</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validator_set.active_validators,<br />            len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));<br />    &#125;;<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;validator_set.active_validators, &#124;obj&#124; &#123;<br />        <b>let</b> vi: &amp;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#61; obj;<br />        <b>spec</b> &#123;<br />            <b>assume</b> len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br />            <b>assert</b> vi.config.validator_index &lt; len(validator_consensus_infos);<br />        &#125;;<br />        <b>let</b> vci &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> validator_consensus_infos, vi.config.validator_index);<br />        &#42;vci &#61; <a href="validator_consensus_info.md#0x1_validator_consensus_info_new">validator_consensus_info::new</a>(<br />            vi.addr,<br />            vi.config.consensus_pubkey,<br />            vi.voting_power<br />        );<br />        <b>spec</b> &#123;<br />            <b>assert</b> len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br />        &#125;;<br />    &#125;);<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;validator_set.pending_inactive, &#124;obj&#124; &#123;<br />        <b>let</b> vi: &amp;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#61; obj;<br />        <b>spec</b> &#123;<br />            <b>assume</b> len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br />            <b>assert</b> vi.config.validator_index &lt; len(validator_consensus_infos);<br />        &#125;;<br />        <b>let</b> vci &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> validator_consensus_infos, vi.config.validator_index);<br />        &#42;vci &#61; <a href="validator_consensus_info.md#0x1_validator_consensus_info_new">validator_consensus_info::new</a>(<br />            vi.addr,<br />            vi.config.consensus_pubkey,<br />            vi.voting_power<br />        );<br />        <b>spec</b> &#123;<br />            <b>assert</b> len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br />        &#125;;<br />    &#125;);<br /><br />    validator_consensus_infos<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_addresses_from_validator_infos"></a>

## Function `addresses_from_validator_infos`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_addresses_from_validator_infos">addresses_from_validator_infos</a>(infos: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_addresses_from_validator_infos">addresses_from_validator_infos</a>(infos: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; &#123;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(infos, &#124;obj&#124; &#123;<br />        <b>let</b> info: &amp;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#61; obj;<br />        info.addr<br />    &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_update_stake_pool"></a>

## Function `update_stake_pool`

Calculate the stake amount of a stake pool for the next epoch.
Update individual validator&apos;s stake pool if <code>commit &#61;&#61; <b>true</b></code>.

1. distribute transaction fees to active/pending_inactive delegations
2. distribute rewards to active/pending_inactive delegations
3. process pending_active, pending_inactive correspondingly
This function shouldn&apos;t abort.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(validator_perf: &amp;<a href="stake.md#0x1_stake_ValidatorPerformance">stake::ValidatorPerformance</a>, pool_address: <b>address</b>, <a href="staking_config.md#0x1_staking_config">staking_config</a>: &amp;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(<br />    validator_perf: &amp;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>,<br />    pool_address: <b>address</b>,<br />    <a href="staking_config.md#0x1_staking_config">staking_config</a>: &amp;StakingConfig,<br />) <b>acquires</b> <a href="stake.md#0x1_stake_StakePool">StakePool</a>, <a href="stake.md#0x1_stake_AptosCoinCapabilities">AptosCoinCapabilities</a>, <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>, <a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a> &#123;<br />    <b>let</b> stake_pool &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />    <b>let</b> validator_config &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br />    <b>let</b> cur_validator_perf &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;validator_perf.validators, validator_config.validator_index);<br />    <b>let</b> num_successful_proposals &#61; cur_validator_perf.successful_proposals;<br />    <b>spec</b> &#123;<br />        // The following addition should not overflow because `num_total_proposals` cannot be larger than 86400,<br />        // the maximum number of proposals in a day (1 proposal per second).<br />        <b>assume</b> cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    <b>let</b> num_total_proposals &#61; cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals;<br />    <b>let</b> (rewards_rate, rewards_rate_denominator) &#61; <a href="staking_config.md#0x1_staking_config_get_reward_rate">staking_config::get_reward_rate</a>(<a href="staking_config.md#0x1_staking_config">staking_config</a>);<br />    <b>let</b> rewards_active &#61; <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(<br />        &amp;<b>mut</b> stake_pool.active,<br />        num_successful_proposals,<br />        num_total_proposals,<br />        rewards_rate,<br />        rewards_rate_denominator<br />    );<br />    <b>let</b> rewards_pending_inactive &#61; <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(<br />        &amp;<b>mut</b> stake_pool.pending_inactive,<br />        num_successful_proposals,<br />        num_total_proposals,<br />        rewards_rate,<br />        rewards_rate_denominator<br />    );<br />    <b>spec</b> &#123;<br />        <b>assume</b> rewards_active &#43; rewards_pending_inactive &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    <b>let</b> rewards_amount &#61; rewards_active &#43; rewards_pending_inactive;<br />    // Pending active <a href="stake.md#0x1_stake">stake</a> can now be active.<br />    <a href="coin.md#0x1_coin_merge">coin::merge</a>(&amp;<b>mut</b> stake_pool.active, <a href="coin.md#0x1_coin_extract_all">coin::extract_all</a>(&amp;<b>mut</b> stake_pool.pending_active));<br /><br />    // Additionally, distribute transaction fees.<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_collect_and_distribute_gas_fees">features::collect_and_distribute_gas_fees</a>()) &#123;<br />        <b>let</b> fees_table &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework).fees_table;<br />        <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(fees_table, pool_address)) &#123;<br />            <b>let</b> <a href="coin.md#0x1_coin">coin</a> &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(fees_table, pool_address);<br />            <a href="coin.md#0x1_coin_merge">coin::merge</a>(&amp;<b>mut</b> stake_pool.active, <a href="coin.md#0x1_coin">coin</a>);<br />        &#125;;<br />    &#125;;<br /><br />    // Pending inactive <a href="stake.md#0x1_stake">stake</a> is only fully unlocked and moved into inactive <b>if</b> the current lockup cycle <b>has</b> expired<br />    <b>let</b> current_lockup_expiration &#61; stake_pool.locked_until_secs;<br />    <b>if</b> (<a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>() &gt;&#61; current_lockup_expiration) &#123;<br />        <a href="coin.md#0x1_coin_merge">coin::merge</a>(<br />            &amp;<b>mut</b> stake_pool.inactive,<br />            <a href="coin.md#0x1_coin_extract_all">coin::extract_all</a>(&amp;<b>mut</b> stake_pool.pending_inactive),<br />        );<br />    &#125;;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="stake.md#0x1_stake_DistributeRewards">DistributeRewards</a> &#123; pool_address, rewards_amount &#125;);<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> stake_pool.distribute_rewards_events,<br />        <a href="stake.md#0x1_stake_DistributeRewardsEvent">DistributeRewardsEvent</a> &#123;<br />            pool_address,<br />            rewards_amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_reconfig_start_time_secs"></a>

## Function `get_reconfig_start_time_secs`

Assuming we are in a middle of a reconfiguration (no matter it is immediate or async), get its start time.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>(): u64 &#123;<br />    <b>if</b> (<a href="reconfiguration_state.md#0x1_reconfiguration_state_is_initialized">reconfiguration_state::is_initialized</a>()) &#123;<br />        <a href="reconfiguration_state.md#0x1_reconfiguration_state_start_time_secs">reconfiguration_state::start_time_secs</a>()<br />    &#125; <b>else</b> &#123;<br />        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_calculate_rewards_amount"></a>

## Function `calculate_rewards_amount`

Calculate the rewards amount.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(stake_amount: u64, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(<br />    stake_amount: u64,<br />    num_successful_proposals: u64,<br />    num_total_proposals: u64,<br />    rewards_rate: u64,<br />    rewards_rate_denominator: u64,<br />): u64 &#123;<br />    <b>spec</b> &#123;<br />        // The following condition must hold because<br />        // (1) num_successful_proposals &lt;&#61; num_total_proposals, and<br />        // (2) `num_total_proposals` cannot be larger than 86400, the maximum number of proposals<br />        //     in a day (1 proposal per second), and `num_total_proposals` is reset <b>to</b> 0 every epoch.<br />        <b>assume</b> num_successful_proposals &#42; <a href="stake.md#0x1_stake_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a> &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    // The rewards amount is equal <b>to</b> (<a href="stake.md#0x1_stake">stake</a> amount &#42; rewards rate &#42; performance multiplier).<br />    // We do multiplication in u128 before division <b>to</b> avoid the overflow and minimize the rounding <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a>.<br />    <b>let</b> rewards_numerator &#61; (stake_amount <b>as</b> u128) &#42; (rewards_rate <b>as</b> u128) &#42; (num_successful_proposals <b>as</b> u128);<br />    <b>let</b> rewards_denominator &#61; (rewards_rate_denominator <b>as</b> u128) &#42; (num_total_proposals <b>as</b> u128);<br />    <b>if</b> (rewards_denominator &gt; 0) &#123;<br />        ((rewards_numerator / rewards_denominator) <b>as</b> u64)<br />    &#125; <b>else</b> &#123;<br />        0<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_distribute_rewards"></a>

## Function `distribute_rewards`

Mint rewards corresponding to current epoch&apos;s <code><a href="stake.md#0x1_stake">stake</a></code> and <code>num_successful_votes</code>.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(<a href="stake.md#0x1_stake">stake</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(<br />    <a href="stake.md#0x1_stake">stake</a>: &amp;<b>mut</b> Coin&lt;AptosCoin&gt;,<br />    num_successful_proposals: u64,<br />    num_total_proposals: u64,<br />    rewards_rate: u64,<br />    rewards_rate_denominator: u64,<br />): u64 <b>acquires</b> <a href="stake.md#0x1_stake_AptosCoinCapabilities">AptosCoinCapabilities</a> &#123;<br />    <b>let</b> stake_amount &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(<a href="stake.md#0x1_stake">stake</a>);<br />    <b>let</b> rewards_amount &#61; <b>if</b> (stake_amount &gt; 0) &#123;<br />        <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(<br />            stake_amount,<br />            num_successful_proposals,<br />            num_total_proposals,<br />            rewards_rate,<br />            rewards_rate_denominator<br />        )<br />    &#125; <b>else</b> &#123;<br />        0<br />    &#125;;<br />    <b>if</b> (rewards_amount &gt; 0) &#123;<br />        <b>let</b> mint_cap &#61; &amp;<b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_AptosCoinCapabilities">AptosCoinCapabilities</a>&gt;(@aptos_framework).mint_cap;<br />        <b>let</b> rewards &#61; <a href="coin.md#0x1_coin_mint">coin::mint</a>(rewards_amount, mint_cap);<br />        <a href="coin.md#0x1_coin_merge">coin::merge</a>(<a href="stake.md#0x1_stake">stake</a>, rewards);<br />    &#125;;<br />    rewards_amount<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_append"></a>

## Function `append`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_append">append</a>&lt;T&gt;(v1: &amp;<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, v2: &amp;<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_append">append</a>&lt;T&gt;(v1: &amp;<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, v2: &amp;<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;) &#123;<br />    <b>while</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(v2)) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(v1, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(v2));<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_find_validator"></a>

## Function `find_validator`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_find_validator">find_validator</a>(v: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;, addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_find_validator">find_validator</a>(v: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, addr: <b>address</b>): Option&lt;u64&gt; &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(v);<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> !(<b>exists</b> j in 0..i: v[j].addr &#61;&#61; addr);<br />        &#125;;<br />        i &lt; len<br />    &#125;) &#123;<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(v, i).addr &#61;&#61; addr) &#123;<br />            <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(i)<br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_generate_validator_info"></a>

## Function `generate_validator_info`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_generate_validator_info">generate_validator_info</a>(addr: <b>address</b>, stake_pool: &amp;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>, config: <a href="stake.md#0x1_stake_ValidatorConfig">stake::ValidatorConfig</a>): <a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_generate_validator_info">generate_validator_info</a>(addr: <b>address</b>, stake_pool: &amp;<a href="stake.md#0x1_stake_StakePool">StakePool</a>, config: <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>): <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#123;<br />    <b>let</b> voting_power &#61; <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool);<br />    <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#123;<br />        addr,<br />        voting_power,<br />        config,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_get_next_epoch_voting_power"></a>

## Function `get_next_epoch_voting_power`

Returns validator&apos;s next epoch voting power, including pending_active, active, and pending_inactive stake.


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool: &amp;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool: &amp;<a href="stake.md#0x1_stake_StakePool">StakePool</a>): u64 &#123;<br />    <b>let</b> value_pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.pending_active);<br />    <b>let</b> value_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.active);<br />    <b>let</b> value_pending_inactive &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;stake_pool.pending_inactive);<br />    <b>spec</b> &#123;<br />        <b>assume</b> value_pending_active &#43; value_active &#43; value_pending_inactive &lt;&#61; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    value_pending_active &#43; value_active &#43; value_pending_inactive<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_update_voting_power_increase"></a>

## Function `update_voting_power_increase`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(increase_amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(increase_amount: u64) <b>acquires</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> &#123;<br />    <b>let</b> validator_set &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />    <b>let</b> voting_power_increase_limit &#61;<br />        (<a href="staking_config.md#0x1_staking_config_get_voting_power_increase_limit">staking_config::get_voting_power_increase_limit</a>(&amp;<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>()) <b>as</b> u128);<br />    validator_set.total_joining_power &#61; validator_set.total_joining_power &#43; (increase_amount <b>as</b> u128);<br /><br />    // Only validator <a href="voting.md#0x1_voting">voting</a> power increase <b>if</b> the current validator set&apos;s <a href="voting.md#0x1_voting">voting</a> power &gt; 0.<br />    <b>if</b> (validator_set.total_voting_power &gt; 0) &#123;<br />        <b>assert</b>!(<br />            validator_set.total_joining_power &lt;&#61; validator_set.total_voting_power &#42; voting_power_increase_limit / 100,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_EVOTING_POWER_INCREASE_EXCEEDS_LIMIT">EVOTING_POWER_INCREASE_EXCEEDS_LIMIT</a>),<br />        );<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_assert_stake_pool_exists"></a>

## Function `assert_stake_pool_exists`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address: <b>address</b>) &#123;<br />    <b>assert</b>!(<a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="stake.md#0x1_stake_ESTAKE_POOL_DOES_NOT_EXIST">ESTAKE_POOL_DOES_NOT_EXIST</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_configure_allowed_validators"></a>

## Function `configure_allowed_validators`



<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_configure_allowed_validators">configure_allowed_validators</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_configure_allowed_validators">configure_allowed_validators</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;<br />) <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> &#123;<br />    <b>let</b> aptos_framework_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(aptos_framework_address)) &#123;<br />        <b>move_to</b>(aptos_framework, <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> &#123; accounts &#125;);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> allowed &#61; <b>borrow_global_mut</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(aptos_framework_address);<br />        allowed.accounts &#61; accounts;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_is_allowed"></a>

## Function `is_allowed`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_is_allowed">is_allowed</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_is_allowed">is_allowed</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool <b>acquires</b> <a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a> &#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@aptos_framework)) &#123;<br />        <b>true</b><br />    &#125; <b>else</b> &#123;<br />        <b>let</b> allowed &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@aptos_framework);<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&amp;allowed.accounts, &amp;<a href="account.md#0x1_account">account</a>)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_assert_owner_cap_exists"></a>

## Function `assert_owner_cap_exists`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>) &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="stake.md#0x1_stake_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_stake_assert_reconfig_not_in_progress"></a>

## Function `assert_reconfig_not_in_progress`



<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_reconfig_not_in_progress">assert_reconfig_not_in_progress</a>() &#123;<br />    <b>assert</b>!(!<a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">reconfiguration_state::is_in_progress</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="stake.md#0x1_stake_ERECONFIGURATION_IN_PROGRESS">ERECONFIGURATION_IN_PROGRESS</a>));<br />&#125;<br /></code></pre>



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
<td>The consensus_scheme attribute within ValidatorSet initializes with the value zero during the module&apos;s initialization and its value remains unchanged afterward.</td>
<td>Formally verified by the data invariant of <a href="#high-level-req-1">ValidatorSet</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The owner of a validator is immutable.</td>
<td>Low</td>
<td>During the initialization of a validator, the owner attribute becomes the signer&apos;s address. This assignment establishes the signer as the owner and controller of the validator entity. Subsequently, the owner attribute remains unchanged throughout the validator&apos;s lifespan, maintaining its assigned value without any modifications.</td>
<td>Formally verified in the schema <a href="#high-level-req-2">ValidatorOwnerNoChange</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The total staked value in the stake pool should remain constant, excluding operations related to adding and withdrawing.</td>
<td>Low</td>
<td>The total staked value (AptosCoin) of a stake pool is grouped by: active, inactive, pending_active, and pending_inactive. The stake value remains constant except during the execution of the add_stake_with_cap or withdraw_with_cap functions or on_new_epoch (which distributes the reward).</td>
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


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>invariant</b> [suspendable] <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework) &#61;&#61;&gt; <a href="stake.md#0x1_stake_validator_set_is_valid">validator_set_is_valid</a>();<br /><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="stake.md#0x1_stake_AptosCoinCapabilities">AptosCoinCapabilities</a>&gt;(@aptos_framework);<br /><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework);<br /><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>apply</b> <a href="stake.md#0x1_stake_ValidatorOwnerNoChange">ValidatorOwnerNoChange</a> <b>to</b> &#42;;<br /><b>apply</b> <a href="stake.md#0x1_stake_ValidatorNotChangeDuringReconfig">ValidatorNotChangeDuringReconfig</a> <b>to</b> &#42; <b>except</b> on_new_epoch;<br /><b>apply</b> <a href="stake.md#0x1_stake_StakePoolNotChangeDuringReconfig">StakePoolNotChangeDuringReconfig</a> <b>to</b> &#42; <b>except</b> on_new_epoch, update_stake_pool;<br /><a id="0x1_stake_ghost_valid_perf"></a>
<b>global</b> <a href="stake.md#0x1_stake_ghost_valid_perf">ghost_valid_perf</a>: <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>;<br /><a id="0x1_stake_ghost_proposer_idx"></a>
<b>global</b> <a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>: Option&lt;u64&gt;;<br /><a id="0x1_stake_ghost_active_num"></a>
<b>global</b> <a href="stake.md#0x1_stake_ghost_active_num">ghost_active_num</a>: u64;<br /><a id="0x1_stake_ghost_pending_inactive_num"></a>
<b>global</b> <a href="stake.md#0x1_stake_ghost_pending_inactive_num">ghost_pending_inactive_num</a>: u64;<br /></code></pre>



<a id="@Specification_1_ValidatorSet"></a>

### Resource `ValidatorSet`


<pre><code><b>struct</b> <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<dl>
<dt>
<code>consensus_scheme: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>active_validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
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



<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>invariant</b> consensus_scheme &#61;&#61; 0;<br /></code></pre>




<a id="0x1_stake_ValidatorNotChangeDuringReconfig"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_ValidatorNotChangeDuringReconfig">ValidatorNotChangeDuringReconfig</a> &#123;<br /><b>ensures</b> (<a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>() &amp;&amp; <b>old</b>(<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework))) &#61;&#61;&gt;<br />    <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework)) &#61;&#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />&#125;<br /></code></pre>




<a id="0x1_stake_StakePoolNotChangeDuringReconfig"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_StakePoolNotChangeDuringReconfig">StakePoolNotChangeDuringReconfig</a> &#123;<br /><b>ensures</b> <b>forall</b> a: <b>address</b> <b>where</b> <b>old</b>(<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a)): <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>() &#61;&#61;&gt;<br />    (<b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).pending_inactive) &#61;&#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).pending_inactive &amp;&amp;<br />    <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).pending_active) &#61;&#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).pending_active &amp;&amp;<br />    <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).inactive) &#61;&#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).inactive &amp;&amp;<br />    <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).active) &#61;&#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a).active);<br />&#125;<br /></code></pre>




<a id="0x1_stake_ValidatorOwnerNoChange"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_ValidatorOwnerNoChange">ValidatorOwnerNoChange</a> &#123;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
    <b>ensures</b> <b>forall</b> addr: <b>address</b> <b>where</b> <b>old</b>(<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr)):<br />    <b>old</b>(<b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr)).pool_address &#61;&#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr).pool_address;<br />&#125;<br /></code></pre>




<a id="0x1_stake_StakedValueNochange"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a> &#123;<br />pool_address: <b>address</b>;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> <b>post</b> post_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
    <b>ensures</b> stake_pool.active.value &#43; stake_pool.inactive.value &#43; stake_pool.pending_active.value &#43; stake_pool.pending_inactive.value &#61;&#61;<br />    post_stake_pool.active.value &#43; post_stake_pool.inactive.value &#43; post_stake_pool.pending_active.value &#43; post_stake_pool.pending_inactive.value;<br />&#125;<br /></code></pre>




<a id="0x1_stake_validator_set_is_valid"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_set_is_valid">validator_set_is_valid</a>(): bool &#123;<br />   <b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />   <a href="stake.md#0x1_stake_validator_set_is_valid_impl">validator_set_is_valid_impl</a>(validator_set)<br />&#125;<br /></code></pre>




<a id="0x1_stake_validator_set_is_valid_impl"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_set_is_valid_impl">validator_set_is_valid_impl</a>(validator_set: <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>): bool &#123;<br />   <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validator_set.active_validators) &amp;&amp;<br />       <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validator_set.pending_inactive) &amp;&amp;<br />       <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validator_set.pending_active) &amp;&amp;<br />       <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(validator_set.active_validators) &amp;&amp;<br />       <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(validator_set.pending_inactive)<br />       &amp;&amp; <a href="stake.md#0x1_stake_spec_validator_indices_active_pending_inactive">spec_validator_indices_active_pending_inactive</a>(validator_set)<br />&#125;<br /></code></pre>



<a id="@Specification_1_initialize_validator_fees"></a>

### Function `initialize_validator_fees`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator_fees">initialize_validator_fees</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>let</b> aptos_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(aptos_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(aptos_addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(aptos_addr);<br /></code></pre>



<a id="@Specification_1_add_transaction_fee"></a>

### Function `add_transaction_fee`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_add_transaction_fee">add_transaction_fee</a>(validator_addr: <b>address</b>, fee: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework);<br /><b>let</b> fees_table &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework).fees_table;<br /><b>let</b> <b>post</b> post_fees_table &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework).fees_table;<br /><b>let</b> collected_fee &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(fees_table, validator_addr);<br /><b>let</b> <b>post</b> post_collected_fee &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_fees_table, validator_addr);<br /><b>ensures</b> <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(fees_table, validator_addr)) &#123;<br />    post_collected_fee.value &#61;&#61; collected_fee.value &#43; fee.value<br />&#125; <b>else</b> &#123;<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(post_fees_table, validator_addr) &amp;&amp;<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_fees_table, validator_addr) &#61;&#61; fee<br />&#125;;<br /></code></pre>



<a id="@Specification_1_get_validator_state"></a>

### Function `get_validator_state`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_get_validator_state">get_validator_state</a>(pool_address: <b>address</b>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>ensures</b> result &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE">VALIDATOR_STATUS_PENDING_ACTIVE</a> &#61;&#61;&gt; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_active, pool_address);<br /><b>ensures</b> result &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_ACTIVE">VALIDATOR_STATUS_ACTIVE</a> &#61;&#61;&gt; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.active_validators, pool_address);<br /><b>ensures</b> result &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE">VALIDATOR_STATUS_PENDING_INACTIVE</a> &#61;&#61;&gt; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_inactive, pool_address);<br /><b>ensures</b> result &#61;&#61; <a href="stake.md#0x1_stake_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a> &#61;&#61;&gt; (<br />    !<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_active, pool_address)<br />        &amp;&amp; !<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.active_validators, pool_address)<br />        &amp;&amp; !<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_inactive, pool_address)<br />);<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> disable_invariants_in_body;<br /><b>let</b> aptos_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(aptos_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(aptos_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(aptos_addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(aptos_addr);<br /><b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(aptos_addr).consensus_scheme &#61;&#61; 0;<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(aptos_addr);<br /></code></pre>



<a id="@Specification_1_remove_validators"></a>

### Function `remove_validators`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_remove_validators">remove_validators</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> active_validators &#61; validator_set.active_validators;<br /><b>let</b> <b>post</b> post_active_validators &#61; post_validator_set.active_validators;<br /><b>let</b> pending_inactive_validators &#61; validator_set.pending_inactive;<br /><b>let</b> <b>post</b> post_pending_inactive_validators &#61; post_validator_set.pending_inactive;<br /><b>invariant</b> len(active_validators) &gt; 0;<br /><b>ensures</b> len(active_validators) &#43; len(pending_inactive_validators) &#61;&#61; len(post_active_validators)<br />    &#43; len(post_pending_inactive_validators);<br /></code></pre>



<a id="@Specification_1_initialize_stake_owner"></a>

### Function `initialize_stake_owner`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_stake_owner">initialize_stake_owner</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_stake_amount: u64, operator: <b>address</b>, voter: <b>address</b>)<br /></code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(addr) &#61;&#61; <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />    consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />    network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />    fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />    validator_index: 0,<br />&#125;;<br /><b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr) &#61;&#61; <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> &#123; pool_address: addr &#125;;<br /><b>let</b> <b>post</b> stakepool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(addr);<br /><b>let</b> <b>post</b> active &#61; stakepool.active.value;<br /><b>let</b> <b>post</b> pending_active &#61; stakepool.pending_active.value;<br /><b>ensures</b> <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(addr) &#61;&#61;&gt;<br />    pending_active &#61;&#61; initial_stake_amount;<br /><b>ensures</b> !<a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(addr) &#61;&#61;&gt;<br />    active &#61;&#61; initial_stake_amount;<br /></code></pre>



<a id="@Specification_1_initialize_validator"></a>

### Function `initialize_validator`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_initialize_validator">initialize_validator</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_of_possession: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>let</b> pubkey_from_pop &#61; <a href="../../aptos-stdlib/doc/bls12381.md#0x1_bls12381_spec_public_key_from_bytes_with_pop">bls12381::spec_public_key_from_bytes_with_pop</a>(<br />    consensus_pubkey,<br />    proof_of_possession_from_bytes(proof_of_possession)<br />);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(pubkey_from_pop);<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> post_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> allowed &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@aptos_framework) &amp;&amp; !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(allowed.accounts, addr);<br /><b>aborts_if</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr).guid_creation_num &#43; 12 &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr).guid_creation_num &#43; 12 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(post_addr);<br /><b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(post_addr) &#61;&#61; <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a> &#123; pool_address: post_addr &#125;;<br /><b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(post_addr) &#61;&#61; <a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a> &#123;<br />    consensus_pubkey,<br />    network_addresses,<br />    fullnode_addresses,<br />    validator_index: 0,<br />&#125;;<br /></code></pre>



<a id="@Specification_1_extract_owner_cap"></a>

### Function `extract_owner_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_extract_owner_cap">extract_owner_cap</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a><br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br /><b>ensures</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br /></code></pre>



<a id="@Specification_1_deposit_owner_cap"></a>

### Function `deposit_owner_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_deposit_owner_cap">deposit_owner_cap</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>




<pre><code><b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br /><b>ensures</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address) &#61;&#61; owner_cap;<br /></code></pre>



<a id="@Specification_1_set_operator_with_cap"></a>

### Function `set_operator_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_operator_with_cap">set_operator_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, new_operator: <b>address</b>)<br /></code></pre>




<pre><code><b>let</b> pool_address &#61; owner_cap.pool_address;<br /><b>let</b> <b>post</b> post_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;<br /><b>ensures</b> post_stake_pool.operator_address &#61;&#61; new_operator;<br /></code></pre>



<a id="@Specification_1_set_delegated_voter_with_cap"></a>

### Function `set_delegated_voter_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">set_delegated_voter_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, new_voter: <b>address</b>)<br /></code></pre>




<pre><code><b>let</b> pool_address &#61; owner_cap.pool_address;<br /><b>let</b> <b>post</b> post_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>ensures</b> post_stake_pool.delegated_voter &#61;&#61; new_voter;<br /></code></pre>



<a id="@Specification_1_add_stake"></a>

### Function `add_stake`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_add_stake">add_stake</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_AddStakeAbortsIfAndEnsures">AddStakeAbortsIfAndEnsures</a>;<br /></code></pre>



<a id="@Specification_1_add_stake_with_cap"></a>

### Function `add_stake_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_add_stake_with_cap">add_stake_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> disable_invariants_in_body;<br /><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;<br /><b>let</b> amount &#61; coins.value;<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>include</b> <a href="stake.md#0x1_stake_AddStakeWithCapAbortsIfAndEnsures">AddStakeWithCapAbortsIfAndEnsures</a> &#123; amount &#125;;<br /></code></pre>



<a id="@Specification_1_reactivate_stake_with_cap"></a>

### Function `reactivate_stake_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_reactivate_stake_with_cap">reactivate_stake_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>, amount: u64)<br /></code></pre>




<pre><code><b>let</b> pool_address &#61; owner_cap.pool_address;<br /><b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>aborts_if</b> !<a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(pool_address);<br /><b>let</b> pre_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> <b>post</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> min_amount &#61; aptos_std::math64::min(amount, pre_stake_pool.pending_inactive.value);<br /><b>ensures</b> stake_pool.pending_inactive.value &#61;&#61; pre_stake_pool.pending_inactive.value &#45; min_amount;<br /><b>ensures</b> stake_pool.active.value &#61;&#61; pre_stake_pool.active.value &#43; min_amount;<br /></code></pre>



<a id="@Specification_1_rotate_consensus_key"></a>

### Function `rotate_consensus_key`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_rotate_consensus_key">rotate_consensus_key</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_of_possession: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>let</b> pre_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> <b>post</b> validator_info &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) !&#61; pre_stake_pool.operator_address;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>let</b> pubkey_from_pop &#61; <a href="../../aptos-stdlib/doc/bls12381.md#0x1_bls12381_spec_public_key_from_bytes_with_pop">bls12381::spec_public_key_from_bytes_with_pop</a>(<br />    new_consensus_pubkey,<br />    proof_of_possession_from_bytes(proof_of_possession)<br />);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(pubkey_from_pop);<br /><b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;<br /><b>ensures</b> validator_info.consensus_pubkey &#61;&#61; new_consensus_pubkey;<br /></code></pre>



<a id="@Specification_1_update_network_and_fullnode_addresses"></a>

### Function `update_network_and_fullnode_addresses`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_update_network_and_fullnode_addresses">update_network_and_fullnode_addresses</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, new_fullnode_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>let</b> pre_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> <b>post</b> validator_info &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) !&#61; pre_stake_pool.operator_address;<br /><b>ensures</b> validator_info.network_addresses &#61;&#61; new_network_addresses;<br /><b>ensures</b> validator_info.fullnode_addresses &#61;&#61; new_fullnode_addresses;<br /></code></pre>



<a id="@Specification_1_increase_lockup_with_cap"></a>

### Function `increase_lockup_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_increase_lockup_with_cap">increase_lockup_with_cap</a>(owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>




<pre><code><b>let</b> config &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> pool_address &#61; owner_cap.pool_address;<br /><b>let</b> pre_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> <b>post</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> now_seconds &#61; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();<br /><b>let</b> lockup &#61; config.recurring_lockup_duration_secs;<br /><b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> pre_stake_pool.locked_until_secs &gt;&#61; lockup &#43; now_seconds;<br /><b>aborts_if</b> lockup &#43; now_seconds &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>ensures</b> stake_pool.locked_until_secs &#61;&#61; lockup &#43; now_seconds;<br /></code></pre>



<a id="@Specification_1_join_validator_set"></a>

### Function `join_validator_set`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_join_validator_set">join_validator_set</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> disable_invariants_in_body;<br /><b>aborts_if</b> !<a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;StakingConfig&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> p_validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) !&#61; stake_pool.operator_address;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.active_validators, pool_address)) &#124;&#124;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_inactive, pool_address)) &#124;&#124;<br />                <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_active, pool_address));<br /><b>let</b> config &#61; <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();<br /><b>let</b> voting_power &#61; <a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool);<br /><b>let</b> minimum_stake &#61; config.minimum_stake;<br /><b>let</b> maximum_stake &#61; config.maximum_stake;<br /><b>aborts_if</b> voting_power &lt; minimum_stake;<br /><b>aborts_if</b> voting_power &gt;maximum_stake;<br /><b>let</b> validator_config &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(validator_config.consensus_pubkey);<br /><b>let</b> validator_set_size &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.active_validators) &#43; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.pending_active) &#43; 1;<br /><b>aborts_if</b> validator_set_size &gt; <a href="stake.md#0x1_stake_MAX_VALIDATOR_SET_SIZE">MAX_VALIDATOR_SET_SIZE</a>;<br /><b>let</b> voting_power_increase_limit &#61; (<a href="staking_config.md#0x1_staking_config_get_voting_power_increase_limit">staking_config::get_voting_power_increase_limit</a>(config) <b>as</b> u128);<br /><b>aborts_if</b> (validator_set.total_joining_power &#43; (voting_power <b>as</b> u128)) &gt; MAX_U128;<br /><b>aborts_if</b> validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;<br /><b>aborts_if</b> validator_set.total_voting_power &gt; 0 &amp;&amp;<br />    (validator_set.total_joining_power &#43; (voting_power <b>as</b> u128)) &#42; 100 &gt; validator_set.total_voting_power &#42; voting_power_increase_limit;<br /><b>let</b> <b>post</b> p_validator_info &#61; <a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a> &#123;<br />    addr: pool_address,<br />    voting_power,<br />    config: validator_config,<br />&#125;;<br /><b>ensures</b> validator_set.total_joining_power &#43; voting_power &#61;&#61; p_validator_set.total_joining_power;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(p_validator_set.pending_active, p_validator_info);<br /></code></pre>



<a id="@Specification_1_unlock_with_cap"></a>

### Function `unlock_with_cap`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_unlock_with_cap">unlock_with_cap</a>(amount: u64, owner_cap: &amp;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>let</b> pool_address &#61; owner_cap.pool_address;<br /><b>let</b> pre_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> <b>post</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>aborts_if</b> amount !&#61; 0 &amp;&amp; !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>modifies</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>include</b> <a href="stake.md#0x1_stake_StakedValueNochange">StakedValueNochange</a>;<br /><b>let</b> min_amount &#61; aptos_std::math64::min(amount,pre_stake_pool.active.value);<br /><b>ensures</b> stake_pool.active.value &#61;&#61; pre_stake_pool.active.value &#45; min_amount;<br /><b>ensures</b> stake_pool.pending_inactive.value &#61;&#61; pre_stake_pool.pending_inactive.value &#43; min_amount;<br /></code></pre>



<a id="@Specification_1_withdraw"></a>

### Function `withdraw`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_withdraw">withdraw</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, withdraw_amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>let</b> ownership_cap &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr);<br /><b>let</b> pool_address &#61; ownership_cap.pool_address;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> bool_find_validator &#61; !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.active_validators, pool_address)) &amp;&amp;<br />            !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_inactive, pool_address)) &amp;&amp;<br />                !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_active, pool_address));<br /><b>aborts_if</b> bool_find_validator &amp;&amp; !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>let</b> new_withdraw_amount_1 &#61; <b>min</b>(withdraw_amount, stake_pool.inactive.value &#43; stake_pool.pending_inactive.value);<br /><b>let</b> new_withdraw_amount_2 &#61; <b>min</b>(withdraw_amount, stake_pool.inactive.value);<br /><b>aborts_if</b> bool_find_validator &amp;&amp; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; stake_pool.locked_until_secs &amp;&amp;<br />            new_withdraw_amount_1 &gt; 0 &amp;&amp; stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &lt; new_withdraw_amount_1;<br /><b>aborts_if</b> !(bool_find_validator &amp;&amp; <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework)) &amp;&amp;<br />            new_withdraw_amount_2 &gt; 0 &amp;&amp; stake_pool.inactive.value &lt; new_withdraw_amount_2;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(addr);<br /><b>include</b> <a href="coin.md#0x1_coin_DepositAbortsIf">coin::DepositAbortsIf</a>&lt;AptosCoin&gt;&#123;account_addr: addr&#125;;<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(addr);<br /><b>let</b> <b>post</b> p_coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(addr);<br /><b>ensures</b> bool_find_validator &amp;&amp; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; stake_pool.locked_until_secs<br />            &amp;&amp; <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr) &amp;&amp; <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(addr) &#61;&#61;&gt;<br />                coin_store.<a href="coin.md#0x1_coin">coin</a>.value &#43; new_withdraw_amount_1 &#61;&#61; p_coin_store.<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>ensures</b> !(bool_find_validator &amp;&amp; <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework))<br />            &amp;&amp; <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr) &amp;&amp; <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(addr) &#61;&#61;&gt;<br />                coin_store.<a href="coin.md#0x1_coin">coin</a>.value &#43; new_withdraw_amount_2 &#61;&#61; p_coin_store.<a href="coin.md#0x1_coin">coin</a>.value;<br /></code></pre>



<a id="@Specification_1_leave_validator_set"></a>

### Function `leave_validator_set`


<pre><code><b>public</b> entry <b>fun</b> <a href="stake.md#0x1_stake_leave_validator_set">leave_validator_set</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> disable_invariants_in_body;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>let</b> config &#61; <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();<br /><b>aborts_if</b> !<a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(config);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator) !&#61; stake_pool.operator_address;<br /><b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> validator_find_bool &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(validator_set.pending_active, pool_address));<br /><b>let</b> active_validators &#61; validator_set.active_validators;<br /><b>let</b> pending_active &#61; validator_set.pending_active;<br /><b>let</b> <b>post</b> post_validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_active_validators &#61; post_validator_set.active_validators;<br /><b>let</b> pending_inactive_validators &#61; validator_set.pending_inactive;<br /><b>let</b> <b>post</b> post_pending_inactive_validators &#61; post_validator_set.pending_inactive;<br /><b>ensures</b> len(active_validators) &#43; len(pending_inactive_validators) &#61;&#61; len(post_active_validators)<br />    &#43; len(post_pending_inactive_validators);<br /><b>aborts_if</b> !validator_find_bool &amp;&amp; !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(active_validators, pool_address));<br /><b>aborts_if</b> !validator_find_bool &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.active_validators) &lt;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(active_validators, pool_address));<br /><b>aborts_if</b> !validator_find_bool &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.active_validators) &lt; 2;<br /><b>aborts_if</b> validator_find_bool &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(validator_set.pending_active) &lt;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(pending_active, pool_address));<br /><b>let</b> <b>post</b> p_validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> validator_stake &#61; (<a href="stake.md#0x1_stake_get_next_epoch_voting_power">get_next_epoch_voting_power</a>(stake_pool) <b>as</b> u128);<br /><b>ensures</b> validator_find_bool &amp;&amp; validator_set.total_joining_power &gt; validator_stake &#61;&#61;&gt;<br />            p_validator_set.total_joining_power &#61;&#61; validator_set.total_joining_power &#45; validator_stake;<br /><b>ensures</b> !validator_find_bool &#61;&#61;&gt; !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(p_validator_set.pending_active, pool_address));<br /></code></pre>



<a id="@Specification_1_is_current_epoch_validator"></a>

### Function `is_current_epoch_validator`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_is_current_epoch_validator">is_current_epoch_validator</a>(pool_address: <b>address</b>): bool<br /></code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;<br /><b>aborts_if</b> !<a href="stake.md#0x1_stake_spec_has_stake_pool">spec_has_stake_pool</a>(pool_address);<br /><b>ensures</b> result &#61;&#61; <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(pool_address);<br /></code></pre>



<a id="@Specification_1_update_performance_statistics"></a>

### Function `update_performance_statistics`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_update_performance_statistics">update_performance_statistics</a>(proposer_index: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>aborts_if</b> <b>false</b>;<br /><b>let</b> validator_perf &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_validator_perf &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework);<br /><b>let</b> validator_len &#61; len(validator_perf.validators);<br /><b>ensures</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>) &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>) &lt; validator_len) &#61;&#61;&gt;<br />    (post_validator_perf.validators[<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>)].successful_proposals &#61;&#61;<br />        validator_perf.validators[<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="stake.md#0x1_stake_ghost_proposer_idx">ghost_proposer_idx</a>)].successful_proposals &#43; 1);<br /></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="stake.md#0x1_stake_on_new_epoch">on_new_epoch</a>()<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>pragma</b> disable_invariants_in_body;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a>;<br /><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;<br /><b>include</b> aptos_framework::aptos_coin::ExistsAptosCoin;<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_next_validator_consensus_infos"></a>

### Function `next_validator_consensus_infos`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_next_validator_consensus_infos">next_validator_consensus_infos</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>aborts_if</b> <b>false</b>;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a>;<br /><b>include</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>() &#61;&#61;&gt; <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">staking_config::StakingRewardsConfigEnabledRequirement</a>;<br /></code></pre>



<a id="@Specification_1_validator_consensus_infos_from_validator_set"></a>

### Function `validator_consensus_infos_from_validator_set`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_validator_consensus_infos_from_validator_set">validator_consensus_infos_from_validator_set</a>(validator_set: &amp;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>invariant</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validator_set.active_validators,<br />    len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));<br /><b>invariant</b> len(validator_set.pending_inactive) &#61;&#61; 0 &#124;&#124;<br />    <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validator_set.pending_inactive,<br />        len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));<br /></code></pre>




<a id="0x1_stake_AddStakeWithCapAbortsIfAndEnsures"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_AddStakeWithCapAbortsIfAndEnsures">AddStakeWithCapAbortsIfAndEnsures</a> &#123;<br />owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>;<br />amount: u64;<br /><b>let</b> pool_address &#61; owner_cap.pool_address;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> config &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> voting_power_increase_limit &#61; config.voting_power_increase_limit;<br /><b>let</b> <b>post</b> post_validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> update_voting_power_increase &#61; amount !&#61; 0 &amp;&amp; (<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.active_validators, pool_address)<br />                                                   &#124;&#124; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_active, pool_address));<br /><b>aborts_if</b> update_voting_power_increase &amp;&amp; validator_set.total_joining_power &#43; amount &gt; MAX_U128;<br /><b>ensures</b> update_voting_power_increase &#61;&#61;&gt; post_validator_set.total_joining_power &#61;&#61; validator_set.total_joining_power &#43; amount;<br /><b>aborts_if</b> update_voting_power_increase &amp;&amp; validator_set.total_voting_power &gt; 0<br />        &amp;&amp; validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;<br /><b>aborts_if</b> update_voting_power_increase &amp;&amp; validator_set.total_voting_power &gt; 0<br />        &amp;&amp; validator_set.total_joining_power &#43; amount &gt; validator_set.total_voting_power &#42; voting_power_increase_limit / 100;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> <b>post</b> post_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> value_pending_active &#61; stake_pool.pending_active.value;<br /><b>let</b> value_active &#61; stake_pool.active.value;<br /><b>ensures</b> amount !&#61; 0 &amp;&amp; <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(pool_address) &#61;&#61;&gt; post_stake_pool.pending_active.value &#61;&#61; value_pending_active &#43; amount;<br /><b>ensures</b> amount !&#61; 0 &amp;&amp; !<a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(pool_address) &#61;&#61;&gt; post_stake_pool.active.value &#61;&#61; value_active &#43; amount;<br /><b>let</b> maximum_stake &#61; config.maximum_stake;<br /><b>let</b> value_pending_inactive &#61; stake_pool.pending_inactive.value;<br /><b>let</b> next_epoch_voting_power &#61; value_pending_active &#43; value_active &#43; value_pending_inactive;<br /><b>let</b> voting_power &#61; next_epoch_voting_power &#43; amount;<br /><b>aborts_if</b> amount !&#61; 0 &amp;&amp; voting_power &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> amount !&#61; 0 &amp;&amp; voting_power &gt; maximum_stake;<br />&#125;<br /></code></pre>




<a id="0x1_stake_AddStakeAbortsIfAndEnsures"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_AddStakeAbortsIfAndEnsures">AddStakeAbortsIfAndEnsures</a> &#123;<br />owner: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />amount: u64;<br /><b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br /><b>let</b> owner_cap &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner_address);<br /><b>include</b> <a href="stake.md#0x1_stake_AddStakeWithCapAbortsIfAndEnsures">AddStakeWithCapAbortsIfAndEnsures</a> &#123; owner_cap &#125;;<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_is_allowed"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_is_allowed">spec_is_allowed</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool &#123;<br />   <b>if</b> (!<b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@aptos_framework)) &#123;<br />       <b>true</b><br />   &#125; <b>else</b> &#123;<br />       <b>let</b> allowed &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(@aptos_framework);<br />       contains(allowed.accounts, <a href="account.md#0x1_account">account</a>)<br />   &#125;<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_find_validator"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(v: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, addr: <b>address</b>): Option&lt;u64&gt;;<br /></code></pre>




<a id="0x1_stake_spec_validators_are_initialized"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized">spec_validators_are_initialized</a>(validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;): bool &#123;<br />   <b>forall</b> i in 0..len(validators):<br />       <a href="stake.md#0x1_stake_spec_has_stake_pool">spec_has_stake_pool</a>(validators[i].addr) &amp;&amp;<br />           <a href="stake.md#0x1_stake_spec_has_validator_config">spec_has_validator_config</a>(validators[i].addr)<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_validators_are_initialized_addrs"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validators_are_initialized_addrs">spec_validators_are_initialized_addrs</a>(addrs: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;): bool &#123;<br />   <b>forall</b> i in 0..len(addrs):<br />       <a href="stake.md#0x1_stake_spec_has_stake_pool">spec_has_stake_pool</a>(addrs[i]) &amp;&amp;<br />           <a href="stake.md#0x1_stake_spec_has_validator_config">spec_has_validator_config</a>(addrs[i])<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid">spec_validator_indices_are_valid</a>(validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;): bool &#123;<br />   <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_addr">spec_validator_indices_are_valid_addr</a>(validators, <a href="stake.md#0x1_stake_spec_validator_index_upper_bound">spec_validator_index_upper_bound</a>()) &amp;&amp;<br />       <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validators, <a href="stake.md#0x1_stake_spec_validator_index_upper_bound">spec_validator_index_upper_bound</a>())<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid_addr"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_addr">spec_validator_indices_are_valid_addr</a>(validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, upper_bound: u64): bool &#123;<br />   <b>forall</b> i in 0..len(validators):<br />       <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(validators[i].addr).validator_index &lt; upper_bound<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid_config"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_indices_are_valid_config">spec_validator_indices_are_valid_config</a>(validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, upper_bound: u64): bool &#123;<br />   <b>forall</b> i in 0..len(validators):<br />       validators[i].config.validator_index &lt; upper_bound<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_validator_indices_active_pending_inactive"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_indices_active_pending_inactive">spec_validator_indices_active_pending_inactive</a>(validator_set: <a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>): bool &#123;<br />   len(validator_set.pending_inactive) &#43; len(validator_set.active_validators) &#61;&#61; <a href="stake.md#0x1_stake_spec_validator_index_upper_bound">spec_validator_index_upper_bound</a>()<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_validator_index_upper_bound"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_validator_index_upper_bound">spec_validator_index_upper_bound</a>(): u64 &#123;<br />   len(<b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework).validators)<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_has_stake_pool"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_has_stake_pool">spec_has_stake_pool</a>(a: <b>address</b>): bool &#123;<br />   <b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(a)<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_has_validator_config"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_has_validator_config">spec_has_validator_config</a>(a: <b>address</b>): bool &#123;<br />   <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(a)<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_rewards_amount"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(<br />   stake_amount: u64,<br />   num_successful_proposals: u64,<br />   num_total_proposals: u64,<br />   rewards_rate: u64,<br />   rewards_rate_denominator: u64,<br />): u64;<br /></code></pre>




<a id="0x1_stake_spec_contains"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">ValidatorInfo</a>&gt;, addr: <b>address</b>): bool &#123;<br />   <b>exists</b> i in 0..len(validators): validators[i].addr &#61;&#61; addr<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_is_current_epoch_validator"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">spec_is_current_epoch_validator</a>(pool_address: <b>address</b>): bool &#123;<br />   <b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br />   !<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_active, pool_address)<br />       &amp;&amp; (<a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.active_validators, pool_address)<br />       &#124;&#124; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(validator_set.pending_inactive, pool_address))<br />&#125;<br /></code></pre>




<a id="0x1_stake_ResourceRequirement"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a> &#123;<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_AptosCoinCapabilities">AptosCoinCapabilities</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;StakingConfig&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;StakingRewardsConfig&gt;(@aptos_framework) &#124;&#124; !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>();<br /><b>requires</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework);<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_get_reward_rate_1"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_get_reward_rate_1">spec_get_reward_rate_1</a>(config: StakingConfig): num &#123;<br />   <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>()) &#123;<br />       <b>let</b> epoch_rewards_rate &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@aptos_framework).rewards_rate;<br />       <b>if</b> (epoch_rewards_rate.value &#61;&#61; 0) &#123;<br />           0<br />       &#125; <b>else</b> &#123;<br />           <b>let</b> denominator_0 &#61; aptos_std::fixed_point64::spec_divide_u128(<a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">staking_config::MAX_REWARDS_RATE</a>, epoch_rewards_rate);<br />           <b>let</b> denominator &#61; <b>if</b> (denominator_0 &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>) &#123;<br />               <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a><br />           &#125; <b>else</b> &#123;<br />               denominator_0<br />           &#125;;<br />           <b>let</b> nominator &#61; aptos_std::fixed_point64::spec_multiply_u128(denominator, epoch_rewards_rate);<br />           nominator<br />       &#125;<br />   &#125; <b>else</b> &#123;<br />           config.rewards_rate<br />   &#125;<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_get_reward_rate_2"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_get_reward_rate_2">spec_get_reward_rate_2</a>(config: StakingConfig): num &#123;<br />   <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>()) &#123;<br />       <b>let</b> epoch_rewards_rate &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@aptos_framework).rewards_rate;<br />       <b>if</b> (epoch_rewards_rate.value &#61;&#61; 0) &#123;<br />           1<br />       &#125; <b>else</b> &#123;<br />           <b>let</b> denominator_0 &#61; aptos_std::fixed_point64::spec_divide_u128(<a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">staking_config::MAX_REWARDS_RATE</a>, epoch_rewards_rate);<br />           <b>let</b> denominator &#61; <b>if</b> (denominator_0 &gt; <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a>) &#123;<br />               <a href="stake.md#0x1_stake_MAX_U64">MAX_U64</a><br />           &#125; <b>else</b> &#123;<br />               denominator_0<br />           &#125;;<br />           denominator<br />       &#125;<br />   &#125; <b>else</b> &#123;<br />           config.rewards_rate_denominator<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_update_stake_pool"></a>

### Function `update_stake_pool`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_stake_pool">update_stake_pool</a>(validator_perf: &amp;<a href="stake.md#0x1_stake_ValidatorPerformance">stake::ValidatorPerformance</a>, pool_address: <b>address</b>, <a href="staking_config.md#0x1_staking_config">staking_config</a>: &amp;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a>;<br /><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_UpdateStakePoolAbortsIf">UpdateStakePoolAbortsIf</a>;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> validator_config &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>let</b> cur_validator_perf &#61; validator_perf.validators[validator_config.validator_index];<br /><b>let</b> num_successful_proposals &#61; cur_validator_perf.successful_proposals;<br /><b>let</b> num_total_proposals &#61; cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals;<br /><b>let</b> rewards_rate &#61; <a href="stake.md#0x1_stake_spec_get_reward_rate_1">spec_get_reward_rate_1</a>(<a href="staking_config.md#0x1_staking_config">staking_config</a>);<br /><b>let</b> rewards_rate_denominator &#61; <a href="stake.md#0x1_stake_spec_get_reward_rate_2">spec_get_reward_rate_2</a>(<a href="staking_config.md#0x1_staking_config">staking_config</a>);<br /><b>let</b> rewards_amount_1 &#61; <b>if</b> (stake_pool.active.value &gt; 0) &#123;<br />    <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(stake_pool.active.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)<br />&#125; <b>else</b> &#123;<br />    0<br />&#125;;<br /><b>let</b> rewards_amount_2 &#61; <b>if</b> (stake_pool.pending_inactive.value &gt; 0) &#123;<br />    <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(stake_pool.pending_inactive.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)<br />&#125; <b>else</b> &#123;<br />    0<br />&#125;;<br /><b>let</b> <b>post</b> post_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>let</b> <b>post</b> post_active_value &#61; post_stake_pool.active.value;<br /><b>let</b> <b>post</b> post_pending_inactive_value &#61; post_stake_pool.pending_inactive.value;<br /><b>let</b> fees_table &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework).fees_table;<br /><b>let</b> <b>post</b> post_fees_table &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(@aptos_framework).fees_table;<br /><b>let</b> <b>post</b> post_inactive_value &#61; post_stake_pool.inactive.value;<br /><b>ensures</b> post_stake_pool.pending_active.value &#61;&#61; 0;<br /><b>ensures</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES">features::COLLECT_AND_DISTRIBUTE_GAS_FEES</a>) &amp;&amp; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(fees_table, pool_address)) &#123;<br />    !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(post_fees_table, pool_address) &amp;&amp;<br />    post_active_value &#61;&#61; stake_pool.active.value &#43; rewards_amount_1 &#43; stake_pool.pending_active.value &#43; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(fees_table, pool_address).value<br />&#125; <b>else</b> &#123;<br />    post_active_value &#61;&#61; stake_pool.active.value &#43; rewards_amount_1 &#43; stake_pool.pending_active.value<br />&#125;;<br /><b>ensures</b> <b>if</b> (<a href="stake.md#0x1_stake_spec_get_reconfig_start_time_secs">spec_get_reconfig_start_time_secs</a>() &gt;&#61; stake_pool.locked_until_secs) &#123;<br />    post_pending_inactive_value &#61;&#61; 0 &amp;&amp;<br />    post_inactive_value &#61;&#61; stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &#43; rewards_amount_2<br />&#125; <b>else</b> &#123;<br />    post_pending_inactive_value &#61;&#61; stake_pool.pending_inactive.value &#43; rewards_amount_2<br />&#125;;<br /></code></pre>




<a id="0x1_stake_UpdateStakePoolAbortsIf"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_UpdateStakePoolAbortsIf">UpdateStakePoolAbortsIf</a> &#123;<br />pool_address: <b>address</b>;<br />validator_perf: <a href="stake.md#0x1_stake_ValidatorPerformance">ValidatorPerformance</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address);<br /><b>aborts_if</b> <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">ValidatorConfig</a>&gt;(pool_address).validator_index &gt;&#61; len(validator_perf.validators);<br /><b>let</b> aptos_addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;().account_address;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">ValidatorFees</a>&gt;(aptos_addr);<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">StakePool</a>&gt;(pool_address);<br /><b>include</b> <a href="stake.md#0x1_stake_DistributeRewardsAbortsIf">DistributeRewardsAbortsIf</a> &#123;<a href="stake.md#0x1_stake">stake</a>: stake_pool.active&#125;;<br /><b>include</b> <a href="stake.md#0x1_stake_DistributeRewardsAbortsIf">DistributeRewardsAbortsIf</a> &#123;<a href="stake.md#0x1_stake">stake</a>: stake_pool.pending_inactive&#125;;<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_reconfig_start_time_secs"></a>

### Function `get_reconfig_start_time_secs`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_get_reconfig_start_time_secs">get_reconfig_start_time_secs</a>(): u64<br /></code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a>;<br /></code></pre>




<a id="0x1_stake_GetReconfigStartTimeRequirement"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">GetReconfigStartTimeRequirement</a> &#123;<br /><b>requires</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>include</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StartTimeSecsRequirement">reconfiguration_state::StartTimeSecsRequirement</a>;<br />&#125;<br /></code></pre>




<a id="0x1_stake_spec_get_reconfig_start_time_secs"></a>


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_spec_get_reconfig_start_time_secs">spec_get_reconfig_start_time_secs</a>(): u64 &#123;<br />   <b>if</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">reconfiguration_state::State</a>&gt;(@aptos_framework)) &#123;<br />       <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_start_time_secs">reconfiguration_state::spec_start_time_secs</a>()<br />   &#125; <b>else</b> &#123;<br />       <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>()<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_calculate_rewards_amount"></a>

### Function `calculate_rewards_amount`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_calculate_rewards_amount">calculate_rewards_amount</a>(stake_amount: u64, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>requires</b> rewards_rate &lt;&#61; <a href="stake.md#0x1_stake_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>;<br /><b>requires</b> rewards_rate_denominator &gt; 0;<br /><b>requires</b> rewards_rate &lt;&#61; rewards_rate_denominator;<br /><b>requires</b> num_successful_proposals &lt;&#61; num_total_proposals;<br /><b>ensures</b> [concrete] (rewards_rate_denominator &#42; num_total_proposals &#61;&#61; 0) &#61;&#61;&gt; result &#61;&#61; 0;<br /><b>ensures</b> [concrete] (rewards_rate_denominator &#42; num_total_proposals &gt; 0) &#61;&#61;&gt; &#123;<br />    <b>let</b> amount &#61; ((stake_amount &#42; rewards_rate &#42; num_successful_proposals) /<br />        (rewards_rate_denominator &#42; num_total_proposals));<br />    result &#61;&#61; amount<br />&#125;;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(<br />    stake_amount,<br />    num_successful_proposals,<br />    num_total_proposals,<br />    rewards_rate,<br />    rewards_rate_denominator);<br /></code></pre>



<a id="@Specification_1_distribute_rewards"></a>

### Function `distribute_rewards`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_distribute_rewards">distribute_rewards</a>(<a href="stake.md#0x1_stake">stake</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64<br /></code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">ResourceRequirement</a>;<br /><b>requires</b> rewards_rate &lt;&#61; <a href="stake.md#0x1_stake_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>;<br /><b>requires</b> rewards_rate_denominator &gt; 0;<br /><b>requires</b> rewards_rate &lt;&#61; rewards_rate_denominator;<br /><b>requires</b> num_successful_proposals &lt;&#61; num_total_proposals;<br /><b>include</b> <a href="stake.md#0x1_stake_DistributeRewardsAbortsIf">DistributeRewardsAbortsIf</a>;<br /><b>ensures</b> <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) &gt; 0 &#61;&#61;&gt;<br />    result &#61;&#61; <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(<br />        <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value),<br />        num_successful_proposals,<br />        num_total_proposals,<br />        rewards_rate,<br />        rewards_rate_denominator);<br /><b>ensures</b> <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) &gt; 0 &#61;&#61;&gt;<br />    <a href="stake.md#0x1_stake">stake</a>.value &#61;&#61; <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) &#43; <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(<br />        <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value),<br />        num_successful_proposals,<br />        num_total_proposals,<br />        rewards_rate,<br />        rewards_rate_denominator);<br /><b>ensures</b> <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) &#61;&#61; 0 &#61;&#61;&gt; result &#61;&#61; 0;<br /><b>ensures</b> <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value) &#61;&#61; 0 &#61;&#61;&gt; <a href="stake.md#0x1_stake">stake</a>.value &#61;&#61; <b>old</b>(<a href="stake.md#0x1_stake">stake</a>.value);<br /></code></pre>




<a id="0x1_stake_DistributeRewardsAbortsIf"></a>


<pre><code><b>schema</b> <a href="stake.md#0x1_stake_DistributeRewardsAbortsIf">DistributeRewardsAbortsIf</a> &#123;<br /><a href="stake.md#0x1_stake">stake</a>: Coin&lt;AptosCoin&gt;;<br />num_successful_proposals: num;<br />num_total_proposals: num;<br />rewards_rate: num;<br />rewards_rate_denominator: num;<br /><b>let</b> stake_amount &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(<a href="stake.md#0x1_stake">stake</a>);<br /><b>let</b> rewards_amount &#61; <b>if</b> (stake_amount &gt; 0) &#123;<br />    <a href="stake.md#0x1_stake_spec_rewards_amount">spec_rewards_amount</a>(stake_amount, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)<br />&#125; <b>else</b> &#123;<br />    0<br />&#125;;<br /><b>let</b> amount &#61; rewards_amount;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;().account_address;<br /><b>aborts_if</b> (rewards_amount &gt; 0) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;AptosCoin&gt;&gt;(addr);<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;AptosCoin&gt;&gt;(addr);<br /><b>include</b> (rewards_amount &gt; 0) &#61;&#61;&gt; <a href="coin.md#0x1_coin_CoinAddAbortsIf">coin::CoinAddAbortsIf</a>&lt;AptosCoin&gt; &#123; amount: amount &#125;;<br />&#125;<br /></code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_append">append</a>&lt;T&gt;(v1: &amp;<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, v2: &amp;<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;)<br /></code></pre>




<pre><code><b>pragma</b> opaque, verify &#61; <b>false</b>;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> len(v1) &#61;&#61; <b>old</b>(len(v1) &#43; len(v2));<br /><b>ensures</b> len(v2) &#61;&#61; 0;<br /><b>ensures</b> (<b>forall</b> i in 0..<b>old</b>(len(v1)): v1[i] &#61;&#61; <b>old</b>(v1[i]));<br /><b>ensures</b> (<b>forall</b> i in <b>old</b>(len(v1))..len(v1): v1[i] &#61;&#61; <b>old</b>(v2[len(v2) &#45; (i &#45; len(v1)) &#45; 1]));<br /></code></pre>



<a id="@Specification_1_find_validator"></a>

### Function `find_validator`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_find_validator">find_validator</a>(v: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;, addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(result) &#61;&#61;&gt; (<b>forall</b> i in 0..len(v): v[i].addr !&#61; addr);<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(result) &#61;&#61;&gt; v[<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(result)].addr &#61;&#61; addr;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(result) &#61;&#61;&gt; <a href="stake.md#0x1_stake_spec_contains">spec_contains</a>(v, addr);<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="stake.md#0x1_stake_spec_find_validator">spec_find_validator</a>(v,addr);<br /></code></pre>



<a id="@Specification_1_update_voting_power_increase"></a>

### Function `update_voting_power_increase`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_update_voting_power_increase">update_voting_power_increase</a>(increase_amount: u64)<br /></code></pre>




<pre><code><b>requires</b> !<a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> aptos &#61; @aptos_framework;<br /><b>let</b> pre_validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(aptos);<br /><b>let</b> <b>post</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">ValidatorSet</a>&gt;(aptos);<br /><b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(aptos);<br /><b>let</b> voting_power_increase_limit &#61; <a href="staking_config.md#0x1_staking_config">staking_config</a>.voting_power_increase_limit;<br /><b>aborts_if</b> pre_validator_set.total_joining_power &#43; increase_amount &gt; MAX_U128;<br /><b>aborts_if</b> pre_validator_set.total_voting_power &gt; 0 &amp;&amp; pre_validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;<br /><b>aborts_if</b> pre_validator_set.total_voting_power &gt; 0 &amp;&amp;<br />    pre_validator_set.total_joining_power &#43; increase_amount &gt; pre_validator_set.total_voting_power &#42; voting_power_increase_limit / 100;<br /><b>ensures</b> validator_set.total_voting_power &gt; 0 &#61;&#61;&gt;<br />    validator_set.total_joining_power &lt;&#61; validator_set.total_voting_power &#42; voting_power_increase_limit / 100;<br /><b>ensures</b> validator_set.total_joining_power &#61;&#61; pre_validator_set.total_joining_power &#43; increase_amount;<br /></code></pre>



<a id="@Specification_1_assert_stake_pool_exists"></a>

### Function `assert_stake_pool_exists`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_stake_pool_exists">assert_stake_pool_exists</a>(pool_address: <b>address</b>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="stake.md#0x1_stake_stake_pool_exists">stake_pool_exists</a>(pool_address);<br /></code></pre>



<a id="@Specification_1_configure_allowed_validators"></a>

### Function `configure_allowed_validators`


<pre><code><b>public</b> <b>fun</b> <a href="stake.md#0x1_stake_configure_allowed_validators">configure_allowed_validators</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>




<pre><code><b>let</b> aptos_framework_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(aptos_framework_address);<br /><b>let</b> <b>post</b> allowed &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">AllowedValidators</a>&gt;(aptos_framework_address);<br /><b>ensures</b> allowed.accounts &#61;&#61; accounts;<br /></code></pre>



<a id="@Specification_1_assert_owner_cap_exists"></a>

### Function `assert_owner_cap_exists`


<pre><code><b>fun</b> <a href="stake.md#0x1_stake_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">OwnerCapability</a>&gt;(owner);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
