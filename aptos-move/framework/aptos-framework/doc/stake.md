
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


<pre><code>use 0x1::account;
use 0x1::aptos_coin;
use 0x1::bls12381;
use 0x1::chain_status;
use 0x1::coin;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::fixed_point64;
use 0x1::math64;
use 0x1::option;
use 0x1::reconfiguration_state;
use 0x1::signer;
use 0x1::staking_config;
use 0x1::system_addresses;
use 0x1::table;
use 0x1::timestamp;
use 0x1::validator_consensus_info;
use 0x1::vector;
</code></pre>



<a id="0x1_stake_OwnerCapability"></a>

## Resource `OwnerCapability`

Capability that represents ownership and can be used to control the validator and the associated stake pool.
Having this be separate from the signer for the account that the validator resources are hosted at allows
modules to have control over a validator.


<pre><code>struct OwnerCapability has store, key
</code></pre>



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


<pre><code>struct StakePool has key
</code></pre>



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


<pre><code>struct ValidatorConfig has copy, drop, store, key
</code></pre>



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


<pre><code>struct ValidatorInfo has copy, drop, store
</code></pre>



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


<pre><code>struct ValidatorSet has copy, drop, store, key
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


<pre><code>struct AptosCoinCapabilities has key
</code></pre>



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



<pre><code>struct IndividualValidatorPerformance has drop, store
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



<pre><code>struct ValidatorPerformance has key
</code></pre>



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



<pre><code>struct RegisterValidatorCandidateEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct RegisterValidatorCandidate has drop, store
</code></pre>



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



<pre><code>struct SetOperatorEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct SetOperator has drop, store
</code></pre>



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



<pre><code>struct AddStakeEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct AddStake has drop, store
</code></pre>



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



<pre><code>struct ReactivateStakeEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct ReactivateStake has drop, store
</code></pre>



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



<pre><code>struct RotateConsensusKeyEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct RotateConsensusKey has drop, store
</code></pre>



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



<pre><code>struct UpdateNetworkAndFullnodeAddressesEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct UpdateNetworkAndFullnodeAddresses has drop, store
</code></pre>



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



<pre><code>struct IncreaseLockupEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct IncreaseLockup has drop, store
</code></pre>



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



<pre><code>struct JoinValidatorSetEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct JoinValidatorSet has drop, store
</code></pre>



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



<pre><code>struct DistributeRewardsEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct DistributeRewards has drop, store
</code></pre>



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



<pre><code>struct UnlockStakeEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct UnlockStake has drop, store
</code></pre>



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



<pre><code>struct WithdrawStakeEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct WithdrawStake has drop, store
</code></pre>



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



<pre><code>struct LeaveValidatorSetEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct LeaveValidatorSet has drop, store
</code></pre>



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


<pre><code>struct ValidatorFees has key
</code></pre>



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


<pre><code>struct AllowedValidators has key
</code></pre>



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



<pre><code>struct Ghost$ghost_valid_perf has copy, drop, store, key
</code></pre>



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



<pre><code>struct Ghost$ghost_proposer_idx has copy, drop, store, key
</code></pre>



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



<pre><code>struct Ghost$ghost_active_num has copy, drop, store, key
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



<pre><code>struct Ghost$ghost_pending_inactive_num has copy, drop, store, key
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



<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_stake_EALREADY_REGISTERED"></a>

Account is already registered as a validator candidate.


<pre><code>const EALREADY_REGISTERED: u64 &#61; 8;
</code></pre>



<a id="0x1_stake_MAX_REWARDS_RATE"></a>

Limit the maximum value of <code>rewards_rate</code> in order to avoid any arithmetic overflow.


<pre><code>const MAX_REWARDS_RATE: u64 &#61; 1000000;
</code></pre>



<a id="0x1_stake_EALREADY_ACTIVE_VALIDATOR"></a>

Account is already a validator or pending validator.


<pre><code>const EALREADY_ACTIVE_VALIDATOR: u64 &#61; 4;
</code></pre>



<a id="0x1_stake_EFEES_TABLE_ALREADY_EXISTS"></a>

Table to store collected transaction fees for each validator already exists.


<pre><code>const EFEES_TABLE_ALREADY_EXISTS: u64 &#61; 19;
</code></pre>



<a id="0x1_stake_EINELIGIBLE_VALIDATOR"></a>

Validator is not defined in the ACL of entities allowed to be validators


<pre><code>const EINELIGIBLE_VALIDATOR: u64 &#61; 17;
</code></pre>



<a id="0x1_stake_EINVALID_LOCKUP"></a>

Cannot update stake pool's lockup to earlier than current lockup.


<pre><code>const EINVALID_LOCKUP: u64 &#61; 18;
</code></pre>



<a id="0x1_stake_EINVALID_PUBLIC_KEY"></a>

Invalid consensus public key


<pre><code>const EINVALID_PUBLIC_KEY: u64 &#61; 11;
</code></pre>



<a id="0x1_stake_ELAST_VALIDATOR"></a>

Can't remove last validator.


<pre><code>const ELAST_VALIDATOR: u64 &#61; 6;
</code></pre>



<a id="0x1_stake_ENOT_OPERATOR"></a>

Account does not have the right operator capability.


<pre><code>const ENOT_OPERATOR: u64 &#61; 9;
</code></pre>



<a id="0x1_stake_ENOT_VALIDATOR"></a>

Account is not a validator.


<pre><code>const ENOT_VALIDATOR: u64 &#61; 5;
</code></pre>



<a id="0x1_stake_ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED"></a>

Validators cannot join or leave post genesis on this test network.


<pre><code>const ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED: u64 &#61; 10;
</code></pre>



<a id="0x1_stake_EOWNER_CAP_ALREADY_EXISTS"></a>

An account cannot own more than one owner capability.


<pre><code>const EOWNER_CAP_ALREADY_EXISTS: u64 &#61; 16;
</code></pre>



<a id="0x1_stake_EOWNER_CAP_NOT_FOUND"></a>

Owner capability does not exist at the provided account.


<pre><code>const EOWNER_CAP_NOT_FOUND: u64 &#61; 15;
</code></pre>



<a id="0x1_stake_ERECONFIGURATION_IN_PROGRESS"></a>

Validator set change temporarily disabled because of in-progress reconfiguration.


<pre><code>const ERECONFIGURATION_IN_PROGRESS: u64 &#61; 20;
</code></pre>



<a id="0x1_stake_ESTAKE_EXCEEDS_MAX"></a>

Total stake exceeds maximum allowed.


<pre><code>const ESTAKE_EXCEEDS_MAX: u64 &#61; 7;
</code></pre>



<a id="0x1_stake_ESTAKE_POOL_DOES_NOT_EXIST"></a>

Stake pool does not exist at the provided pool address.


<pre><code>const ESTAKE_POOL_DOES_NOT_EXIST: u64 &#61; 14;
</code></pre>



<a id="0x1_stake_ESTAKE_TOO_HIGH"></a>

Too much stake to join validator set.


<pre><code>const ESTAKE_TOO_HIGH: u64 &#61; 3;
</code></pre>



<a id="0x1_stake_ESTAKE_TOO_LOW"></a>

Not enough stake to join validator set.


<pre><code>const ESTAKE_TOO_LOW: u64 &#61; 2;
</code></pre>



<a id="0x1_stake_EVALIDATOR_CONFIG"></a>

Validator Config not published.


<pre><code>const EVALIDATOR_CONFIG: u64 &#61; 1;
</code></pre>



<a id="0x1_stake_EVALIDATOR_SET_TOO_LARGE"></a>

Validator set exceeds the limit


<pre><code>const EVALIDATOR_SET_TOO_LARGE: u64 &#61; 12;
</code></pre>



<a id="0x1_stake_EVOTING_POWER_INCREASE_EXCEEDS_LIMIT"></a>

Voting power increase has exceeded the limit for this current epoch.


<pre><code>const EVOTING_POWER_INCREASE_EXCEEDS_LIMIT: u64 &#61; 13;
</code></pre>



<a id="0x1_stake_MAX_VALIDATOR_SET_SIZE"></a>

Limit the maximum size to u16::max, it's the current limit of the bitvec
https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos-bitvec/src/lib.rs#L20


<pre><code>const MAX_VALIDATOR_SET_SIZE: u64 &#61; 65536;
</code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_ACTIVE"></a>



<pre><code>const VALIDATOR_STATUS_ACTIVE: u64 &#61; 2;
</code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code>const VALIDATOR_STATUS_INACTIVE: u64 &#61; 4;
</code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_PENDING_ACTIVE"></a>

Validator status enum. We can switch to proper enum later once Move supports it.


<pre><code>const VALIDATOR_STATUS_PENDING_ACTIVE: u64 &#61; 1;
</code></pre>



<a id="0x1_stake_VALIDATOR_STATUS_PENDING_INACTIVE"></a>



<pre><code>const VALIDATOR_STATUS_PENDING_INACTIVE: u64 &#61; 3;
</code></pre>



<a id="0x1_stake_initialize_validator_fees"></a>

## Function `initialize_validator_fees`

Initializes the resource storing information about collected transaction fees per validator.
Used by <code>transaction_fee.move</code> to initialize fee collection and distribution.


