
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


<pre><code>use 0x1::account;<br/>use 0x1::aptos_coin;<br/>use 0x1::bls12381;<br/>use 0x1::chain_status;<br/>use 0x1::coin;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::fixed_point64;<br/>use 0x1::math64;<br/>use 0x1::option;<br/>use 0x1::reconfiguration_state;<br/>use 0x1::signer;<br/>use 0x1::staking_config;<br/>use 0x1::system_addresses;<br/>use 0x1::table;<br/>use 0x1::timestamp;<br/>use 0x1::validator_consensus_info;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_stake_OwnerCapability"></a>

## Resource `OwnerCapability`

Capability that represents ownership and can be used to control the validator and the associated stake pool.
Having this be separate from the signer for the account that the validator resources are hosted at allows
modules to have control over a validator.


<pre><code>struct OwnerCapability has store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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


<pre><code>struct StakePool has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active: coin::Coin&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inactive: coin::Coin&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: coin::Coin&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: coin::Coin&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_until_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>operator_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>delegated_voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>initialize_validator_events: event::EventHandle&lt;stake::RegisterValidatorCandidateEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>set_operator_events: event::EventHandle&lt;stake::SetOperatorEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_stake_events: event::EventHandle&lt;stake::AddStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reactivate_stake_events: event::EventHandle&lt;stake::ReactivateStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>rotate_consensus_key_events: event::EventHandle&lt;stake::RotateConsensusKeyEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_network_and_fullnode_addresses_events: event::EventHandle&lt;stake::UpdateNetworkAndFullnodeAddressesEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>increase_lockup_events: event::EventHandle&lt;stake::IncreaseLockupEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>join_validator_set_events: event::EventHandle&lt;stake::JoinValidatorSetEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>distribute_rewards_events: event::EventHandle&lt;stake::DistributeRewardsEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_stake_events: event::EventHandle&lt;stake::UnlockStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_stake_events: event::EventHandle&lt;stake::WithdrawStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>leave_validator_set_events: event::EventHandle&lt;stake::LeaveValidatorSetEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ValidatorConfig"></a>

## Resource `ValidatorConfig`

Validator info stored in validator address.


<pre><code>struct ValidatorConfig has copy, drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>consensus_pubkey: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>fullnode_addresses: vector&lt;u8&gt;</code>
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


<pre><code>struct ValidatorInfo has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_power: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>config: stake::ValidatorConfig</code>
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
3. on_new_epoch processes two pending queues and refresh ValidatorInfo from the owner's address.


<pre><code>struct ValidatorSet has copy, drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>consensus_scheme: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>active_validators: vector&lt;stake::ValidatorInfo&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: vector&lt;stake::ValidatorInfo&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: vector&lt;stake::ValidatorInfo&gt;</code>
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


<pre><code>struct AptosCoinCapabilities has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_IndividualValidatorPerformance"></a>

## Struct `IndividualValidatorPerformance`



<pre><code>struct IndividualValidatorPerformance has drop, store<br/></code></pre>



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



<pre><code>struct ValidatorPerformance has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>validators: vector&lt;stake::IndividualValidatorPerformance&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RegisterValidatorCandidateEvent"></a>

## Struct `RegisterValidatorCandidateEvent`



<pre><code>struct RegisterValidatorCandidateEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RegisterValidatorCandidate"></a>

## Struct `RegisterValidatorCandidate`



<pre><code>&#35;[event]<br/>struct RegisterValidatorCandidate has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_SetOperatorEvent"></a>

## Struct `SetOperatorEvent`



<pre><code>struct SetOperatorEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_operator: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_SetOperator"></a>

## Struct `SetOperator`



<pre><code>&#35;[event]<br/>struct SetOperator has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_operator: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_AddStakeEvent"></a>

## Struct `AddStakeEvent`



<pre><code>struct AddStakeEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>&#35;[event]<br/>struct AddStake has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>struct ReactivateStakeEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>&#35;[event]<br/>struct ReactivateStake has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>struct RotateConsensusKeyEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_consensus_pubkey: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_consensus_pubkey: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_RotateConsensusKey"></a>

## Struct `RotateConsensusKey`



<pre><code>&#35;[event]<br/>struct RotateConsensusKey has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_consensus_pubkey: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_consensus_pubkey: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_UpdateNetworkAndFullnodeAddressesEvent"></a>

## Struct `UpdateNetworkAndFullnodeAddressesEvent`



<pre><code>struct UpdateNetworkAndFullnodeAddressesEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_network_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_network_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_fullnode_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_fullnode_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_UpdateNetworkAndFullnodeAddresses"></a>

## Struct `UpdateNetworkAndFullnodeAddresses`



<pre><code>&#35;[event]<br/>struct UpdateNetworkAndFullnodeAddresses has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_network_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_network_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_fullnode_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_fullnode_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_IncreaseLockupEvent"></a>

## Struct `IncreaseLockupEvent`



<pre><code>struct IncreaseLockupEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>&#35;[event]<br/>struct IncreaseLockup has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>struct JoinValidatorSetEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_JoinValidatorSet"></a>

## Struct `JoinValidatorSet`



<pre><code>&#35;[event]<br/>struct JoinValidatorSet has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_DistributeRewardsEvent"></a>

## Struct `DistributeRewardsEvent`



<pre><code>struct DistributeRewardsEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>&#35;[event]<br/>struct DistributeRewards has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>struct UnlockStakeEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>&#35;[event]<br/>struct UnlockStake has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>struct WithdrawStakeEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>&#35;[event]<br/>struct WithdrawStake has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
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



<pre><code>struct LeaveValidatorSetEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_LeaveValidatorSet"></a>

## Struct `LeaveValidatorSet`



<pre><code>&#35;[event]<br/>struct LeaveValidatorSet has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_ValidatorFees"></a>

## Resource `ValidatorFees`

Stores transaction fees assigned to validators. All fees are distributed to validators
at the end of the epoch.


<pre><code>struct ValidatorFees has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>fees_table: table::Table&lt;address, coin::Coin&lt;aptos_coin::AptosCoin&gt;&gt;</code>
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


<pre><code>struct AllowedValidators has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>accounts: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_valid_perf"></a>

## Resource `Ghost$ghost_valid_perf`



<pre><code>struct Ghost$ghost_valid_perf has copy, drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: stake::ValidatorPerformance</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_proposer_idx"></a>

## Resource `Ghost$ghost_proposer_idx`



<pre><code>struct Ghost$ghost_proposer_idx has copy, drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: option::Option&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_stake_Ghost$ghost_active_num"></a>

## Resource `Ghost$ghost_active_num`



<pre><code>struct Ghost$ghost_active_num has copy, drop, store, key<br/></code></pre>



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



<pre><code>struct Ghost$ghost_pending_inactive_num has copy, drop, store, key<br/></code></pre>



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



<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_stake_EALREADY_REGISTERED"></a>

Account is already registered as a validator candidate.


<pre><code>const EALREADY_REGISTERED: u64 &#61; 8;<br/></code></pre>



<a id="0x1_stake_MAX_REWARDS_RATE"></a>

Limit the maximum value of <code>rewards_rate</code> in order to avoid any arithmetic overflow.


<pre><code>const MAX_REWARDS_RATE: u64 &#61; 1000000;<br/></code></pre>



<a id="0x1_stake_EALREADY_ACTIVE_VALIDATOR"></a>

Account is already a validator or pending validator.


<pre><code>const EALREADY_ACTIVE_VALIDATOR: u64 &#61; 4;<br/></code></pre>



<a id="0x1_stake_EFEES_TABLE_ALREADY_EXISTS"></a>

Table to store collected transaction fees for each validator already exists.


<pre><code>const EFEES_TABLE_ALREADY_EXISTS: u64 &#61; 19;<br/></code></pre>



<a id="0x1_stake_EINELIGIBLE_VALIDATOR"></a>

Validator is not defined in the ACL of entities allowed to be validators


<pre><code>const EINELIGIBLE_VALIDATOR: u64 &#61; 17;<br/></code></pre>



<a id="0x1_stake_EINVALID_LOCKUP"></a>

Cannot update stake pool's lockup to earlier than current lockup.


<pre><code>const EINVALID_LOCKUP: u64 &#61; 18;<br/></code></pre>



<a id="0x1_stake_EINVALID_PUBLIC_KEY"></a>

Invalid consensus public key


<pre><code>const EINVALID_PUBLIC_KEY: u64 &#61; 11;<br/></code></pre>



<a id="0x1_stake_ELAST_VALIDATOR"></a>

Can't remove last validator.


<pre><code>const ELAST_VALIDATOR: u64 &#61; 6;<br/></code></pre>



<a id="0x1_stake_ENOT_OPERATOR"></a>

Account does not have the right operator capability.


<pre><code>const ENOT_OPERATOR: u64 &#61; 9;<br/></code></pre>



<a id="0x1_stake_ENOT_VALIDATOR"></a>

Account is not a validator.


<pre><code>const ENOT_VALIDATOR: u64 &#61; 5;<br/></code></pre>



<a id="0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED"></a>

Validators cannot join or leave post genesis on this test network.


<pre><code>const ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED: u64 &#61; 10;<br/></code></pre>



<a id="0x1_stake_EOWNER_CAP_ALREADY_EXISTS"></a>

An account cannot own more than one owner capability.


<pre><code>const EOWNER_CAP_ALREADY_EXISTS: u64 &#61; 16;<br/></code></pre>



<a id="0x1_stake_EOWNER_CAP_NOT_FOUND"></a>

Owner capability does not exist at the provided account.


<pre><code>const EOWNER_CAP_NOT_FOUND: u64 &#61; 15;<br/></code></pre>



<a id="0x1_stake_ERECONFIGURATION_IN_PROGRESS"></a>

Validator set change temporarily disabled because of in-progress reconfiguration.


<pre><code>const ERECONFIGURATION_IN_PROGRESS: u64 &#61; 20;<br/></code></pre>



<a id="0x1_stake_ESTAKE_EXCEEDS_MAX"></a>

Total stake exceeds maximum allowed.


<pre><code>const ESTAKE_EXCEEDS_MAX: u64 &#61; 7;<br/></code></pre>



<a id="0x1_stake_ESTAKE_POOL_DOES_NOT_EXIST"></a>

Stake pool does not exist at the provided pool address.


<pre><code>const ESTAKE_POOL_DOES_NOT_EXIST: u64 &#61; 14;<br/></code></pre>



<a id="0x1_stake_ESTAKE_TOO_HIGH"></a>

Too much stake to join validator set.


<pre><code>const ESTAKE_TOO_HIGH: u64 &#61; 3;<br/></code></pre>



<a id="0x1_stake_ESTAKE_TOO_LOW"></a>

Not enough stake to join validator set.


<pre><code>const ESTAKE_TOO_LOW: u64 &#61; 2;<br/></code></pre>



<a id="0x1_stake_EVALIDATOR_CONFIG"></a>

Validator Config not published.


<pre><code>const EVALIDATOR_CONFIG: u64 &#61; 1;<br/></code></pre>



<a id="0x1_stake_EVALIDATOR_SET_TOO_LARGE"></a>

Validator set exceeds the limit


<pre><code>const EVALIDATOR_SET_TOO_LARGE: u64 &#61; 12;<br/></code></pre>



<a id="0x1_stake_EVOTING_POWER_INCREASE_EXCEEDS_LIMIT"></a>

Voting power increase has exceeded the limit for this current epoch.


<pre><code>const EVOTING_POWER_INCREASE_EXCEEDS_LIMIT: u64 &#61; 13;<br/></code></pre>



<a id="0x1_stake_MAX_VALIDATOR_SET_SIZE"></a>

Limit the maximum size to u16::max, it's the current limit of the bitvec
https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-bitvec/src/lib.rs#L20


<pre><code>const MAX_VALIDATOR_SET_SIZE: u64 &#61; 65536;<br/></code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_ACTIVE"></a>



<pre><code>const VALIDATOR_STATUS_ACTIVE: u64 &#61; 2;<br/></code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code>const VALIDATOR_STATUS_INACTIVE: u64 &#61; 4;<br/></code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE"></a>

Validator status enum. We can switch to proper enum later once Move supports it.


<pre><code>const VALIDATOR_STATUS_PENDING_ACTIVE: u64 &#61; 1;<br/></code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE"></a>



<pre><code>const VALIDATOR_STATUS_PENDING_INACTIVE: u64 &#61; 3;<br/></code></pre>



<a id="0x1_stake_initialize_validator_fees"></a>

## Function `initialize_validator_fees`

Initializes the resource storing information about collected transaction fees per validator.
Used by <code>transaction_fee.move</code> to initialize fee collection and distribution.