<pre><code>public(friend) fun initialize_validator_fees(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_validator_fees(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(
        !exists&lt;ValidatorFees&gt;(@aptos_framework),
        error::already_exists(EFEES_TABLE_ALREADY_EXISTS)
    );
    move_to(aptos_framework, ValidatorFees &#123; fees_table: table::new() &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_stake_add_transaction_fee"></a>

## Function `add_transaction_fee`

Stores the transaction fee collected to the specified validator address.


<pre><code>public(friend) fun add_transaction_fee(validator_addr: address, fee: coin::Coin&lt;aptos_coin::AptosCoin&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun add_transaction_fee(validator_addr: address, fee: Coin&lt;AptosCoin&gt;) acquires ValidatorFees &#123;
    let fees_table &#61; &amp;mut borrow_global_mut&lt;ValidatorFees&gt;(@aptos_framework).fees_table;
    if (table::contains(fees_table, validator_addr)) &#123;
        let collected_fee &#61; table::borrow_mut(fees_table, validator_addr);
        coin::merge(collected_fee, fee);
    &#125; else &#123;
        table::add(fees_table, validator_addr, fee);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_lockup_secs"></a>

## Function `get_lockup_secs`

Return the lockup expiration of the stake pool at <code>pool_address</code>.
This will throw an error if there's no stake pool at <code>pool_address</code>.


<pre><code>&#35;[view]
public fun get_lockup_secs(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_lockup_secs(pool_address: address): u64 acquires StakePool &#123;
    assert_stake_pool_exists(pool_address);
    borrow_global&lt;StakePool&gt;(pool_address).locked_until_secs
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_remaining_lockup_secs"></a>

## Function `get_remaining_lockup_secs`

Return the remaining lockup of the stake pool at <code>pool_address</code>.
This will throw an error if there's no stake pool at <code>pool_address</code>.


<pre><code>&#35;[view]
public fun get_remaining_lockup_secs(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_remaining_lockup_secs(pool_address: address): u64 acquires StakePool &#123;
    assert_stake_pool_exists(pool_address);
    let lockup_time &#61; borrow_global&lt;StakePool&gt;(pool_address).locked_until_secs;
    if (lockup_time &lt;&#61; timestamp::now_seconds()) &#123;
        0
    &#125; else &#123;
        lockup_time &#45; timestamp::now_seconds()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_stake"></a>

## Function `get_stake`

Return the different stake amounts for <code>pool_address</code> (whether the validator is active or not).
The returned amounts are for (active, inactive, pending_active, pending_inactive) stake respectively.


<pre><code>&#35;[view]
public fun get_stake(pool_address: address): (u64, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_stake(pool_address: address): (u64, u64, u64, u64) acquires StakePool &#123;
    assert_stake_pool_exists(pool_address);
    let stake_pool &#61; borrow_global&lt;StakePool&gt;(pool_address);
    (
        coin::value(&amp;stake_pool.active),
        coin::value(&amp;stake_pool.inactive),
        coin::value(&amp;stake_pool.pending_active),
        coin::value(&amp;stake_pool.pending_inactive),
    )
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_validator_state"></a>

## Function `get_validator_state`

Returns the validator's state.


<pre><code>&#35;[view]
public fun get_validator_state(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_validator_state(pool_address: address): u64 acquires ValidatorSet &#123;
    let validator_set &#61; borrow_global&lt;ValidatorSet&gt;(@aptos_framework);
    if (option::is_some(&amp;find_validator(&amp;validator_set.pending_active, pool_address))) &#123;
        VALIDATOR_STATUS_PENDING_ACTIVE
    &#125; else if (option::is_some(&amp;find_validator(&amp;validator_set.active_validators, pool_address))) &#123;
        VALIDATOR_STATUS_ACTIVE
    &#125; else if (option::is_some(&amp;find_validator(&amp;validator_set.pending_inactive, pool_address))) &#123;
        VALIDATOR_STATUS_PENDING_INACTIVE
    &#125; else &#123;
        VALIDATOR_STATUS_INACTIVE
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_current_epoch_voting_power"></a>

## Function `get_current_epoch_voting_power`

Return the voting power of the validator in the current epoch.
This is the same as the validator's total active and pending_inactive stake.


<pre><code>&#35;[view]
public fun get_current_epoch_voting_power(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_current_epoch_voting_power(pool_address: address): u64 acquires StakePool, ValidatorSet &#123;
    assert_stake_pool_exists(pool_address);
    let validator_state &#61; get_validator_state(pool_address);
    // Both active and pending inactive validators can still vote in the current epoch.
    if (validator_state &#61;&#61; VALIDATOR_STATUS_ACTIVE &#124;&#124; validator_state &#61;&#61; VALIDATOR_STATUS_PENDING_INACTIVE) &#123;
        let active_stake &#61; coin::value(&amp;borrow_global&lt;StakePool&gt;(pool_address).active);
        let pending_inactive_stake &#61; coin::value(&amp;borrow_global&lt;StakePool&gt;(pool_address).pending_inactive);
        active_stake &#43; pending_inactive_stake
    &#125; else &#123;
        0
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_delegated_voter"></a>

## Function `get_delegated_voter`

Return the delegated voter of the validator at <code>pool_address</code>.


<pre><code>&#35;[view]
public fun get_delegated_voter(pool_address: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegated_voter(pool_address: address): address acquires StakePool &#123;
    assert_stake_pool_exists(pool_address);
    borrow_global&lt;StakePool&gt;(pool_address).delegated_voter
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_operator"></a>

## Function `get_operator`

Return the operator of the validator at <code>pool_address</code>.


<pre><code>&#35;[view]
public fun get_operator(pool_address: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_operator(pool_address: address): address acquires StakePool &#123;
    assert_stake_pool_exists(pool_address);
    borrow_global&lt;StakePool&gt;(pool_address).operator_address
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_owned_pool_address"></a>

## Function `get_owned_pool_address`

Return the pool address in <code>owner_cap</code>.


<pre><code>public fun get_owned_pool_address(owner_cap: &amp;stake::OwnerCapability): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_owned_pool_address(owner_cap: &amp;OwnerCapability): address &#123;
    owner_cap.pool_address
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_validator_index"></a>

## Function `get_validator_index`

Return the validator index for <code>pool_address</code>.


<pre><code>&#35;[view]
public fun get_validator_index(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_validator_index(pool_address: address): u64 acquires ValidatorConfig &#123;
    assert_stake_pool_exists(pool_address);
    borrow_global&lt;ValidatorConfig&gt;(pool_address).validator_index
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_current_epoch_proposal_counts"></a>

## Function `get_current_epoch_proposal_counts`

Return the number of successful and failed proposals for the proposal at the given validator index.


<pre><code>&#35;[view]
public fun get_current_epoch_proposal_counts(validator_index: u64): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_current_epoch_proposal_counts(validator_index: u64): (u64, u64) acquires ValidatorPerformance &#123;
    let validator_performances &#61; &amp;borrow_global&lt;ValidatorPerformance&gt;(@aptos_framework).validators;
    let validator_performance &#61; vector::borrow(validator_performances, validator_index);
    (validator_performance.successful_proposals, validator_performance.failed_proposals)
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_validator_config"></a>

## Function `get_validator_config`

Return the validator's config.


<pre><code>&#35;[view]
public fun get_validator_config(pool_address: address): (vector&lt;u8&gt;, vector&lt;u8&gt;, vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_validator_config(
    pool_address: address
): (vector&lt;u8&gt;, vector&lt;u8&gt;, vector&lt;u8&gt;) acquires ValidatorConfig &#123;
    assert_stake_pool_exists(pool_address);
    let validator_config &#61; borrow_global&lt;ValidatorConfig&gt;(pool_address);
    (validator_config.consensus_pubkey, validator_config.network_addresses, validator_config.fullnode_addresses)
&#125;
</code></pre>



</details>

<a id="0x1_stake_stake_pool_exists"></a>

## Function `stake_pool_exists`



<pre><code>&#35;[view]
public fun stake_pool_exists(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun stake_pool_exists(addr: address): bool &#123;
    exists&lt;StakePool&gt;(addr)
&#125;
</code></pre>



</details>

<a id="0x1_stake_initialize"></a>

## Function `initialize`

Initialize validator set to the core resource account.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);

    move_to(aptos_framework, ValidatorSet &#123;
        consensus_scheme: 0,
        active_validators: vector::empty(),
        pending_active: vector::empty(),
        pending_inactive: vector::empty(),
        total_voting_power: 0,
        total_joining_power: 0,
    &#125;);

    move_to(aptos_framework, ValidatorPerformance &#123;
        validators: vector::empty(),
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_stake_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

This is only called during Genesis, which is where MintCapability<AptosCoin> can be created.
Beyond genesis, no one can create AptosCoin mint/burn capabilities.


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: MintCapability&lt;AptosCoin&gt;) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    move_to(aptos_framework, AptosCoinCapabilities &#123; mint_cap &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_stake_remove_validators"></a>

## Function `remove_validators`

Allow on chain governance to remove validators from the validator set.


<pre><code>public fun remove_validators(aptos_framework: &amp;signer, validators: &amp;vector&lt;address&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_validators(
    aptos_framework: &amp;signer,
    validators: &amp;vector&lt;address&gt;,
) acquires ValidatorSet &#123;
    assert_reconfig_not_in_progress();
    system_addresses::assert_aptos_framework(aptos_framework);
    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);
    let active_validators &#61; &amp;mut validator_set.active_validators;
    let pending_inactive &#61; &amp;mut validator_set.pending_inactive;
    spec &#123;
        update ghost_active_num &#61; len(active_validators);
        update ghost_pending_inactive_num &#61; len(pending_inactive);
    &#125;;
    let len_validators &#61; vector::length(validators);
    let i &#61; 0;
    // Remove each validator from the validator set.
    while (&#123;
        spec &#123;
            invariant i &lt;&#61; len_validators;
            invariant spec_validators_are_initialized(active_validators);
            invariant spec_validator_indices_are_valid(active_validators);
            invariant spec_validators_are_initialized(pending_inactive);
            invariant spec_validator_indices_are_valid(pending_inactive);
            invariant ghost_active_num &#43; ghost_pending_inactive_num &#61;&#61; len(active_validators) &#43; len(pending_inactive);
        &#125;;
        i &lt; len_validators
    &#125;) &#123;
        let validator &#61; &#42;vector::borrow(validators, i);
        let validator_index &#61; find_validator(active_validators, validator);
        if (option::is_some(&amp;validator_index)) &#123;
            let validator_info &#61; vector::swap_remove(active_validators, &#42;option::borrow(&amp;validator_index));
            vector::push_back(pending_inactive, validator_info);
            spec &#123;
                update ghost_active_num &#61; ghost_active_num &#45; 1;
                update ghost_pending_inactive_num &#61; ghost_pending_inactive_num &#43; 1;
            &#125;;
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_stake_initialize_stake_owner"></a>

## Function `initialize_stake_owner`

Initialize the validator account and give ownership to the signing account
except it leaves the ValidatorConfig to be set by another entity.
Note: this triggers setting the operator and owner, set it to the account's address
to set later.


<pre><code>public entry fun initialize_stake_owner(owner: &amp;signer, initial_stake_amount: u64, operator: address, voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize_stake_owner(
    owner: &amp;signer,
    initial_stake_amount: u64,
    operator: address,
    voter: address,
) acquires AllowedValidators, OwnerCapability, StakePool, ValidatorSet &#123;
    initialize_owner(owner);
    move_to(owner, ValidatorConfig &#123;
        consensus_pubkey: vector::empty(),
        network_addresses: vector::empty(),
        fullnode_addresses: vector::empty(),
        validator_index: 0,
    &#125;);

    if (initial_stake_amount &gt; 0) &#123;
        add_stake(owner, initial_stake_amount);
    &#125;;

    let account_address &#61; signer::address_of(owner);
    if (account_address !&#61; operator) &#123;
        set_operator(owner, operator)
    &#125;;
    if (account_address !&#61; voter) &#123;
        set_delegated_voter(owner, voter)
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_stake_initialize_validator"></a>

## Function `initialize_validator`

Initialize the validator account and give ownership to the signing account.


<pre><code>public entry fun initialize_validator(account: &amp;signer, consensus_pubkey: vector&lt;u8&gt;, proof_of_possession: vector&lt;u8&gt;, network_addresses: vector&lt;u8&gt;, fullnode_addresses: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize_validator(
    account: &amp;signer,
    consensus_pubkey: vector&lt;u8&gt;,
    proof_of_possession: vector&lt;u8&gt;,
    network_addresses: vector&lt;u8&gt;,
    fullnode_addresses: vector&lt;u8&gt;,
) acquires AllowedValidators &#123;
    // Checks the public key has a valid proof&#45;of&#45;possession to prevent rogue&#45;key attacks.
    let pubkey_from_pop &#61; &amp;mut bls12381::public_key_from_bytes_with_pop(
        consensus_pubkey,
        &amp;proof_of_possession_from_bytes(proof_of_possession)
    );
    assert!(option::is_some(pubkey_from_pop), error::invalid_argument(EINVALID_PUBLIC_KEY));

    initialize_owner(account);
    move_to(account, ValidatorConfig &#123;
        consensus_pubkey,
        network_addresses,
        fullnode_addresses,
        validator_index: 0,
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_stake_initialize_owner"></a>

## Function `initialize_owner`



<pre><code>fun initialize_owner(owner: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_owner(owner: &amp;signer) acquires AllowedValidators &#123;
    let owner_address &#61; signer::address_of(owner);
    assert!(is_allowed(owner_address), error::not_found(EINELIGIBLE_VALIDATOR));
    assert!(!stake_pool_exists(owner_address), error::already_exists(EALREADY_REGISTERED));

    move_to(owner, StakePool &#123;
        active: coin::zero&lt;AptosCoin&gt;(),
        pending_active: coin::zero&lt;AptosCoin&gt;(),
        pending_inactive: coin::zero&lt;AptosCoin&gt;(),
        inactive: coin::zero&lt;AptosCoin&gt;(),
        locked_until_secs: 0,
        operator_address: owner_address,
        delegated_voter: owner_address,
        // Events.
        initialize_validator_events: account::new_event_handle&lt;RegisterValidatorCandidateEvent&gt;(owner),
        set_operator_events: account::new_event_handle&lt;SetOperatorEvent&gt;(owner),
        add_stake_events: account::new_event_handle&lt;AddStakeEvent&gt;(owner),
        reactivate_stake_events: account::new_event_handle&lt;ReactivateStakeEvent&gt;(owner),
        rotate_consensus_key_events: account::new_event_handle&lt;RotateConsensusKeyEvent&gt;(owner),
        update_network_and_fullnode_addresses_events: account::new_event_handle&lt;UpdateNetworkAndFullnodeAddressesEvent&gt;(
            owner
        ),
        increase_lockup_events: account::new_event_handle&lt;IncreaseLockupEvent&gt;(owner),
        join_validator_set_events: account::new_event_handle&lt;JoinValidatorSetEvent&gt;(owner),
        distribute_rewards_events: account::new_event_handle&lt;DistributeRewardsEvent&gt;(owner),
        unlock_stake_events: account::new_event_handle&lt;UnlockStakeEvent&gt;(owner),
        withdraw_stake_events: account::new_event_handle&lt;WithdrawStakeEvent&gt;(owner),
        leave_validator_set_events: account::new_event_handle&lt;LeaveValidatorSetEvent&gt;(owner),
    &#125;);

    move_to(owner, OwnerCapability &#123; pool_address: owner_address &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_stake_extract_owner_cap"></a>

## Function `extract_owner_cap`

Extract and return owner capability from the signing account.


<pre><code>public fun extract_owner_cap(owner: &amp;signer): stake::OwnerCapability
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract_owner_cap(owner: &amp;signer): OwnerCapability acquires OwnerCapability &#123;
    let owner_address &#61; signer::address_of(owner);
    assert_owner_cap_exists(owner_address);
    move_from&lt;OwnerCapability&gt;(owner_address)
&#125;
</code></pre>



</details>

<a id="0x1_stake_deposit_owner_cap"></a>

## Function `deposit_owner_cap`

Deposit <code>owner_cap</code> into <code>account</code>. This requires <code>account</code> to not already have ownership of another
staking pool.


<pre><code>public fun deposit_owner_cap(owner: &amp;signer, owner_cap: stake::OwnerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_owner_cap(owner: &amp;signer, owner_cap: OwnerCapability) &#123;
    assert!(!exists&lt;OwnerCapability&gt;(signer::address_of(owner)), error::not_found(EOWNER_CAP_ALREADY_EXISTS));
    move_to(owner, owner_cap);
&#125;
</code></pre>



</details>

<a id="0x1_stake_destroy_owner_cap"></a>

## Function `destroy_owner_cap`

Destroy <code>owner_cap</code>.


<pre><code>public fun destroy_owner_cap(owner_cap: stake::OwnerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_owner_cap(owner_cap: OwnerCapability) &#123;
    let OwnerCapability &#123; pool_address: _ &#125; &#61; owner_cap;
&#125;
</code></pre>



</details>

<a id="0x1_stake_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the stake pool.


<pre><code>public entry fun set_operator(owner: &amp;signer, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_operator(owner: &amp;signer, new_operator: address) acquires OwnerCapability, StakePool &#123;
    let owner_address &#61; signer::address_of(owner);
    assert_owner_cap_exists(owner_address);
    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);
    set_operator_with_cap(ownership_cap, new_operator);
&#125;
</code></pre>



</details>

<a id="0x1_stake_set_operator_with_cap"></a>

## Function `set_operator_with_cap`

Allows an account with ownership capability to change the operator of the stake pool.


<pre><code>public fun set_operator_with_cap(owner_cap: &amp;stake::OwnerCapability, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_operator_with_cap(owner_cap: &amp;OwnerCapability, new_operator: address) acquires StakePool &#123;
    let pool_address &#61; owner_cap.pool_address;
    assert_stake_pool_exists(pool_address);
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    let old_operator &#61; stake_pool.operator_address;
    stake_pool.operator_address &#61; new_operator;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            SetOperator &#123;
                pool_address,
                old_operator,
                new_operator,
            &#125;,
        );
    &#125;;

    event::emit_event(
        &amp;mut stake_pool.set_operator_events,
        SetOperatorEvent &#123;
            pool_address,
            old_operator,
            new_operator,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_stake_set_delegated_voter"></a>

## Function `set_delegated_voter`

Allows an owner to change the delegated voter of the stake pool.


<pre><code>public entry fun set_delegated_voter(owner: &amp;signer, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_delegated_voter(owner: &amp;signer, new_voter: address) acquires OwnerCapability, StakePool &#123;
    let owner_address &#61; signer::address_of(owner);
    assert_owner_cap_exists(owner_address);
    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);
    set_delegated_voter_with_cap(ownership_cap, new_voter);
&#125;
</code></pre>



</details>

<a id="0x1_stake_set_delegated_voter_with_cap"></a>

## Function `set_delegated_voter_with_cap`

Allows an owner to change the delegated voter of the stake pool.


<pre><code>public fun set_delegated_voter_with_cap(owner_cap: &amp;stake::OwnerCapability, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_delegated_voter_with_cap(owner_cap: &amp;OwnerCapability, new_voter: address) acquires StakePool &#123;
    let pool_address &#61; owner_cap.pool_address;
    assert_stake_pool_exists(pool_address);
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    stake_pool.delegated_voter &#61; new_voter;
&#125;
</code></pre>



</details>

<a id="0x1_stake_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins from the <code>account</code> owning the StakePool.


<pre><code>public entry fun add_stake(owner: &amp;signer, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_stake(owner: &amp;signer, amount: u64) acquires OwnerCapability, StakePool, ValidatorSet &#123;
    let owner_address &#61; signer::address_of(owner);
    assert_owner_cap_exists(owner_address);
    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);
    add_stake_with_cap(ownership_cap, coin::withdraw&lt;AptosCoin&gt;(owner, amount));
&#125;
</code></pre>



</details>

<a id="0x1_stake_add_stake_with_cap"></a>

## Function `add_stake_with_cap`

Add <code>coins</code> into <code>pool_address</code>. this requires the corresponding <code>owner_cap</code> to be passed in.


<pre><code>public fun add_stake_with_cap(owner_cap: &amp;stake::OwnerCapability, coins: coin::Coin&lt;aptos_coin::AptosCoin&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_stake_with_cap(owner_cap: &amp;OwnerCapability, coins: Coin&lt;AptosCoin&gt;) acquires StakePool, ValidatorSet &#123;
    assert_reconfig_not_in_progress();
    let pool_address &#61; owner_cap.pool_address;
    assert_stake_pool_exists(pool_address);

    let amount &#61; coin::value(&amp;coins);
    if (amount &#61;&#61; 0) &#123;
        coin::destroy_zero(coins);
        return
    &#125;;

    // Only track and validate voting power increase for active and pending_active validator.
    // Pending_inactive validator will be removed from the validator set in the next epoch.
    // Inactive validator&apos;s total stake will be tracked when they join the validator set.
    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);
    // Search directly rather using get_validator_state to save on unnecessary loops.
    if (option::is_some(&amp;find_validator(&amp;validator_set.active_validators, pool_address)) &#124;&#124;
        option::is_some(&amp;find_validator(&amp;validator_set.pending_active, pool_address))) &#123;
        update_voting_power_increase(amount);
    &#125;;

    // Add to pending_active if it&apos;s a current validator because the stake is not counted until the next epoch.
    // Otherwise, the delegation can be added to active directly as the validator is also activated in the epoch.
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    if (is_current_epoch_validator(pool_address)) &#123;
        coin::merge&lt;AptosCoin&gt;(&amp;mut stake_pool.pending_active, coins);
    &#125; else &#123;
        coin::merge&lt;AptosCoin&gt;(&amp;mut stake_pool.active, coins);
    &#125;;

    let (_, maximum_stake) &#61; staking_config::get_required_stake(&amp;staking_config::get());
    let voting_power &#61; get_next_epoch_voting_power(stake_pool);
    assert!(voting_power &lt;&#61; maximum_stake, error::invalid_argument(ESTAKE_EXCEEDS_MAX));

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            AddStake &#123;
                pool_address,
                amount_added: amount,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.add_stake_events,
        AddStakeEvent &#123;
            pool_address,
            amount_added: amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_stake_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code>public entry fun reactivate_stake(owner: &amp;signer, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reactivate_stake(owner: &amp;signer, amount: u64) acquires OwnerCapability, StakePool &#123;
    assert_reconfig_not_in_progress();
    let owner_address &#61; signer::address_of(owner);
    assert_owner_cap_exists(owner_address);
    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);
    reactivate_stake_with_cap(ownership_cap, amount);
&#125;
</code></pre>



</details>

<a id="0x1_stake_reactivate_stake_with_cap"></a>

## Function `reactivate_stake_with_cap`



<pre><code>public fun reactivate_stake_with_cap(owner_cap: &amp;stake::OwnerCapability, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reactivate_stake_with_cap(owner_cap: &amp;OwnerCapability, amount: u64) acquires StakePool &#123;
    assert_reconfig_not_in_progress();
    let pool_address &#61; owner_cap.pool_address;
    assert_stake_pool_exists(pool_address);

    // Cap the amount to reactivate by the amount in pending_inactive.
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    let total_pending_inactive &#61; coin::value(&amp;stake_pool.pending_inactive);
    amount &#61; min(amount, total_pending_inactive);

    // Since this does not count as a voting power change (pending inactive still counts as voting power in the
    // current epoch), stake can be immediately moved from pending inactive to active.
    // We also don&apos;t need to check voting power increase as there&apos;s none.
    let reactivated_coins &#61; coin::extract(&amp;mut stake_pool.pending_inactive, amount);
    coin::merge(&amp;mut stake_pool.active, reactivated_coins);

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            ReactivateStake &#123;
                pool_address,
                amount,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.reactivate_stake_events,
        ReactivateStakeEvent &#123;
            pool_address,
            amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_stake_rotate_consensus_key"></a>

## Function `rotate_consensus_key`

Rotate the consensus key of the validator, it'll take effect in next epoch.


<pre><code>public entry fun rotate_consensus_key(operator: &amp;signer, pool_address: address, new_consensus_pubkey: vector&lt;u8&gt;, proof_of_possession: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun rotate_consensus_key(
    operator: &amp;signer,
    pool_address: address,
    new_consensus_pubkey: vector&lt;u8&gt;,
    proof_of_possession: vector&lt;u8&gt;,
) acquires StakePool, ValidatorConfig &#123;
    assert_reconfig_not_in_progress();
    assert_stake_pool_exists(pool_address);

    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    assert!(signer::address_of(operator) &#61;&#61; stake_pool.operator_address, error::unauthenticated(ENOT_OPERATOR));

    assert!(exists&lt;ValidatorConfig&gt;(pool_address), error::not_found(EVALIDATOR_CONFIG));
    let validator_info &#61; borrow_global_mut&lt;ValidatorConfig&gt;(pool_address);
    let old_consensus_pubkey &#61; validator_info.consensus_pubkey;
    // Checks the public key has a valid proof&#45;of&#45;possession to prevent rogue&#45;key attacks.
    let pubkey_from_pop &#61; &amp;mut bls12381::public_key_from_bytes_with_pop(
        new_consensus_pubkey,
        &amp;proof_of_possession_from_bytes(proof_of_possession)
    );
    assert!(option::is_some(pubkey_from_pop), error::invalid_argument(EINVALID_PUBLIC_KEY));
    validator_info.consensus_pubkey &#61; new_consensus_pubkey;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            RotateConsensusKey &#123;
                pool_address,
                old_consensus_pubkey,
                new_consensus_pubkey,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.rotate_consensus_key_events,
        RotateConsensusKeyEvent &#123;
            pool_address,
            old_consensus_pubkey,
            new_consensus_pubkey,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_stake_update_network_and_fullnode_addresses"></a>

## Function `update_network_and_fullnode_addresses`

Update the network and full node addresses of the validator. This only takes effect in the next epoch.


<pre><code>public entry fun update_network_and_fullnode_addresses(operator: &amp;signer, pool_address: address, new_network_addresses: vector&lt;u8&gt;, new_fullnode_addresses: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_network_and_fullnode_addresses(
    operator: &amp;signer,
    pool_address: address,
    new_network_addresses: vector&lt;u8&gt;,
    new_fullnode_addresses: vector&lt;u8&gt;,
) acquires StakePool, ValidatorConfig &#123;
    assert_reconfig_not_in_progress();
    assert_stake_pool_exists(pool_address);
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    assert!(signer::address_of(operator) &#61;&#61; stake_pool.operator_address, error::unauthenticated(ENOT_OPERATOR));
    assert!(exists&lt;ValidatorConfig&gt;(pool_address), error::not_found(EVALIDATOR_CONFIG));
    let validator_info &#61; borrow_global_mut&lt;ValidatorConfig&gt;(pool_address);
    let old_network_addresses &#61; validator_info.network_addresses;
    validator_info.network_addresses &#61; new_network_addresses;
    let old_fullnode_addresses &#61; validator_info.fullnode_addresses;
    validator_info.fullnode_addresses &#61; new_fullnode_addresses;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            UpdateNetworkAndFullnodeAddresses &#123;
                pool_address,
                old_network_addresses,
                new_network_addresses,
                old_fullnode_addresses,
                new_fullnode_addresses,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.update_network_and_fullnode_addresses_events,
        UpdateNetworkAndFullnodeAddressesEvent &#123;
            pool_address,
            old_network_addresses,
            new_network_addresses,
            old_fullnode_addresses,
            new_fullnode_addresses,
        &#125;,
    );

&#125;
</code></pre>



</details>

<a id="0x1_stake_increase_lockup"></a>

## Function `increase_lockup`

Similar to increase_lockup_with_cap but will use ownership capability from the signing account.


<pre><code>public entry fun increase_lockup(owner: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun increase_lockup(owner: &amp;signer) acquires OwnerCapability, StakePool &#123;
    let owner_address &#61; signer::address_of(owner);
    assert_owner_cap_exists(owner_address);
    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);
    increase_lockup_with_cap(ownership_cap);
&#125;
</code></pre>



</details>

<a id="0x1_stake_increase_lockup_with_cap"></a>

## Function `increase_lockup_with_cap`

Unlock from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
directly inactive if it's not from an active validator.


<pre><code>public fun increase_lockup_with_cap(owner_cap: &amp;stake::OwnerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun increase_lockup_with_cap(owner_cap: &amp;OwnerCapability) acquires StakePool &#123;
    let pool_address &#61; owner_cap.pool_address;
    assert_stake_pool_exists(pool_address);
    let config &#61; staking_config::get();

    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    let old_locked_until_secs &#61; stake_pool.locked_until_secs;
    let new_locked_until_secs &#61; timestamp::now_seconds() &#43; staking_config::get_recurring_lockup_duration(&amp;config);
    assert!(old_locked_until_secs &lt; new_locked_until_secs, error::invalid_argument(EINVALID_LOCKUP));
    stake_pool.locked_until_secs &#61; new_locked_until_secs;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            IncreaseLockup &#123;
                pool_address,
                old_locked_until_secs,
                new_locked_until_secs,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.increase_lockup_events,
        IncreaseLockupEvent &#123;
            pool_address,
            old_locked_until_secs,
            new_locked_until_secs,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_stake_join_validator_set"></a>

## Function `join_validator_set`

This can only called by the operator of the validator/staking pool.


<pre><code>public entry fun join_validator_set(operator: &amp;signer, pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun join_validator_set(
    operator: &amp;signer,
    pool_address: address
) acquires StakePool, ValidatorConfig, ValidatorSet &#123;
    assert!(
        staking_config::get_allow_validator_set_change(&amp;staking_config::get()),
        error::invalid_argument(ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED),
    );

    join_validator_set_internal(operator, pool_address);
&#125;
</code></pre>



</details>

<a id="0x1_stake_join_validator_set_internal"></a>

## Function `join_validator_set_internal`

Request to have <code>pool_address</code> join the validator set. Can only be called after calling <code>initialize_validator</code>.
If the validator has the required stake (more than minimum and less than maximum allowed), they will be
added to the pending_active queue. All validators in this queue will be added to the active set when the next
epoch starts (eligibility will be rechecked).

This internal version can only be called by the Genesis module during Genesis.


<pre><code>public(friend) fun join_validator_set_internal(operator: &amp;signer, pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun join_validator_set_internal(
    operator: &amp;signer,
    pool_address: address
) acquires StakePool, ValidatorConfig, ValidatorSet &#123;
    assert_reconfig_not_in_progress();
    assert_stake_pool_exists(pool_address);
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    assert!(signer::address_of(operator) &#61;&#61; stake_pool.operator_address, error::unauthenticated(ENOT_OPERATOR));
    assert!(
        get_validator_state(pool_address) &#61;&#61; VALIDATOR_STATUS_INACTIVE,
        error::invalid_state(EALREADY_ACTIVE_VALIDATOR),
    );

    let config &#61; staking_config::get();
    let (minimum_stake, maximum_stake) &#61; staking_config::get_required_stake(&amp;config);
    let voting_power &#61; get_next_epoch_voting_power(stake_pool);
    assert!(voting_power &gt;&#61; minimum_stake, error::invalid_argument(ESTAKE_TOO_LOW));
    assert!(voting_power &lt;&#61; maximum_stake, error::invalid_argument(ESTAKE_TOO_HIGH));

    // Track and validate voting power increase.
    update_voting_power_increase(voting_power);

    // Add validator to pending_active, to be activated in the next epoch.
    let validator_config &#61; borrow_global_mut&lt;ValidatorConfig&gt;(pool_address);
    assert!(!vector::is_empty(&amp;validator_config.consensus_pubkey), error::invalid_argument(EINVALID_PUBLIC_KEY));

    // Validate the current validator set size has not exceeded the limit.
    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);
    vector::push_back(
        &amp;mut validator_set.pending_active,
        generate_validator_info(pool_address, stake_pool, &#42;validator_config)
    );
    let validator_set_size &#61; vector::length(&amp;validator_set.active_validators) &#43; vector::length(
        &amp;validator_set.pending_active
    );
    assert!(validator_set_size &lt;&#61; MAX_VALIDATOR_SET_SIZE, error::invalid_argument(EVALIDATOR_SET_TOO_LARGE));

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(JoinValidatorSet &#123; pool_address &#125;);
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.join_validator_set_events,
        JoinValidatorSetEvent &#123; pool_address &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_stake_unlock"></a>

## Function `unlock`

Similar to unlock_with_cap but will use ownership capability from the signing account.


<pre><code>public entry fun unlock(owner: &amp;signer, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock(owner: &amp;signer, amount: u64) acquires OwnerCapability, StakePool &#123;
    assert_reconfig_not_in_progress();
    let owner_address &#61; signer::address_of(owner);
    assert_owner_cap_exists(owner_address);
    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);
    unlock_with_cap(amount, ownership_cap);
&#125;
</code></pre>



</details>

<a id="0x1_stake_unlock_with_cap"></a>

## Function `unlock_with_cap`

Unlock <code>amount</code> from the active stake. Only possible if the lockup has expired.


<pre><code>public fun unlock_with_cap(amount: u64, owner_cap: &amp;stake::OwnerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unlock_with_cap(amount: u64, owner_cap: &amp;OwnerCapability) acquires StakePool &#123;
    assert_reconfig_not_in_progress();
    // Short&#45;circuit if amount to unlock is 0 so we don&apos;t emit events.
    if (amount &#61;&#61; 0) &#123;
        return
    &#125;;

    // Unlocked coins are moved to pending_inactive. When the current lockup cycle expires, they will be moved into
    // inactive in the earliest possible epoch transition.
    let pool_address &#61; owner_cap.pool_address;
    assert_stake_pool_exists(pool_address);
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    // Cap amount to unlock by maximum active stake.
    let amount &#61; min(amount, coin::value(&amp;stake_pool.active));
    let unlocked_stake &#61; coin::extract(&amp;mut stake_pool.active, amount);
    coin::merge&lt;AptosCoin&gt;(&amp;mut stake_pool.pending_inactive, unlocked_stake);

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            UnlockStake &#123;
                pool_address,
                amount_unlocked: amount,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.unlock_stake_events,
        UnlockStakeEvent &#123;
            pool_address,
            amount_unlocked: amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_stake_withdraw"></a>

## Function `withdraw`

Withdraw from <code>account</code>'s inactive stake.


<pre><code>public entry fun withdraw(owner: &amp;signer, withdraw_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun withdraw(
    owner: &amp;signer,
    withdraw_amount: u64
) acquires OwnerCapability, StakePool, ValidatorSet &#123;
    let owner_address &#61; signer::address_of(owner);
    assert_owner_cap_exists(owner_address);
    let ownership_cap &#61; borrow_global&lt;OwnerCapability&gt;(owner_address);
    let coins &#61; withdraw_with_cap(ownership_cap, withdraw_amount);
    coin::deposit&lt;AptosCoin&gt;(owner_address, coins);
&#125;
</code></pre>



</details>

<a id="0x1_stake_withdraw_with_cap"></a>

## Function `withdraw_with_cap`

Withdraw from <code>pool_address</code>'s inactive stake with the corresponding <code>owner_cap</code>.


<pre><code>public fun withdraw_with_cap(owner_cap: &amp;stake::OwnerCapability, withdraw_amount: u64): coin::Coin&lt;aptos_coin::AptosCoin&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_with_cap(
    owner_cap: &amp;OwnerCapability,
    withdraw_amount: u64
): Coin&lt;AptosCoin&gt; acquires StakePool, ValidatorSet &#123;
    assert_reconfig_not_in_progress();
    let pool_address &#61; owner_cap.pool_address;
    assert_stake_pool_exists(pool_address);
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    // There&apos;s an edge case where a validator unlocks their stake and leaves the validator set before
    // the stake is fully unlocked (the current lockup cycle has not expired yet).
    // This can leave their stake stuck in pending_inactive even after the current lockup cycle expires.
    if (get_validator_state(pool_address) &#61;&#61; VALIDATOR_STATUS_INACTIVE &amp;&amp;
        timestamp::now_seconds() &gt;&#61; stake_pool.locked_until_secs) &#123;
        let pending_inactive_stake &#61; coin::extract_all(&amp;mut stake_pool.pending_inactive);
        coin::merge(&amp;mut stake_pool.inactive, pending_inactive_stake);
    &#125;;

    // Cap withdraw amount by total inactive coins.
    withdraw_amount &#61; min(withdraw_amount, coin::value(&amp;stake_pool.inactive));
    if (withdraw_amount &#61;&#61; 0) return coin::zero&lt;AptosCoin&gt;();

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            WithdrawStake &#123;
                pool_address,
                amount_withdrawn: withdraw_amount,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.withdraw_stake_events,
        WithdrawStakeEvent &#123;
            pool_address,
            amount_withdrawn: withdraw_amount,
        &#125;,
    );

    coin::extract(&amp;mut stake_pool.inactive, withdraw_amount)
&#125;
</code></pre>



</details>

<a id="0x1_stake_leave_validator_set"></a>

## Function `leave_validator_set`

Request to have <code>pool_address</code> leave the validator set. The validator is only actually removed from the set when
the next epoch starts.
The last validator in the set cannot leave. This is an edge case that should never happen as long as the network
is still operational.

Can only be called by the operator of the validator/staking pool.


<pre><code>public entry fun leave_validator_set(operator: &amp;signer, pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun leave_validator_set(
    operator: &amp;signer,
    pool_address: address
) acquires StakePool, ValidatorSet &#123;
    assert_reconfig_not_in_progress();
    let config &#61; staking_config::get();
    assert!(
        staking_config::get_allow_validator_set_change(&amp;config),
        error::invalid_argument(ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED),
    );

    assert_stake_pool_exists(pool_address);
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    // Account has to be the operator.
    assert!(signer::address_of(operator) &#61;&#61; stake_pool.operator_address, error::unauthenticated(ENOT_OPERATOR));

    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);
    // If the validator is still pending_active, directly kick the validator out.
    let maybe_pending_active_index &#61; find_validator(&amp;validator_set.pending_active, pool_address);
    if (option::is_some(&amp;maybe_pending_active_index)) &#123;
        vector::swap_remove(
            &amp;mut validator_set.pending_active, option::extract(&amp;mut maybe_pending_active_index));

        // Decrease the voting power increase as the pending validator&apos;s voting power was added when they requested
        // to join. Now that they changed their mind, their voting power should not affect the joining limit of this
        // epoch.
        let validator_stake &#61; (get_next_epoch_voting_power(stake_pool) as u128);
        // total_joining_power should be larger than validator_stake but just in case there has been a small
        // rounding error somewhere that can lead to an underflow, we still want to allow this transaction to
        // succeed.
        if (validator_set.total_joining_power &gt; validator_stake) &#123;
            validator_set.total_joining_power &#61; validator_set.total_joining_power &#45; validator_stake;
        &#125; else &#123;
            validator_set.total_joining_power &#61; 0;
        &#125;;
    &#125; else &#123;
        // Validate that the validator is already part of the validator set.
        let maybe_active_index &#61; find_validator(&amp;validator_set.active_validators, pool_address);
        assert!(option::is_some(&amp;maybe_active_index), error::invalid_state(ENOT_VALIDATOR));
        let validator_info &#61; vector::swap_remove(
            &amp;mut validator_set.active_validators, option::extract(&amp;mut maybe_active_index));
        assert!(vector::length(&amp;validator_set.active_validators) &gt; 0, error::invalid_state(ELAST_VALIDATOR));
        vector::push_back(&amp;mut validator_set.pending_inactive, validator_info);

        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(LeaveValidatorSet &#123; pool_address &#125;);
        &#125;;
        event::emit_event(
            &amp;mut stake_pool.leave_validator_set_events,
            LeaveValidatorSetEvent &#123;
                pool_address,
            &#125;,
        );
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_stake_is_current_epoch_validator"></a>

## Function `is_current_epoch_validator`

Returns true if the current validator can still vote in the current epoch.
This includes validators that requested to leave but are still in the pending_inactive queue and will be removed
when the epoch starts.


<pre><code>public fun is_current_epoch_validator(pool_address: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_current_epoch_validator(pool_address: address): bool acquires ValidatorSet &#123;
    assert_stake_pool_exists(pool_address);
    let validator_state &#61; get_validator_state(pool_address);
    validator_state &#61;&#61; VALIDATOR_STATUS_ACTIVE &#124;&#124; validator_state &#61;&#61; VALIDATOR_STATUS_PENDING_INACTIVE
&#125;
</code></pre>



</details>

<a id="0x1_stake_update_performance_statistics"></a>

## Function `update_performance_statistics`

Update the validator performance (proposal statistics). This is only called by block::prologue().
This function cannot abort.


<pre><code>public(friend) fun update_performance_statistics(proposer_index: option::Option&lt;u64&gt;, failed_proposer_indices: vector&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun update_performance_statistics(
    proposer_index: Option&lt;u64&gt;,
    failed_proposer_indices: vector&lt;u64&gt;
) acquires ValidatorPerformance &#123;
    // Validator set cannot change until the end of the epoch, so the validator index in arguments should
    // match with those of the validators in ValidatorPerformance resource.
    let validator_perf &#61; borrow_global_mut&lt;ValidatorPerformance&gt;(@aptos_framework);
    let validator_len &#61; vector::length(&amp;validator_perf.validators);

    spec &#123;
        update ghost_valid_perf &#61; validator_perf;
        update ghost_proposer_idx &#61; proposer_index;
    &#125;;
    // proposer_index is an option because it can be missing (for NilBlocks)
    if (option::is_some(&amp;proposer_index)) &#123;
        let cur_proposer_index &#61; option::extract(&amp;mut proposer_index);
        // Here, and in all other vector::borrow, skip any validator indices that are out of bounds,
        // this ensures that this function doesn&apos;t abort if there are out of bounds errors.
        if (cur_proposer_index &lt; validator_len) &#123;
            let validator &#61; vector::borrow_mut(&amp;mut validator_perf.validators, cur_proposer_index);
            spec &#123;
                assume validator.successful_proposals &#43; 1 &lt;&#61; MAX_U64;
            &#125;;
            validator.successful_proposals &#61; validator.successful_proposals &#43; 1;
        &#125;;
    &#125;;

    let f &#61; 0;
    let f_len &#61; vector::length(&amp;failed_proposer_indices);
    while (&#123;
        spec &#123;
            invariant len(validator_perf.validators) &#61;&#61; validator_len;
            invariant (option::spec_is_some(ghost_proposer_idx) &amp;&amp; option::spec_borrow(
                ghost_proposer_idx
            ) &lt; validator_len) &#61;&#61;&gt;
                (validator_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals &#61;&#61;
                    ghost_valid_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals &#43; 1);
        &#125;;
        f &lt; f_len
    &#125;) &#123;
        let validator_index &#61; &#42;vector::borrow(&amp;failed_proposer_indices, f);
        if (validator_index &lt; validator_len) &#123;
            let validator &#61; vector::borrow_mut(&amp;mut validator_perf.validators, validator_index);
            spec &#123;
                assume validator.failed_proposals &#43; 1 &lt;&#61; MAX_U64;
            &#125;;
            validator.failed_proposals &#61; validator.failed_proposals &#43; 1;
        &#125;;
        f &#61; f &#43; 1;
    &#125;;
&#125;
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


<pre><code>public(friend) fun on_new_epoch()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(
) acquires StakePool, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorFees &#123;
    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);
    let config &#61; staking_config::get();
    let validator_perf &#61; borrow_global_mut&lt;ValidatorPerformance&gt;(@aptos_framework);

    // Process pending stake and distribute transaction fees and rewards for each currently active validator.
    vector::for_each_ref(&amp;validator_set.active_validators, &#124;validator&#124; &#123;
        let validator: &amp;ValidatorInfo &#61; validator;
        update_stake_pool(validator_perf, validator.addr, &amp;config);
    &#125;);

    // Process pending stake and distribute transaction fees and rewards for each currently pending_inactive validator
    // (requested to leave but not removed yet).
    vector::for_each_ref(&amp;validator_set.pending_inactive, &#124;validator&#124; &#123;
        let validator: &amp;ValidatorInfo &#61; validator;
        update_stake_pool(validator_perf, validator.addr, &amp;config);
    &#125;);

    // Activate currently pending_active validators.
    append(&amp;mut validator_set.active_validators, &amp;mut validator_set.pending_active);

    // Officially deactivate all pending_inactive validators. They will now no longer receive rewards.
    validator_set.pending_inactive &#61; vector::empty();

    // Update active validator set so that network address/public key change takes effect.
    // Moreover, recalculate the total voting power, and deactivate the validator whose
    // voting power is less than the minimum required stake.
    let next_epoch_validators &#61; vector::empty();
    let (minimum_stake, _) &#61; staking_config::get_required_stake(&amp;config);
    let vlen &#61; vector::length(&amp;validator_set.active_validators);
    let total_voting_power &#61; 0;
    let i &#61; 0;
    while (&#123;
        spec &#123;
            invariant spec_validators_are_initialized(next_epoch_validators);
            invariant i &lt;&#61; vlen;
        &#125;;
        i &lt; vlen
    &#125;) &#123;
        let old_validator_info &#61; vector::borrow_mut(&amp;mut validator_set.active_validators, i);
        let pool_address &#61; old_validator_info.addr;
        let validator_config &#61; borrow_global_mut&lt;ValidatorConfig&gt;(pool_address);
        let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
        let new_validator_info &#61; generate_validator_info(pool_address, stake_pool, &#42;validator_config);

        // A validator needs at least the min stake required to join the validator set.
        if (new_validator_info.voting_power &gt;&#61; minimum_stake) &#123;
            spec &#123;
                assume total_voting_power &#43; new_validator_info.voting_power &lt;&#61; MAX_U128;
            &#125;;
            total_voting_power &#61; total_voting_power &#43; (new_validator_info.voting_power as u128);
            vector::push_back(&amp;mut next_epoch_validators, new_validator_info);
        &#125;;
        i &#61; i &#43; 1;
    &#125;;

    validator_set.active_validators &#61; next_epoch_validators;
    validator_set.total_voting_power &#61; total_voting_power;
    validator_set.total_joining_power &#61; 0;

    // Update validator indices, reset performance scores, and renew lockups.
    validator_perf.validators &#61; vector::empty();
    let recurring_lockup_duration_secs &#61; staking_config::get_recurring_lockup_duration(&amp;config);
    let vlen &#61; vector::length(&amp;validator_set.active_validators);
    let validator_index &#61; 0;
    while (&#123;
        spec &#123;
            invariant spec_validators_are_initialized(validator_set.active_validators);
            invariant len(validator_set.pending_active) &#61;&#61; 0;
            invariant len(validator_set.pending_inactive) &#61;&#61; 0;
            invariant 0 &lt;&#61; validator_index &amp;&amp; validator_index &lt;&#61; vlen;
            invariant vlen &#61;&#61; len(validator_set.active_validators);
            invariant forall i in 0..validator_index:
                global&lt;ValidatorConfig&gt;(validator_set.active_validators[i].addr).validator_index &lt; validator_index;
            invariant forall i in 0..validator_index:
                validator_set.active_validators[i].config.validator_index &lt; validator_index;
            invariant len(validator_perf.validators) &#61;&#61; validator_index;
        &#125;;
        validator_index &lt; vlen
    &#125;) &#123;
        let validator_info &#61; vector::borrow_mut(&amp;mut validator_set.active_validators, validator_index);
        validator_info.config.validator_index &#61; validator_index;
        let validator_config &#61; borrow_global_mut&lt;ValidatorConfig&gt;(validator_info.addr);
        validator_config.validator_index &#61; validator_index;

        vector::push_back(&amp;mut validator_perf.validators, IndividualValidatorPerformance &#123;
            successful_proposals: 0,
            failed_proposals: 0,
        &#125;);

        // Automatically renew a validator&apos;s lockup for validators that will still be in the validator set in the
        // next epoch.
        let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(validator_info.addr);
        let now_secs &#61; timestamp::now_seconds();
        let reconfig_start_secs &#61; if (chain_status::is_operating()) &#123;
            get_reconfig_start_time_secs()
        &#125; else &#123;
            now_secs
        &#125;;
        if (stake_pool.locked_until_secs &lt;&#61; reconfig_start_secs) &#123;
            spec &#123;
                assume now_secs &#43; recurring_lockup_duration_secs &lt;&#61; MAX_U64;
            &#125;;
            stake_pool.locked_until_secs &#61; now_secs &#43; recurring_lockup_duration_secs;
        &#125;;

        validator_index &#61; validator_index &#43; 1;
    &#125;;

    if (features::periodical_reward_rate_decrease_enabled()) &#123;
        // Update rewards rate after reward distribution.
        staking_config::calculate_and_save_latest_epoch_rewards_rate();
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_stake_cur_validator_consensus_infos"></a>

## Function `cur_validator_consensus_infos`

Return the <code>ValidatorConsensusInfo</code> of each current validator, sorted by current validator index.


<pre><code>public fun cur_validator_consensus_infos(): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun cur_validator_consensus_infos(): vector&lt;ValidatorConsensusInfo&gt; acquires ValidatorSet &#123;
    let validator_set &#61; borrow_global&lt;ValidatorSet&gt;(@aptos_framework);
    validator_consensus_infos_from_validator_set(validator_set)
&#125;
</code></pre>



</details>

<a id="0x1_stake_next_validator_consensus_infos"></a>

## Function `next_validator_consensus_infos`



<pre><code>public fun next_validator_consensus_infos(): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun next_validator_consensus_infos(): vector&lt;ValidatorConsensusInfo&gt; acquires ValidatorSet, ValidatorPerformance, StakePool, ValidatorFees, ValidatorConfig &#123;
    // Init.
    let cur_validator_set &#61; borrow_global&lt;ValidatorSet&gt;(@aptos_framework);
    let staking_config &#61; staking_config::get();
    let validator_perf &#61; borrow_global&lt;ValidatorPerformance&gt;(@aptos_framework);
    let (minimum_stake, _) &#61; staking_config::get_required_stake(&amp;staking_config);
    let (rewards_rate, rewards_rate_denominator) &#61; staking_config::get_reward_rate(&amp;staking_config);

    // Compute new validator set.
    let new_active_validators &#61; vector[];
    let num_new_actives &#61; 0;
    let candidate_idx &#61; 0;
    let new_total_power &#61; 0;
    let num_cur_actives &#61; vector::length(&amp;cur_validator_set.active_validators);
    let num_cur_pending_actives &#61; vector::length(&amp;cur_validator_set.pending_active);
    spec &#123;
        assume num_cur_actives &#43; num_cur_pending_actives &lt;&#61; MAX_U64;
    &#125;;
    let num_candidates &#61; num_cur_actives &#43; num_cur_pending_actives;
    while (&#123;
        spec &#123;
            invariant candidate_idx &lt;&#61; num_candidates;
            invariant spec_validators_are_initialized(new_active_validators);
            invariant len(new_active_validators) &#61;&#61; num_new_actives;
            invariant forall i in 0..len(new_active_validators):
                new_active_validators[i].config.validator_index &#61;&#61; i;
            invariant num_new_actives &lt;&#61; candidate_idx;
            invariant spec_validators_are_initialized(new_active_validators);
        &#125;;
        candidate_idx &lt; num_candidates
    &#125;) &#123;
        let candidate_in_current_validator_set &#61; candidate_idx &lt; num_cur_actives;
        let candidate &#61; if (candidate_idx &lt; num_cur_actives) &#123;
            vector::borrow(&amp;cur_validator_set.active_validators, candidate_idx)
        &#125; else &#123;
            vector::borrow(&amp;cur_validator_set.pending_active, candidate_idx &#45; num_cur_actives)
        &#125;;
        let stake_pool &#61; borrow_global&lt;StakePool&gt;(candidate.addr);
        let cur_active &#61; coin::value(&amp;stake_pool.active);
        let cur_pending_active &#61; coin::value(&amp;stake_pool.pending_active);
        let cur_pending_inactive &#61; coin::value(&amp;stake_pool.pending_inactive);

        let cur_reward &#61; if (candidate_in_current_validator_set &amp;&amp; cur_active &gt; 0) &#123;
            spec &#123;
                assert candidate.config.validator_index &lt; len(validator_perf.validators);
            &#125;;
            let cur_perf &#61; vector::borrow(&amp;validator_perf.validators, candidate.config.validator_index);
            spec &#123;
                assume cur_perf.successful_proposals &#43; cur_perf.failed_proposals &lt;&#61; MAX_U64;
            &#125;;
            calculate_rewards_amount(cur_active, cur_perf.successful_proposals, cur_perf.successful_proposals &#43; cur_perf.failed_proposals, rewards_rate, rewards_rate_denominator)
        &#125; else &#123;
            0
        &#125;;

        let cur_fee &#61; 0;
        if (features::collect_and_distribute_gas_fees()) &#123;
            let fees_table &#61; &amp;borrow_global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;
            if (table::contains(fees_table, candidate.addr)) &#123;
                let fee_coin &#61; table::borrow(fees_table, candidate.addr);
                cur_fee &#61; coin::value(fee_coin);
            &#125;
        &#125;;

        let lockup_expired &#61; get_reconfig_start_time_secs() &gt;&#61; stake_pool.locked_until_secs;
        spec &#123;
            assume cur_active &#43; cur_pending_active &#43; cur_reward &#43; cur_fee &lt;&#61; MAX_U64;
            assume cur_active &#43; cur_pending_inactive &#43; cur_pending_active &#43; cur_reward &#43; cur_fee &lt;&#61; MAX_U64;
        &#125;;
        let new_voting_power &#61;
            cur_active
            &#43; if (lockup_expired) &#123; 0 &#125; else &#123; cur_pending_inactive &#125;
            &#43; cur_pending_active
            &#43; cur_reward &#43; cur_fee;

        if (new_voting_power &gt;&#61; minimum_stake) &#123;
            let config &#61; &#42;borrow_global&lt;ValidatorConfig&gt;(candidate.addr);
            config.validator_index &#61; num_new_actives;
            let new_validator_info &#61; ValidatorInfo &#123;
                addr: candidate.addr,
                voting_power: new_voting_power,
                config,
            &#125;;

            // Update ValidatorSet.
            spec &#123;
                assume new_total_power &#43; new_voting_power &lt;&#61; MAX_U128;
            &#125;;
            new_total_power &#61; new_total_power &#43; (new_voting_power as u128);
            vector::push_back(&amp;mut new_active_validators, new_validator_info);
            num_new_actives &#61; num_new_actives &#43; 1;

        &#125;;
        candidate_idx &#61; candidate_idx &#43; 1;
    &#125;;

    let new_validator_set &#61; ValidatorSet &#123;
        consensus_scheme: cur_validator_set.consensus_scheme,
        active_validators: new_active_validators,
        pending_inactive: vector[],
        pending_active: vector[],
        total_voting_power: new_total_power,
        total_joining_power: 0,
    &#125;;

    validator_consensus_infos_from_validator_set(&amp;new_validator_set)
&#125;
</code></pre>



</details>

<a id="0x1_stake_validator_consensus_infos_from_validator_set"></a>

## Function `validator_consensus_infos_from_validator_set`



<pre><code>fun validator_consensus_infos_from_validator_set(validator_set: &amp;stake::ValidatorSet): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validator_consensus_infos_from_validator_set(validator_set: &amp;ValidatorSet): vector&lt;ValidatorConsensusInfo&gt; &#123;
    let validator_consensus_infos &#61; vector[];

    let num_active &#61; vector::length(&amp;validator_set.active_validators);
    let num_pending_inactive &#61; vector::length(&amp;validator_set.pending_inactive);
    spec &#123;
        assume num_active &#43; num_pending_inactive &lt;&#61; MAX_U64;
    &#125;;
    let total &#61; num_active &#43; num_pending_inactive;

    // Pre&#45;fill the return value with dummy values.
    let idx &#61; 0;
    while (&#123;
        spec &#123;
            invariant idx &lt;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);
            invariant len(validator_consensus_infos) &#61;&#61; idx;
            invariant len(validator_consensus_infos) &lt;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);
        &#125;;
        idx &lt; total
    &#125;) &#123;
        vector::push_back(&amp;mut validator_consensus_infos, validator_consensus_info::default());
        idx &#61; idx &#43; 1;
    &#125;;
    spec &#123;
        assert len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);
        assert spec_validator_indices_are_valid_config(validator_set.active_validators,
            len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));
    &#125;;

    vector::for_each_ref(&amp;validator_set.active_validators, &#124;obj&#124; &#123;
        let vi: &amp;ValidatorInfo &#61; obj;
        spec &#123;
            assume len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);
            assert vi.config.validator_index &lt; len(validator_consensus_infos);
        &#125;;
        let vci &#61; vector::borrow_mut(&amp;mut validator_consensus_infos, vi.config.validator_index);
        &#42;vci &#61; validator_consensus_info::new(
            vi.addr,
            vi.config.consensus_pubkey,
            vi.voting_power
        );
        spec &#123;
            assert len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);
        &#125;;
    &#125;);

    vector::for_each_ref(&amp;validator_set.pending_inactive, &#124;obj&#124; &#123;
        let vi: &amp;ValidatorInfo &#61; obj;
        spec &#123;
            assume len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);
            assert vi.config.validator_index &lt; len(validator_consensus_infos);
        &#125;;
        let vci &#61; vector::borrow_mut(&amp;mut validator_consensus_infos, vi.config.validator_index);
        &#42;vci &#61; validator_consensus_info::new(
            vi.addr,
            vi.config.consensus_pubkey,
            vi.voting_power
        );
        spec &#123;
            assert len(validator_consensus_infos) &#61;&#61; len(validator_set.active_validators) &#43; len(validator_set.pending_inactive);
        &#125;;
    &#125;);

    validator_consensus_infos
&#125;
</code></pre>



</details>

<a id="0x1_stake_addresses_from_validator_infos"></a>

## Function `addresses_from_validator_infos`



<pre><code>fun addresses_from_validator_infos(infos: &amp;vector&lt;stake::ValidatorInfo&gt;): vector&lt;address&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun addresses_from_validator_infos(infos: &amp;vector&lt;ValidatorInfo&gt;): vector&lt;address&gt; &#123;
    vector::map_ref(infos, &#124;obj&#124; &#123;
        let info: &amp;ValidatorInfo &#61; obj;
        info.addr
    &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_stake_update_stake_pool"></a>

## Function `update_stake_pool`

Calculate the stake amount of a stake pool for the next epoch.
Update individual validator's stake pool if <code>commit &#61;&#61; true</code>.

1. distribute transaction fees to active/pending_inactive delegations
2. distribute rewards to active/pending_inactive delegations
3. process pending_active, pending_inactive correspondingly
This function shouldn't abort.


<pre><code>fun update_stake_pool(validator_perf: &amp;stake::ValidatorPerformance, pool_address: address, staking_config: &amp;staking_config::StakingConfig)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_stake_pool(
    validator_perf: &amp;ValidatorPerformance,
    pool_address: address,
    staking_config: &amp;StakingConfig,
) acquires StakePool, AptosCoinCapabilities, ValidatorConfig, ValidatorFees &#123;
    let stake_pool &#61; borrow_global_mut&lt;StakePool&gt;(pool_address);
    let validator_config &#61; borrow_global&lt;ValidatorConfig&gt;(pool_address);
    let cur_validator_perf &#61; vector::borrow(&amp;validator_perf.validators, validator_config.validator_index);
    let num_successful_proposals &#61; cur_validator_perf.successful_proposals;
    spec &#123;
        // The following addition should not overflow because `num_total_proposals` cannot be larger than 86400,
        // the maximum number of proposals in a day (1 proposal per second).
        assume cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals &lt;&#61; MAX_U64;
    &#125;;
    let num_total_proposals &#61; cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals;
    let (rewards_rate, rewards_rate_denominator) &#61; staking_config::get_reward_rate(staking_config);
    let rewards_active &#61; distribute_rewards(
        &amp;mut stake_pool.active,
        num_successful_proposals,
        num_total_proposals,
        rewards_rate,
        rewards_rate_denominator
    );
    let rewards_pending_inactive &#61; distribute_rewards(
        &amp;mut stake_pool.pending_inactive,
        num_successful_proposals,
        num_total_proposals,
        rewards_rate,
        rewards_rate_denominator
    );
    spec &#123;
        assume rewards_active &#43; rewards_pending_inactive &lt;&#61; MAX_U64;
    &#125;;
    let rewards_amount &#61; rewards_active &#43; rewards_pending_inactive;
    // Pending active stake can now be active.
    coin::merge(&amp;mut stake_pool.active, coin::extract_all(&amp;mut stake_pool.pending_active));

    // Additionally, distribute transaction fees.
    if (features::collect_and_distribute_gas_fees()) &#123;
        let fees_table &#61; &amp;mut borrow_global_mut&lt;ValidatorFees&gt;(@aptos_framework).fees_table;
        if (table::contains(fees_table, pool_address)) &#123;
            let coin &#61; table::remove(fees_table, pool_address);
            coin::merge(&amp;mut stake_pool.active, coin);
        &#125;;
    &#125;;

    // Pending inactive stake is only fully unlocked and moved into inactive if the current lockup cycle has expired
    let current_lockup_expiration &#61; stake_pool.locked_until_secs;
    if (get_reconfig_start_time_secs() &gt;&#61; current_lockup_expiration) &#123;
        coin::merge(
            &amp;mut stake_pool.inactive,
            coin::extract_all(&amp;mut stake_pool.pending_inactive),
        );
    &#125;;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(DistributeRewards &#123; pool_address, rewards_amount &#125;);
    &#125;;
    event::emit_event(
        &amp;mut stake_pool.distribute_rewards_events,
        DistributeRewardsEvent &#123;
            pool_address,
            rewards_amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_reconfig_start_time_secs"></a>

## Function `get_reconfig_start_time_secs`

Assuming we are in a middle of a reconfiguration (no matter it is immediate or async), get its start time.


<pre><code>fun get_reconfig_start_time_secs(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_reconfig_start_time_secs(): u64 &#123;
    if (reconfiguration_state::is_initialized()) &#123;
        reconfiguration_state::start_time_secs()
    &#125; else &#123;
        timestamp::now_seconds()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_calculate_rewards_amount"></a>

## Function `calculate_rewards_amount`

Calculate the rewards amount.


<pre><code>fun calculate_rewards_amount(stake_amount: u64, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_rewards_amount(
    stake_amount: u64,
    num_successful_proposals: u64,
    num_total_proposals: u64,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
): u64 &#123;
    spec &#123;
        // The following condition must hold because
        // (1) num_successful_proposals &lt;&#61; num_total_proposals, and
        // (2) `num_total_proposals` cannot be larger than 86400, the maximum number of proposals
        //     in a day (1 proposal per second), and `num_total_proposals` is reset to 0 every epoch.
        assume num_successful_proposals &#42; MAX_REWARDS_RATE &lt;&#61; MAX_U64;
    &#125;;
    // The rewards amount is equal to (stake amount &#42; rewards rate &#42; performance multiplier).
    // We do multiplication in u128 before division to avoid the overflow and minimize the rounding error.
    let rewards_numerator &#61; (stake_amount as u128) &#42; (rewards_rate as u128) &#42; (num_successful_proposals as u128);
    let rewards_denominator &#61; (rewards_rate_denominator as u128) &#42; (num_total_proposals as u128);
    if (rewards_denominator &gt; 0) &#123;
        ((rewards_numerator / rewards_denominator) as u64)
    &#125; else &#123;
        0
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_distribute_rewards"></a>

## Function `distribute_rewards`

Mint rewards corresponding to current epoch's <code>stake</code> and <code>num_successful_votes</code>.


<pre><code>fun distribute_rewards(stake: &amp;mut coin::Coin&lt;aptos_coin::AptosCoin&gt;, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun distribute_rewards(
    stake: &amp;mut Coin&lt;AptosCoin&gt;,
    num_successful_proposals: u64,
    num_total_proposals: u64,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
): u64 acquires AptosCoinCapabilities &#123;
    let stake_amount &#61; coin::value(stake);
    let rewards_amount &#61; if (stake_amount &gt; 0) &#123;
        calculate_rewards_amount(
            stake_amount,
            num_successful_proposals,
            num_total_proposals,
            rewards_rate,
            rewards_rate_denominator
        )
    &#125; else &#123;
        0
    &#125;;
    if (rewards_amount &gt; 0) &#123;
        let mint_cap &#61; &amp;borrow_global&lt;AptosCoinCapabilities&gt;(@aptos_framework).mint_cap;
        let rewards &#61; coin::mint(rewards_amount, mint_cap);
        coin::merge(stake, rewards);
    &#125;;
    rewards_amount
&#125;
</code></pre>



</details>

<a id="0x1_stake_append"></a>

## Function `append`



<pre><code>fun append&lt;T&gt;(v1: &amp;mut vector&lt;T&gt;, v2: &amp;mut vector&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun append&lt;T&gt;(v1: &amp;mut vector&lt;T&gt;, v2: &amp;mut vector&lt;T&gt;) &#123;
    while (!vector::is_empty(v2)) &#123;
        vector::push_back(v1, vector::pop_back(v2));
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_find_validator"></a>

## Function `find_validator`



<pre><code>fun find_validator(v: &amp;vector&lt;stake::ValidatorInfo&gt;, addr: address): option::Option&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun find_validator(v: &amp;vector&lt;ValidatorInfo&gt;, addr: address): Option&lt;u64&gt; &#123;
    let i &#61; 0;
    let len &#61; vector::length(v);
    while (&#123;
        spec &#123;
            invariant !(exists j in 0..i: v[j].addr &#61;&#61; addr);
        &#125;;
        i &lt; len
    &#125;) &#123;
        if (vector::borrow(v, i).addr &#61;&#61; addr) &#123;
            return option::some(i)
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
    option::none()
&#125;
</code></pre>



</details>

<a id="0x1_stake_generate_validator_info"></a>

## Function `generate_validator_info`



<pre><code>fun generate_validator_info(addr: address, stake_pool: &amp;stake::StakePool, config: stake::ValidatorConfig): stake::ValidatorInfo
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun generate_validator_info(addr: address, stake_pool: &amp;StakePool, config: ValidatorConfig): ValidatorInfo &#123;
    let voting_power &#61; get_next_epoch_voting_power(stake_pool);
    ValidatorInfo &#123;
        addr,
        voting_power,
        config,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_get_next_epoch_voting_power"></a>

## Function `get_next_epoch_voting_power`

Returns validator's next epoch voting power, including pending_active, active, and pending_inactive stake.


<pre><code>fun get_next_epoch_voting_power(stake_pool: &amp;stake::StakePool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_next_epoch_voting_power(stake_pool: &amp;StakePool): u64 &#123;
    let value_pending_active &#61; coin::value(&amp;stake_pool.pending_active);
    let value_active &#61; coin::value(&amp;stake_pool.active);
    let value_pending_inactive &#61; coin::value(&amp;stake_pool.pending_inactive);
    spec &#123;
        assume value_pending_active &#43; value_active &#43; value_pending_inactive &lt;&#61; MAX_U64;
    &#125;;
    value_pending_active &#43; value_active &#43; value_pending_inactive
&#125;
</code></pre>



</details>

<a id="0x1_stake_update_voting_power_increase"></a>

## Function `update_voting_power_increase`



<pre><code>fun update_voting_power_increase(increase_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_voting_power_increase(increase_amount: u64) acquires ValidatorSet &#123;
    let validator_set &#61; borrow_global_mut&lt;ValidatorSet&gt;(@aptos_framework);
    let voting_power_increase_limit &#61;
        (staking_config::get_voting_power_increase_limit(&amp;staking_config::get()) as u128);
    validator_set.total_joining_power &#61; validator_set.total_joining_power &#43; (increase_amount as u128);

    // Only validator voting power increase if the current validator set&apos;s voting power &gt; 0.
    if (validator_set.total_voting_power &gt; 0) &#123;
        assert!(
            validator_set.total_joining_power &lt;&#61; validator_set.total_voting_power &#42; voting_power_increase_limit / 100,
            error::invalid_argument(EVOTING_POWER_INCREASE_EXCEEDS_LIMIT),
        );
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_assert_stake_pool_exists"></a>

## Function `assert_stake_pool_exists`



<pre><code>fun assert_stake_pool_exists(pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_stake_pool_exists(pool_address: address) &#123;
    assert!(stake_pool_exists(pool_address), error::invalid_argument(ESTAKE_POOL_DOES_NOT_EXIST));
&#125;
</code></pre>



</details>

<a id="0x1_stake_configure_allowed_validators"></a>

## Function `configure_allowed_validators`



<pre><code>public fun configure_allowed_validators(aptos_framework: &amp;signer, accounts: vector&lt;address&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun configure_allowed_validators(
    aptos_framework: &amp;signer,
    accounts: vector&lt;address&gt;
) acquires AllowedValidators &#123;
    let aptos_framework_address &#61; signer::address_of(aptos_framework);
    system_addresses::assert_aptos_framework(aptos_framework);
    if (!exists&lt;AllowedValidators&gt;(aptos_framework_address)) &#123;
        move_to(aptos_framework, AllowedValidators &#123; accounts &#125;);
    &#125; else &#123;
        let allowed &#61; borrow_global_mut&lt;AllowedValidators&gt;(aptos_framework_address);
        allowed.accounts &#61; accounts;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_is_allowed"></a>

## Function `is_allowed`



<pre><code>fun is_allowed(account: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_allowed(account: address): bool acquires AllowedValidators &#123;
    if (!exists&lt;AllowedValidators&gt;(@aptos_framework)) &#123;
        true
    &#125; else &#123;
        let allowed &#61; borrow_global&lt;AllowedValidators&gt;(@aptos_framework);
        vector::contains(&amp;allowed.accounts, &amp;account)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_stake_assert_owner_cap_exists"></a>

## Function `assert_owner_cap_exists`



<pre><code>fun assert_owner_cap_exists(owner: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_owner_cap_exists(owner: address) &#123;
    assert!(exists&lt;OwnerCapability&gt;(owner), error::not_found(EOWNER_CAP_NOT_FOUND));
&#125;
</code></pre>



</details>

<a id="0x1_stake_assert_reconfig_not_in_progress"></a>

## Function `assert_reconfig_not_in_progress`



<pre><code>fun assert_reconfig_not_in_progress()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_reconfig_not_in_progress() &#123;
    assert!(!reconfiguration_state::is_in_progress(), error::invalid_state(ERECONFIGURATION_IN_PROGRESS));
&#125;
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


<pre><code>pragma verify &#61; true;
invariant [suspendable] exists&lt;ValidatorSet&gt;(@aptos_framework) &#61;&#61;&gt; validator_set_is_valid();
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;ValidatorPerformance&gt;(@aptos_framework);
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;ValidatorSet&gt;(@aptos_framework);
apply ValidatorOwnerNoChange to &#42;;
apply ValidatorNotChangeDuringReconfig to &#42; except on_new_epoch;
apply StakePoolNotChangeDuringReconfig to &#42; except on_new_epoch, update_stake_pool;
<a id="0x1_stake_ghost_valid_perf"></a>
global ghost_valid_perf: ValidatorPerformance;
<a id="0x1_stake_ghost_proposer_idx"></a>
global ghost_proposer_idx: Option&lt;u64&gt;;
<a id="0x1_stake_ghost_active_num"></a>
global ghost_active_num: u64;
<a id="0x1_stake_ghost_pending_inactive_num"></a>
global ghost_pending_inactive_num: u64;
</code></pre>



<a id="@Specification_1_ValidatorSet"></a>

### Resource `ValidatorSet`


<pre><code>struct ValidatorSet has copy, drop, store, key
</code></pre>



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
invariant consensus_scheme &#61;&#61; 0;
</code></pre>




<a id="0x1_stake_ValidatorNotChangeDuringReconfig"></a>


<pre><code>schema ValidatorNotChangeDuringReconfig &#123;
    ensures (reconfiguration_state::spec_is_in_progress() &amp;&amp; old(exists&lt;ValidatorSet&gt;(@aptos_framework))) &#61;&#61;&gt;
        old(global&lt;ValidatorSet&gt;(@aptos_framework)) &#61;&#61; global&lt;ValidatorSet&gt;(@aptos_framework);
&#125;
</code></pre>




<a id="0x1_stake_StakePoolNotChangeDuringReconfig"></a>


<pre><code>schema StakePoolNotChangeDuringReconfig &#123;
    ensures forall a: address where old(exists&lt;StakePool&gt;(a)): reconfiguration_state::spec_is_in_progress() &#61;&#61;&gt;
        (old(global&lt;StakePool&gt;(a).pending_inactive) &#61;&#61; global&lt;StakePool&gt;(a).pending_inactive &amp;&amp;
        old(global&lt;StakePool&gt;(a).pending_active) &#61;&#61; global&lt;StakePool&gt;(a).pending_active &amp;&amp;
        old(global&lt;StakePool&gt;(a).inactive) &#61;&#61; global&lt;StakePool&gt;(a).inactive &amp;&amp;
        old(global&lt;StakePool&gt;(a).active) &#61;&#61; global&lt;StakePool&gt;(a).active);
&#125;
</code></pre>




<a id="0x1_stake_ValidatorOwnerNoChange"></a>


<pre><code>schema ValidatorOwnerNoChange &#123;
    // This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
    ensures forall addr: address where old(exists&lt;OwnerCapability&gt;(addr)):
        old(global&lt;OwnerCapability&gt;(addr)).pool_address &#61;&#61; global&lt;OwnerCapability&gt;(addr).pool_address;
&#125;
</code></pre>




<a id="0x1_stake_StakedValueNochange"></a>


<pre><code>schema StakedValueNochange &#123;
    pool_address: address;
    let stake_pool &#61; global&lt;StakePool&gt;(pool_address);
    let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
    // This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    ensures stake_pool.active.value &#43; stake_pool.inactive.value &#43; stake_pool.pending_active.value &#43; stake_pool.pending_inactive.value &#61;&#61;
        post_stake_pool.active.value &#43; post_stake_pool.inactive.value &#43; post_stake_pool.pending_active.value &#43; post_stake_pool.pending_inactive.value;
&#125;
</code></pre>




<a id="0x1_stake_validator_set_is_valid"></a>


<pre><code>fun validator_set_is_valid(): bool &#123;
   let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
   validator_set_is_valid_impl(validator_set)
&#125;
</code></pre>




<a id="0x1_stake_validator_set_is_valid_impl"></a>


<pre><code>fun validator_set_is_valid_impl(validator_set: ValidatorSet): bool &#123;
   spec_validators_are_initialized(validator_set.active_validators) &amp;&amp;
       spec_validators_are_initialized(validator_set.pending_inactive) &amp;&amp;
       spec_validators_are_initialized(validator_set.pending_active) &amp;&amp;
       spec_validator_indices_are_valid(validator_set.active_validators) &amp;&amp;
       spec_validator_indices_are_valid(validator_set.pending_inactive)
       &amp;&amp; spec_validator_indices_active_pending_inactive(validator_set)
&#125;
</code></pre>



<a id="@Specification_1_initialize_validator_fees"></a>

### Function `initialize_validator_fees`


<pre><code>public(friend) fun initialize_validator_fees(aptos_framework: &amp;signer)
</code></pre>




<pre><code>let aptos_addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
aborts_if exists&lt;ValidatorFees&gt;(aptos_addr);
ensures exists&lt;ValidatorFees&gt;(aptos_addr);
</code></pre>



<a id="@Specification_1_add_transaction_fee"></a>

### Function `add_transaction_fee`


<pre><code>public(friend) fun add_transaction_fee(validator_addr: address, fee: coin::Coin&lt;aptos_coin::AptosCoin&gt;)
</code></pre>




<pre><code>aborts_if !exists&lt;ValidatorFees&gt;(@aptos_framework);
let fees_table &#61; global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;
let post post_fees_table &#61; global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;
let collected_fee &#61; table::spec_get(fees_table, validator_addr);
let post post_collected_fee &#61; table::spec_get(post_fees_table, validator_addr);
ensures if (table::spec_contains(fees_table, validator_addr)) &#123;
    post_collected_fee.value &#61;&#61; collected_fee.value &#43; fee.value
&#125; else &#123;
    table::spec_contains(post_fees_table, validator_addr) &amp;&amp;
    table::spec_get(post_fees_table, validator_addr) &#61;&#61; fee
&#125;;
</code></pre>



<a id="@Specification_1_get_validator_state"></a>

### Function `get_validator_state`


<pre><code>&#35;[view]
public fun get_validator_state(pool_address: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);
let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
ensures result &#61;&#61; VALIDATOR_STATUS_PENDING_ACTIVE &#61;&#61;&gt; spec_contains(validator_set.pending_active, pool_address);
ensures result &#61;&#61; VALIDATOR_STATUS_ACTIVE &#61;&#61;&gt; spec_contains(validator_set.active_validators, pool_address);
ensures result &#61;&#61; VALIDATOR_STATUS_PENDING_INACTIVE &#61;&#61;&gt; spec_contains(validator_set.pending_inactive, pool_address);
ensures result &#61;&#61; VALIDATOR_STATUS_INACTIVE &#61;&#61;&gt; (
    !spec_contains(validator_set.pending_active, pool_address)
        &amp;&amp; !spec_contains(validator_set.active_validators, pool_address)
        &amp;&amp; !spec_contains(validator_set.pending_inactive, pool_address)
);
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)
</code></pre>




<pre><code>pragma disable_invariants_in_body;
let aptos_addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
aborts_if exists&lt;ValidatorSet&gt;(aptos_addr);
aborts_if exists&lt;ValidatorPerformance&gt;(aptos_addr);
ensures exists&lt;ValidatorSet&gt;(aptos_addr);
ensures global&lt;ValidatorSet&gt;(aptos_addr).consensus_scheme &#61;&#61; 0;
ensures exists&lt;ValidatorPerformance&gt;(aptos_addr);
</code></pre>



<a id="@Specification_1_remove_validators"></a>

### Function `remove_validators`


<pre><code>public fun remove_validators(aptos_framework: &amp;signer, validators: &amp;vector&lt;address&gt;)
</code></pre>




<pre><code>requires chain_status::is_operating();
let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
let post post_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
let active_validators &#61; validator_set.active_validators;
let post post_active_validators &#61; post_validator_set.active_validators;
let pending_inactive_validators &#61; validator_set.pending_inactive;
let post post_pending_inactive_validators &#61; post_validator_set.pending_inactive;
invariant len(active_validators) &gt; 0;
ensures len(active_validators) &#43; len(pending_inactive_validators) &#61;&#61; len(post_active_validators)
    &#43; len(post_pending_inactive_validators);
</code></pre>



<a id="@Specification_1_initialize_stake_owner"></a>

### Function `initialize_stake_owner`


<pre><code>public entry fun initialize_stake_owner(owner: &amp;signer, initial_stake_amount: u64, operator: address, voter: address)
</code></pre>




<pre><code>include ResourceRequirement;
let addr &#61; signer::address_of(owner);
ensures global&lt;ValidatorConfig&gt;(addr) &#61;&#61; ValidatorConfig &#123;
    consensus_pubkey: vector::empty(),
    network_addresses: vector::empty(),
    fullnode_addresses: vector::empty(),
    validator_index: 0,
&#125;;
ensures global&lt;OwnerCapability&gt;(addr) &#61;&#61; OwnerCapability &#123; pool_address: addr &#125;;
let post stakepool &#61; global&lt;StakePool&gt;(addr);
let post active &#61; stakepool.active.value;
let post pending_active &#61; stakepool.pending_active.value;
ensures spec_is_current_epoch_validator(addr) &#61;&#61;&gt;
    pending_active &#61;&#61; initial_stake_amount;
ensures !spec_is_current_epoch_validator(addr) &#61;&#61;&gt;
    active &#61;&#61; initial_stake_amount;
</code></pre>



<a id="@Specification_1_initialize_validator"></a>

### Function `initialize_validator`


<pre><code>public entry fun initialize_validator(account: &amp;signer, consensus_pubkey: vector&lt;u8&gt;, proof_of_possession: vector&lt;u8&gt;, network_addresses: vector&lt;u8&gt;, fullnode_addresses: vector&lt;u8&gt;)
</code></pre>




<pre><code>let pubkey_from_pop &#61; bls12381::spec_public_key_from_bytes_with_pop(
    consensus_pubkey,
    proof_of_possession_from_bytes(proof_of_possession)
);
aborts_if !option::spec_is_some(pubkey_from_pop);
let addr &#61; signer::address_of(account);
let post_addr &#61; signer::address_of(account);
let allowed &#61; global&lt;AllowedValidators&gt;(@aptos_framework);
aborts_if exists&lt;ValidatorConfig&gt;(addr);
aborts_if exists&lt;AllowedValidators&gt;(@aptos_framework) &amp;&amp; !vector::spec_contains(allowed.accounts, addr);
aborts_if stake_pool_exists(addr);
aborts_if exists&lt;OwnerCapability&gt;(addr);
aborts_if !exists&lt;account::Account&gt;(addr);
aborts_if global&lt;account::Account&gt;(addr).guid_creation_num &#43; 12 &gt; MAX_U64;
aborts_if global&lt;account::Account&gt;(addr).guid_creation_num &#43; 12 &gt;&#61; account::MAX_GUID_CREATION_NUM;
ensures exists&lt;StakePool&gt;(post_addr);
ensures global&lt;OwnerCapability&gt;(post_addr) &#61;&#61; OwnerCapability &#123; pool_address: post_addr &#125;;
ensures global&lt;ValidatorConfig&gt;(post_addr) &#61;&#61; ValidatorConfig &#123;
    consensus_pubkey,
    network_addresses,
    fullnode_addresses,
    validator_index: 0,
&#125;;
</code></pre>



<a id="@Specification_1_extract_owner_cap"></a>

### Function `extract_owner_cap`


<pre><code>public fun extract_owner_cap(owner: &amp;signer): stake::OwnerCapability
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
let owner_address &#61; signer::address_of(owner);
aborts_if !exists&lt;OwnerCapability&gt;(owner_address);
ensures !exists&lt;OwnerCapability&gt;(owner_address);
</code></pre>



<a id="@Specification_1_deposit_owner_cap"></a>

### Function `deposit_owner_cap`


<pre><code>public fun deposit_owner_cap(owner: &amp;signer, owner_cap: stake::OwnerCapability)
</code></pre>




<pre><code>let owner_address &#61; signer::address_of(owner);
aborts_if exists&lt;OwnerCapability&gt;(owner_address);
ensures exists&lt;OwnerCapability&gt;(owner_address);
ensures global&lt;OwnerCapability&gt;(owner_address) &#61;&#61; owner_cap;
</code></pre>



<a id="@Specification_1_set_operator_with_cap"></a>

### Function `set_operator_with_cap`


<pre><code>public fun set_operator_with_cap(owner_cap: &amp;stake::OwnerCapability, new_operator: address)
</code></pre>




<pre><code>let pool_address &#61; owner_cap.pool_address;
let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
modifies global&lt;StakePool&gt;(pool_address);
include StakedValueNochange;
ensures post_stake_pool.operator_address &#61;&#61; new_operator;
</code></pre>



<a id="@Specification_1_set_delegated_voter_with_cap"></a>

### Function `set_delegated_voter_with_cap`


<pre><code>public fun set_delegated_voter_with_cap(owner_cap: &amp;stake::OwnerCapability, new_voter: address)
</code></pre>




<pre><code>let pool_address &#61; owner_cap.pool_address;
let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
include StakedValueNochange;
aborts_if !exists&lt;StakePool&gt;(pool_address);
modifies global&lt;StakePool&gt;(pool_address);
ensures post_stake_pool.delegated_voter &#61;&#61; new_voter;
</code></pre>



<a id="@Specification_1_add_stake"></a>

### Function `add_stake`


<pre><code>public entry fun add_stake(owner: &amp;signer, amount: u64)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
pragma aborts_if_is_partial;
aborts_if reconfiguration_state::spec_is_in_progress();
include ResourceRequirement;
include AddStakeAbortsIfAndEnsures;
</code></pre>



<a id="@Specification_1_add_stake_with_cap"></a>

### Function `add_stake_with_cap`


<pre><code>public fun add_stake_with_cap(owner_cap: &amp;stake::OwnerCapability, coins: coin::Coin&lt;aptos_coin::AptosCoin&gt;)
</code></pre>




<pre><code>pragma disable_invariants_in_body;
pragma verify_duration_estimate &#61; 300;
include ResourceRequirement;
let amount &#61; coins.value;
aborts_if reconfiguration_state::spec_is_in_progress();
include AddStakeWithCapAbortsIfAndEnsures &#123; amount &#125;;
</code></pre>



<a id="@Specification_1_reactivate_stake_with_cap"></a>

### Function `reactivate_stake_with_cap`


<pre><code>public fun reactivate_stake_with_cap(owner_cap: &amp;stake::OwnerCapability, amount: u64)
</code></pre>




<pre><code>let pool_address &#61; owner_cap.pool_address;
include StakedValueNochange;
aborts_if reconfiguration_state::spec_is_in_progress();
aborts_if !stake_pool_exists(pool_address);
let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let post stake_pool &#61; global&lt;StakePool&gt;(pool_address);
modifies global&lt;StakePool&gt;(pool_address);
let min_amount &#61; aptos_std::math64::min(amount, pre_stake_pool.pending_inactive.value);
ensures stake_pool.pending_inactive.value &#61;&#61; pre_stake_pool.pending_inactive.value &#45; min_amount;
ensures stake_pool.active.value &#61;&#61; pre_stake_pool.active.value &#43; min_amount;
</code></pre>



<a id="@Specification_1_rotate_consensus_key"></a>

### Function `rotate_consensus_key`


<pre><code>public entry fun rotate_consensus_key(operator: &amp;signer, pool_address: address, new_consensus_pubkey: vector&lt;u8&gt;, proof_of_possession: vector&lt;u8&gt;)
</code></pre>




<pre><code>let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let post validator_info &#61; global&lt;ValidatorConfig&gt;(pool_address);
aborts_if reconfiguration_state::spec_is_in_progress();
aborts_if !exists&lt;StakePool&gt;(pool_address);
aborts_if signer::address_of(operator) !&#61; pre_stake_pool.operator_address;
aborts_if !exists&lt;ValidatorConfig&gt;(pool_address);
let pubkey_from_pop &#61; bls12381::spec_public_key_from_bytes_with_pop(
    new_consensus_pubkey,
    proof_of_possession_from_bytes(proof_of_possession)
);
aborts_if !option::spec_is_some(pubkey_from_pop);
modifies global&lt;ValidatorConfig&gt;(pool_address);
include StakedValueNochange;
ensures validator_info.consensus_pubkey &#61;&#61; new_consensus_pubkey;
</code></pre>



<a id="@Specification_1_update_network_and_fullnode_addresses"></a>

### Function `update_network_and_fullnode_addresses`


<pre><code>public entry fun update_network_and_fullnode_addresses(operator: &amp;signer, pool_address: address, new_network_addresses: vector&lt;u8&gt;, new_fullnode_addresses: vector&lt;u8&gt;)
</code></pre>




<pre><code>let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let post validator_info &#61; global&lt;ValidatorConfig&gt;(pool_address);
modifies global&lt;ValidatorConfig&gt;(pool_address);
include StakedValueNochange;
aborts_if reconfiguration_state::spec_is_in_progress();
aborts_if !exists&lt;StakePool&gt;(pool_address);
aborts_if !exists&lt;ValidatorConfig&gt;(pool_address);
aborts_if signer::address_of(operator) !&#61; pre_stake_pool.operator_address;
ensures validator_info.network_addresses &#61;&#61; new_network_addresses;
ensures validator_info.fullnode_addresses &#61;&#61; new_fullnode_addresses;
</code></pre>



<a id="@Specification_1_increase_lockup_with_cap"></a>

### Function `increase_lockup_with_cap`


<pre><code>public fun increase_lockup_with_cap(owner_cap: &amp;stake::OwnerCapability)
</code></pre>




<pre><code>let config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
let pool_address &#61; owner_cap.pool_address;
let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let post stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let now_seconds &#61; timestamp::spec_now_seconds();
let lockup &#61; config.recurring_lockup_duration_secs;
modifies global&lt;StakePool&gt;(pool_address);
include StakedValueNochange;
aborts_if !exists&lt;StakePool&gt;(pool_address);
aborts_if pre_stake_pool.locked_until_secs &gt;&#61; lockup &#43; now_seconds;
aborts_if lockup &#43; now_seconds &gt; MAX_U64;
aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);
ensures stake_pool.locked_until_secs &#61;&#61; lockup &#43; now_seconds;
</code></pre>



<a id="@Specification_1_join_validator_set"></a>

### Function `join_validator_set`


<pre><code>public entry fun join_validator_set(operator: &amp;signer, pool_address: address)
</code></pre>




<pre><code>pragma disable_invariants_in_body;
aborts_if !staking_config::get_allow_validator_set_change(staking_config::get());
aborts_if !exists&lt;StakePool&gt;(pool_address);
aborts_if !exists&lt;ValidatorConfig&gt;(pool_address);
aborts_if !exists&lt;StakingConfig&gt;(@aptos_framework);
aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);
aborts_if reconfiguration_state::spec_is_in_progress();
let stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
let post p_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
aborts_if signer::address_of(operator) !&#61; stake_pool.operator_address;
aborts_if option::spec_is_some(spec_find_validator(validator_set.active_validators, pool_address)) &#124;&#124;
            option::spec_is_some(spec_find_validator(validator_set.pending_inactive, pool_address)) &#124;&#124;
                option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));
let config &#61; staking_config::get();
let voting_power &#61; get_next_epoch_voting_power(stake_pool);
let minimum_stake &#61; config.minimum_stake;
let maximum_stake &#61; config.maximum_stake;
aborts_if voting_power &lt; minimum_stake;
aborts_if voting_power &gt;maximum_stake;
let validator_config &#61; global&lt;ValidatorConfig&gt;(pool_address);
aborts_if vector::is_empty(validator_config.consensus_pubkey);
let validator_set_size &#61; vector::length(validator_set.active_validators) &#43; vector::length(validator_set.pending_active) &#43; 1;
aborts_if validator_set_size &gt; MAX_VALIDATOR_SET_SIZE;
let voting_power_increase_limit &#61; (staking_config::get_voting_power_increase_limit(config) as u128);
aborts_if (validator_set.total_joining_power &#43; (voting_power as u128)) &gt; MAX_U128;
aborts_if validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;
aborts_if validator_set.total_voting_power &gt; 0 &amp;&amp;
    (validator_set.total_joining_power &#43; (voting_power as u128)) &#42; 100 &gt; validator_set.total_voting_power &#42; voting_power_increase_limit;
let post p_validator_info &#61; ValidatorInfo &#123;
    addr: pool_address,
    voting_power,
    config: validator_config,
&#125;;
ensures validator_set.total_joining_power &#43; voting_power &#61;&#61; p_validator_set.total_joining_power;
ensures vector::spec_contains(p_validator_set.pending_active, p_validator_info);
</code></pre>



<a id="@Specification_1_unlock_with_cap"></a>

### Function `unlock_with_cap`


<pre><code>public fun unlock_with_cap(amount: u64, owner_cap: &amp;stake::OwnerCapability)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
let pool_address &#61; owner_cap.pool_address;
let pre_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let post stake_pool &#61; global&lt;StakePool&gt;(pool_address);
aborts_if reconfiguration_state::spec_is_in_progress();
aborts_if amount !&#61; 0 &amp;&amp; !exists&lt;StakePool&gt;(pool_address);
modifies global&lt;StakePool&gt;(pool_address);
include StakedValueNochange;
let min_amount &#61; aptos_std::math64::min(amount,pre_stake_pool.active.value);
ensures stake_pool.active.value &#61;&#61; pre_stake_pool.active.value &#45; min_amount;
ensures stake_pool.pending_inactive.value &#61;&#61; pre_stake_pool.pending_inactive.value &#43; min_amount;
</code></pre>



<a id="@Specification_1_withdraw"></a>

### Function `withdraw`


<pre><code>public entry fun withdraw(owner: &amp;signer, withdraw_amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
aborts_if reconfiguration_state::spec_is_in_progress();
let addr &#61; signer::address_of(owner);
let ownership_cap &#61; global&lt;OwnerCapability&gt;(addr);
let pool_address &#61; ownership_cap.pool_address;
let stake_pool &#61; global&lt;StakePool&gt;(pool_address);
aborts_if !exists&lt;OwnerCapability&gt;(addr);
aborts_if !exists&lt;StakePool&gt;(pool_address);
aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);
let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
let bool_find_validator &#61; !option::spec_is_some(spec_find_validator(validator_set.active_validators, pool_address)) &amp;&amp;
            !option::spec_is_some(spec_find_validator(validator_set.pending_inactive, pool_address)) &amp;&amp;
                !option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));
aborts_if bool_find_validator &amp;&amp; !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
let new_withdraw_amount_1 &#61; min(withdraw_amount, stake_pool.inactive.value &#43; stake_pool.pending_inactive.value);
let new_withdraw_amount_2 &#61; min(withdraw_amount, stake_pool.inactive.value);
aborts_if bool_find_validator &amp;&amp; timestamp::now_seconds() &gt; stake_pool.locked_until_secs &amp;&amp;
            new_withdraw_amount_1 &gt; 0 &amp;&amp; stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &lt; new_withdraw_amount_1;
aborts_if !(bool_find_validator &amp;&amp; exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework)) &amp;&amp;
            new_withdraw_amount_2 &gt; 0 &amp;&amp; stake_pool.inactive.value &lt; new_withdraw_amount_2;
aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr);
include coin::DepositAbortsIf&lt;AptosCoin&gt;&#123;account_addr: addr&#125;;
let coin_store &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr);
let post p_coin_store &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr);
ensures bool_find_validator &amp;&amp; timestamp::now_seconds() &gt; stake_pool.locked_until_secs
            &amp;&amp; exists&lt;account::Account&gt;(addr) &amp;&amp; exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr) &#61;&#61;&gt;
                coin_store.coin.value &#43; new_withdraw_amount_1 &#61;&#61; p_coin_store.coin.value;
ensures !(bool_find_validator &amp;&amp; exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework))
            &amp;&amp; exists&lt;account::Account&gt;(addr) &amp;&amp; exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(addr) &#61;&#61;&gt;
                coin_store.coin.value &#43; new_withdraw_amount_2 &#61;&#61; p_coin_store.coin.value;
</code></pre>



<a id="@Specification_1_leave_validator_set"></a>

### Function `leave_validator_set`


<pre><code>public entry fun leave_validator_set(operator: &amp;signer, pool_address: address)
</code></pre>




<pre><code>pragma disable_invariants_in_body;
requires chain_status::is_operating();
aborts_if reconfiguration_state::spec_is_in_progress();
let config &#61; staking_config::get();
aborts_if !staking_config::get_allow_validator_set_change(config);
aborts_if !exists&lt;StakePool&gt;(pool_address);
aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);
aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);
let stake_pool &#61; global&lt;StakePool&gt;(pool_address);
aborts_if signer::address_of(operator) !&#61; stake_pool.operator_address;
let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
let validator_find_bool &#61; option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));
let active_validators &#61; validator_set.active_validators;
let pending_active &#61; validator_set.pending_active;
let post post_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
let post post_active_validators &#61; post_validator_set.active_validators;
let pending_inactive_validators &#61; validator_set.pending_inactive;
let post post_pending_inactive_validators &#61; post_validator_set.pending_inactive;
ensures len(active_validators) &#43; len(pending_inactive_validators) &#61;&#61; len(post_active_validators)
    &#43; len(post_pending_inactive_validators);
aborts_if !validator_find_bool &amp;&amp; !option::spec_is_some(spec_find_validator(active_validators, pool_address));
aborts_if !validator_find_bool &amp;&amp; vector::length(validator_set.active_validators) &lt;&#61; option::spec_borrow(spec_find_validator(active_validators, pool_address));
aborts_if !validator_find_bool &amp;&amp; vector::length(validator_set.active_validators) &lt; 2;
aborts_if validator_find_bool &amp;&amp; vector::length(validator_set.pending_active) &lt;&#61; option::spec_borrow(spec_find_validator(pending_active, pool_address));
let post p_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
let validator_stake &#61; (get_next_epoch_voting_power(stake_pool) as u128);
ensures validator_find_bool &amp;&amp; validator_set.total_joining_power &gt; validator_stake &#61;&#61;&gt;
            p_validator_set.total_joining_power &#61;&#61; validator_set.total_joining_power &#45; validator_stake;
ensures !validator_find_bool &#61;&#61;&gt; !option::spec_is_some(spec_find_validator(p_validator_set.pending_active, pool_address));
</code></pre>



<a id="@Specification_1_is_current_epoch_validator"></a>

### Function `is_current_epoch_validator`


<pre><code>public fun is_current_epoch_validator(pool_address: address): bool
</code></pre>




<pre><code>include ResourceRequirement;
aborts_if !spec_has_stake_pool(pool_address);
ensures result &#61;&#61; spec_is_current_epoch_validator(pool_address);
</code></pre>



<a id="@Specification_1_update_performance_statistics"></a>

### Function `update_performance_statistics`


<pre><code>public(friend) fun update_performance_statistics(proposer_index: option::Option&lt;u64&gt;, failed_proposer_indices: vector&lt;u64&gt;)
</code></pre>




<pre><code>requires chain_status::is_operating();
aborts_if false;
let validator_perf &#61; global&lt;ValidatorPerformance&gt;(@aptos_framework);
let post post_validator_perf &#61; global&lt;ValidatorPerformance&gt;(@aptos_framework);
let validator_len &#61; len(validator_perf.validators);
ensures (option::spec_is_some(ghost_proposer_idx) &amp;&amp; option::spec_borrow(ghost_proposer_idx) &lt; validator_len) &#61;&#61;&gt;
    (post_validator_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals &#61;&#61;
        validator_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals &#43; 1);
</code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch()
</code></pre>




<pre><code>pragma verify &#61; false;
pragma disable_invariants_in_body;
include ResourceRequirement;
include GetReconfigStartTimeRequirement;
include staking_config::StakingRewardsConfigRequirement;
include aptos_framework::aptos_coin::ExistsAptosCoin;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
aborts_if false;
</code></pre>



<a id="@Specification_1_next_validator_consensus_infos"></a>

### Function `next_validator_consensus_infos`


<pre><code>public fun next_validator_consensus_infos(): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
aborts_if false;
include ResourceRequirement;
include GetReconfigStartTimeRequirement;
include features::spec_periodical_reward_rate_decrease_enabled() &#61;&#61;&gt; staking_config::StakingRewardsConfigEnabledRequirement;
</code></pre>



<a id="@Specification_1_validator_consensus_infos_from_validator_set"></a>

### Function `validator_consensus_infos_from_validator_set`


<pre><code>fun validator_consensus_infos_from_validator_set(validator_set: &amp;stake::ValidatorSet): vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;
</code></pre>




<pre><code>aborts_if false;
invariant spec_validator_indices_are_valid_config(validator_set.active_validators,
    len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));
invariant len(validator_set.pending_inactive) &#61;&#61; 0 &#124;&#124;
    spec_validator_indices_are_valid_config(validator_set.pending_inactive,
        len(validator_set.active_validators) &#43; len(validator_set.pending_inactive));
</code></pre>




<a id="0x1_stake_AddStakeWithCapAbortsIfAndEnsures"></a>


<pre><code>schema AddStakeWithCapAbortsIfAndEnsures &#123;
    owner_cap: OwnerCapability;
    amount: u64;
    let pool_address &#61; owner_cap.pool_address;
    aborts_if !exists&lt;StakePool&gt;(pool_address);
    let config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
    let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
    let voting_power_increase_limit &#61; config.voting_power_increase_limit;
    let post post_validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
    let update_voting_power_increase &#61; amount !&#61; 0 &amp;&amp; (spec_contains(validator_set.active_validators, pool_address)
                                                       &#124;&#124; spec_contains(validator_set.pending_active, pool_address));
    aborts_if update_voting_power_increase &amp;&amp; validator_set.total_joining_power &#43; amount &gt; MAX_U128;
    ensures update_voting_power_increase &#61;&#61;&gt; post_validator_set.total_joining_power &#61;&#61; validator_set.total_joining_power &#43; amount;
    aborts_if update_voting_power_increase &amp;&amp; validator_set.total_voting_power &gt; 0
            &amp;&amp; validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;
    aborts_if update_voting_power_increase &amp;&amp; validator_set.total_voting_power &gt; 0
            &amp;&amp; validator_set.total_joining_power &#43; amount &gt; validator_set.total_voting_power &#42; voting_power_increase_limit / 100;
    let stake_pool &#61; global&lt;StakePool&gt;(pool_address);
    let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
    let value_pending_active &#61; stake_pool.pending_active.value;
    let value_active &#61; stake_pool.active.value;
    ensures amount !&#61; 0 &amp;&amp; spec_is_current_epoch_validator(pool_address) &#61;&#61;&gt; post_stake_pool.pending_active.value &#61;&#61; value_pending_active &#43; amount;
    ensures amount !&#61; 0 &amp;&amp; !spec_is_current_epoch_validator(pool_address) &#61;&#61;&gt; post_stake_pool.active.value &#61;&#61; value_active &#43; amount;
    let maximum_stake &#61; config.maximum_stake;
    let value_pending_inactive &#61; stake_pool.pending_inactive.value;
    let next_epoch_voting_power &#61; value_pending_active &#43; value_active &#43; value_pending_inactive;
    let voting_power &#61; next_epoch_voting_power &#43; amount;
    aborts_if amount !&#61; 0 &amp;&amp; voting_power &gt; MAX_U64;
    aborts_if amount !&#61; 0 &amp;&amp; voting_power &gt; maximum_stake;
&#125;
</code></pre>




<a id="0x1_stake_AddStakeAbortsIfAndEnsures"></a>


<pre><code>schema AddStakeAbortsIfAndEnsures &#123;
    owner: signer;
    amount: u64;
    let owner_address &#61; signer::address_of(owner);
    aborts_if !exists&lt;OwnerCapability&gt;(owner_address);
    let owner_cap &#61; global&lt;OwnerCapability&gt;(owner_address);
    include AddStakeWithCapAbortsIfAndEnsures &#123; owner_cap &#125;;
&#125;
</code></pre>




<a id="0x1_stake_spec_is_allowed"></a>


<pre><code>fun spec_is_allowed(account: address): bool &#123;
   if (!exists&lt;AllowedValidators&gt;(@aptos_framework)) &#123;
       true
   &#125; else &#123;
       let allowed &#61; global&lt;AllowedValidators&gt;(@aptos_framework);
       contains(allowed.accounts, account)
   &#125;
&#125;
</code></pre>




<a id="0x1_stake_spec_find_validator"></a>


<pre><code>fun spec_find_validator(v: vector&lt;ValidatorInfo&gt;, addr: address): Option&lt;u64&gt;;
</code></pre>




<a id="0x1_stake_spec_validators_are_initialized"></a>


<pre><code>fun spec_validators_are_initialized(validators: vector&lt;ValidatorInfo&gt;): bool &#123;
   forall i in 0..len(validators):
       spec_has_stake_pool(validators[i].addr) &amp;&amp;
           spec_has_validator_config(validators[i].addr)
&#125;
</code></pre>




<a id="0x1_stake_spec_validators_are_initialized_addrs"></a>


<pre><code>fun spec_validators_are_initialized_addrs(addrs: vector&lt;address&gt;): bool &#123;
   forall i in 0..len(addrs):
       spec_has_stake_pool(addrs[i]) &amp;&amp;
           spec_has_validator_config(addrs[i])
&#125;
</code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid"></a>


<pre><code>fun spec_validator_indices_are_valid(validators: vector&lt;ValidatorInfo&gt;): bool &#123;
   spec_validator_indices_are_valid_addr(validators, spec_validator_index_upper_bound()) &amp;&amp;
       spec_validator_indices_are_valid_config(validators, spec_validator_index_upper_bound())
&#125;
</code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid_addr"></a>


<pre><code>fun spec_validator_indices_are_valid_addr(validators: vector&lt;ValidatorInfo&gt;, upper_bound: u64): bool &#123;
   forall i in 0..len(validators):
       global&lt;ValidatorConfig&gt;(validators[i].addr).validator_index &lt; upper_bound
&#125;
</code></pre>




<a id="0x1_stake_spec_validator_indices_are_valid_config"></a>


<pre><code>fun spec_validator_indices_are_valid_config(validators: vector&lt;ValidatorInfo&gt;, upper_bound: u64): bool &#123;
   forall i in 0..len(validators):
       validators[i].config.validator_index &lt; upper_bound
&#125;
</code></pre>




<a id="0x1_stake_spec_validator_indices_active_pending_inactive"></a>


<pre><code>fun spec_validator_indices_active_pending_inactive(validator_set: ValidatorSet): bool &#123;
   len(validator_set.pending_inactive) &#43; len(validator_set.active_validators) &#61;&#61; spec_validator_index_upper_bound()
&#125;
</code></pre>




<a id="0x1_stake_spec_validator_index_upper_bound"></a>


<pre><code>fun spec_validator_index_upper_bound(): u64 &#123;
   len(global&lt;ValidatorPerformance&gt;(@aptos_framework).validators)
&#125;
</code></pre>




<a id="0x1_stake_spec_has_stake_pool"></a>


<pre><code>fun spec_has_stake_pool(a: address): bool &#123;
   exists&lt;StakePool&gt;(a)
&#125;
</code></pre>




<a id="0x1_stake_spec_has_validator_config"></a>


<pre><code>fun spec_has_validator_config(a: address): bool &#123;
   exists&lt;ValidatorConfig&gt;(a)
&#125;
</code></pre>




<a id="0x1_stake_spec_rewards_amount"></a>


<pre><code>fun spec_rewards_amount(
   stake_amount: u64,
   num_successful_proposals: u64,
   num_total_proposals: u64,
   rewards_rate: u64,
   rewards_rate_denominator: u64,
): u64;
</code></pre>




<a id="0x1_stake_spec_contains"></a>


<pre><code>fun spec_contains(validators: vector&lt;ValidatorInfo&gt;, addr: address): bool &#123;
   exists i in 0..len(validators): validators[i].addr &#61;&#61; addr
&#125;
</code></pre>




<a id="0x1_stake_spec_is_current_epoch_validator"></a>


<pre><code>fun spec_is_current_epoch_validator(pool_address: address): bool &#123;
   let validator_set &#61; global&lt;ValidatorSet&gt;(@aptos_framework);
   !spec_contains(validator_set.pending_active, pool_address)
       &amp;&amp; (spec_contains(validator_set.active_validators, pool_address)
       &#124;&#124; spec_contains(validator_set.pending_inactive, pool_address))
&#125;
</code></pre>




<a id="0x1_stake_ResourceRequirement"></a>


<pre><code>schema ResourceRequirement &#123;
    requires exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);
    requires exists&lt;ValidatorPerformance&gt;(@aptos_framework);
    requires exists&lt;ValidatorSet&gt;(@aptos_framework);
    requires exists&lt;StakingConfig&gt;(@aptos_framework);
    requires exists&lt;StakingRewardsConfig&gt;(@aptos_framework) &#124;&#124; !features::spec_periodical_reward_rate_decrease_enabled();
    requires exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
    requires exists&lt;ValidatorFees&gt;(@aptos_framework);
&#125;
</code></pre>




<a id="0x1_stake_spec_get_reward_rate_1"></a>


<pre><code>fun spec_get_reward_rate_1(config: StakingConfig): num &#123;
   if (features::spec_periodical_reward_rate_decrease_enabled()) &#123;
       let epoch_rewards_rate &#61; global&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework).rewards_rate;
       if (epoch_rewards_rate.value &#61;&#61; 0) &#123;
           0
       &#125; else &#123;
           let denominator_0 &#61; aptos_std::fixed_point64::spec_divide_u128(staking_config::MAX_REWARDS_RATE, epoch_rewards_rate);
           let denominator &#61; if (denominator_0 &gt; MAX_U64) &#123;
               MAX_U64
           &#125; else &#123;
               denominator_0
           &#125;;
           let nominator &#61; aptos_std::fixed_point64::spec_multiply_u128(denominator, epoch_rewards_rate);
           nominator
       &#125;
   &#125; else &#123;
           config.rewards_rate
   &#125;
&#125;
</code></pre>




<a id="0x1_stake_spec_get_reward_rate_2"></a>


<pre><code>fun spec_get_reward_rate_2(config: StakingConfig): num &#123;
   if (features::spec_periodical_reward_rate_decrease_enabled()) &#123;
       let epoch_rewards_rate &#61; global&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework).rewards_rate;
       if (epoch_rewards_rate.value &#61;&#61; 0) &#123;
           1
       &#125; else &#123;
           let denominator_0 &#61; aptos_std::fixed_point64::spec_divide_u128(staking_config::MAX_REWARDS_RATE, epoch_rewards_rate);
           let denominator &#61; if (denominator_0 &gt; MAX_U64) &#123;
               MAX_U64
           &#125; else &#123;
               denominator_0
           &#125;;
           denominator
       &#125;
   &#125; else &#123;
           config.rewards_rate_denominator
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_update_stake_pool"></a>

### Function `update_stake_pool`


<pre><code>fun update_stake_pool(validator_perf: &amp;stake::ValidatorPerformance, pool_address: address, staking_config: &amp;staking_config::StakingConfig)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
include ResourceRequirement;
include GetReconfigStartTimeRequirement;
include staking_config::StakingRewardsConfigRequirement;
include UpdateStakePoolAbortsIf;
let stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let validator_config &#61; global&lt;ValidatorConfig&gt;(pool_address);
let cur_validator_perf &#61; validator_perf.validators[validator_config.validator_index];
let num_successful_proposals &#61; cur_validator_perf.successful_proposals;
let num_total_proposals &#61; cur_validator_perf.successful_proposals &#43; cur_validator_perf.failed_proposals;
let rewards_rate &#61; spec_get_reward_rate_1(staking_config);
let rewards_rate_denominator &#61; spec_get_reward_rate_2(staking_config);
let rewards_amount_1 &#61; if (stake_pool.active.value &gt; 0) &#123;
    spec_rewards_amount(stake_pool.active.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
&#125; else &#123;
    0
&#125;;
let rewards_amount_2 &#61; if (stake_pool.pending_inactive.value &gt; 0) &#123;
    spec_rewards_amount(stake_pool.pending_inactive.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
&#125; else &#123;
    0
&#125;;
let post post_stake_pool &#61; global&lt;StakePool&gt;(pool_address);
let post post_active_value &#61; post_stake_pool.active.value;
let post post_pending_inactive_value &#61; post_stake_pool.pending_inactive.value;
let fees_table &#61; global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;
let post post_fees_table &#61; global&lt;ValidatorFees&gt;(@aptos_framework).fees_table;
let post post_inactive_value &#61; post_stake_pool.inactive.value;
ensures post_stake_pool.pending_active.value &#61;&#61; 0;
ensures if (features::spec_is_enabled(features::COLLECT_AND_DISTRIBUTE_GAS_FEES) &amp;&amp; table::spec_contains(fees_table, pool_address)) &#123;
    !table::spec_contains(post_fees_table, pool_address) &amp;&amp;
    post_active_value &#61;&#61; stake_pool.active.value &#43; rewards_amount_1 &#43; stake_pool.pending_active.value &#43; table::spec_get(fees_table, pool_address).value
&#125; else &#123;
    post_active_value &#61;&#61; stake_pool.active.value &#43; rewards_amount_1 &#43; stake_pool.pending_active.value
&#125;;
ensures if (spec_get_reconfig_start_time_secs() &gt;&#61; stake_pool.locked_until_secs) &#123;
    post_pending_inactive_value &#61;&#61; 0 &amp;&amp;
    post_inactive_value &#61;&#61; stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &#43; rewards_amount_2
&#125; else &#123;
    post_pending_inactive_value &#61;&#61; stake_pool.pending_inactive.value &#43; rewards_amount_2
&#125;;
</code></pre>




<a id="0x1_stake_UpdateStakePoolAbortsIf"></a>


<pre><code>schema UpdateStakePoolAbortsIf &#123;
    pool_address: address;
    validator_perf: ValidatorPerformance;
    aborts_if !exists&lt;StakePool&gt;(pool_address);
    aborts_if !exists&lt;ValidatorConfig&gt;(pool_address);
    aborts_if global&lt;ValidatorConfig&gt;(pool_address).validator_index &gt;&#61; len(validator_perf.validators);
    let aptos_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;
    aborts_if !exists&lt;ValidatorFees&gt;(aptos_addr);
    let stake_pool &#61; global&lt;StakePool&gt;(pool_address);
    include DistributeRewardsAbortsIf &#123;stake: stake_pool.active&#125;;
    include DistributeRewardsAbortsIf &#123;stake: stake_pool.pending_inactive&#125;;
&#125;
</code></pre>



<a id="@Specification_1_get_reconfig_start_time_secs"></a>

### Function `get_reconfig_start_time_secs`


<pre><code>fun get_reconfig_start_time_secs(): u64
</code></pre>




<pre><code>include GetReconfigStartTimeRequirement;
</code></pre>




<a id="0x1_stake_GetReconfigStartTimeRequirement"></a>


<pre><code>schema GetReconfigStartTimeRequirement &#123;
    requires exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
    include reconfiguration_state::StartTimeSecsRequirement;
&#125;
</code></pre>




<a id="0x1_stake_spec_get_reconfig_start_time_secs"></a>


<pre><code>fun spec_get_reconfig_start_time_secs(): u64 &#123;
   if (exists&lt;reconfiguration_state::State&gt;(@aptos_framework)) &#123;
       reconfiguration_state::spec_start_time_secs()
   &#125; else &#123;
       timestamp::spec_now_seconds()
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_calculate_rewards_amount"></a>

### Function `calculate_rewards_amount`


<pre><code>fun calculate_rewards_amount(stake_amount: u64, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64
</code></pre>




<pre><code>pragma opaque;
pragma verify_duration_estimate &#61; 300;
requires rewards_rate &lt;&#61; MAX_REWARDS_RATE;
requires rewards_rate_denominator &gt; 0;
requires rewards_rate &lt;&#61; rewards_rate_denominator;
requires num_successful_proposals &lt;&#61; num_total_proposals;
ensures [concrete] (rewards_rate_denominator &#42; num_total_proposals &#61;&#61; 0) &#61;&#61;&gt; result &#61;&#61; 0;
ensures [concrete] (rewards_rate_denominator &#42; num_total_proposals &gt; 0) &#61;&#61;&gt; &#123;
    let amount &#61; ((stake_amount &#42; rewards_rate &#42; num_successful_proposals) /
        (rewards_rate_denominator &#42; num_total_proposals));
    result &#61;&#61; amount
&#125;;
aborts_if false;
ensures [abstract] result &#61;&#61; spec_rewards_amount(
    stake_amount,
    num_successful_proposals,
    num_total_proposals,
    rewards_rate,
    rewards_rate_denominator);
</code></pre>



<a id="@Specification_1_distribute_rewards"></a>

### Function `distribute_rewards`


<pre><code>fun distribute_rewards(stake: &amp;mut coin::Coin&lt;aptos_coin::AptosCoin&gt;, num_successful_proposals: u64, num_total_proposals: u64, rewards_rate: u64, rewards_rate_denominator: u64): u64
</code></pre>




<pre><code>include ResourceRequirement;
requires rewards_rate &lt;&#61; MAX_REWARDS_RATE;
requires rewards_rate_denominator &gt; 0;
requires rewards_rate &lt;&#61; rewards_rate_denominator;
requires num_successful_proposals &lt;&#61; num_total_proposals;
include DistributeRewardsAbortsIf;
ensures old(stake.value) &gt; 0 &#61;&#61;&gt;
    result &#61;&#61; spec_rewards_amount(
        old(stake.value),
        num_successful_proposals,
        num_total_proposals,
        rewards_rate,
        rewards_rate_denominator);
ensures old(stake.value) &gt; 0 &#61;&#61;&gt;
    stake.value &#61;&#61; old(stake.value) &#43; spec_rewards_amount(
        old(stake.value),
        num_successful_proposals,
        num_total_proposals,
        rewards_rate,
        rewards_rate_denominator);
ensures old(stake.value) &#61;&#61; 0 &#61;&#61;&gt; result &#61;&#61; 0;
ensures old(stake.value) &#61;&#61; 0 &#61;&#61;&gt; stake.value &#61;&#61; old(stake.value);
</code></pre>




<a id="0x1_stake_DistributeRewardsAbortsIf"></a>


<pre><code>schema DistributeRewardsAbortsIf &#123;
    stake: Coin&lt;AptosCoin&gt;;
    num_successful_proposals: num;
    num_total_proposals: num;
    rewards_rate: num;
    rewards_rate_denominator: num;
    let stake_amount &#61; coin::value(stake);
    let rewards_amount &#61; if (stake_amount &gt; 0) &#123;
        spec_rewards_amount(stake_amount, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
    &#125; else &#123;
        0
    &#125;;
    let amount &#61; rewards_amount;
    let addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;
    aborts_if (rewards_amount &gt; 0) &amp;&amp; !exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(addr);
    modifies global&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(addr);
    include (rewards_amount &gt; 0) &#61;&#61;&gt; coin::CoinAddAbortsIf&lt;AptosCoin&gt; &#123; amount: amount &#125;;
&#125;
</code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code>fun append&lt;T&gt;(v1: &amp;mut vector&lt;T&gt;, v2: &amp;mut vector&lt;T&gt;)
</code></pre>




<pre><code>pragma opaque, verify &#61; false;
aborts_if false;
ensures len(v1) &#61;&#61; old(len(v1) &#43; len(v2));
ensures len(v2) &#61;&#61; 0;
ensures (forall i in 0..old(len(v1)): v1[i] &#61;&#61; old(v1[i]));
ensures (forall i in old(len(v1))..len(v1): v1[i] &#61;&#61; old(v2[len(v2) &#45; (i &#45; len(v1)) &#45; 1]));
</code></pre>



<a id="@Specification_1_find_validator"></a>

### Function `find_validator`


<pre><code>fun find_validator(v: &amp;vector&lt;stake::ValidatorInfo&gt;, addr: address): option::Option&lt;u64&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures option::is_none(result) &#61;&#61;&gt; (forall i in 0..len(v): v[i].addr !&#61; addr);
ensures option::is_some(result) &#61;&#61;&gt; v[option::borrow(result)].addr &#61;&#61; addr;
ensures option::is_some(result) &#61;&#61;&gt; spec_contains(v, addr);
ensures [abstract] result &#61;&#61; spec_find_validator(v,addr);
</code></pre>



<a id="@Specification_1_update_voting_power_increase"></a>

### Function `update_voting_power_increase`


<pre><code>fun update_voting_power_increase(increase_amount: u64)
</code></pre>




<pre><code>requires !reconfiguration_state::spec_is_in_progress();
aborts_if !exists&lt;ValidatorSet&gt;(@aptos_framework);
aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);
let aptos &#61; @aptos_framework;
let pre_validator_set &#61; global&lt;ValidatorSet&gt;(aptos);
let post validator_set &#61; global&lt;ValidatorSet&gt;(aptos);
let staking_config &#61; global&lt;staking_config::StakingConfig&gt;(aptos);
let voting_power_increase_limit &#61; staking_config.voting_power_increase_limit;
aborts_if pre_validator_set.total_joining_power &#43; increase_amount &gt; MAX_U128;
aborts_if pre_validator_set.total_voting_power &gt; 0 &amp;&amp; pre_validator_set.total_voting_power &#42; voting_power_increase_limit &gt; MAX_U128;
aborts_if pre_validator_set.total_voting_power &gt; 0 &amp;&amp;
    pre_validator_set.total_joining_power &#43; increase_amount &gt; pre_validator_set.total_voting_power &#42; voting_power_increase_limit / 100;
ensures validator_set.total_voting_power &gt; 0 &#61;&#61;&gt;
    validator_set.total_joining_power &lt;&#61; validator_set.total_voting_power &#42; voting_power_increase_limit / 100;
ensures validator_set.total_joining_power &#61;&#61; pre_validator_set.total_joining_power &#43; increase_amount;
</code></pre>



<a id="@Specification_1_assert_stake_pool_exists"></a>

### Function `assert_stake_pool_exists`


<pre><code>fun assert_stake_pool_exists(pool_address: address)
</code></pre>




<pre><code>aborts_if !stake_pool_exists(pool_address);
</code></pre>



<a id="@Specification_1_configure_allowed_validators"></a>

### Function `configure_allowed_validators`


<pre><code>public fun configure_allowed_validators(aptos_framework: &amp;signer, accounts: vector&lt;address&gt;)
</code></pre>




<pre><code>let aptos_framework_address &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(aptos_framework_address);
let post allowed &#61; global&lt;AllowedValidators&gt;(aptos_framework_address);
ensures allowed.accounts &#61;&#61; accounts;
</code></pre>



<a id="@Specification_1_assert_owner_cap_exists"></a>

### Function `assert_owner_cap_exists`


<pre><code>fun assert_owner_cap_exists(owner: address)
</code></pre>




<pre><code>aborts_if !exists&lt;OwnerCapability&gt;(owner);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