<pre><code>public(friend) fun initialize_validator_fees(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_validator_fees(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(<br/>        !exists&lt;ValidatorFees&gt;(@aptos_framework),<br/>        error::already_exists(EFEES_TABLE_ALREADY_EXISTS)<br/>    );<br/>    move_to(aptos_framework, ValidatorFees &#123; fees_table: table::new() &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_add_transaction_fee"></a>

## Function `add_transaction_fee`

Stores the transaction fee collected to the specified validator address.


<pre><code>public(friend) fun add_transaction_fee(validator_addr: address, fee: coin::Coin&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun add_transaction_fee(validator_addr: address, fee: Coin&lt;AptosCoin&gt;) acquires ValidatorFees &#123;<br/>    let fees_table &#61; &amp;mut borrow_global_mut&lt;ValidatorFees&gt;(@aptos_framework).fees_table;<br/>    if (table::contains(fees_table, validator_addr)) &#123;<br/>        let collected_fee &#61; table::borrow_mut(fees_table, validator_addr);<br/>        coin::merge(collected_fee, fee);<br/>    &#125; else &#123;<br/>        table::add(fees_table, validator_addr, fee);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_lockup_secs"></a>

## Function `get_lockup_secs`

Return the lockup expiration of the stake pool at <code>pool_address</code>.
This will throw an error if there's no stake pool at <code>pool_address</code>.


<pre><code>&#35;[view]<br/>public fun get_lockup_secs(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_lockup_secs(pool_address: address): u64 acquires StakePool &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    borrow_global&lt;StakePool&gt;(pool_address).locked_until_secs<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_remaining_lockup_secs"></a>

## Function `get_remaining_lockup_secs`

Return the remaining lockup of the stake pool at <code>pool_address</code>.
This will throw an error if there's no stake pool at <code>pool_address</code>.


<pre><code>&#35;[view]<br/>public fun get_remaining_lockup_secs(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_remaining_lockup_secs(pool_address: address): u64 acquires StakePool &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    let lockup_time &#61; borrow_global&lt;StakePool&gt;(pool_address).locked_until_secs;<br/>    if (lockup_time &lt;&#61; timestamp::now_seconds()) &#123;<br/>        0<br/>    &#125; else &#123;<br/>        lockup_time &#45; timestamp::now_seconds()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_stake"></a>

## Function `get_stake`

Return the different stake amounts for <code>pool_address</code> (whether the validator is active or not).
The returned amounts are for (active, inactive, pending_active, pending_inactive) stake respectively.


<pre><code>&#35;[view]<br/>public fun get_stake(pool_address: address): (u64, u64, u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_stake(pool_address: address): (u64, u64, u64, u64) acquires StakePool &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    let stake_pool &#61; borrow_global&lt;StakePool&gt;(pool_address);<br/>    (<br/>        coin::value(&amp;stake_pool.active),<br/>        coin::value(&amp;stake_pool.inactive),<br/>        coin::value(&amp;stake_pool.pending_active),<br/>        coin::value(&amp;stake_pool.pending_inactive),<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_validator_state"></a>

## Function `get_validator_state`

Returns the validator's state.


<pre><code>&#35;[view]<br/>public fun get_validator_state(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_validator_state(pool_address: address): u64 acquires ValidatorSet &#123;<br/>    let validator_set &#61; borrow_global&lt;ValidatorSet&gt;(@aptos_framework);<br/>    if (option::is_some(&amp;find_validator(&amp;validator_set.pending_active, pool_address))) &#123;<br/>        VALIDATOR_STATUS_PENDING_ACTIVE<br/>    &#125; else if (option::is_some(&amp;find_validator(&amp;validator_set.active_validators, pool_address))) &#123;<br/>        VALIDATOR_STATUS_ACTIVE<br/>    &#125; else if (option::is_some(&amp;find_validator(&amp;validator_set.pending_inactive, pool_address))) &#123;<br/>        VALIDATOR_STATUS_PENDING_INACTIVE<br/>    &#125; else &#123;<br/>        VALIDATOR_STATUS_INACTIVE<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_current_epoch_voting_power"></a>

## Function `get_current_epoch_voting_power`

Return the voting power of the validator in the current epoch.
This is the same as the validator's total active and pending_inactive stake.


<pre><code>&#35;[view]<br/>public fun get_current_epoch_voting_power(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_current_epoch_voting_power(pool_address: address): u64 acquires StakePool, ValidatorSet &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    let validator_state &#61; get_validator_state(pool_address);<br/>    // Both active and pending inactive validators can still vote in the current epoch.<br/>    if (validator_state &#61;&#61; VALIDATOR_STATUS_ACTIVE &#124;&#124; validator_state &#61;&#61; VALIDATOR_STATUS_PENDING_INACTIVE) &#123;<br/>        let active_stake &#61; coin::value(&amp;borrow_global&lt;StakePool&gt;(pool_address).active);<br/>        let pending_inactive_stake &#61; coin::value(&amp;borrow_global&lt;StakePool&gt;(pool_address).pending_inactive);<br/>        active_stake &#43; pending_inactive_stake<br/>    &#125; else &#123;<br/>        0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_delegated_voter"></a>

## Function `get_delegated_voter`

Return the delegated voter of the validator at <code>pool_address</code>.


<pre><code>&#35;[view]<br/>public fun get_delegated_voter(pool_address: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegated_voter(pool_address: address): address acquires StakePool &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    borrow_global&lt;StakePool&gt;(pool_address).delegated_voter<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_operator"></a>

## Function `get_operator`

Return the operator of the validator at <code>pool_address</code>.


<pre><code>&#35;[view]<br/>public fun get_operator(pool_address: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_operator(pool_address: address): address acquires StakePool &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    borrow_global&lt;StakePool&gt;(pool_address).operator_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_owned_pool_address"></a>

## Function `get_owned_pool_address`

Return the pool address in <code>owner_cap</code>.


<pre><code>public fun get_owned_pool_address(owner_cap: &amp;stake::OwnerCapability): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_owned_pool_address(owner_cap: &amp;OwnerCapability): address &#123;<br/>    owner_cap.pool_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_validator_index"></a>

## Function `get_validator_index`

Return the validator index for <code>pool_address</code>.


<pre><code>&#35;[view]<br/>public fun get_validator_index(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_validator_index(pool_address: address): u64 acquires ValidatorConfig &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    borrow_global&lt;ValidatorConfig&gt;(pool_address).validator_index<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_current_epoch_proposal_counts"></a>

## Function `get_current_epoch_proposal_counts`

Return the number of successful and failed proposals for the proposal at the given validator index.


<pre><code>&#35;[view]<br/>public fun get_current_epoch_proposal_counts(validator_index: u64): (u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_current_epoch_proposal_counts(validator_index: u64): (u64, u64) acquires ValidatorPerformance &#123;<br/>    let validator_performances &#61; &amp;borrow_global&lt;ValidatorPerformance&gt;(@aptos_framework).validators;<br/>    let validator_performance &#61; vector::borrow(validator_performances, validator_index);<br/>    (validator_performance.successful_proposals, validator_performance.failed_proposals)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_validator_config"></a>

## Function `get_validator_config`

Return the validator's config.


<pre><code>&#35;[view]<br/>public fun get_validator_config(pool_address: address): (vector&lt;u8&gt;, vector&lt;u8&gt;, vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_validator_config(<br/>    pool_address: address<br/>): (vector&lt;u8&gt;, vector&lt;u8&gt;, vector&lt;u8&gt;) acquires ValidatorConfig &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    let validator_config &#61; borrow_global&lt;ValidatorConfig&gt;(pool_address);<br/>    (validator_config.consensus_pubkey, validator_config.network_addresses, validator_config.fullnode_addresses)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_stake_pool_exists"></a>

## Function `stake_pool_exists`



<pre><code>&#35;[view]<br/>public fun stake_pool_exists(addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun stake_pool_exists(addr: address): bool &#123;<br/>    exists&lt;StakePool&gt;(addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_initialize"></a>

## Function `initialize`

Initialize validator set to the core resource account.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    move_to(aptos_framework, ValidatorSet &#123;<br/>        consensus_scheme: 0,<br/>        active_validators: vector::empty(),<br/>        pending_active: vector::empty(),<br/>        pending_inactive: vector::empty(),<br/>        total_voting_power: 0,<br/>        total_joining_power: 0,<br/>    &#125;);<br/><br/>    move_to(aptos_framework, ValidatorPerformance &#123;<br/>        validators: vector::empty(),<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

This is only called during Genesis, which is where MintCapability<AptosCoin> can be created.
Beyond genesis, no one can create AptosCoin mint/burn capabilities.


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: MintCapability&lt;AptosCoin&gt;) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    move_to(aptos_framework, AptosCoinCapabilities &#123; mint_cap &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_remove_validators"></a>

## Function `remove_validators`

Allow on chain governance to remove validators from the validator set.


<pre><code>public fun remove_validators(aptos_framework: &amp;signer, validators: &amp;vector&lt;address&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_validators(<br/>    aptos_framework: &amp;signer,<br/>    validators: &amp;vector&lt;address&gt;,<br/>) acquires ValidatorSet &#123;<br/>    assert_reconfig_not_in_progress();<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);<br/>    let active_validators &#61; &amp;mut validator_set.active_validators;<br/>    let pending_inactive &#61; &amp;mut validator_set.pending_inactive;<br/>    spec &#123;<br/>        update ghost_active_num &#61; len(active_validators);<br/>        update ghost_pending_inactive_num &#61; len(pending_inactive);<br/>    &#125;;<br/>    let len_validators &#61; vector::length(validators);<br/>    let i &#61; 0;<br/>    // Remove each validator from the validator set.<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant i &lt;&#61; len_validators;<br/>            invariant spec_validators_are_initialized(active_validators);<br/>            invariant spec_validator_indices_are_valid(active_validators);<br/>            invariant spec_validators_are_initialized(pending_inactive);<br/>            invariant spec_validator_indices_are_valid(pending_inactive);<br/>            invariant ghost_active_num &#43; ghost_pending_inactive_num &#61;&#61; len(active_validators) &#43; len(pending_inactive);<br/>        &#125;;<br/>        i &lt; len_validators<br/>    &#125;) &#123;<br/>        let validator &#61; &#42;vector::borrow(validators, i);<br/>        let validator_index &#61; find_validator(active_validators, validator);<br/>        if (option::is_some(&amp;validator_index)) &#123;<br/>            let validator_info &#61; vector::swap_remove(active_validators, &#42;option::borrow(&amp;validator_index));<br/>            vector::push_back(pending_inactive, validator_info);<br/>            spec &#123;<br/>                update ghost_active_num &#61; ghost_active_num &#45; 1;<br/>                update ghost_pending_inactive_num &#61; ghost_pending_inactive_num &#43; 1;<br/>            &#125;;<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_initialize_stake_owner"></a>

## Function `initialize_stake_owner`

Initialize the validator account and give ownership to the signing account
except it leaves the ValidatorConfig to be set by another entity.
Note: this triggers setting the operator and owner, set it to the account's address
to set later.


<pre><code>public entry fun initialize_stake_owner(owner: &amp;signer, initial_stake_amount: u64, operator: address, voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize_stake_owner(<br/>    owner: &amp;signer,<br/>    initial_stake_amount: u64,<br/>    operator: address,<br/>    voter: address,<br/>) acquires AllowedValidators, OwnerCapability, StakePool, ValidatorSet &#123;<br/>    initialize_owner(owner);<br/>    move_to(owner, ValidatorConfig &#123;<br/>        consensus_pubkey: vector::empty(),<br/>        network_addresses: vector::empty(),<br/>        fullnode_addresses: vector::empty(),<br/>        validator_index: 0,<br/>    &#125;);<br/><br/>    if (initial_stake_amount &gt; 0) &#123;<br/>        add_stake(owner, initial_stake_amount);<br/>    &#125;;<br/><br/>    let account_address &#61; signer::address_of(owner);<br/>    if (account_address !&#61; operator) &#123;<br/>        set_operator(owner, operator)<br/>    &#125;;<br/>    if (account_address !&#61; voter) &#123;<br/>        set_delegated_voter(owner, voter)<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_initialize_validator"></a>

## Function `initialize_validator`

Initialize the validator account and give ownership to the signing account.


<pre><code>public entry fun initialize_validator(account: &amp;signer, consensus_pubkey: vector&lt;u8&gt;, proof_of_possession: vector&lt;u8&gt;, network_addresses: vector&lt;u8&gt;, fullnode_addresses: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize_validator(<br/>    account: &amp;signer,<br/>    consensus_pubkey: vector&lt;u8&gt;,<br/>    proof_of_possession: vector&lt;u8&gt;,<br/>    network_addresses: vector&lt;u8&gt;,<br/>    fullnode_addresses: vector&lt;u8&gt;,<br/>) acquires AllowedValidators &#123;<br/>    // Checks the public key has a valid proof&#45;of&#45;possession to prevent rogue&#45;key attacks.<br/>    let pubkey_from_pop &#61; &amp;mut bls12381::public_key_from_bytes_with_pop(<br/>        consensus_pubkey,<br/>        &amp;proof_of_possession_from_bytes(proof_of_possession)<br/>    );<br/>    assert!(option::is_some(pubkey_from_pop), error::invalid_argument(EINVALID_PUBLIC_KEY));<br/><br/>    initialize_owner(account);<br/>    move_to(account, ValidatorConfig &#123;<br/>        consensus_pubkey,<br/>        network_addresses,<br/>        fullnode_addresses,<br/>        validator_index: 0,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_initialize_owner"></a>

## Function `initialize_owner`



<pre><code>fun initialize_owner(owner: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_owner(owner: &amp;signer) acquires AllowedValidators &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert!(is_allowed(owner_address), error::not_found(EINELIGIBLE_VALIDATOR));<br/>    assert!(!stake_pool_exists(owner_address), error::already_exists(EALREADY_REGISTERED));<br/><br/>    move_to(owner, StakePool &#123;<br/>        active: coin::zero&lt;AptosCoin&gt;(),<br/>        pending_active: coin::zero&lt;AptosCoin&gt;(),<br/>        pending_inactive: coin::zero&lt;AptosCoin&gt;(),<br/>        inactive: coin::zero&lt;AptosCoin&gt;(),<br/>        locked_until_secs: 0,<br/>        operator_address: owner_address,<br/>        delegated_voter: owner_address,<br/>        // Events.<br/>        initialize_validator_events: account::new_event_handle&lt;RegisterValidatorCandidateEvent&gt;(owner),<br/>        set_operator_events: account::new_event_handle&lt;SetOperatorEvent&gt;(owner),<br/>        add_stake_events: account::new_event_handle&lt;AddStakeEvent&gt;(owner),<br/>        reactivate_stake_events: account::new_event_handle&lt;ReactivateStakeEvent&gt;(owner),<br/>        rotate_consensus_key_events: account::new_event_handle&lt;RotateConsensusKeyEvent&gt;(owner),<br/>        update_network_and_fullnode_addresses_events: account::new_event_handle&lt;UpdateNetworkAndFullnodeAddressesEvent&gt;(<br/>            owner<br/>        ),<br/>        increase_lockup_events: account::new_event_handle&lt;IncreaseLockupEvent&gt;(owner),<br/>        join_validator_set_events: account::new_event_handle&lt;JoinValidatorSetEvent&gt;(owner),<br/>        distribute_rewards_events: account::new_event_handle&lt;DistributeRewardsEvent&gt;(owner),<br/>        unlock_stake_events: account::new_event_handle&lt;UnlockStakeEvent&gt;(owner),<br/>        withdraw_stake_events: account::new_event_handle&lt;WithdrawStakeEvent&gt;(owner),<br/>        leave_validator_set_events: account::new_event_handle&lt;LeaveValidatorSetEvent&gt;(owner),<br/>    &#125;);<br/><br/>    move_to(owner, OwnerCapability &#123; pool_address: owner_address &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_extract_owner_cap"></a>

## Function `extract_owner_cap`

Extract and return owner capability from the signing account.


<pre><code>public fun extract_owner_cap(owner: &amp;signer): stake::OwnerCapability<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract_owner_cap(owner: &amp;signer): OwnerCapability acquires OwnerCapability &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert_owner_cap_exists(owner_address);<br/>    move_from&lt;OwnerCapability&gt;(owner_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_deposit_owner_cap"></a>

## Function `deposit_owner_cap`

Deposit <code>owner_cap</code> into <code>account</code>. This requires <code>account</code> to not already have ownership of another
staking pool.


<pre><code>public fun deposit_owner_cap(owner: &amp;signer, owner_cap: stake::OwnerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_owner_cap(owner: &amp;signer, owner_cap: OwnerCapability) &#123;<br/>    assert!(!exists&lt;OwnerCapability&gt;(signer::address_of(owner)), error::not_found(EOWNER_CAP_ALREADY_EXISTS));<br/>    move_to(owner, owner_cap);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_destroy_owner_cap"></a>

## Function `destroy_owner_cap`

Destroy <code>owner_cap</code>.


<pre><code>public fun destroy_owner_cap(owner_cap: stake::OwnerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_owner_cap(owner_cap: OwnerCapability) &#123;<br/>    let OwnerCapability &#123; pool_address: _ &#125; &#61; owner_cap;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the stake pool.


<pre><code>public entry fun set_operator(owner: &amp;signer, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_operator(owner: &amp;signer, new_operator: address) acquires OwnerCapability, StakePool &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert_owner_cap_exists(owner_address);<br/>    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);<br/>    set_operator_with_cap(ownership_cap, new_operator);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_set_operator_with_cap"></a>

## Function `set_operator_with_cap`

Allows an account with ownership capability to change the operator of the stake pool.


<pre><code>public fun set_operator_with_cap(owner_cap: &amp;stake::OwnerCapability, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_operator_with_cap(owner_cap: &amp;OwnerCapability, new_operator: address) acquires StakePool &#123;<br/>    let pool_address &#61; owner_cap.pool_address;<br/>    assert_stake_pool_exists(pool_address);<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    let old_operator &#61; stake_pool.operator_address;<br/>    stake_pool.operator_address &#61; new_operator;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            SetOperator &#123;<br/>                pool_address,<br/>                old_operator,<br/>                new_operator,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/><br/>    event::emit_event(<br/>        &amp;mut stake_pool.set_operator_events,<br/>        SetOperatorEvent &#123;<br/>            pool_address,<br/>            old_operator,<br/>            new_operator,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_set_delegated_voter"></a>

## Function `set_delegated_voter`

Allows an owner to change the delegated voter of the stake pool.


<pre><code>public entry fun set_delegated_voter(owner: &amp;signer, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_delegated_voter(owner: &amp;signer, new_voter: address) acquires OwnerCapability, StakePool &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert_owner_cap_exists(owner_address);<br/>    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);<br/>    set_delegated_voter_with_cap(ownership_cap, new_voter);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_set_delegated_voter_with_cap"></a>

## Function `set_delegated_voter_with_cap`

Allows an owner to change the delegated voter of the stake pool.


<pre><code>public fun set_delegated_voter_with_cap(owner_cap: &amp;stake::OwnerCapability, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_delegated_voter_with_cap(owner_cap: &amp;OwnerCapability, new_voter: address) acquires StakePool &#123;<br/>    let pool_address &#61; owner_cap.pool_address;<br/>    assert_stake_pool_exists(pool_address);<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    stake_pool.delegated_voter &#61; new_voter;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins from the <code>account</code> owning the StakePool.


<pre><code>public entry fun add_stake(owner: &amp;signer, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_stake(owner: &amp;signer, amount: u64) acquires OwnerCapability, StakePool, ValidatorSet &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert_owner_cap_exists(owner_address);<br/>    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);<br/>    add_stake_with_cap(ownership_cap, coin::withdraw&lt;AptosCoin&gt;(owner, amount));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_add_stake_with_cap"></a>

## Function `add_stake_with_cap`

Add <code>coins</code> into <code>pool_address</code>. this requires the corresponding <code>owner_cap</code> to be passed in.


<pre><code>public fun add_stake_with_cap(owner_cap: &amp;stake::OwnerCapability, coins: coin::Coin&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_stake_with_cap(owner_cap: &amp;OwnerCapability, coins: Coin&lt;AptosCoin&gt;) acquires StakePool, ValidatorSet &#123;<br/>    assert_reconfig_not_in_progress();<br/>    let pool_address &#61; owner_cap.pool_address;<br/>    assert_stake_pool_exists(pool_address);<br/><br/>    let amount &#61; coin::value(&amp;coins);<br/>    if (amount &#61;&#61; 0) &#123;<br/>        coin::destroy_zero(coins);<br/>        return<br/>    &#125;;<br/><br/>    // Only track and validate voting power increase for active and pending_active validator.<br/>    // Pending_inactive validator will be removed from the validator set in the next epoch.<br/>    // Inactive validator&apos;s total stake will be tracked when they join the validator set.<br/>    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);<br/>    // Search directly rather using get_validator_state to save on unnecessary loops.<br/>    if (option::is_some(&amp;find_validator(&amp;validator_set.active_validators, pool_address)) &#124;&#124;<br/>        option::is_some(&amp;find_validator(&amp;validator_set.pending_active, pool_address))) &#123;<br/>        update_voting_power_increase(amount);<br/>    &#125;;<br/><br/>    // Add to pending_active if it&apos;s a current validator because the stake is not counted until the next epoch.<br/>    // Otherwise, the delegation can be added to active directly as the validator is also activated in the epoch.<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    if (is_current_epoch_validator(pool_address)) &#123;<br/>        coin::merge&lt;AptosCoin&gt;(&amp;mut stake_pool.pending_active, coins);<br/>    &#125; else &#123;<br/>        coin::merge&lt;AptosCoin&gt;(&amp;mut stake_pool.active, coins);<br/>    &#125;;<br/><br/>    let (_, maximum_stake) &#61; staking_config::get_required_stake(&amp;staking_config::get());<br/>    let voting_power &#61; get_next_epoch_voting_power(stake_pool);<br/>    assert!(voting_power &lt;&#61; maximum_stake, error::invalid_argument(ESTAKE_EXCEEDS_MAX));<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            AddStake &#123;<br/>                pool_address,<br/>                amount_added: amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.add_stake_events,<br/>        AddStakeEvent &#123;<br/>            pool_address,<br/>            amount_added: amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code>public entry fun reactivate_stake(owner: &amp;signer, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reactivate_stake(owner: &amp;signer, amount: u64) acquires OwnerCapability, StakePool &#123;<br/>    assert_reconfig_not_in_progress();<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert_owner_cap_exists(owner_address);<br/>    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);<br/>    reactivate_stake_with_cap(ownership_cap, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_reactivate_stake_with_cap"></a>

## Function `reactivate_stake_with_cap`



<pre><code>public fun reactivate_stake_with_cap(owner_cap: &amp;stake::OwnerCapability, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reactivate_stake_with_cap(owner_cap: &amp;OwnerCapability, amount: u64) acquires StakePool &#123;<br/>    assert_reconfig_not_in_progress();<br/>    let pool_address &#61; owner_cap.pool_address;<br/>    assert_stake_pool_exists(pool_address);<br/><br/>    // Cap the amount to reactivate by the amount in pending_inactive.<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    let total_pending_inactive &#61; coin::value(&amp;stake_pool.pending_inactive);<br/>    amount &#61; min(amount, total_pending_inactive);<br/><br/>    // Since this does not count as a voting power change (pending inactive still counts as voting power in the<br/>    // current epoch), stake can be immediately moved from pending inactive to active.<br/>    // We also don&apos;t need to check voting power increase as there&apos;s none.<br/>    let reactivated_coins &#61; coin::extract(&amp;mut stake_pool.pending_inactive, amount);<br/>    coin::merge(&amp;mut stake_pool.active, reactivated_coins);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            ReactivateStake &#123;<br/>                pool_address,<br/>                amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.reactivate_stake_events,<br/>        ReactivateStakeEvent &#123;<br/>            pool_address,<br/>            amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_rotate_consensus_key"></a>

## Function `rotate_consensus_key`

Rotate the consensus key of the validator, it'll take effect in next epoch.


<pre><code>public entry fun rotate_consensus_key(operator: &amp;signer, pool_address: address, new_consensus_pubkey: vector&lt;u8&gt;, proof_of_possession: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun rotate_consensus_key(<br/>    operator: &amp;signer,<br/>    pool_address: address,<br/>    new_consensus_pubkey: vector&lt;u8&gt;,<br/>    proof_of_possession: vector&lt;u8&gt;,<br/>) acquires StakePool, ValidatorConfig &#123;<br/>    assert_reconfig_not_in_progress();<br/>    assert_stake_pool_exists(pool_address);<br/><br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    assert!(signer::address_of(operator) &#61;&#61; stake_pool.operator_address, error::unauthenticated(ENOT_OPERATOR));<br/><br/>    assert!(exists&lt;ValidatorConfig&gt;(pool_address), error::not_found(EVALIDATOR_CONFIG));<br/>    let validator_info &#61; borrow_global_mut&lt;ValidatorConfig&gt;(pool_address);<br/>    let old_consensus_pubkey &#61; validator_info.consensus_pubkey;<br/>    // Checks the public key has a valid proof&#45;of&#45;possession to prevent rogue&#45;key attacks.<br/>    let pubkey_from_pop &#61; &amp;mut bls12381::public_key_from_bytes_with_pop(<br/>        new_consensus_pubkey,<br/>        &amp;proof_of_possession_from_bytes(proof_of_possession)<br/>    );<br/>    assert!(option::is_some(pubkey_from_pop), error::invalid_argument(EINVALID_PUBLIC_KEY));<br/>    validator_info.consensus_pubkey &#61; new_consensus_pubkey;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            RotateConsensusKey &#123;<br/>                pool_address,<br/>                old_consensus_pubkey,<br/>                new_consensus_pubkey,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.rotate_consensus_key_events,<br/>        RotateConsensusKeyEvent &#123;<br/>            pool_address,<br/>            old_consensus_pubkey,<br/>            new_consensus_pubkey,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_update_network_and_fullnode_addresses"></a>

## Function `update_network_and_fullnode_addresses`

Update the network and full node addresses of the validator. This only takes effect in the next epoch.


<pre><code>public entry fun update_network_and_fullnode_addresses(operator: &amp;signer, pool_address: address, new_network_addresses: vector&lt;u8&gt;, new_fullnode_addresses: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_network_and_fullnode_addresses(<br/>    operator: &amp;signer,<br/>    pool_address: address,<br/>    new_network_addresses: vector&lt;u8&gt;,<br/>    new_fullnode_addresses: vector&lt;u8&gt;,<br/>) acquires StakePool, ValidatorConfig &#123;<br/>    assert_reconfig_not_in_progress();<br/>    assert_stake_pool_exists(pool_address);<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    assert!(signer::address_of(operator) &#61;&#61; stake_pool.operator_address, error::unauthenticated(ENOT_OPERATOR));<br/>    assert!(exists&lt;ValidatorConfig&gt;(pool_address), error::not_found(EVALIDATOR_CONFIG));<br/>    let validator_info &#61; borrow_global_mut&lt;ValidatorConfig&gt;(pool_address);<br/>    let old_network_addresses &#61; validator_info.network_addresses;<br/>    validator_info.network_addresses &#61; new_network_addresses;<br/>    let old_fullnode_addresses &#61; validator_info.fullnode_addresses;<br/>    validator_info.fullnode_addresses &#61; new_fullnode_addresses;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            UpdateNetworkAndFullnodeAddresses &#123;<br/>                pool_address,<br/>                old_network_addresses,<br/>                new_network_addresses,<br/>                old_fullnode_addresses,<br/>                new_fullnode_addresses,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.update_network_and_fullnode_addresses_events,<br/>        UpdateNetworkAndFullnodeAddressesEvent &#123;<br/>            pool_address,<br/>            old_network_addresses,<br/>            new_network_addresses,<br/>            old_fullnode_addresses,<br/>            new_fullnode_addresses,<br/>        &#125;,<br/>    );<br/><br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_increase_lockup"></a>

## Function `increase_lockup`

Similar to increase_lockup_with_cap but will use ownership capability from the signing account.


<pre><code>public entry fun increase_lockup(owner: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun increase_lockup(owner: &amp;signer) acquires OwnerCapability, StakePool &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert_owner_cap_exists(owner_address);<br/>    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);<br/>    increase_lockup_with_cap(ownership_cap);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_increase_lockup_with_cap"></a>

## Function `increase_lockup_with_cap`

Unlock from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
directly inactive if it's not from an active validator.


<pre><code>public fun increase_lockup_with_cap(owner_cap: &amp;stake::OwnerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun increase_lockup_with_cap(owner_cap: &amp;OwnerCapability) acquires StakePool &#123;<br/>    let pool_address &#61; owner_cap.pool_address;<br/>    assert_stake_pool_exists(pool_address);<br/>    let config &#61; staking_config::get();<br/><br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    let old_locked_until_secs &#61; stake_pool.locked_until_secs;<br/>    let new_locked_until_secs &#61; timestamp::now_seconds() &#43; staking_config::get_recurring_lockup_duration(&amp;config);<br/>    assert!(old_locked_until_secs &lt; new_locked_until_secs, error::invalid_argument(EINVALID_LOCKUP));<br/>    stake_pool.locked_until_secs &#61; new_locked_until_secs;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            IncreaseLockup &#123;<br/>                pool_address,<br/>                old_locked_until_secs,<br/>                new_locked_until_secs,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.increase_lockup_events,<br/>        IncreaseLockupEvent &#123;<br/>            pool_address,<br/>            old_locked_until_secs,<br/>            new_locked_until_secs,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_join_validator_set"></a>

## Function `join_validator_set`

This can only called by the operator of the validator/staking pool.


<pre><code>public entry fun join_validator_set(operator: &amp;signer, pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun join_validator_set(<br/>    operator: &amp;signer,<br/>    pool_address: address<br/>) acquires StakePool, ValidatorConfig, ValidatorSet &#123;<br/>    assert!(<br/>        staking_config::get_allow_validator_set_change(&amp;staking_config::get()),<br/>        error::invalid_argument(ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED),<br/>    );<br/><br/>    join_validator_set_internal(operator, pool_address);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_join_validator_set_internal"></a>

## Function `join_validator_set_internal`

Request to have <code>pool_address</code> join the validator set. Can only be called after calling <code>initialize_validator</code>.
If the validator has the required stake (more than minimum and less than maximum allowed), they will be
added to the pending_active queue. All validators in this queue will be added to the active set when the next
epoch starts (eligibility will be rechecked).

This internal version can only be called by the Genesis module during Genesis.


<pre><code>public(friend) fun join_validator_set_internal(operator: &amp;signer, pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun join_validator_set_internal(<br/>    operator: &amp;signer,<br/>    pool_address: address<br/>) acquires StakePool, ValidatorConfig, ValidatorSet &#123;<br/>    assert_reconfig_not_in_progress();<br/>    assert_stake_pool_exists(pool_address);<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    assert!(signer::address_of(operator) &#61;&#61; stake_pool.operator_address, error::unauthenticated(ENOT_OPERATOR));<br/>    assert!(<br/>        get_validator_state(pool_address) &#61;&#61; VALIDATOR_STATUS_INACTIVE,<br/>        error::invalid_state(EALREADY_ACTIVE_VALIDATOR),<br/>    );<br/><br/>    let config &#61; staking_config::get();<br/>    let (minimum_stake, maximum_stake) &#61; staking_config::get_required_stake(&amp;config);<br/>    let voting_power &#61; get_next_epoch_voting_power(stake_pool);<br/>    assert!(voting_power &gt;&#61; minimum_stake, error::invalid_argument(ESTAKE_TOO_LOW));<br/>    assert!(voting_power &lt;&#61; maximum_stake, error::invalid_argument(ESTAKE_TOO_HIGH));<br/><br/>    // Track and validate voting power increase.<br/>    update_voting_power_increase(voting_power);<br/><br/>    // Add validator to pending_active, to be activated in the next epoch.<br/>    let validator_config &#61; borrow_global_mut&lt;ValidatorConfig&gt;(pool_address);<br/>    assert!(!vector::is_empty(&amp;validator_config.consensus_pubkey), error::invalid_argument(EINVALID_PUBLIC_KEY));<br/><br/>    // Validate the current validator set size has not exceeded the limit.<br/>    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);<br/>    vector::push_back(<br/>        &amp;mut validator_set.pending_active,<br/>        generate_validator_info(pool_address, stake_pool, &#42;validator_config)<br/>    );<br/>    let validator_set_size &#61; vector::length(&amp;validator_set.active_validators) &#43; vector::length(<br/>        &amp;validator_set.pending_active<br/>    );<br/>    assert!(validator_set_size &lt;&#61; MAX_VALIDATOR_SET_SIZE, error::invalid_argument(EVALIDATOR_SET_TOO_LARGE));<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(JoinValidatorSet &#123; pool_address &#125;);<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.join_validator_set_events,<br/>        JoinValidatorSetEvent &#123; pool_address &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_unlock"></a>

## Function `unlock`

Similar to unlock_with_cap but will use ownership capability from the signing account.


<pre><code>public entry fun unlock(owner: &amp;signer, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock(owner: &amp;signer, amount: u64) acquires OwnerCapability, StakePool &#123;<br/>    assert_reconfig_not_in_progress();<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert_owner_cap_exists(owner_address);<br/>    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);<br/>    unlock_with_cap(amount, ownership_cap);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_unlock_with_cap"></a>

## Function `unlock_with_cap`

Unlock <code>amount</code> from the active stake. Only possible if the lockup has expired.


<pre><code>public fun unlock_with_cap(amount: u64, owner_cap: &amp;stake::OwnerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unlock_with_cap(amount: u64, owner_cap: &amp;OwnerCapability) acquires StakePool &#123;<br/>    assert_reconfig_not_in_progress();<br/>    // Short&#45;circuit if amount to unlock is 0 so we don&apos;t emit events.<br/>    if (amount &#61;&#61; 0) &#123;<br/>        return<br/>    &#125;;<br/><br/>    // Unlocked coins are moved to pending_inactive. When the current lockup cycle expires, they will be moved into<br/>    // inactive in the earliest possible epoch transition.<br/>    let pool_address &#61; owner_cap.pool_address;<br/>    assert_stake_pool_exists(pool_address);<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    // Cap amount to unlock by maximum active stake.<br/>    let amount &#61; min(amount, coin::value(&amp;stake_pool.active));<br/>    let unlocked_stake &#61; coin::extract(&amp;mut stake_pool.active, amount);<br/>    coin::merge&lt;AptosCoin&gt;(&amp;mut stake_pool.pending_inactive, unlocked_stake);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            UnlockStake &#123;<br/>                pool_address,<br/>                amount_unlocked: amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.unlock_stake_events,<br/>        UnlockStakeEvent &#123;<br/>            pool_address,<br/>            amount_unlocked: amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_withdraw"></a>

## Function `withdraw`

Withdraw from <code>account</code>'s inactive stake.


<pre><code>public entry fun withdraw(owner: &amp;signer, withdraw_amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun withdraw(<br/>    owner: &amp;signer,<br/>    withdraw_amount: u64<br/>) acquires OwnerCapability, StakePool, ValidatorSet &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert_owner_cap_exists(owner_address);<br/>    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);<br/>    let coins &#61; withdraw_with_cap(ownership_cap, withdraw_amount);<br/>    coin::deposit&lt;AptosCoin&gt;(owner_address, coins);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_withdraw_with_cap"></a>

## Function `withdraw_with_cap`

Withdraw from <code>pool_address</code>'s inactive stake with the corresponding <code>owner_cap</code>.


<pre><code>public fun withdraw_with_cap(owner_cap: &amp;stake::OwnerCapability, withdraw_amount: u64): coin::Coin&lt;aptos_coin::AptosCoin&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_with_cap(<br/>    owner_cap: &amp;OwnerCapability,<br/>    withdraw_amount: u64<br/>): Coin&lt;AptosCoin&gt; acquires StakePool, ValidatorSet &#123;<br/>    assert_reconfig_not_in_progress();<br/>    let pool_address &#61; owner_cap.pool_address;<br/>    assert_stake_pool_exists(pool_address);<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    // There&apos;s an edge case where a validator unlocks their stake and leaves the validator set before<br/>    // the stake is fully unlocked (the current lockup cycle has not expired yet).<br/>    // This can leave their stake stuck in pending_inactive even after the current lockup cycle expires.<br/>    if (get_validator_state(pool_address) &#61;&#61; VALIDATOR_STATUS_INACTIVE &amp;&amp;<br/>        timestamp::now_seconds() &gt;&#61; stake_pool.locked_until_secs) &#123;<br/>        let pending_inactive_stake &#61; coin::extract_all(&amp;mut stake_pool.pending_inactive);<br/>        coin::merge(&amp;mut stake_pool.inactive, pending_inactive_stake);<br/>    &#125;;<br/><br/>    // Cap withdraw amount by total inactive coins.<br/>    withdraw_amount &#61; min(withdraw_amount, coin::value(&amp;stake_pool.inactive));<br/>    if (withdraw_amount &#61;&#61; 0) return coin::zero&lt;AptosCoin&gt;();<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            WithdrawStake &#123;<br/>                pool_address,<br/>                amount_withdrawn: withdraw_amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.withdraw_stake_events,<br/>        WithdrawStakeEvent &#123;<br/>            pool_address,<br/>            amount_withdrawn: withdraw_amount,<br/>        &#125;,<br/>    );<br/><br/>    coin::extract(&amp;mut stake_pool.inactive, withdraw_amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_leave_validator_set"></a>

## Function `leave_validator_set`

Request to have <code>pool_address</code> leave the validator set. The validator is only actually removed from the set when
the next epoch starts.
The last validator in the set cannot leave. This is an edge case that should never happen as long as the network
is still operational.

Can only be called by the operator of the validator/staking pool.


<pre><code>public entry fun leave_validator_set(operator: &amp;signer, pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun leave_validator_set(<br/>    operator: &amp;signer,<br/>    pool_address: address<br/>) acquires StakePool, ValidatorSet &#123;<br/>    assert_reconfig_not_in_progress();<br/>    let config &#61; staking_config::get();<br/>    assert!(<br/>        staking_config::get_allow_validator_set_change(&amp;config),<br/>        error::invalid_argument(ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED),<br/>    );<br/><br/>    assert_stake_pool_exists(pool_address);<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    // Account has to be the operator.<br/>    assert!(signer::address_of(operator) &#61;&#61; stake_pool.operator_address, error::unauthenticated(ENOT_OPERATOR));<br/><br/>    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);<br/>    // If the validator is still pending_active, directly kick the validator out.<br/>    let maybe_pending_active_index &#61; find_validator(&amp;validator_set.pending_active, pool_address);<br/>    if (option::is_some(&amp;maybe_pending_active_index)) &#123;<br/>        vector::swap_remove(<br/>            &amp;mut validator_set.pending_active, option::extract(&amp;mut maybe_pending_active_index));<br/><br/>        // Decrease the voting power increase as the pending validator&apos;s voting power was added when they requested<br/>        // to join. Now that they changed their mind, their voting power should not affect the joining limit of this<br/>        // epoch.<br/>        let validator_stake &#61; (get_next_epoch_voting_power(stake_pool) as u128);<br/>        // total_joining_power should be larger than validator_stake but just in case there has been a small<br/>        // rounding error somewhere that can lead to an underflow, we still want to allow this transaction to<br/>        // succeed.<br/>        if (validator_set.total_joining_power &gt; validator_stake) &#123;<br/>            validator_set.total_joining_power &#61; validator_set.total_joining_power &#45; validator_stake;<br/>        &#125; else &#123;<br/>            validator_set.total_joining_power &#61; 0;<br/>        &#125;;<br/>    &#125; else &#123;<br/>        // Validate that the validator is already part of the validator set.<br/>        let maybe_active_index &#61; find_validator(&amp;validator_set.active_validators, pool_address);<br/>        assert!(option::is_some(&amp;maybe_active_index), error::invalid_state(ENOT_VALIDATOR));<br/>        let validator_info &#61; vector::swap_remove(<br/>            &amp;mut validator_set.active_validators, option::extract(&amp;mut maybe_active_index));<br/>        assert!(vector::length(&amp;validator_set.active_validators) &gt; 0, error::invalid_state(ELAST_VALIDATOR));<br/>        vector::push_back(&amp;mut validator_set.pending_inactive, validator_info);<br/><br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(LeaveValidatorSet &#123; pool_address &#125;);<br/>        &#125;;<br/>        event::emit_event(<br/>            &amp;mut stake_pool.leave_validator_set_events,<br/>            LeaveValidatorSetEvent &#123;<br/>                pool_address,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_is_current_epoch_validator"></a>

## Function `is_current_epoch_validator`

Returns true if the current validator can still vote in the current epoch.
This includes validators that requested to leave but are still in the pending_inactive queue and will be removed
when the epoch starts.


<pre><code>public fun is_current_epoch_validator(pool_address: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_current_epoch_validator(pool_address: address): bool acquires ValidatorSet &#123;<br/>    assert_stake_pool_exists(pool_address);<br/>    let validator_state &#61; get_validator_state(pool_address);<br/>    validator_state &#61;&#61; VALIDATOR_STATUS_ACTIVE &#124;&#124; validator_state &#61;&#61; VALIDATOR_STATUS_PENDING_INACTIVE<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_update_performance_statistics"></a>

## Function `update_performance_statistics`

Update the validator performance (proposal statistics). This is only called by block::prologue().
This function cannot abort.


<pre><code>public(friend) fun update_performance_statistics(proposer_index: option::Option&lt;u64&gt;, failed_proposer_indices: vector&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun update_performance_statistics(<br/>    proposer_index: Option&lt;u64&gt;,<br/>    failed_proposer_indices: vector&lt;u64&gt;<br/>) acquires ValidatorPerformance &#123;<br/>    // Validator set cannot change until the end of the epoch, so the validator index in arguments should<br/>    // match with those of the validators in ValidatorPerformance resource.<br/>    let validator_perf &#61; borrow_global_mut&lt;ValidatorPerformance&gt;(@aptos_framework);<br/>    let validator_len &#61; vector::length(&amp;validator_perf.validators);<br/><br/>    spec &#123;<br/>        update ghost_valid_perf &#61; validator_perf;<br/>        update ghost_proposer_idx &#61; proposer_index;<br/>    &#125;;<br/>    // proposer_index is an option because it can be missing (for NilBlocks)<br/>    if (option::is_some(&amp;proposer_index)) &#123;<br/>        let cur_proposer_index &#61; option::extract(&amp;mut proposer_index);<br/>        // Here, and in all other vector::borrow, skip any validator indices that are out of bounds,<br/>        // this ensures that this function doesn&apos;t abort if there are out of bounds errors.<br/>        if (cur_proposer_index &lt; validator_len) &#123;<br/>            let validator &#61; vector::borrow_mut(&amp;mut validator_perf.validators, cur_proposer_index);<br/>            spec &#123;<br/>                assume validator.successful_proposals &#43; 1 &lt;&#61; MAX_U64;<br/>            &#125;;<br/>            validator.successful_proposals &#61; validator.successful_proposals &#43; 1;<br/>        &#125;;<br/>    &#125;;<br/><br/>    let f &#61; 0;<br/>    let f_len &#61; vector::length(&amp;failed_proposer_indices);<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant len(validator_perf.validators) &#61;&#61; validator_len;<br/>            invariant (option::spec_is_some(ghost_proposer_idx) &amp;&amp; option::spec_borrow(<br/>                ghost_proposer_idx<br/>            ) &lt; validator_len) &#61;&#61;&gt;<br/>                (validator_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals &#61;&#61;<br/>                    ghost_valid_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals &#43; 1);<br/>        &#125;;<br/>        f &lt; f_len<br/>    &#125;) &#123;<br/>        let validator_index &#61; &#42;vector::borrow(&amp;failed_proposer_indices, f);<br/>        if (validator_index &lt; validator_len) &#123;<br/>            let validator &#61; vector::borrow_mut(&amp;mut validator_perf.validators, validator_index);<br/>            spec &#123;<br/>                assume validator.failed_proposals &#43; 1 &lt;&#61; MAX_U64;<br/>            &#125;;<br/>            validator.failed_proposals &#61; validator.failed_proposals &#43; 1;<br/>        &#125;;<br/>        f &#61; f &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



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


<pre><code>public(friend) fun on_new_epoch()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(<br/>) acquires StakePool, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorFees &#123;<br/>    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);<br/>    let config &#61; staking_config::get();<br/>    let validator_perf &#61; borrow_global_mut&lt;ValidatorPerformance&gt;(@aptos_framework);<br/><br/>    // Process pending stake and distribute transaction fees and rewards for each currently active validator.<br/>    vector::for_each_ref(&amp;validator_set.active_validators, &#124;validator&#124; &#123;<br/>        let validator: &amp;ValidatorInfo &#61; validator;<br/>        update_stake_pool(validator_perf, validator.addr, &amp;config);<br/>    &#125;);<br/><br/>    // Process pending stake and distribute transaction fees and rewards for each currently pending_inactive validator<br/>    // (requested to leave but not removed yet).<br/>    vector::for_each_ref(&amp;validator_set.pending_inactive, &#124;validator&#124; &#123;<br/>        let validator: &amp;ValidatorInfo &#61; validator;<br/>        update_stake_pool(validator_perf, validator.addr, &amp;config);<br/>    &#125;);<br/><br/>    // Activate currently pending_active validators.<br/>    append(&amp;mut validator_set.active_validators, &amp;mut validator_set.pending_active);<br/><br/>    // Officially deactivate all pending_inactive validators. They will now no longer receive rewards.<br/>    validator_set.pending_inactive &#61; vector::empty();<br/><br/>    // Update active validator set so that network address/public key change takes effect.<br/>    // Moreover, recalculate the total voting power, and deactivate the validator whose<br/>    // voting power is less than the minimum required stake.<br/>    let next_epoch_validators &#61; vector::empty();<br/>    let (minimum_stake, _) &#61; staking_config::get_required_stake(&amp;config);<br/>    let vlen &#61; vector::length(&amp;validator_set.active_validators);<br/>    let total_voting_power &#61; 0;<br/>    let i &#61; 0;<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant spec_validators_are_initialized(next_epoch_validators);<br/>            invariant i &lt;&#61; vlen;<br/>        &#125;;<br/>        i &lt; vlen<br/>    &#125;) &#123;<br/>        let old_validator_info &#61; vector::borrow_mut(&amp;mut validator_set.active_validators, i);<br/>        let pool_address &#61; old_validator_info.addr;<br/>        let validator_config &#61; borrow_global_mut&lt;ValidatorConfig&gt;(pool_address);<br/>        let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>        let new_validator_info &#61; generate_validator_info(pool_address, stake_pool, &#42;validator_config);<br/><br/>        // A validator needs at least the min stake required to join the validator set.<br/>        if (new_validator_info.voting_power &gt;&#61; minimum_stake) &#123;<br/>            spec &#123;<br/>                assume total_voting_power &#43; new_validator_info.voting_power &lt;&#61; MAX_U128;<br/>            &#125;;<br/>            total_voting_power &#61; total_voting_power &#43; (new_validator_info.voting_power as u128);<br/>            vector::push_back(&amp;mut next_epoch_validators, new_validator_info);<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/><br/>    validator_set.active_validators &#61; next_epoch_validators;<br/>    validator_set.total_voting_power &#61; total_voting_power;<br/>    validator_set.total_joining_power &#61; 0;<br/><br/>    // Update validator indices, reset performance scores, and renew lockups.<br/>    validator_perf.validators &#61; vector::empty();<br/>    let recurring_lockup_duration_secs &#61; staking_config::get_recurring_lockup_duration(&amp;config);<br/>    let vlen &#61; vector::length(&amp;validator_set.active_validators);<br/>    let validator_index &#61; 0;<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant spec_validators_are_initialized(validator_set.active_validators);<br/>            invariant len(validator_set.pending_active) &#61;&#61; 0;<br/>            invariant len(validator_set.pending_inactive) &#61;&#61; 0;<br/>            invariant 0 &lt;&#61; validator_index &amp;&amp; validator_index &lt;&#61; vlen;<br/>            invariant vlen &#61;&#61; len(validator_set.active_validators);<br/>            invariant forall i in 0..validator_index:<br/>                global&lt;ValidatorConfig&gt;(validator_set.active_validators[i].addr).validator_index &lt; validator_index;<br/>            invariant forall i in 0..validator_index:<br/>                validator_set.active_validators[i].config.validator_index &lt; validator_index;<br/>            invariant len(validator_perf.validators) &#61;&#61; validator_index;<br/>        &#125;;<br/>        validator_index &lt; vlen<br/>    &#125;) &#123;<br/>        let validator_info &#61; vector::borrow_mut(&amp;mut validator_set.active_validators, validator_index);<br/>        validator_info.config.validator_index &#61; validator_index;<br/>        let validator_config &#61; borrow_global_mut&lt;ValidatorConfig&gt;(validator_info.addr);<br/>        validator_config.validator_index &#61; validator_index;<br/><br/>        vector::push_back(&amp;mut validator_perf.validators, IndividualValidatorPerformance &#123;<br/>            successful_proposals: 0,<br/>            failed_proposals: 0,<br/>        &#125;);<br/><br/>        // Automatically renew a validator&apos;s lockup for validators that will still be in the validator set in the<br/>        // next epoch.<br/>        let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(validator_info.addr);<br/>        let now_secs &#61; timestamp::now_seconds();<br/>        let reconfig_start_secs &#61; if (chain_status::is_operating()) &#123;<br/>            get_reconfig_start_time_secs()<br/>        &#125; else &#123;<br/>            now_secs<br/>        &#125;;<br/>        if (stake_pool.locked_until_secs &lt;&#61; reconfig_start_secs) &#123;<br/>            spec &#123;<br/>                assume now_secs &#43; recurring_lockup_duration_secs &lt;&#61; MAX_U64;<br/>            &#125;;<br/>            stake_pool.locked_until_secs &#61; now_secs &#43; recurring_lockup_duration_secs;<br/>        &#125;;<br/><br/>        validator_index &#61; validator_index &#43; 1;<br/>    &#125;;<br/><br/>    if (features::periodical_reward_rate_decrease_enabled()) &#123;<br/>        // Update rewards rate after reward distribution.<br/>        staking_config::calculate_and_save_latest_epoch_rewards_rate();<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_cur_validator_consensus_infos"></a>

## Function `cur_validator_consensus_infos`

Return the <code>ValidatorConsensusInfo</code> of each current validator, sorted by current validator index.


<pre><code>public fun cur_validator_consensus_infos(): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun cur_validator_consensus_infos(): vector&lt;ValidatorConsensusInfo&gt; acquires ValidatorSet &#123;<br/>    let validator_set &#61; borrow_global&lt;ValidatorSet&gt;(@aptos_framework);<br/>    validator_consensus_infos_from_validator_set(validator_set)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_next_validator_consensus_infos"></a>

## Function `next_validator_consensus_infos`



<pre><code>public fun next_validator_consensus_infos(): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun next_validator_consensus_infos(): vector&lt;ValidatorConsensusInfo&gt; acquires ValidatorSet, ValidatorPerformance, StakePool, ValidatorFees, ValidatorConfig &#123;<br/>    // Init.<br/>    let cur_validator_set &#61; borrow_global&lt;ValidatorSet&gt;(@aptos_framework);<br/>    let staking_config &#61; staking_config::get();<br/>    let validator_perf &#61; borrow_global&lt;ValidatorPerformance&gt;(@aptos_framework);<br/>    let (minimum_stake, _) &#61; staking_config::get_required_stake(&amp;staking_config);<br/>    let (rewards_rate, rewards_rate_denominator) &#61; staking_config::get_reward_rate(&amp;staking_config);<br/><br/>    // Compute new validator set.<br/>    let new_active_validators &#61; vector[];<br/>    let num_new_actives &#61; 0;<br/>    let candidate_idx &#61; 0;<br/>    let new_total_power &#61; 0;<br/>    let num_cur_actives &#61; vector::length(&amp;cur_validator_set.active_validators);<br/>    let num_cur_pending_actives &#61; vector::length(&amp;cur_validator_set.pending_active);<br/>    spec &#123;<br/>        assume num_cur_actives &#43; num_cur_pending_actives &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    let num_candidates &#61; num_cur_actives &#43; num_cur_pending_actives;<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant candidate_idx &lt;&#61; num_candidates;<br/>            invariant spec_validators_are_initialized(new_active_validators);<br/>            invariant len(new_active_validators) &#61;&#61; num_new_actives;<br/>            invariant forall i in 0..len(new_active_validators):<br/>                new_active_validators[i].config.validator_index &#61;&#61; i;<br/>            invariant num_new_actives &lt;&#61; candidate_idx;<br/>            invariant spec_validators_are_initialized(new_active_validators);<br/>        &#125;;<br/>        candidate_idx &lt; num_candidates<br/>    &#125;) &#123;<br/>        let candidate_in_current_validator_set &#61; candidate_idx &lt; num_cur_actives;<br/>        let candidate &#61; if (candidate_idx &lt; num_cur_actives) &#123;<br/>            vector::borrow(&amp;cur_validator_set.active_validators, candidate_idx)<br/>        &#125; else &#123;<br/>            vector::borrow(&amp;cur_validator_set.pending_active, candidate_idx &#45; num_cur_actives)<br/>        &#125;;<br/>        let stake_pool &#61; borrow_global&lt;StakePool&gt;(candidate.addr);<br/>        let cur_active &#61; coin::value(&amp;stake_pool.active);<br/>        let cur_pending_active &#61; coin::value(&amp;stake_pool.pending_active);<br/>        let cur_pending_inactive &#61; coin::value(&amp;stake_pool.pending_inactive);<br/><br/>        let cur_reward &#61; if (candidate_in_current_validator_set &amp;&amp; cur_active &gt; 0) &#123;<br/>            spec &#123;<br/>                assert candidate.config.validator_index &lt; len(validator_perf.validators);<br/>            &#125;;<br/>            let cur_perf &#61; vector::borrow(&amp;validator_perf.validators, candidate.config.validator_index);<br/>            spec &#123;<br/>                assume cur_perf.successful_proposals &#43; cur_perf.failed_proposals &lt;&#61; MAX_U64;<br/>            &#125;;<br/>            calculate_rewards_amount(cur_active, cur_perf.successful_proposals, cur_perf.successful_proposals &#43; cur_perf.failed_proposals, rewards_rate, rewards_rate_denominator)<br/>        &#125; else &#123;<br/>            0<br/>        &#125;;<br/><br/>        let cur_fee &#61; 0;<br/>        if (features::collect_and_distribute_gas_fees()) &#123;<br/>            let fees_table &#61; &amp;borrow_global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;<br/>            if (table::contains(fees_table, candidate.addr)) &#123;<br/>                let fee_coin &#61; table::borrow(fees_table, candidate.addr);<br/>                cur_fee &#61; coin::value(fee_coin);<br/>            &#125;<br/>        &#125;;<br/><br/>        let lockup_expired &#61; get_reconfig_start_time_secs() &gt;&#61; stake_pool.locked_until_secs;<br/>        spec &#123;<br/>            assume cur_active &#43; cur_pending_active &#43; cur_reward &#43; cur_fee &lt;&#61; MAX_U64;<br/>            assume cur_active &#43; cur_pending_inactive &#43; cur_pending_active &#43; cur_reward &#43; cur_fee &lt;&#61; MAX_U64;<br/>        &#125;;<br/>        let new_voting_power &#61;<br/>            cur_active<br/>            &#43; if (lockup_expired) &#123; 0 &#125; else &#123; cur_pending_inactive &#125;<br/>            &#43; cur_pending_active<br/>            &#43; cur_reward &#43; cur_fee;<br/><br/>        if (new_voting_power &gt;&#61; minimum_stake) &#123;<br/>            let config &#61; &#42;borrow_global&lt;ValidatorConfig&gt;(candidate.addr);<br/>            config.validator_index &#61; num_new_actives;<br/>            let new_validator_info &#61; ValidatorInfo &#123;<br/>                addr: candidate.addr,<br/>                voting_power: new_voting_power,<br/>                config,<br/>            &#125;;<br/><br/>            // Update ValidatorSet.<br/>            spec &#123;<br/>                assume new_total_power &#43; new_voting_power &lt;&#61; MAX_U128;<br/>            &#125;;<br/>            new_total_power &#61; new_total_power &#43; (new_voting_power as u128);<br/>            vector::push_back(&amp;mut new_active_validators, new_validator_info);<br/>            num_new_actives &#61; num_new_actives &#43; 1;<br/><br/>        &#125;;<br/>        candidate_idx &#61; candidate_idx &#43; 1;<br/>    &#125;;<br/><br/>    let new_validator_set &#61; ValidatorSet &#123;<br/>        consensus_scheme: cur_validator_set.consensus_scheme,<br/>        active_validators: new_active_validators,<br/>        pending_inactive: vector[],<br/>        pending_active: vector[],<br/>        total_voting_power: new_total_power,<br/>        total_joining_power: 0,<br/>    &#125;;<br/><br/>    validator_consensus_infos_from_validator_set(&amp;new_validator_set)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_validator_consensus_infos_from_validator_set"></a>

## Function `validator_consensus_infos_from_validator_set`



<pre><code>fun validator_consensus_infos_from_validator_set(validator_set: &amp;stake::ValidatorSet): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validator_consensus_infos_from_validator_set(validator_set: &amp;ValidatorSet): vector&lt;ValidatorConsensusInfo&gt; &#123;<br/>    let validator_consensus_infos &#61; vector[];<br/><br/>    let num_active &#61; vector::length(&amp;validator_set.active_validators);<br/>    let num_pending_inactive &#61; vector::length(&amp;validator_set.pending_inactive);<br/>    spec &#123;<br/>        assume num_active &#43; num_pending_inactive &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    let total &#61; num_active &#43; num_pending_inactive;<br/><br/>    // Pre&#45;fill the return value with dummy values.<br/>    let idx &#61; 0;<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant idx &lt;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br/>            invariant len(validator_consensus_infos) &#61;&#61; idx;<br/>            invariant len(validator_consensus_infos) &lt;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br/>        &#125;;<br/>        idx &lt; total<br/>    &#125;) &#123;<br/>        vector::push_back(&amp;mut validator_consensus_infos, validator_consensus_info::default());<br/>        idx &#61; idx &#43; 1;<br/>    &#125;;<br/>    spec &#123;<br/>        assert len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br/>        assert spec_validator_indices_are_valid_config(validator_set.active_validators,<br/>            len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));<br/>    &#125;;<br/><br/>    vector::for_each_ref(&amp;validator_set.active_validators, &#124;obj&#124; &#123;<br/>        let vi: &amp;ValidatorInfo &#61; obj;<br/>        spec &#123;<br/>            assume len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br/>            assert vi.config.validator_index &lt; len(validator_consensus_infos);<br/>        &#125;;<br/>        let vci &#61; vector::borrow_mut(&amp;mut validator_consensus_infos, vi.config.validator_index);<br/>        &#42;vci &#61; validator_consensus_info::new(<br/>            vi.addr,<br/>            vi.config.consensus_pubkey,<br/>            vi.voting_power<br/>        );<br/>        spec &#123;<br/>            assert len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br/>        &#125;;<br/>    &#125;);<br/><br/>    vector::for_each_ref(&amp;validator_set.pending_inactive, &#124;obj&#124; &#123;<br/>        let vi: &amp;ValidatorInfo &#61; obj;<br/>        spec &#123;<br/>            assume len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br/>            assert vi.config.validator_index &lt; len(validator_consensus_infos);<br/>        &#125;;<br/>        let vci &#61; vector::borrow_mut(&amp;mut validator_consensus_infos, vi.config.validator_index);<br/>        &#42;vci &#61; validator_consensus_info::new(<br/>            vi.addr,<br/>            vi.config.consensus_pubkey,<br/>            vi.voting_power<br/>        );<br/>        spec &#123;<br/>            assert len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);<br/>        &#125;;<br/>    &#125;);<br/><br/>    validator_consensus_infos<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_addresses_from_validator_infos"></a>

## Function `addresses_from_validator_infos`



<pre><code>fun addresses_from_validator_infos(infos: &amp;vector&lt;stake::ValidatorInfo&gt;): vector&lt;address&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun addresses_from_validator_infos(infos: &amp;vector&lt;ValidatorInfo&gt;): vector&lt;address&gt; &#123;<br/>    vector::map_ref(infos, &#124;obj&#124; &#123;<br/>        let info: &amp;ValidatorInfo &#61; obj;<br/>        info.addr<br/>    &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_update_stake_pool"></a>

## Function `update_stake_pool`

Calculate the stake amount of a stake pool for the next epoch.
Update individual validator's stake pool if <code>commit &#61;&#61; true</code>.

1. distribute transaction fees to active/pending_inactive delegations
2. distribute rewards to active/pending_inactive delegations
3. process pending_active, pending_inactive correspondingly
This function shouldn't abort.


<pre><code>fun update_stake_pool(validator_perf: &amp;stake::ValidatorPerformance, pool_address: address, staking_config: &amp;staking_config::StakingConfig)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_stake_pool(<br/>    validator_perf: &amp;ValidatorPerformance,<br/>    pool_address: address,<br/>    staking_config: &amp;StakingConfig,<br/>) acquires StakePool, AptosCoinCapabilities, ValidatorConfig, ValidatorFees &#123;<br/>    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);<br/>    let validator_config &#61; borrow_global&lt;ValidatorConfig&gt;(pool_address);<br/>    let cur_validator_perf &#61; vector::borrow(&amp;validator_perf.validators, validator_config.validator_index);<br/>    let num_successful_proposals &#61; cur_validator_perf.successful_proposals;<br/>    spec &#123;<br/>        // The following addition should not overflow because `num_total_proposals` cannot be larger than 86400,<br/>        // the maximum number of proposals in a day (1 proposal per second).<br/>        assume cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    let num_total_proposals &#61; cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals;<br/>    let (rewards_rate, rewards_rate_denominator) &#61; staking_config::get_reward_rate(staking_config);<br/>    let rewards_active &#61; distribute_rewards(<br/>        &amp;mut stake_pool.active,<br/>        num_successful_proposals,<br/>        num_total_proposals,<br/>        rewards_rate,<br/>        rewards_rate_denominator<br/>    );<br/>    let rewards_pending_inactive &#61; distribute_rewards(<br/>        &amp;mut stake_pool.pending_inactive,<br/>        num_successful_proposals,<br/>        num_total_proposals,<br/>        rewards_rate,<br/>        rewards_rate_denominator<br/>    );<br/>    spec &#123;<br/>        assume rewards_active &#43; rewards_pending_inactive &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    let rewards_amount &#61; rewards_active &#43; rewards_pending_inactive;<br/>    // Pending active stake can now be active.<br/>    coin::merge(&amp;mut stake_pool.active, coin::extract_all(&amp;mut stake_pool.pending_active));<br/><br/>    // Additionally, distribute transaction fees.<br/>    if (features::collect_and_distribute_gas_fees()) &#123;<br/>        let fees_table &#61; &amp;mut borrow_global_mut&lt;ValidatorFees&gt;(@aptos_framework).fees_table;<br/>        if (table::contains(fees_table, pool_address)) &#123;<br/>            let coin &#61; table::remove(fees_table, pool_address);<br/>            coin::merge(&amp;mut stake_pool.active, coin);<br/>        &#125;;<br/>    &#125;;<br/><br/>    // Pending inactive stake is only fully unlocked and moved into inactive if the current lockup cycle has expired<br/>    let current_lockup_expiration &#61; stake_pool.locked_until_secs;<br/>    if (get_reconfig_start_time_secs() &gt;&#61; current_lockup_expiration) &#123;<br/>        coin::merge(<br/>            &amp;mut stake_pool.inactive,<br/>            coin::extract_all(&amp;mut stake_pool.pending_inactive),<br/>        );<br/>    &#125;;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(DistributeRewards &#123; pool_address, rewards_amount &#125;);<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut stake_pool.distribute_rewards_events,<br/>        DistributeRewardsEvent &#123;<br/>            pool_address,<br/>            rewards_amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_reconfig_start_time_secs"></a>

## Function `get_reconfig_start_time_secs`

Assuming we are in a middle of a reconfiguration (no matter it is immediate or async), get its start time.


<pre><code>fun get_reconfig_start_time_secs(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_reconfig_start_time_secs(): u64 &#123;<br/>    if (reconfiguration_state::is_initialized()) &#123;<br/>        reconfiguration_state::start_time_secs()<br/>    &#125; else &#123;<br/>        timestamp::now_seconds()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_calculate_rewards_amount"></a>

## Function `calculate_rewards_amount`

Calculate the rewards amount.


<pre><code>fun calculate_rewards_amount(stake_amount: u64, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_rewards_amount(<br/>    stake_amount: u64,<br/>    num_successful_proposals: u64,<br/>    num_total_proposals: u64,<br/>    rewards_rate: u64,<br/>    rewards_rate_denominator: u64,<br/>): u64 &#123;<br/>    spec &#123;<br/>        // The following condition must hold because<br/>        // (1) num_successful_proposals &lt;&#61; num_total_proposals, and<br/>        // (2) `num_total_proposals` cannot be larger than 86400, the maximum number of proposals<br/>        //     in a day (1 proposal per second), and `num_total_proposals` is reset to 0 every epoch.<br/>        assume num_successful_proposals &#42; MAX_REWARDS_RATE &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    // The rewards amount is equal to (stake amount &#42; rewards rate &#42; performance multiplier).<br/>    // We do multiplication in u128 before division to avoid the overflow and minimize the rounding error.<br/>    let rewards_numerator &#61; (stake_amount as u128) &#42; (rewards_rate as u128) &#42; (num_successful_proposals as u128);<br/>    let rewards_denominator &#61; (rewards_rate_denominator as u128) &#42; (num_total_proposals as u128);<br/>    if (rewards_denominator &gt; 0) &#123;<br/>        ((rewards_numerator / rewards_denominator) as u64)<br/>    &#125; else &#123;<br/>        0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_distribute_rewards"></a>

## Function `distribute_rewards`

Mint rewards corresponding to current epoch's <code>stake</code> and <code>num_successful_votes</code>.


<pre><code>fun distribute_rewards(stake: &amp;mut coin::Coin&lt;aptos_coin::AptosCoin&gt;, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun distribute_rewards(<br/>    stake: &amp;mut Coin&lt;AptosCoin&gt;,<br/>    num_successful_proposals: u64,<br/>    num_total_proposals: u64,<br/>    rewards_rate: u64,<br/>    rewards_rate_denominator: u64,<br/>): u64 acquires AptosCoinCapabilities &#123;<br/>    let stake_amount &#61; coin::value(stake);<br/>    let rewards_amount &#61; if (stake_amount &gt; 0) &#123;<br/>        calculate_rewards_amount(<br/>            stake_amount,<br/>            num_successful_proposals,<br/>            num_total_proposals,<br/>            rewards_rate,<br/>            rewards_rate_denominator<br/>        )<br/>    &#125; else &#123;<br/>        0<br/>    &#125;;<br/>    if (rewards_amount &gt; 0) &#123;<br/>        let mint_cap &#61; &amp;borrow_global&lt;AptosCoinCapabilities&gt;(@aptos_framework).mint_cap;<br/>        let rewards &#61; coin::mint(rewards_amount, mint_cap);<br/>        coin::merge(stake, rewards);<br/>    &#125;;<br/>    rewards_amount<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_append"></a>

## Function `append`



<pre><code>fun append&lt;T&gt;(v1: &amp;mut vector&lt;T&gt;, v2: &amp;mut vector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun append&lt;T&gt;(v1: &amp;mut vector&lt;T&gt;, v2: &amp;mut vector&lt;T&gt;) &#123;<br/>    while (!vector::is_empty(v2)) &#123;<br/>        vector::push_back(v1, vector::pop_back(v2));<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_find_validator"></a>

## Function `find_validator`



<pre><code>fun find_validator(v: &amp;vector&lt;stake::ValidatorInfo&gt;, addr: address): option::Option&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun find_validator(v: &amp;vector&lt;ValidatorInfo&gt;, addr: address): Option&lt;u64&gt; &#123;<br/>    let i &#61; 0;<br/>    let len &#61; vector::length(v);<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant !(exists j in 0..i: v[j].addr &#61;&#61; addr);<br/>        &#125;;<br/>        i &lt; len<br/>    &#125;) &#123;<br/>        if (vector::borrow(v, i).addr &#61;&#61; addr) &#123;<br/>            return option::some(i)<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    option::none()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_generate_validator_info"></a>

## Function `generate_validator_info`



<pre><code>fun generate_validator_info(addr: address, stake_pool: &amp;stake::StakePool, config: stake::ValidatorConfig): stake::ValidatorInfo<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun generate_validator_info(addr: address, stake_pool: &amp;StakePool, config: ValidatorConfig): ValidatorInfo &#123;<br/>    let voting_power &#61; get_next_epoch_voting_power(stake_pool);<br/>    ValidatorInfo &#123;<br/>        addr,<br/>        voting_power,<br/>        config,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_get_next_epoch_voting_power"></a>

## Function `get_next_epoch_voting_power`

Returns validator's next epoch voting power, including pending_active, active, and pending_inactive stake.


<pre><code>fun get_next_epoch_voting_power(stake_pool: &amp;stake::StakePool): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_next_epoch_voting_power(stake_pool: &amp;StakePool): u64 &#123;<br/>    let value_pending_active &#61; coin::value(&amp;stake_pool.pending_active);<br/>    let value_active &#61; coin::value(&amp;stake_pool.active);<br/>    let value_pending_inactive &#61; coin::value(&amp;stake_pool.pending_inactive);<br/>    spec &#123;<br/>        assume value_pending_active &#43; value_active &#43; value_pending_inactive &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    value_pending_active &#43; value_active &#43; value_pending_inactive<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_update_voting_power_increase"></a>

## Function `update_voting_power_increase`



<pre><code>fun update_voting_power_increase(increase_amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_voting_power_increase(increase_amount: u64) acquires ValidatorSet &#123;<br/>    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);<br/>    let voting_power_increase_limit &#61;<br/>        (staking_config::get_voting_power_increase_limit(&amp;staking_config::get()) as u128);<br/>    validator_set.total_joining_power &#61; validator_set.total_joining_power &#43; (increase_amount as u128);<br/><br/>    // Only validator voting power increase if the current validator set&apos;s voting power &gt; 0.<br/>    if (validator_set.total_voting_power &gt; 0) &#123;<br/>        assert!(<br/>            validator_set.total_joining_power &lt;&#61; validator_set.total_voting_power &#42; voting_power_increase_limit / 100,<br/>            error::invalid_argument(EVOTING_POWER_INCREASE_EXCEEDS_LIMIT),<br/>        );<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_assert_stake_pool_exists"></a>

## Function `assert_stake_pool_exists`



<pre><code>fun assert_stake_pool_exists(pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_stake_pool_exists(pool_address: address) &#123;<br/>    assert!(stake_pool_exists(pool_address), error::invalid_argument(ESTAKE_POOL_DOES_NOT_EXIST));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_configure_allowed_validators"></a>

## Function `configure_allowed_validators`



<pre><code>public fun configure_allowed_validators(aptos_framework: &amp;signer, accounts: vector&lt;address&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun configure_allowed_validators(<br/>    aptos_framework: &amp;signer,<br/>    accounts: vector&lt;address&gt;<br/>) acquires AllowedValidators &#123;<br/>    let aptos_framework_address &#61; signer::address_of(aptos_framework);<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    if (!exists&lt;AllowedValidators&gt;(aptos_framework_address)) &#123;<br/>        move_to(aptos_framework, AllowedValidators &#123; accounts &#125;);<br/>    &#125; else &#123;<br/>        let allowed &#61; borrow_global_mut&lt;AllowedValidators&gt;(aptos_framework_address);<br/>        allowed.accounts &#61; accounts;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_is_allowed"></a>

## Function `is_allowed`



<pre><code>fun is_allowed(account: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_allowed(account: address): bool acquires AllowedValidators &#123;<br/>    if (!exists&lt;AllowedValidators&gt;(@aptos_framework)) &#123;<br/>        true<br/>    &#125; else &#123;<br/>        let allowed &#61; borrow_global&lt;AllowedValidators&gt;(@aptos_framework);<br/>        vector::contains(&amp;allowed.accounts, &amp;account)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_assert_owner_cap_exists"></a>

## Function `assert_owner_cap_exists`



<pre><code>fun assert_owner_cap_exists(owner: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_owner_cap_exists(owner: address) &#123;<br/>    assert!(exists&lt;OwnerCapability&gt;(owner), error::not_found(EOWNER_CAP_NOT_FOUND));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_stake_assert_reconfig_not_in_progress"></a>

## Function `assert_reconfig_not_in_progress`



<pre><code>fun assert_reconfig_not_in_progress()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_reconfig_not_in_progress() &#123;<br/>    assert!(!reconfiguration_state::is_in_progress(), error::invalid_state(ERECONFIGURATION_IN_PROGRESS));<br/>&#125;<br/></code></pre>



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


<pre><code>pragma verify &#61; true;<br/>invariant [suspendable] exists&lt;ValidatorSet&gt;(@aptos_framework) &#61;&#61;&gt; validator_set_is_valid();<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;ValidatorPerformance&gt;(@aptos_framework);<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;ValidatorSet&gt;(@aptos_framework);<br/>apply ValidatorOwnerNoChange to &#42;;<br/>apply ValidatorNotChangeDuringReconfig to &#42; except on_new_epoch;<br/>apply StakePoolNotChangeDuringReconfig to &#42; except on_new_epoch, update_stake_pool;<br/><a id="0x1_stake_ghost_valid_perf"></a>
global ghost_valid_perf: ValidatorPerformance;<br/><a id="0x1_stake_ghost_proposer_idx"></a>
global ghost_proposer_idx: Option&lt;u64&gt;;<br/><a id="0x1_stake_ghost_active_num"></a>
global ghost_active_num: u64;<br/><a id="0x1_stake_ghost_pending_inactive_num"></a>
global ghost_pending_inactive_num: u64;<br/></code></pre>



<a id="@Specification_1_ValidatorSet"></a>

### Resource `ValidatorSet`


<pre><code>struct ValidatorSet has copy, drop, store, key<br/></code></pre>



<dl>
<dt>
<code>consensus_scheme: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>active_validators: vector&lt;stake::ValidatorInfo&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive: vector&lt;stake::ValidatorInfo&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_active: vector&lt;stake::ValidatorInfo&gt;</code>
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
invariant consensus_scheme &#61;&#61; 0;<br/></code></pre>




<a id="0x1_stake_ValidatorNotChangeDuringReconfig"></a>


<pre><code>schema ValidatorNotChangeDuringReconfig &#123;<br/>ensures (reconfiguration_state::spec_is_in_progress() &amp;&amp; old(exists&lt;ValidatorSet&gt;(@aptos_framework))) &#61;&#61;&gt;<br/>    old(global&lt;ValidatorSet&gt;(@aptos_framework)) &#61;&#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>&#125;<br/></code></pre>




<a id="0x1_stake_StakePoolNotChangeDuringReconfig"></a>


<pre><code>schema StakePoolNotChangeDuringReconfig &#123;<br/>ensures forall a: address where old(exists&lt;StakePool&gt;(a)): reconfiguration_state::spec_is_in_progress() &#61;&#61;&gt;<br/>    (old(global&lt;StakePool&gt;(a).pending_inactive) &#61;&#61; global&lt;StakePool&gt;(a).pending_inactive &amp;&amp;<br/>    old(global&lt;StakePool&gt;(a).pending_active) &#61;&#61; global&lt;StakePool&gt;(a).pending_active &amp;&amp;<br/>    old(global&lt;StakePool&gt;(a).inactive) &#61;&#61; global&lt;StakePool&gt;(a).inactive &amp;&amp;<br/>    old(global&lt;StakePool&gt;(a).active) &#61;&#61; global&lt;StakePool&gt;(a).active);<br/>&#125;<br/></code></pre>




<a id="0x1_stake_ValidatorOwnerNoChange"></a>


<pre><code>schema ValidatorOwnerNoChange &#123;<br/>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
    ensures forall addr: address where old(exists&lt;OwnerCapability&gt;(addr)):<br/>    old(global&lt;OwnerCapability&gt;(addr)).pool_address &#61;&#61; global&lt;OwnerCapability&gt;(addr).pool_address;<br/>&#125;<br/></code></pre>




<a id="0x1_stake_StakedValueNochange"></a>


<pre><code>schema StakedValueNochange &#123;<br/>pool_address: address;<br/>let stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    ensures stake_pool.active.value &#43; stake_pool.inactive.value &#43; stake_pool.pending_active.value &#43; stake_pool.pending_inactive.value &#61;&#61;<br/>    post_stake_pool.active.value &#43; post_stake_pool.inactive.value &#43; post_stake_pool.pending_active.value &#43; post_stake_pool.pending_inactive.value;<br/>&#125;<br/></code></pre>




<a id="0x1_stake_validator_set_is_valid"></a>


<pre><code>fun validator_set_is_valid(): bool &#123;<br/>   let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>   validator_set_is_valid_impl(validator_set)<br/>&#125;<br/></code></pre>




<a id="0x1_stake_validator_set_is_valid_impl"></a>


<pre><code>fun validator_set_is_valid_impl(validator_set: ValidatorSet): bool &#123;<br/>   spec_validators_are_initialized(validator_set.active_validators) &amp;&amp;<br/>       spec_validators_are_initialized(validator_set.pending_inactive) &amp;&amp;<br/>       spec_validators_are_initialized(validator_set.pending_active) &amp;&amp;<br/>       spec_validator_indices_are_valid(validator_set.active_validators) &amp;&amp;<br/>       spec_validator_indices_are_valid(validator_set.pending_inactive)<br/>       &amp;&amp; spec_validator_indices_active_pending_inactive(validator_set)<br/>&#125;<br/></code></pre>



<a id="@Specification_1_initialize_validator_fees"></a>

### Function `initialize_validator_fees`


<pre><code>public(friend) fun initialize_validator_fees(aptos_framework: &amp;signer)<br/></code></pre>




<pre><code>let aptos_addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);<br/>aborts_if exists&lt;ValidatorFees&gt;(aptos_addr);<br/>ensures exists&lt;ValidatorFees&gt;(aptos_addr);<br/></code></pre>



<a id="@Specification_1_add_transaction_fee"></a>

### Function `add_transaction_fee`


<pre><code>public(friend) fun add_transaction_fee(validator_addr: address, fee: coin::Coin&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>




<pre><code>aborts_if !exists&lt;ValidatorFees&gt;(@aptos_framework);<br/>let fees_table &#61; global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;<br/>let post post_fees_table &#61; global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;<br/>let collected_fee &#61; table::spec_get(fees_table, validator_addr);<br/>let post post_collected_fee &#61; table::spec_get(post_fees_table, validator_addr);<br/>ensures if (table::spec_contains(fees_table, validator_addr)) &#123;<br/>    post_collected_fee.value &#61;&#61; collected_fee.value &#43; fee.value<br/>&#125; else &#123;<br/>    table::spec_contains(post_fees_table, validator_addr) &amp;&amp;<br/>    table::spec_get(post_fees_table, validator_addr) &#61;&#61; fee<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_get_validator_state"></a>

### Function `get_validator_state`


<pre><code>&#35;[view]<br/>public fun get_validator_state(pool_address: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);<br/>let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>ensures result &#61;&#61; VALIDATOR_STATUS_PENDING_ACTIVE &#61;&#61;&gt; spec_contains(validator_set.pending_active, pool_address);<br/>ensures result &#61;&#61; VALIDATOR_STATUS_ACTIVE &#61;&#61;&gt; spec_contains(validator_set.active_validators, pool_address);<br/>ensures result &#61;&#61; VALIDATOR_STATUS_PENDING_INACTIVE &#61;&#61;&gt; spec_contains(validator_set.pending_inactive, pool_address);<br/>ensures result &#61;&#61; VALIDATOR_STATUS_INACTIVE &#61;&#61;&gt; (<br/>    !spec_contains(validator_set.pending_active, pool_address)<br/>        &amp;&amp; !spec_contains(validator_set.active_validators, pool_address)<br/>        &amp;&amp; !spec_contains(validator_set.pending_inactive, pool_address)<br/>);<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)<br/></code></pre>




<pre><code>pragma disable_invariants_in_body;<br/>let aptos_addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);<br/>aborts_if exists&lt;ValidatorSet&gt;(aptos_addr);<br/>aborts_if exists&lt;ValidatorPerformance&gt;(aptos_addr);<br/>ensures exists&lt;ValidatorSet&gt;(aptos_addr);<br/>ensures global&lt;ValidatorSet&gt;(aptos_addr).consensus_scheme &#61;&#61; 0;<br/>ensures exists&lt;ValidatorPerformance&gt;(aptos_addr);<br/></code></pre>



<a id="@Specification_1_remove_validators"></a>

### Function `remove_validators`


<pre><code>public fun remove_validators(aptos_framework: &amp;signer, validators: &amp;vector&lt;address&gt;)<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let post post_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let active_validators &#61; validator_set.active_validators;<br/>let post post_active_validators &#61; post_validator_set.active_validators;<br/>let pending_inactive_validators &#61; validator_set.pending_inactive;<br/>let post post_pending_inactive_validators &#61; post_validator_set.pending_inactive;<br/>invariant len(active_validators) &gt; 0;<br/>ensures len(active_validators) &#43; len(pending_inactive_validators) &#61;&#61; len(post_active_validators)<br/>    &#43; len(post_pending_inactive_validators);<br/></code></pre>



<a id="@Specification_1_initialize_stake_owner"></a>

### Function `initialize_stake_owner`


<pre><code>public entry fun initialize_stake_owner(owner: &amp;signer, initial_stake_amount: u64, operator: address, voter: address)<br/></code></pre>




<pre><code>include ResourceRequirement;<br/>let addr &#61; signer::address_of(owner);<br/>ensures global&lt;ValidatorConfig&gt;(addr) &#61;&#61; ValidatorConfig &#123;<br/>    consensus_pubkey: vector::empty(),<br/>    network_addresses: vector::empty(),<br/>    fullnode_addresses: vector::empty(),<br/>    validator_index: 0,<br/>&#125;;<br/>ensures global&lt;OwnerCapability&gt;(addr) &#61;&#61; OwnerCapability &#123; pool_address: addr &#125;;<br/>let post stakepool &#61; global&lt;StakePool&gt;(addr);<br/>let post active &#61; stakepool.active.value;<br/>let post pending_active &#61; stakepool.pending_active.value;<br/>ensures spec_is_current_epoch_validator(addr) &#61;&#61;&gt;<br/>    pending_active &#61;&#61; initial_stake_amount;<br/>ensures !spec_is_current_epoch_validator(addr) &#61;&#61;&gt;<br/>    active &#61;&#61; initial_stake_amount;<br/></code></pre>



<a id="@Specification_1_initialize_validator"></a>

### Function `initialize_validator`


<pre><code>public entry fun initialize_validator(account: &amp;signer, consensus_pubkey: vector&lt;u8&gt;, proof_of_possession: vector&lt;u8&gt;, network_addresses: vector&lt;u8&gt;, fullnode_addresses: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>let pubkey_from_pop &#61; bls12381::spec_public_key_from_bytes_with_pop(<br/>    consensus_pubkey,<br/>    proof_of_possession_from_bytes(proof_of_possession)<br/>);<br/>aborts_if !option::spec_is_some(pubkey_from_pop);<br/>let addr &#61; signer::address_of(account);<br/>let post_addr &#61; signer::address_of(account);<br/>let allowed &#61; global&lt;AllowedValidators&gt;(@aptos_framework);<br/>aborts_if exists&lt;ValidatorConfig&gt;(addr);<br/>aborts_if exists&lt;AllowedValidators&gt;(@aptos_framework) &amp;&amp; !vector::spec_contains(allowed.accounts, addr);<br/>aborts_if stake_pool_exists(addr);<br/>aborts_if exists&lt;OwnerCapability&gt;(addr);<br/>aborts_if !exists&lt;account::Account&gt;(addr);<br/>aborts_if global&lt;account::Account&gt;(addr).guid_creation_num &#43; 12 &gt; MAX_U64;<br/>aborts_if global&lt;account::Account&gt;(addr).guid_creation_num &#43; 12 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>ensures exists&lt;StakePool&gt;(post_addr);<br/>ensures global&lt;OwnerCapability&gt;(post_addr) &#61;&#61; OwnerCapability &#123; pool_address: post_addr &#125;;<br/>ensures global&lt;ValidatorConfig&gt;(post_addr) &#61;&#61; ValidatorConfig &#123;<br/>    consensus_pubkey,<br/>    network_addresses,<br/>    fullnode_addresses,<br/>    validator_index: 0,<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_extract_owner_cap"></a>

### Function `extract_owner_cap`


<pre><code>public fun extract_owner_cap(owner: &amp;signer): stake::OwnerCapability<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>let owner_address &#61; signer::address_of(owner);<br/>aborts_if !exists&lt;OwnerCapability&gt;(owner_address);<br/>ensures !exists&lt;OwnerCapability&gt;(owner_address);<br/></code></pre>



<a id="@Specification_1_deposit_owner_cap"></a>

### Function `deposit_owner_cap`


<pre><code>public fun deposit_owner_cap(owner: &amp;signer, owner_cap: stake::OwnerCapability)<br/></code></pre>




<pre><code>let owner_address &#61; signer::address_of(owner);<br/>aborts_if exists&lt;OwnerCapability&gt;(owner_address);<br/>ensures exists&lt;OwnerCapability&gt;(owner_address);<br/>ensures global&lt;OwnerCapability&gt;(owner_address) &#61;&#61; owner_cap;<br/></code></pre>



<a id="@Specification_1_set_operator_with_cap"></a>

### Function `set_operator_with_cap`


<pre><code>public fun set_operator_with_cap(owner_cap: &amp;stake::OwnerCapability, new_operator: address)<br/></code></pre>




<pre><code>let pool_address &#61; owner_cap.pool_address;<br/>let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>modifies global&lt;StakePool&gt;(pool_address);<br/>include StakedValueNochange;<br/>ensures post_stake_pool.operator_address &#61;&#61; new_operator;<br/></code></pre>



<a id="@Specification_1_set_delegated_voter_with_cap"></a>

### Function `set_delegated_voter_with_cap`


<pre><code>public fun set_delegated_voter_with_cap(owner_cap: &amp;stake::OwnerCapability, new_voter: address)<br/></code></pre>




<pre><code>let pool_address &#61; owner_cap.pool_address;<br/>let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>include StakedValueNochange;<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>modifies global&lt;StakePool&gt;(pool_address);<br/>ensures post_stake_pool.delegated_voter &#61;&#61; new_voter;<br/></code></pre>



<a id="@Specification_1_add_stake"></a>

### Function `add_stake`


<pre><code>public entry fun add_stake(owner: &amp;signer, amount: u64)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>pragma aborts_if_is_partial;<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>include ResourceRequirement;<br/>include AddStakeAbortsIfAndEnsures;<br/></code></pre>



<a id="@Specification_1_add_stake_with_cap"></a>

### Function `add_stake_with_cap`


<pre><code>public fun add_stake_with_cap(owner_cap: &amp;stake::OwnerCapability, coins: coin::Coin&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>




<pre><code>pragma disable_invariants_in_body;<br/>pragma verify_duration_estimate &#61; 300;<br/>include ResourceRequirement;<br/>let amount &#61; coins.value;<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>include AddStakeWithCapAbortsIfAndEnsures &#123; amount &#125;;<br/></code></pre>



<a id="@Specification_1_reactivate_stake_with_cap"></a>

### Function `reactivate_stake_with_cap`


<pre><code>public fun reactivate_stake_with_cap(owner_cap: &amp;stake::OwnerCapability, amount: u64)<br/></code></pre>




<pre><code>let pool_address &#61; owner_cap.pool_address;<br/>include StakedValueNochange;<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>aborts_if !stake_pool_exists(pool_address);<br/>let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let post stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>modifies global&lt;StakePool&gt;(pool_address);<br/>let min_amount &#61; aptos_std::math64::min(amount, pre_stake_pool.pending_inactive.value);<br/>ensures stake_pool.pending_inactive.value &#61;&#61; pre_stake_pool.pending_inactive.value &#45; min_amount;<br/>ensures stake_pool.active.value &#61;&#61; pre_stake_pool.active.value &#43; min_amount;<br/></code></pre>



<a id="@Specification_1_rotate_consensus_key"></a>

### Function `rotate_consensus_key`


<pre><code>public entry fun rotate_consensus_key(operator: &amp;signer, pool_address: address, new_consensus_pubkey: vector&lt;u8&gt;, proof_of_possession: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let post validator_info &#61; global&lt;ValidatorConfig&gt;(pool_address);<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>aborts_if signer::address_of(operator) !&#61; pre_stake_pool.operator_address;<br/>aborts_if !exists&lt;ValidatorConfig&gt;(pool_address);<br/>let pubkey_from_pop &#61; bls12381::spec_public_key_from_bytes_with_pop(<br/>    new_consensus_pubkey,<br/>    proof_of_possession_from_bytes(proof_of_possession)<br/>);<br/>aborts_if !option::spec_is_some(pubkey_from_pop);<br/>modifies global&lt;ValidatorConfig&gt;(pool_address);<br/>include StakedValueNochange;<br/>ensures validator_info.consensus_pubkey &#61;&#61; new_consensus_pubkey;<br/></code></pre>



<a id="@Specification_1_update_network_and_fullnode_addresses"></a>

### Function `update_network_and_fullnode_addresses`


<pre><code>public entry fun update_network_and_fullnode_addresses(operator: &amp;signer, pool_address: address, new_network_addresses: vector&lt;u8&gt;, new_fullnode_addresses: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let post validator_info &#61; global&lt;ValidatorConfig&gt;(pool_address);<br/>modifies global&lt;ValidatorConfig&gt;(pool_address);<br/>include StakedValueNochange;<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>aborts_if !exists&lt;ValidatorConfig&gt;(pool_address);<br/>aborts_if signer::address_of(operator) !&#61; pre_stake_pool.operator_address;<br/>ensures validator_info.network_addresses &#61;&#61; new_network_addresses;<br/>ensures validator_info.fullnode_addresses &#61;&#61; new_fullnode_addresses;<br/></code></pre>



<a id="@Specification_1_increase_lockup_with_cap"></a>

### Function `increase_lockup_with_cap`


<pre><code>public fun increase_lockup_with_cap(owner_cap: &amp;stake::OwnerCapability)<br/></code></pre>




<pre><code>let config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>let pool_address &#61; owner_cap.pool_address;<br/>let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let post stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let now_seconds &#61; timestamp::spec_now_seconds();<br/>let lockup &#61; config.recurring_lockup_duration_secs;<br/>modifies global&lt;StakePool&gt;(pool_address);<br/>include StakedValueNochange;<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>aborts_if pre_stake_pool.locked_until_secs &gt;&#61; lockup &#43; now_seconds;<br/>aborts_if lockup &#43; now_seconds &gt; MAX_U64;<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>ensures stake_pool.locked_until_secs &#61;&#61; lockup &#43; now_seconds;<br/></code></pre>



<a id="@Specification_1_join_validator_set"></a>

### Function `join_validator_set`


<pre><code>public entry fun join_validator_set(operator: &amp;signer, pool_address: address)<br/></code></pre>




<pre><code>pragma disable_invariants_in_body;<br/>aborts_if !staking_config::get_allow_validator_set_change(staking_config::get());<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>aborts_if !exists&lt;ValidatorConfig&gt;(pool_address);<br/>aborts_if !exists&lt;StakingConfig&gt;(@aptos_framework);<br/>aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>let stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let post p_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>aborts_if signer::address_of(operator) !&#61; stake_pool.operator_address;<br/>aborts_if option::spec_is_some(spec_find_validator(validator_set.active_validators, pool_address)) &#124;&#124;<br/>            option::spec_is_some(spec_find_validator(validator_set.pending_inactive, pool_address)) &#124;&#124;<br/>                option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));<br/>let config &#61; staking_config::get();<br/>let voting_power &#61; get_next_epoch_voting_power(stake_pool);<br/>let minimum_stake &#61; config.minimum_stake;<br/>let maximum_stake &#61; config.maximum_stake;<br/>aborts_if voting_power &lt; minimum_stake;<br/>aborts_if voting_power &gt;maximum_stake;<br/>let validator_config &#61; global&lt;ValidatorConfig&gt;(pool_address);<br/>aborts_if vector::is_empty(validator_config.consensus_pubkey);<br/>let validator_set_size &#61; vector::length(validator_set.active_validators) &#43; vector::length(validator_set.pending_active) &#43; 1;<br/>aborts_if validator_set_size &gt; MAX_VALIDATOR_SET_SIZE;<br/>let voting_power_increase_limit &#61; (staking_config::get_voting_power_increase_limit(config) as u128);<br/>aborts_if (validator_set.total_joining_power &#43; (voting_power as u128)) &gt; MAX_U128;<br/>aborts_if validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;<br/>aborts_if validator_set.total_voting_power &gt; 0 &amp;&amp;<br/>    (validator_set.total_joining_power &#43; (voting_power as u128)) &#42; 100 &gt; validator_set.total_voting_power &#42; voting_power_increase_limit;<br/>let post p_validator_info &#61; ValidatorInfo &#123;<br/>    addr: pool_address,<br/>    voting_power,<br/>    config: validator_config,<br/>&#125;;<br/>ensures validator_set.total_joining_power &#43; voting_power &#61;&#61; p_validator_set.total_joining_power;<br/>ensures vector::spec_contains(p_validator_set.pending_active, p_validator_info);<br/></code></pre>



<a id="@Specification_1_unlock_with_cap"></a>

### Function `unlock_with_cap`


<pre><code>public fun unlock_with_cap(amount: u64, owner_cap: &amp;stake::OwnerCapability)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>let pool_address &#61; owner_cap.pool_address;<br/>let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let post stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>aborts_if amount !&#61; 0 &amp;&amp; !exists&lt;StakePool&gt;(pool_address);<br/>modifies global&lt;StakePool&gt;(pool_address);<br/>include StakedValueNochange;<br/>let min_amount &#61; aptos_std::math64::min(amount,pre_stake_pool.active.value);<br/>ensures stake_pool.active.value &#61;&#61; pre_stake_pool.active.value &#45; min_amount;<br/>ensures stake_pool.pending_inactive.value &#61;&#61; pre_stake_pool.pending_inactive.value &#43; min_amount;<br/></code></pre>



<a id="@Specification_1_withdraw"></a>

### Function `withdraw`


<pre><code>public entry fun withdraw(owner: &amp;signer, withdraw_amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>let addr &#61; signer::address_of(owner);<br/>let ownership_cap &#61; global&lt;OwnerCapability&gt;(addr);<br/>let pool_address &#61; ownership_cap.pool_address;<br/>let stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>aborts_if !exists&lt;OwnerCapability&gt;(addr);<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);<br/>let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let bool_find_validator &#61; !option::spec_is_some(spec_find_validator(validator_set.active_validators, pool_address)) &amp;&amp;<br/>            !option::spec_is_some(spec_find_validator(validator_set.pending_inactive, pool_address)) &amp;&amp;<br/>                !option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));<br/>aborts_if bool_find_validator &amp;&amp; !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>let new_withdraw_amount_1 &#61; min(withdraw_amount, stake_pool.inactive.value &#43; stake_pool.pending_inactive.value);<br/>let new_withdraw_amount_2 &#61; min(withdraw_amount, stake_pool.inactive.value);<br/>aborts_if bool_find_validator &amp;&amp; timestamp::now_seconds() &gt; stake_pool.locked_until_secs &amp;&amp;<br/>            new_withdraw_amount_1 &gt; 0 &amp;&amp; stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &lt; new_withdraw_amount_1;<br/>aborts_if !(bool_find_validator &amp;&amp; exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework)) &amp;&amp;<br/>            new_withdraw_amount_2 &gt; 0 &amp;&amp; stake_pool.inactive.value &lt; new_withdraw_amount_2;<br/>aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr);<br/>include coin::DepositAbortsIf&lt;AptosCoin&gt;&#123;account_addr: addr&#125;;<br/>let coin_store &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr);<br/>let post p_coin_store &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr);<br/>ensures bool_find_validator &amp;&amp; timestamp::now_seconds() &gt; stake_pool.locked_until_secs<br/>            &amp;&amp; exists&lt;account::Account&gt;(addr) &amp;&amp; exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr) &#61;&#61;&gt;<br/>                coin_store.coin.value &#43; new_withdraw_amount_1 &#61;&#61; p_coin_store.coin.value;<br/>ensures !(bool_find_validator &amp;&amp; exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework))<br/>            &amp;&amp; exists&lt;account::Account&gt;(addr) &amp;&amp; exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr) &#61;&#61;&gt;<br/>                coin_store.coin.value &#43; new_withdraw_amount_2 &#61;&#61; p_coin_store.coin.value;<br/></code></pre>



<a id="@Specification_1_leave_validator_set"></a>

### Function `leave_validator_set`


<pre><code>public entry fun leave_validator_set(operator: &amp;signer, pool_address: address)<br/></code></pre>




<pre><code>pragma disable_invariants_in_body;<br/>requires chain_status::is_operating();<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>let config &#61; staking_config::get();<br/>aborts_if !staking_config::get_allow_validator_set_change(config);<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);<br/>aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>let stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>aborts_if signer::address_of(operator) !&#61; stake_pool.operator_address;<br/>let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let validator_find_bool &#61; option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));<br/>let active_validators &#61; validator_set.active_validators;<br/>let pending_active &#61; validator_set.pending_active;<br/>let post post_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let post post_active_validators &#61; post_validator_set.active_validators;<br/>let pending_inactive_validators &#61; validator_set.pending_inactive;<br/>let post post_pending_inactive_validators &#61; post_validator_set.pending_inactive;<br/>ensures len(active_validators) &#43; len(pending_inactive_validators) &#61;&#61; len(post_active_validators)<br/>    &#43; len(post_pending_inactive_validators);<br/>aborts_if !validator_find_bool &amp;&amp; !option::spec_is_some(spec_find_validator(active_validators, pool_address));<br/>aborts_if !validator_find_bool &amp;&amp; vector::length(validator_set.active_validators) &lt;&#61; option::spec_borrow(spec_find_validator(active_validators, pool_address));<br/>aborts_if !validator_find_bool &amp;&amp; vector::length(validator_set.active_validators) &lt; 2;<br/>aborts_if validator_find_bool &amp;&amp; vector::length(validator_set.pending_active) &lt;&#61; option::spec_borrow(spec_find_validator(pending_active, pool_address));<br/>let post p_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let validator_stake &#61; (get_next_epoch_voting_power(stake_pool) as u128);<br/>ensures validator_find_bool &amp;&amp; validator_set.total_joining_power &gt; validator_stake &#61;&#61;&gt;<br/>            p_validator_set.total_joining_power &#61;&#61; validator_set.total_joining_power &#45; validator_stake;<br/>ensures !validator_find_bool &#61;&#61;&gt; !option::spec_is_some(spec_find_validator(p_validator_set.pending_active, pool_address));<br/></code></pre>



<a id="@Specification_1_is_current_epoch_validator"></a>

### Function `is_current_epoch_validator`


<pre><code>public fun is_current_epoch_validator(pool_address: address): bool<br/></code></pre>




<pre><code>include ResourceRequirement;<br/>aborts_if !spec_has_stake_pool(pool_address);<br/>ensures result &#61;&#61; spec_is_current_epoch_validator(pool_address);<br/></code></pre>



<a id="@Specification_1_update_performance_statistics"></a>

### Function `update_performance_statistics`


<pre><code>public(friend) fun update_performance_statistics(proposer_index: option::Option&lt;u64&gt;, failed_proposer_indices: vector&lt;u64&gt;)<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>aborts_if false;<br/>let validator_perf &#61; global&lt;ValidatorPerformance&gt;(@aptos_framework);<br/>let post post_validator_perf &#61; global&lt;ValidatorPerformance&gt;(@aptos_framework);<br/>let validator_len &#61; len(validator_perf.validators);<br/>ensures (option::spec_is_some(ghost_proposer_idx) &amp;&amp; option::spec_borrow(ghost_proposer_idx) &lt; validator_len) &#61;&#61;&gt;<br/>    (post_validator_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals &#61;&#61;<br/>        validator_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals &#43; 1);<br/></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch()<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>pragma disable_invariants_in_body;<br/>include ResourceRequirement;<br/>include GetReconfigStartTimeRequirement;<br/>include staking_config::StakingRewardsConfigRequirement;<br/>include aptos_framework::aptos_coin::ExistsAptosCoin;<br/>// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_next_validator_consensus_infos"></a>

### Function `next_validator_consensus_infos`


<pre><code>public fun next_validator_consensus_infos(): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>aborts_if false;<br/>include ResourceRequirement;<br/>include GetReconfigStartTimeRequirement;<br/>include features::spec_periodical_reward_rate_decrease_enabled() &#61;&#61;&gt; staking_config::StakingRewardsConfigEnabledRequirement;<br/></code></pre>



<a id="@Specification_1_validator_consensus_infos_from_validator_set"></a>

### Function `validator_consensus_infos_from_validator_set`


<pre><code>fun validator_consensus_infos_from_validator_set(validator_set: &amp;stake::ValidatorSet): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;<br/></code></pre>




<pre><code>aborts_if false;<br/>invariant spec_validator_indices_are_valid_config(validator_set.active_validators,<br/>    len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));<br/>invariant len(validator_set.pending_inactive) &#61;&#61; 0 &#124;&#124;<br/>    spec_validator_indices_are_valid_config(validator_set.pending_inactive,<br/>        len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));<br/></code></pre>




<a id="0x1_stake_AddStakeWithCapAbortsIfAndEnsures"></a>


<pre><code>schema AddStakeWithCapAbortsIfAndEnsures &#123;<br/>owner_cap: OwnerCapability;<br/>amount: u64;<br/>let pool_address &#61; owner_cap.pool_address;<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>let config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let voting_power_increase_limit &#61; config.voting_power_increase_limit;<br/>let post post_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>let update_voting_power_increase &#61; amount !&#61; 0 &amp;&amp; (spec_contains(validator_set.active_validators, pool_address)<br/>                                                   &#124;&#124; spec_contains(validator_set.pending_active, pool_address));<br/>aborts_if update_voting_power_increase &amp;&amp; validator_set.total_joining_power &#43; amount &gt; MAX_U128;<br/>ensures update_voting_power_increase &#61;&#61;&gt; post_validator_set.total_joining_power &#61;&#61; validator_set.total_joining_power &#43; amount;<br/>aborts_if update_voting_power_increase &amp;&amp; validator_set.total_voting_power &gt; 0<br/>        &amp;&amp; validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;<br/>aborts_if update_voting_power_increase &amp;&amp; validator_set.total_voting_power &gt; 0<br/>        &amp;&amp; validator_set.total_joining_power &#43; amount &gt; validator_set.total_voting_power &#42; voting_power_increase_limit / 100;<br/>let stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let value_pending_active &#61; stake_pool.pending_active.value;<br/>let value_active &#61; stake_pool.active.value;<br/>ensures amount !&#61; 0 &amp;&amp; spec_is_current_epoch_validator(pool_address) &#61;&#61;&gt; post_stake_pool.pending_active.value &#61;&#61; value_pending_active &#43; amount;<br/>ensures amount !&#61; 0 &amp;&amp; !spec_is_current_epoch_validator(pool_address) &#61;&#61;&gt; post_stake_pool.active.value &#61;&#61; value_active &#43; amount;<br/>let maximum_stake &#61; config.maximum_stake;<br/>let value_pending_inactive &#61; stake_pool.pending_inactive.value;<br/>let next_epoch_voting_power &#61; value_pending_active &#43; value_active &#43; value_pending_inactive;<br/>let voting_power &#61; next_epoch_voting_power &#43; amount;<br/>aborts_if amount !&#61; 0 &amp;&amp; voting_power &gt; MAX_U64;<br/>aborts_if amount !&#61; 0 &amp;&amp; voting_power &gt; maximum_stake;<br/>&#125;<br/></code></pre>




<a id="0x1_stake_AddStakeAbortsIfAndEnsures"></a>


<pre><code>schema AddStakeAbortsIfAndEnsures &#123;<br/>owner: signer;<br/>amount: u64;<br/>let owner_address &#61; signer::address_of(owner);<br/>aborts_if !exists&lt;OwnerCapability&gt;(owner_address);<br/>let owner_cap &#61; global&lt;OwnerCapability&gt;(owner_address);<br/>include AddStakeWithCapAbortsIfAndEnsures &#123; owner_cap &#125;;<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_is_allowed"></a>


<pre><code>fun spec_is_allowed(account: address): bool &#123;<br/>   if (!exists&lt;AllowedValidators&gt;(@aptos_framework)) &#123;<br/>       true<br/>   &#125; else &#123;<br/>       let allowed &#61; global&lt;AllowedValidators&gt;(@aptos_framework);<br/>       contains(allowed.accounts, account)<br/>   &#125;<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_find_validator"></a>


<pre><code>fun spec_find_validator(v: vector&lt;ValidatorInfo&gt;, addr: address): Option&lt;u64&gt;;<br/></code></pre>




<a id="0x1_stake_spec_validators_are_initialized"></a>


<pre><code>fun spec_validators_are_initialized(validators: vector&lt;ValidatorInfo&gt;): bool &#123;<br/>   forall i in 0..len(validators):<br/>       spec_has_stake_pool(validators[i].addr) &amp;&amp;<br/>           spec_has_validator_config(validators[i].addr)<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_validators_are_initialized_addrs"></a>


<pre><code>fun spec_validators_are_initialized_addrs(addrs: vector&lt;address&gt;): bool &#123;<br/>   forall i in 0..len(addrs):<br/>       spec_has_stake_pool(addrs[i]) &amp;&amp;<br/>           spec_has_validator_config(addrs[i])<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid"></a>


<pre><code>fun spec_validator_indices_are_valid(validators: vector&lt;ValidatorInfo&gt;): bool &#123;<br/>   spec_validator_indices_are_valid_addr(validators, spec_validator_index_upper_bound()) &amp;&amp;<br/>       spec_validator_indices_are_valid_config(validators, spec_validator_index_upper_bound())<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid_addr"></a>


<pre><code>fun spec_validator_indices_are_valid_addr(validators: vector&lt;ValidatorInfo&gt;, upper_bound: u64): bool &#123;<br/>   forall i in 0..len(validators):<br/>       global&lt;ValidatorConfig&gt;(validators[i].addr).validator_index &lt; upper_bound<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid_config"></a>


<pre><code>fun spec_validator_indices_are_valid_config(validators: vector&lt;ValidatorInfo&gt;, upper_bound: u64): bool &#123;<br/>   forall i in 0..len(validators):<br/>       validators[i].config.validator_index &lt; upper_bound<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_validator_indices_active_pending_inactive"></a>


<pre><code>fun spec_validator_indices_active_pending_inactive(validator_set: ValidatorSet): bool &#123;<br/>   len(validator_set.pending_inactive) &#43; len(validator_set.active_validators) &#61;&#61; spec_validator_index_upper_bound()<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_validator_index_upper_bound"></a>


<pre><code>fun spec_validator_index_upper_bound(): u64 &#123;<br/>   len(global&lt;ValidatorPerformance&gt;(@aptos_framework).validators)<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_has_stake_pool"></a>


<pre><code>fun spec_has_stake_pool(a: address): bool &#123;<br/>   exists&lt;StakePool&gt;(a)<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_has_validator_config"></a>


<pre><code>fun spec_has_validator_config(a: address): bool &#123;<br/>   exists&lt;ValidatorConfig&gt;(a)<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_rewards_amount"></a>


<pre><code>fun spec_rewards_amount(<br/>   stake_amount: u64,<br/>   num_successful_proposals: u64,<br/>   num_total_proposals: u64,<br/>   rewards_rate: u64,<br/>   rewards_rate_denominator: u64,<br/>): u64;<br/></code></pre>




<a id="0x1_stake_spec_contains"></a>


<pre><code>fun spec_contains(validators: vector&lt;ValidatorInfo&gt;, addr: address): bool &#123;<br/>   exists i in 0..len(validators): validators[i].addr &#61;&#61; addr<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_is_current_epoch_validator"></a>


<pre><code>fun spec_is_current_epoch_validator(pool_address: address): bool &#123;<br/>   let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);<br/>   !spec_contains(validator_set.pending_active, pool_address)<br/>       &amp;&amp; (spec_contains(validator_set.active_validators, pool_address)<br/>       &#124;&#124; spec_contains(validator_set.pending_inactive, pool_address))<br/>&#125;<br/></code></pre>




<a id="0x1_stake_ResourceRequirement"></a>


<pre><code>schema ResourceRequirement &#123;<br/>requires exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);<br/>requires exists&lt;ValidatorPerformance&gt;(@aptos_framework);<br/>requires exists&lt;ValidatorSet&gt;(@aptos_framework);<br/>requires exists&lt;StakingConfig&gt;(@aptos_framework);<br/>requires exists&lt;StakingRewardsConfig&gt;(@aptos_framework) &#124;&#124; !features::spec_periodical_reward_rate_decrease_enabled();<br/>requires exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>requires exists&lt;ValidatorFees&gt;(@aptos_framework);<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_get_reward_rate_1"></a>


<pre><code>fun spec_get_reward_rate_1(config: StakingConfig): num &#123;<br/>   if (features::spec_periodical_reward_rate_decrease_enabled()) &#123;<br/>       let epoch_rewards_rate &#61; global&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework).rewards_rate;<br/>       if (epoch_rewards_rate.value &#61;&#61; 0) &#123;<br/>           0<br/>       &#125; else &#123;<br/>           let denominator_0 &#61; aptos_std::fixed_point64::spec_divide_u128(staking_config::MAX_REWARDS_RATE, epoch_rewards_rate);<br/>           let denominator &#61; if (denominator_0 &gt; MAX_U64) &#123;<br/>               MAX_U64<br/>           &#125; else &#123;<br/>               denominator_0<br/>           &#125;;<br/>           let nominator &#61; aptos_std::fixed_point64::spec_multiply_u128(denominator, epoch_rewards_rate);<br/>           nominator<br/>       &#125;<br/>   &#125; else &#123;<br/>           config.rewards_rate<br/>   &#125;<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_get_reward_rate_2"></a>


<pre><code>fun spec_get_reward_rate_2(config: StakingConfig): num &#123;<br/>   if (features::spec_periodical_reward_rate_decrease_enabled()) &#123;<br/>       let epoch_rewards_rate &#61; global&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework).rewards_rate;<br/>       if (epoch_rewards_rate.value &#61;&#61; 0) &#123;<br/>           1<br/>       &#125; else &#123;<br/>           let denominator_0 &#61; aptos_std::fixed_point64::spec_divide_u128(staking_config::MAX_REWARDS_RATE, epoch_rewards_rate);<br/>           let denominator &#61; if (denominator_0 &gt; MAX_U64) &#123;<br/>               MAX_U64<br/>           &#125; else &#123;<br/>               denominator_0<br/>           &#125;;<br/>           denominator<br/>       &#125;<br/>   &#125; else &#123;<br/>           config.rewards_rate_denominator<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_update_stake_pool"></a>

### Function `update_stake_pool`


<pre><code>fun update_stake_pool(validator_perf: &amp;stake::ValidatorPerformance, pool_address: address, staking_config: &amp;staking_config::StakingConfig)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>include ResourceRequirement;<br/>include GetReconfigStartTimeRequirement;<br/>include staking_config::StakingRewardsConfigRequirement;<br/>include UpdateStakePoolAbortsIf;<br/>let stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let validator_config &#61; global&lt;ValidatorConfig&gt;(pool_address);<br/>let cur_validator_perf &#61; validator_perf.validators[validator_config.validator_index];<br/>let num_successful_proposals &#61; cur_validator_perf.successful_proposals;<br/>let num_total_proposals &#61; cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals;<br/>let rewards_rate &#61; spec_get_reward_rate_1(staking_config);<br/>let rewards_rate_denominator &#61; spec_get_reward_rate_2(staking_config);<br/>let rewards_amount_1 &#61; if (stake_pool.active.value &gt; 0) &#123;<br/>    spec_rewards_amount(stake_pool.active.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)<br/>&#125; else &#123;<br/>    0<br/>&#125;;<br/>let rewards_amount_2 &#61; if (stake_pool.pending_inactive.value &gt; 0) &#123;<br/>    spec_rewards_amount(stake_pool.pending_inactive.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)<br/>&#125; else &#123;<br/>    0<br/>&#125;;<br/>let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>let post post_active_value &#61; post_stake_pool.active.value;<br/>let post post_pending_inactive_value &#61; post_stake_pool.pending_inactive.value;<br/>let fees_table &#61; global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;<br/>let post post_fees_table &#61; global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;<br/>let post post_inactive_value &#61; post_stake_pool.inactive.value;<br/>ensures post_stake_pool.pending_active.value &#61;&#61; 0;<br/>ensures if (features::spec_is_enabled(features::COLLECT_AND_DISTRIBUTE_GAS_FEES) &amp;&amp; table::spec_contains(fees_table, pool_address)) &#123;<br/>    !table::spec_contains(post_fees_table, pool_address) &amp;&amp;<br/>    post_active_value &#61;&#61; stake_pool.active.value &#43; rewards_amount_1 &#43; stake_pool.pending_active.value &#43; table::spec_get(fees_table, pool_address).value<br/>&#125; else &#123;<br/>    post_active_value &#61;&#61; stake_pool.active.value &#43; rewards_amount_1 &#43; stake_pool.pending_active.value<br/>&#125;;<br/>ensures if (spec_get_reconfig_start_time_secs() &gt;&#61; stake_pool.locked_until_secs) &#123;<br/>    post_pending_inactive_value &#61;&#61; 0 &amp;&amp;<br/>    post_inactive_value &#61;&#61; stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &#43; rewards_amount_2<br/>&#125; else &#123;<br/>    post_pending_inactive_value &#61;&#61; stake_pool.pending_inactive.value &#43; rewards_amount_2<br/>&#125;;<br/></code></pre>




<a id="0x1_stake_UpdateStakePoolAbortsIf"></a>


<pre><code>schema UpdateStakePoolAbortsIf &#123;<br/>pool_address: address;<br/>validator_perf: ValidatorPerformance;<br/>aborts_if !exists&lt;StakePool&gt;(pool_address);<br/>aborts_if !exists&lt;ValidatorConfig&gt;(pool_address);<br/>aborts_if global&lt;ValidatorConfig&gt;(pool_address).validator_index &gt;&#61; len(validator_perf.validators);<br/>let aptos_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;<br/>aborts_if !exists&lt;ValidatorFees&gt;(aptos_addr);<br/>let stake_pool &#61; global&lt;StakePool&gt;(pool_address);<br/>include DistributeRewardsAbortsIf &#123;stake: stake_pool.active&#125;;<br/>include DistributeRewardsAbortsIf &#123;stake: stake_pool.pending_inactive&#125;;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_reconfig_start_time_secs"></a>

### Function `get_reconfig_start_time_secs`


<pre><code>fun get_reconfig_start_time_secs(): u64<br/></code></pre>




<pre><code>include GetReconfigStartTimeRequirement;<br/></code></pre>




<a id="0x1_stake_GetReconfigStartTimeRequirement"></a>


<pre><code>schema GetReconfigStartTimeRequirement &#123;<br/>requires exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>include reconfiguration_state::StartTimeSecsRequirement;<br/>&#125;<br/></code></pre>




<a id="0x1_stake_spec_get_reconfig_start_time_secs"></a>


<pre><code>fun spec_get_reconfig_start_time_secs(): u64 &#123;<br/>   if (exists&lt;reconfiguration_state::State&gt;(@aptos_framework)) &#123;<br/>       reconfiguration_state::spec_start_time_secs()<br/>   &#125; else &#123;<br/>       timestamp::spec_now_seconds()<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_calculate_rewards_amount"></a>

### Function `calculate_rewards_amount`


<pre><code>fun calculate_rewards_amount(stake_amount: u64, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/>pragma verify_duration_estimate &#61; 300;<br/>requires rewards_rate &lt;&#61; MAX_REWARDS_RATE;<br/>requires rewards_rate_denominator &gt; 0;<br/>requires rewards_rate &lt;&#61; rewards_rate_denominator;<br/>requires num_successful_proposals &lt;&#61; num_total_proposals;<br/>ensures [concrete] (rewards_rate_denominator &#42; num_total_proposals &#61;&#61; 0) &#61;&#61;&gt; result &#61;&#61; 0;<br/>ensures [concrete] (rewards_rate_denominator &#42; num_total_proposals &gt; 0) &#61;&#61;&gt; &#123;<br/>    let amount &#61; ((stake_amount &#42; rewards_rate &#42; num_successful_proposals) /<br/>        (rewards_rate_denominator &#42; num_total_proposals));<br/>    result &#61;&#61; amount<br/>&#125;;<br/>aborts_if false;<br/>ensures [abstract] result &#61;&#61; spec_rewards_amount(<br/>    stake_amount,<br/>    num_successful_proposals,<br/>    num_total_proposals,<br/>    rewards_rate,<br/>    rewards_rate_denominator);<br/></code></pre>



<a id="@Specification_1_distribute_rewards"></a>

### Function `distribute_rewards`


<pre><code>fun distribute_rewards(stake: &amp;mut coin::Coin&lt;aptos_coin::AptosCoin&gt;, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64<br/></code></pre>




<pre><code>include ResourceRequirement;<br/>requires rewards_rate &lt;&#61; MAX_REWARDS_RATE;<br/>requires rewards_rate_denominator &gt; 0;<br/>requires rewards_rate &lt;&#61; rewards_rate_denominator;<br/>requires num_successful_proposals &lt;&#61; num_total_proposals;<br/>include DistributeRewardsAbortsIf;<br/>ensures old(stake.value) &gt; 0 &#61;&#61;&gt;<br/>    result &#61;&#61; spec_rewards_amount(<br/>        old(stake.value),<br/>        num_successful_proposals,<br/>        num_total_proposals,<br/>        rewards_rate,<br/>        rewards_rate_denominator);<br/>ensures old(stake.value) &gt; 0 &#61;&#61;&gt;<br/>    stake.value &#61;&#61; old(stake.value) &#43; spec_rewards_amount(<br/>        old(stake.value),<br/>        num_successful_proposals,<br/>        num_total_proposals,<br/>        rewards_rate,<br/>        rewards_rate_denominator);<br/>ensures old(stake.value) &#61;&#61; 0 &#61;&#61;&gt; result &#61;&#61; 0;<br/>ensures old(stake.value) &#61;&#61; 0 &#61;&#61;&gt; stake.value &#61;&#61; old(stake.value);<br/></code></pre>




<a id="0x1_stake_DistributeRewardsAbortsIf"></a>


<pre><code>schema DistributeRewardsAbortsIf &#123;<br/>stake: Coin&lt;AptosCoin&gt;;<br/>num_successful_proposals: num;<br/>num_total_proposals: num;<br/>rewards_rate: num;<br/>rewards_rate_denominator: num;<br/>let stake_amount &#61; coin::value(stake);<br/>let rewards_amount &#61; if (stake_amount &gt; 0) &#123;<br/>    spec_rewards_amount(stake_amount, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)<br/>&#125; else &#123;<br/>    0<br/>&#125;;<br/>let amount &#61; rewards_amount;<br/>let addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;<br/>aborts_if (rewards_amount &gt; 0) &amp;&amp; !exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(addr);<br/>modifies global&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(addr);<br/>include (rewards_amount &gt; 0) &#61;&#61;&gt; coin::CoinAddAbortsIf&lt;AptosCoin&gt; &#123; amount: amount &#125;;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code>fun append&lt;T&gt;(v1: &amp;mut vector&lt;T&gt;, v2: &amp;mut vector&lt;T&gt;)<br/></code></pre>




<pre><code>pragma opaque, verify &#61; false;<br/>aborts_if false;<br/>ensures len(v1) &#61;&#61; old(len(v1) &#43; len(v2));<br/>ensures len(v2) &#61;&#61; 0;<br/>ensures (forall i in 0..old(len(v1)): v1[i] &#61;&#61; old(v1[i]));<br/>ensures (forall i in old(len(v1))..len(v1): v1[i] &#61;&#61; old(v2[len(v2) &#45; (i &#45; len(v1)) &#45; 1]));<br/></code></pre>



<a id="@Specification_1_find_validator"></a>

### Function `find_validator`


<pre><code>fun find_validator(v: &amp;vector&lt;stake::ValidatorInfo&gt;, addr: address): option::Option&lt;u64&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures option::is_none(result) &#61;&#61;&gt; (forall i in 0..len(v): v[i].addr !&#61; addr);<br/>ensures option::is_some(result) &#61;&#61;&gt; v[option::borrow(result)].addr &#61;&#61; addr;<br/>ensures option::is_some(result) &#61;&#61;&gt; spec_contains(v, addr);<br/>ensures [abstract] result &#61;&#61; spec_find_validator(v,addr);<br/></code></pre>



<a id="@Specification_1_update_voting_power_increase"></a>

### Function `update_voting_power_increase`


<pre><code>fun update_voting_power_increase(increase_amount: u64)<br/></code></pre>




<pre><code>requires !reconfiguration_state::spec_is_in_progress();<br/>aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);<br/>aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>let aptos &#61; @aptos_framework;<br/>let pre_validator_set &#61; global&lt;ValidatorSet&gt;(aptos);<br/>let post validator_set &#61; global&lt;ValidatorSet&gt;(aptos);<br/>let staking_config &#61; global&lt;staking_config::StakingConfig&gt;(aptos);<br/>let voting_power_increase_limit &#61; staking_config.voting_power_increase_limit;<br/>aborts_if pre_validator_set.total_joining_power &#43; increase_amount &gt; MAX_U128;<br/>aborts_if pre_validator_set.total_voting_power &gt; 0 &amp;&amp; pre_validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;<br/>aborts_if pre_validator_set.total_voting_power &gt; 0 &amp;&amp;<br/>    pre_validator_set.total_joining_power &#43; increase_amount &gt; pre_validator_set.total_voting_power &#42; voting_power_increase_limit / 100;<br/>ensures validator_set.total_voting_power &gt; 0 &#61;&#61;&gt;<br/>    validator_set.total_joining_power &lt;&#61; validator_set.total_voting_power &#42; voting_power_increase_limit / 100;<br/>ensures validator_set.total_joining_power &#61;&#61; pre_validator_set.total_joining_power &#43; increase_amount;<br/></code></pre>



<a id="@Specification_1_assert_stake_pool_exists"></a>

### Function `assert_stake_pool_exists`


<pre><code>fun assert_stake_pool_exists(pool_address: address)<br/></code></pre>




<pre><code>aborts_if !stake_pool_exists(pool_address);<br/></code></pre>



<a id="@Specification_1_configure_allowed_validators"></a>

### Function `configure_allowed_validators`


<pre><code>public fun configure_allowed_validators(aptos_framework: &amp;signer, accounts: vector&lt;address&gt;)<br/></code></pre>




<pre><code>let aptos_framework_address &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(aptos_framework_address);<br/>let post allowed &#61; global&lt;AllowedValidators&gt;(aptos_framework_address);<br/>ensures allowed.accounts &#61;&#61; accounts;<br/></code></pre>



<a id="@Specification_1_assert_owner_cap_exists"></a>

### Function `assert_owner_cap_exists`


<pre><code>fun assert_owner_cap_exists(owner: address)<br/></code></pre>




<pre><code>aborts_if !exists&lt;OwnerCapability&gt;(owner);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
