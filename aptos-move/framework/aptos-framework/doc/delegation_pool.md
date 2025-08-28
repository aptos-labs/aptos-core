
<a id="0x1_delegation_pool"></a>

# Module `0x1::delegation_pool`


Allow multiple delegators to participate in the same stake pool in order to collect the minimum
stake required to join the validator set. Delegators are rewarded out of the validator rewards
proportionally to their stake and provided the same stake-management API as the stake pool owner.

The main accounting logic in the delegation pool contract handles the following:
1. Tracks how much stake each delegator owns, privately deposited as well as earned.
Accounting individual delegator stakes is achieved through the shares-based pool defined at
<code>aptos_std::pool_u64</code>, hence delegators own shares rather than absolute stakes into the delegation pool.
2. Tracks rewards earned by the stake pool, implicitly by the delegation one, in the meantime
and distribute them accordingly.
3. Tracks lockup cycles on the stake pool in order to separate inactive stake (not earning rewards)
from pending_inactive stake (earning rewards) and allow its delegators to withdraw the former.
4. Tracks how much commission fee has to be paid to the operator out of incoming rewards before
distributing them to the internal pool_u64 pools.

In order to distinguish between stakes in different states and route rewards accordingly,
separate pool_u64 pools are used for individual stake states:
1. one of <code>active</code> + <code>pending_active</code> stake
2. one of <code>inactive</code> stake FOR each past observed lockup cycle (OLC) on the stake pool
3. one of <code>pending_inactive</code> stake scheduled during this ongoing OLC

As stake-state transitions and rewards are computed only at the stake pool level, the delegation pool
gets outdated. To mitigate this, at any interaction with the delegation pool, a process of synchronization
to the underlying stake pool is executed before the requested operation itself.

At synchronization:
- stake deviations between the two pools are actually the rewards produced in the meantime.
- the commission fee is extracted from the rewards, the remaining stake is distributed to the internal
pool_u64 pools and then the commission stake used to buy shares for operator.
- if detecting that the lockup expired on the stake pool, the delegation pool will isolate its
pending_inactive stake (now inactive) and create a new pool_u64 to host future pending_inactive stake
scheduled this newly started lockup.
Detecting a lockup expiration on the stake pool resumes to detecting new inactive stake.

Accounting main invariants:
- each stake-management operation (add/unlock/reactivate/withdraw) and operator change triggers
the synchronization process before executing its own function.
- each OLC maps to one or more real lockups on the stake pool, but not the opposite. Actually, only a real
lockup with 'activity' (which inactivated some unlocking stake) triggers the creation of a new OLC.
- unlocking and/or unlocked stake originating from different real lockups are never mixed together into
the same pool_u64. This invalidates the accounting of which rewards belong to whom.
- no delegator can have unlocking and/or unlocked stake (pending withdrawals) in different OLCs. This ensures
delegators do not have to keep track of the OLCs when they unlocked. When creating a new pending withdrawal,
the existing one is executed (withdrawn) if is already inactive.
- <code>add_stake</code> fees are always refunded, but only after the epoch when they have been charged ends.
- withdrawing pending_inactive stake (when validator had gone inactive before its lockup expired)
does not inactivate any stake additional to the requested one to ensure OLC would not advance indefinitely.
- the pending withdrawal exists at an OLC iff delegator owns some shares within the shares pool of that OLC.

Example flow:
<ol>
<li>A node operator creates a delegation pool by calling
<code>initialize_delegation_pool</code> and sets
its commission fee to 0% (for simplicity). A stake pool is created with no initial stake and owned by
a resource account controlled by the delegation pool.</li>
<li>Delegator A adds 100 stake which is converted to 100 shares into the active pool_u64</li>
<li>Operator joins the validator set as the stake pool has now the minimum stake</li>
<li>The stake pool earned rewards and now has 200 active stake. A's active shares are worth 200 coins as
the commission fee is 0%.</li>
<li></li>
<ol>
<li>A requests <code>unlock</code> for 100 stake</li>
<li>Synchronization detects 200 - 100 active rewards which are entirely (0% commission) added to the active pool.</li>
<li>100 coins = (100 * 100) / 200 = 50 shares are redeemed from the active pool and exchanged for 100 shares
into the pending_inactive one on A's behalf</li>
</ol>
<li>Delegator B adds 200 stake which is converted to (200 * 50) / 100 = 100 shares into the active pool</li>
<li>The stake pool earned rewards and now has 600 active and 200 pending_inactive stake.</li>
<li></li>
<ol>
<li>A requests <code>reactivate_stake</code> for 100 stake</li>
<li>
Synchronization detects 600 - 300 active and 200 - 100 pending_inactive rewards which are both entirely
distributed to their corresponding pools
</li>
<li>
100 coins = (100 * 100) / 200 = 50 shares are redeemed from the pending_inactive pool and exchanged for
(100 * 150) / 600 = 25 shares into the active one on A's behalf
</li>
</ol>
<li>The lockup expires on the stake pool, inactivating the entire pending_inactive stake</li>
<li></li>
<ol>
<li>B requests <code>unlock</code> for 100 stake</li>
<li>
Synchronization detects no active or pending_inactive rewards, but 0 -> 100 inactive stake on the stake pool,
so it advances the observed lockup cycle and creates a pool_u64 for the new lockup, hence allowing previous
pending_inactive shares to be redeemed</li>
<li>
100 coins = (100 * 175) / 700 = 25 shares are redeemed from the active pool and exchanged for 100 shares
into the new pending_inactive one on B's behalf
</li>
</ol>
<li>The stake pool earned rewards and now has some pending_inactive rewards.</li>
<li></li>
<ol>
<li>A requests <code>withdraw</code> for its entire inactive stake</li>
<li>
Synchronization detects no new inactive stake, but some pending_inactive rewards which are distributed
to the (2nd) pending_inactive pool
</li>
<li>
A's 50 shares = (50 * 100) / 50 = 100 coins are redeemed from the (1st) inactive pool and 100 stake is
transferred to A
</li>
</ol>
</ol>



-  [Resource `DelegationPoolOwnership`](#0x1_delegation_pool_DelegationPoolOwnership)
-  [Struct `ObservedLockupCycle`](#0x1_delegation_pool_ObservedLockupCycle)
-  [Resource `DelegationPool`](#0x1_delegation_pool_DelegationPool)
-  [Struct `VotingRecordKey`](#0x1_delegation_pool_VotingRecordKey)
-  [Struct `VoteDelegation`](#0x1_delegation_pool_VoteDelegation)
-  [Struct `DelegatedVotes`](#0x1_delegation_pool_DelegatedVotes)
-  [Resource `GovernanceRecords`](#0x1_delegation_pool_GovernanceRecords)
-  [Resource `BeneficiaryForOperator`](#0x1_delegation_pool_BeneficiaryForOperator)
-  [Resource `NextCommissionPercentage`](#0x1_delegation_pool_NextCommissionPercentage)
-  [Resource `DelegationPoolAllowlisting`](#0x1_delegation_pool_DelegationPoolAllowlisting)
-  [Enum `DelegationPermission`](#0x1_delegation_pool_DelegationPermission)
-  [Struct `AddStake`](#0x1_delegation_pool_AddStake)
-  [Struct `AddStakeEvent`](#0x1_delegation_pool_AddStakeEvent)
-  [Struct `ReactivateStake`](#0x1_delegation_pool_ReactivateStake)
-  [Struct `ReactivateStakeEvent`](#0x1_delegation_pool_ReactivateStakeEvent)
-  [Struct `UnlockStake`](#0x1_delegation_pool_UnlockStake)
-  [Struct `UnlockStakeEvent`](#0x1_delegation_pool_UnlockStakeEvent)
-  [Struct `WithdrawStake`](#0x1_delegation_pool_WithdrawStake)
-  [Struct `WithdrawStakeEvent`](#0x1_delegation_pool_WithdrawStakeEvent)
-  [Struct `DistributeCommissionEvent`](#0x1_delegation_pool_DistributeCommissionEvent)
-  [Struct `DistributeCommission`](#0x1_delegation_pool_DistributeCommission)
-  [Struct `Vote`](#0x1_delegation_pool_Vote)
-  [Struct `VoteEvent`](#0x1_delegation_pool_VoteEvent)
-  [Struct `CreateProposal`](#0x1_delegation_pool_CreateProposal)
-  [Struct `CreateProposalEvent`](#0x1_delegation_pool_CreateProposalEvent)
-  [Struct `DelegateVotingPower`](#0x1_delegation_pool_DelegateVotingPower)
-  [Struct `DelegateVotingPowerEvent`](#0x1_delegation_pool_DelegateVotingPowerEvent)
-  [Struct `SetBeneficiaryForOperator`](#0x1_delegation_pool_SetBeneficiaryForOperator)
-  [Struct `CommissionPercentageChange`](#0x1_delegation_pool_CommissionPercentageChange)
-  [Struct `EnableDelegatorsAllowlisting`](#0x1_delegation_pool_EnableDelegatorsAllowlisting)
-  [Struct `DisableDelegatorsAllowlisting`](#0x1_delegation_pool_DisableDelegatorsAllowlisting)
-  [Struct `AllowlistDelegator`](#0x1_delegation_pool_AllowlistDelegator)
-  [Struct `RemoveDelegatorFromAllowlist`](#0x1_delegation_pool_RemoveDelegatorFromAllowlist)
-  [Struct `EvictDelegator`](#0x1_delegation_pool_EvictDelegator)
-  [Constants](#@Constants_0)
-  [Function `owner_cap_exists`](#0x1_delegation_pool_owner_cap_exists)
-  [Function `get_owned_pool_address`](#0x1_delegation_pool_get_owned_pool_address)
-  [Function `delegation_pool_exists`](#0x1_delegation_pool_delegation_pool_exists)
-  [Function `partial_governance_voting_enabled`](#0x1_delegation_pool_partial_governance_voting_enabled)
-  [Function `observed_lockup_cycle`](#0x1_delegation_pool_observed_lockup_cycle)
-  [Function `is_next_commission_percentage_effective`](#0x1_delegation_pool_is_next_commission_percentage_effective)
-  [Function `operator_commission_percentage`](#0x1_delegation_pool_operator_commission_percentage)
-  [Function `operator_commission_percentage_next_lockup_cycle`](#0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle)
-  [Function `shareholders_count_active_pool`](#0x1_delegation_pool_shareholders_count_active_pool)
-  [Function `get_delegation_pool_stake`](#0x1_delegation_pool_get_delegation_pool_stake)
-  [Function `get_pending_withdrawal`](#0x1_delegation_pool_get_pending_withdrawal)
-  [Function `get_stake`](#0x1_delegation_pool_get_stake)
-  [Function `get_add_stake_fee`](#0x1_delegation_pool_get_add_stake_fee)
-  [Function `can_withdraw_pending_inactive`](#0x1_delegation_pool_can_withdraw_pending_inactive)
-  [Function `calculate_and_update_voter_total_voting_power`](#0x1_delegation_pool_calculate_and_update_voter_total_voting_power)
-  [Function `calculate_and_update_remaining_voting_power`](#0x1_delegation_pool_calculate_and_update_remaining_voting_power)
-  [Function `calculate_and_update_delegator_voter`](#0x1_delegation_pool_calculate_and_update_delegator_voter)
-  [Function `calculate_and_update_voting_delegation`](#0x1_delegation_pool_calculate_and_update_voting_delegation)
-  [Function `get_expected_stake_pool_address`](#0x1_delegation_pool_get_expected_stake_pool_address)
-  [Function `min_remaining_secs_for_commission_change`](#0x1_delegation_pool_min_remaining_secs_for_commission_change)
-  [Function `allowlisting_enabled`](#0x1_delegation_pool_allowlisting_enabled)
-  [Function `delegator_allowlisted`](#0x1_delegation_pool_delegator_allowlisted)
-  [Function `get_delegators_allowlist`](#0x1_delegation_pool_get_delegators_allowlist)
-  [Function `check_delegation_pool_management_permission`](#0x1_delegation_pool_check_delegation_pool_management_permission)
-  [Function `grant_delegation_pool_management_permission`](#0x1_delegation_pool_grant_delegation_pool_management_permission)
-  [Function `check_stake_management_permission`](#0x1_delegation_pool_check_stake_management_permission)
-  [Function `grant_stake_management_permission`](#0x1_delegation_pool_grant_stake_management_permission)
-  [Function `initialize_delegation_pool`](#0x1_delegation_pool_initialize_delegation_pool)
-  [Function `beneficiary_for_operator`](#0x1_delegation_pool_beneficiary_for_operator)
-  [Function `enable_partial_governance_voting`](#0x1_delegation_pool_enable_partial_governance_voting)
-  [Function `vote`](#0x1_delegation_pool_vote)
-  [Function `create_proposal`](#0x1_delegation_pool_create_proposal)
-  [Function `assert_owner_cap_exists`](#0x1_delegation_pool_assert_owner_cap_exists)
-  [Function `assert_delegation_pool_exists`](#0x1_delegation_pool_assert_delegation_pool_exists)
-  [Function `assert_min_active_balance`](#0x1_delegation_pool_assert_min_active_balance)
-  [Function `assert_min_pending_inactive_balance`](#0x1_delegation_pool_assert_min_pending_inactive_balance)
-  [Function `assert_partial_governance_voting_enabled`](#0x1_delegation_pool_assert_partial_governance_voting_enabled)
-  [Function `assert_allowlisting_enabled`](#0x1_delegation_pool_assert_allowlisting_enabled)
-  [Function `assert_delegator_allowlisted`](#0x1_delegation_pool_assert_delegator_allowlisted)
-  [Function `coins_to_redeem_to_ensure_min_stake`](#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake)
-  [Function `coins_to_transfer_to_ensure_min_stake`](#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake)
-  [Function `retrieve_stake_pool_owner`](#0x1_delegation_pool_retrieve_stake_pool_owner)
-  [Function `get_pool_address`](#0x1_delegation_pool_get_pool_address)
-  [Function `get_delegator_active_shares`](#0x1_delegation_pool_get_delegator_active_shares)
-  [Function `get_delegator_pending_inactive_shares`](#0x1_delegation_pool_get_delegator_pending_inactive_shares)
-  [Function `get_used_voting_power`](#0x1_delegation_pool_get_used_voting_power)
-  [Function `create_resource_account_seed`](#0x1_delegation_pool_create_resource_account_seed)
-  [Function `borrow_mut_used_voting_power`](#0x1_delegation_pool_borrow_mut_used_voting_power)
-  [Function `update_and_borrow_mut_delegator_vote_delegation`](#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation)
-  [Function `update_and_borrow_mut_delegated_votes`](#0x1_delegation_pool_update_and_borrow_mut_delegated_votes)
-  [Function `olc_with_index`](#0x1_delegation_pool_olc_with_index)
-  [Function `calculate_total_voting_power`](#0x1_delegation_pool_calculate_total_voting_power)
-  [Function `calculate_and_update_delegator_voter_internal`](#0x1_delegation_pool_calculate_and_update_delegator_voter_internal)
-  [Function `calculate_and_update_delegated_votes`](#0x1_delegation_pool_calculate_and_update_delegated_votes)
-  [Function `borrow_mut_delegators_allowlist`](#0x1_delegation_pool_borrow_mut_delegators_allowlist)
-  [Function `set_operator`](#0x1_delegation_pool_set_operator)
-  [Function `set_beneficiary_for_operator`](#0x1_delegation_pool_set_beneficiary_for_operator)
-  [Function `update_commission_percentage`](#0x1_delegation_pool_update_commission_percentage)
-  [Function `set_delegated_voter`](#0x1_delegation_pool_set_delegated_voter)
-  [Function `delegate_voting_power`](#0x1_delegation_pool_delegate_voting_power)
-  [Function `enable_delegators_allowlisting`](#0x1_delegation_pool_enable_delegators_allowlisting)
-  [Function `disable_delegators_allowlisting`](#0x1_delegation_pool_disable_delegators_allowlisting)
-  [Function `allowlist_delegator`](#0x1_delegation_pool_allowlist_delegator)
-  [Function `remove_delegator_from_allowlist`](#0x1_delegation_pool_remove_delegator_from_allowlist)
-  [Function `evict_delegator`](#0x1_delegation_pool_evict_delegator)
-  [Function `add_stake`](#0x1_delegation_pool_add_stake)
-  [Function `unlock`](#0x1_delegation_pool_unlock)
-  [Function `unlock_internal`](#0x1_delegation_pool_unlock_internal)
-  [Function `reactivate_stake`](#0x1_delegation_pool_reactivate_stake)
-  [Function `withdraw`](#0x1_delegation_pool_withdraw)
-  [Function `withdraw_internal`](#0x1_delegation_pool_withdraw_internal)
-  [Function `pending_withdrawal_exists`](#0x1_delegation_pool_pending_withdrawal_exists)
-  [Function `pending_inactive_shares_pool_mut`](#0x1_delegation_pool_pending_inactive_shares_pool_mut)
-  [Function `pending_inactive_shares_pool`](#0x1_delegation_pool_pending_inactive_shares_pool)
-  [Function `execute_pending_withdrawal`](#0x1_delegation_pool_execute_pending_withdrawal)
-  [Function `buy_in_active_shares`](#0x1_delegation_pool_buy_in_active_shares)
-  [Function `buy_in_pending_inactive_shares`](#0x1_delegation_pool_buy_in_pending_inactive_shares)
-  [Function `amount_to_shares_to_redeem`](#0x1_delegation_pool_amount_to_shares_to_redeem)
-  [Function `redeem_active_shares`](#0x1_delegation_pool_redeem_active_shares)
-  [Function `redeem_inactive_shares`](#0x1_delegation_pool_redeem_inactive_shares)
-  [Function `calculate_stake_pool_drift`](#0x1_delegation_pool_calculate_stake_pool_drift)
-  [Function `synchronize_delegation_pool`](#0x1_delegation_pool_synchronize_delegation_pool)
-  [Function `assert_and_update_proposal_used_voting_power`](#0x1_delegation_pool_assert_and_update_proposal_used_voting_power)
-  [Function `update_governance_records_for_buy_in_active_shares`](#0x1_delegation_pool_update_governance_records_for_buy_in_active_shares)
-  [Function `update_governance_records_for_buy_in_pending_inactive_shares`](#0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares)
-  [Function `update_governanace_records_for_redeem_active_shares`](#0x1_delegation_pool_update_governanace_records_for_redeem_active_shares)
-  [Function `update_governanace_records_for_redeem_pending_inactive_shares`](#0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares)
-  [Function `multiply_then_divide`](#0x1_delegation_pool_multiply_then_divide)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="aptos_governance.md#0x1_aptos_governance">0x1::aptos_governance</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound">0x1::pool_u64_unbound</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_delegation_pool_DelegationPoolOwnership"></a>

## Resource `DelegationPoolOwnership`

Capability that represents ownership over privileged operations on the underlying stake pool.


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>
 equal to address of the resource account owning the stake pool
</dd>
</dl>


</details>

<a id="0x1_delegation_pool_ObservedLockupCycle"></a>

## Struct `ObservedLockupCycle`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DelegationPool"></a>

## Resource `DelegationPool`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active_shares: <a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a></code>
</dt>
<dd>

</dd>
<dt>
<code>observed_lockup_cycle: <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a></code>
</dt>
<dd>

</dd>
<dt>
<code>inactive_shares: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a>, <a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_withdrawals: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<b>address</b>, <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>

</dd>
<dt>
<code>total_coins_inactive: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>operator_commission_percentage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>add_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_AddStakeEvent">delegation_pool::AddStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reactivate_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_ReactivateStakeEvent">delegation_pool::ReactivateStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_UnlockStakeEvent">delegation_pool::UnlockStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_WithdrawStakeEvent">delegation_pool::WithdrawStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>distribute_commission_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">delegation_pool::DistributeCommissionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_VotingRecordKey"></a>

## Struct `VotingRecordKey`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_VotingRecordKey">VotingRecordKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_VoteDelegation"></a>

## Struct `VoteDelegation`

Track delegated voter of each delegator.


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">VoteDelegation</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pending_voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>last_locked_until_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DelegatedVotes"></a>

## Struct `DelegatedVotes`

Track total voting power of each voter.


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active_shares: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_inactive_shares: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>active_shares_next_lockup: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>last_locked_until_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_GovernanceRecords"></a>

## Resource `GovernanceRecords`

Track governance information of a delegation(e.g. voter delegation/voting power calculation).
This struct should be stored in the delegation pool resource account.


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_VotingRecordKey">delegation_pool::VotingRecordKey</a>, u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>votes_per_proposal: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;u64, u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_delegation: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<b>address</b>, <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">delegation_pool::VoteDelegation</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delegated_votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<b>address</b>, <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">delegation_pool::DelegatedVotes</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_VoteEvent">delegation_pool::VoteEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_proposal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_CreateProposalEvent">delegation_pool::CreateProposalEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delegate_voting_power_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPowerEvent">delegation_pool::DelegateVotingPowerEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_BeneficiaryForOperator"></a>

## Resource `BeneficiaryForOperator`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>beneficiary_for_operator: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_NextCommissionPercentage"></a>

## Resource `NextCommissionPercentage`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>commission_percentage_next_lockup_cycle: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>effective_after_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DelegationPoolAllowlisting"></a>

## Resource `DelegationPoolAllowlisting`

Tracks a delegation pool's allowlist of delegators.
If allowlisting is enabled, existing delegators are not implicitly allowlisted and they can be individually
evicted later by the pool owner.


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allowlist: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<b>address</b>, bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DelegationPermission"></a>

## Enum `DelegationPermission`



<pre><code>enum <a href="delegation_pool.md#0x1_delegation_pool_DelegationPermission">DelegationPermission</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>DelegationPoolManagementPermission</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>StakeManagementPermission</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_delegation_pool_AddStake"></a>

## Struct `AddStake`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_AddStake">AddStake</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_added: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>add_stake_fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_AddStakeEvent"></a>

## Struct `AddStakeEvent`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_AddStakeEvent">AddStakeEvent</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_added: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>add_stake_fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_ReactivateStake"></a>

## Struct `ReactivateStake`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStake">ReactivateStake</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_reactivated: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_ReactivateStakeEvent"></a>

## Struct `ReactivateStakeEvent`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStakeEvent">ReactivateStakeEvent</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount_reactivated: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_UnlockStake"></a>

## Struct `UnlockStake`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_UnlockStake">UnlockStake</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
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

<a id="0x1_delegation_pool_UnlockStakeEvent"></a>

## Struct `UnlockStakeEvent`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_UnlockStakeEvent">UnlockStakeEvent</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
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

<a id="0x1_delegation_pool_WithdrawStake"></a>

## Struct `WithdrawStake`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStake">WithdrawStake</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
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

<a id="0x1_delegation_pool_WithdrawStakeEvent"></a>

## Struct `WithdrawStakeEvent`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStakeEvent">WithdrawStakeEvent</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
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

<a id="0x1_delegation_pool_DistributeCommissionEvent"></a>

## Struct `DistributeCommissionEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">DistributeCommissionEvent</a> <b>has</b> drop, store
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
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>commission_active: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>commission_pending_inactive: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DistributeCommission"></a>

## Struct `DistributeCommission`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DistributeCommission">DistributeCommission</a> <b>has</b> drop, store
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
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>beneficiary: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>commission_active: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>commission_pending_inactive: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_Vote"></a>

## Struct `Vote`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_Vote">Vote</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>num_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>should_pass: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_VoteEvent"></a>

## Struct `VoteEvent`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_VoteEvent">VoteEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>num_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>should_pass: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_CreateProposal"></a>

## Struct `CreateProposal`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_CreateProposal">CreateProposal</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_CreateProposalEvent"></a>

## Struct `CreateProposalEvent`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_CreateProposalEvent">CreateProposalEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DelegateVotingPower"></a>

## Struct `DelegateVotingPower`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPower">DelegateVotingPower</a> <b>has</b> drop, store
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
<code>delegator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DelegateVotingPowerEvent"></a>

## Struct `DelegateVotingPowerEvent`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPowerEvent">DelegateVotingPowerEvent</a> <b>has</b> drop, store
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
<code>delegator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_SetBeneficiaryForOperator"></a>

## Struct `SetBeneficiaryForOperator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_SetBeneficiaryForOperator">SetBeneficiaryForOperator</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_beneficiary: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_beneficiary: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_CommissionPercentageChange"></a>

## Struct `CommissionPercentageChange`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_CommissionPercentageChange">CommissionPercentageChange</a> <b>has</b> drop, store
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
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>commission_percentage_next_lockup_cycle: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_EnableDelegatorsAllowlisting"></a>

## Struct `EnableDelegatorsAllowlisting`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_EnableDelegatorsAllowlisting">EnableDelegatorsAllowlisting</a> <b>has</b> drop, store
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

<a id="0x1_delegation_pool_DisableDelegatorsAllowlisting"></a>

## Struct `DisableDelegatorsAllowlisting`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DisableDelegatorsAllowlisting">DisableDelegatorsAllowlisting</a> <b>has</b> drop, store
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

<a id="0x1_delegation_pool_AllowlistDelegator"></a>

## Struct `AllowlistDelegator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_AllowlistDelegator">AllowlistDelegator</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_RemoveDelegatorFromAllowlist"></a>

## Struct `RemoveDelegatorFromAllowlist`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_RemoveDelegatorFromAllowlist">RemoveDelegatorFromAllowlist</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_EvictDelegator"></a>

## Struct `EvictDelegator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_EvictDelegator">EvictDelegator</a> <b>has</b> drop, store
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
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_delegation_pool_MAX_U64"></a>



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_delegation_pool_EDEPRECATED_FUNCTION"></a>

Function is deprecated.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>: u64 = 12;
</code></pre>



<a id="0x1_delegation_pool_EDISABLED_FUNCTION"></a>

The function is disabled or hasn't been enabled.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDISABLED_FUNCTION">EDISABLED_FUNCTION</a>: u64 = 13;
</code></pre>



<a id="0x1_delegation_pool_ENOT_OPERATOR"></a>

The account is not the operator of the stake pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ENOT_OPERATOR">ENOT_OPERATOR</a>: u64 = 18;
</code></pre>



<a id="0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS"></a>

Account is already owning a delegation pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>: u64 = 2;
</code></pre>



<a id="0x1_delegation_pool_EOWNER_CAP_NOT_FOUND"></a>

Delegation pool owner capability does not exist at the provided account.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a>: u64 = 4;
</code></pre>



<a id="0x1_delegation_pool_EINSUFFICIENT_PROPOSER_STAKE"></a>

The voter does not have sufficient stake to create a proposal.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EINSUFFICIENT_PROPOSER_STAKE">EINSUFFICIENT_PROPOSER_STAKE</a>: u64 = 15;
</code></pre>



<a id="0x1_delegation_pool_ENO_VOTING_POWER"></a>

The voter does not have any voting power on this proposal.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ENO_VOTING_POWER">ENO_VOTING_POWER</a>: u64 = 16;
</code></pre>



<a id="0x1_delegation_pool_EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING"></a>

The stake pool has already voted on the proposal before enabling partial governance voting on this delegation pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING">EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING</a>: u64 = 17;
</code></pre>



<a id="0x1_delegation_pool_ECANNOT_EVICT_ALLOWLISTED_DELEGATOR"></a>

Cannot evict an allowlisted delegator, should remove them from the allowlist first.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ECANNOT_EVICT_ALLOWLISTED_DELEGATOR">ECANNOT_EVICT_ALLOWLISTED_DELEGATOR</a>: u64 = 26;
</code></pre>



<a id="0x1_delegation_pool_ECANNOT_UNLOCK_NULL_SHAREHOLDER"></a>

Cannot unlock the accumulated active stake of NULL_SHAREHOLDER(0x0).


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ECANNOT_UNLOCK_NULL_SHAREHOLDER">ECANNOT_UNLOCK_NULL_SHAREHOLDER</a>: u64 = 27;
</code></pre>



<a id="0x1_delegation_pool_ECAN_NO_LONGER_SET_DELEGATED_VOTER"></a>

Use delegator voting flow instead. Delegation pools can no longer specify a single delegated voter.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ECAN_NO_LONGER_SET_DELEGATED_VOTER">ECAN_NO_LONGER_SET_DELEGATED_VOTER</a>: u64 = 29;
</code></pre>



<a id="0x1_delegation_pool_ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED"></a>

Changing operator commission rate in delegation pool is not supported.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED">ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED</a>: u64 = 22;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATION_POOLS_DISABLED"></a>

Creating delegation pools is not enabled yet.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOLS_DISABLED">EDELEGATION_POOLS_DISABLED</a>: u64 = 10;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST"></a>

Delegation pool does not exist at the provided pool address.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST">EDELEGATION_POOL_DOES_NOT_EXIST</a>: u64 = 3;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_ENABLED"></a>

Delegators allowlisting should be enabled to perform this operation.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_ENABLED">EDELEGATORS_ALLOWLISTING_NOT_ENABLED</a>: u64 = 24;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED"></a>

Delegators allowlisting is not supported.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED">EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED</a>: u64 = 23;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW"></a>

Delegator's active balance cannot be less than <code><a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a></code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW">EDELEGATOR_ACTIVE_BALANCE_TOO_LOW</a>: u64 = 8;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_NOT_ALLOWLISTED"></a>

Cannot add/reactivate stake unless being allowlisted by the pool owner.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_NOT_ALLOWLISTED">EDELEGATOR_NOT_ALLOWLISTED</a>: u64 = 25;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW"></a>

Delegator's pending_inactive balance cannot be less than <code><a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a></code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW">EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW</a>: u64 = 9;
</code></pre>



<a id="0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE"></a>

Commission percentage has to be between 0 and <code><a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a></code> - 100%.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>: u64 = 5;
</code></pre>



<a id="0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK"></a>

There is not enough <code>active</code> stake on the stake pool to <code>unlock</code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK">ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK</a>: u64 = 6;
</code></pre>



<a id="0x1_delegation_pool_ENO_DELEGATION_PERMISSION"></a>

Signer does not have permission to perform delegation logic.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ENO_DELEGATION_PERMISSION">ENO_DELEGATION_PERMISSION</a>: u64 = 28;
</code></pre>



<a id="0x1_delegation_pool_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED"></a>

Changing beneficiaries for operators is not supported.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED">EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED</a>: u64 = 19;
</code></pre>



<a id="0x1_delegation_pool_EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED"></a>

Partial governance voting hasn't been enabled on this delegation pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED">EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED</a>: u64 = 14;
</code></pre>



<a id="0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS"></a>

There is a pending withdrawal to be executed before <code>unlock</code>ing any new stake.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS">EPENDING_WITHDRAWAL_EXISTS</a>: u64 = 4;
</code></pre>



<a id="0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC"></a>

Slashing (if implemented) should not be applied to already <code>inactive</code> stake.
Not only it invalidates the accounting of past observed lockup cycles (OLC),
but is also unfair to delegators whose stake has been inactive before validator started misbehaving.
Additionally, the inactive stake does not count on the voting power of validator.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC">ESLASHED_INACTIVE_STAKE_ON_PAST_OLC</a>: u64 = 7;
</code></pre>



<a id="0x1_delegation_pool_ETOO_LARGE_COMMISSION_INCREASE"></a>

Commission percentage increase is too large.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ETOO_LARGE_COMMISSION_INCREASE">ETOO_LARGE_COMMISSION_INCREASE</a>: u64 = 20;
</code></pre>



<a id="0x1_delegation_pool_ETOO_LATE_COMMISSION_CHANGE"></a>

Commission percentage change is too late in this lockup period, and should be done at least a quarter (1/4) of the lockup duration before the lockup cycle ends.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ETOO_LATE_COMMISSION_CHANGE">ETOO_LATE_COMMISSION_CHANGE</a>: u64 = 21;
</code></pre>



<a id="0x1_delegation_pool_EWITHDRAW_ZERO_STAKE"></a>

Cannot request to withdraw zero stake.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EWITHDRAW_ZERO_STAKE">EWITHDRAW_ZERO_STAKE</a>: u64 = 11;
</code></pre>



<a id="0x1_delegation_pool_MAX_COMMISSION_INCREASE"></a>

Maximum commission percentage increase per lockup cycle. 10% is represented as 1000.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MAX_COMMISSION_INCREASE">MAX_COMMISSION_INCREASE</a>: u64 = 1000;
</code></pre>



<a id="0x1_delegation_pool_MAX_FEE"></a>

Maximum operator percentage fee(of double digit precision): 22.85% is represented as 2285


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>: u64 = 10000;
</code></pre>



<a id="0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL"></a>

Minimum coins to exist on a shares pool at all times.
Enforced per delegator for both active and pending_inactive pools.
This constraint ensures the share price cannot overly increase and lead to
substantial losses when buying shares (can lose at most 1 share which may
be worth a lot if current share price is high).
This constraint is not enforced on inactive pools as they only allow redeems
(can lose at most 1 coin regardless of current share price).


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>: u64 = 1000000000;
</code></pre>



<a id="0x1_delegation_pool_MODULE_SALT"></a>



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MODULE_SALT">MODULE_SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 100, 101, 108, 101, 103, 97, 116, 105, 111, 110, 95, 112, 111, 111, 108];
</code></pre>



<a id="0x1_delegation_pool_NULL_SHAREHOLDER"></a>

Special shareholder temporarily owning the <code>add_stake</code> fees charged during this epoch.
On each <code>add_stake</code> operation any resulted fee is used to buy active shares for this shareholder.
First synchronization after this epoch ends will distribute accumulated fees to the rest of the pool as refunds.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>: <b>address</b> = 0x0;
</code></pre>



<a id="0x1_delegation_pool_SHARES_SCALING_FACTOR"></a>

Scaling factor of shares pools used within the delegation pool


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>: u64 = 10000000000000000;
</code></pre>



<a id="0x1_delegation_pool_owner_cap_exists"></a>

## Function `owner_cap_exists`

Return whether supplied address <code>addr</code> is owner of a delegation pool.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_owned_pool_address"></a>

## Function `get_owned_pool_address`

Return address of the delegation pool owned by <code>owner</code> or fail if there is none.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(owner: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(owner: <b>address</b>): <b>address</b> <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner);
    <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>&gt;(owner).pool_address
}
</code></pre>



</details>

<a id="0x1_delegation_pool_delegation_pool_exists"></a>

## Function `delegation_pool_exists`

Return whether a delegation pool exists at supplied address <code>addr</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegation_pool_exists">delegation_pool_exists</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegation_pool_exists">delegation_pool_exists</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_partial_governance_voting_enabled"></a>

## Function `partial_governance_voting_enabled`

Return whether a delegation pool has already enabled partial governance voting.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address) && <a href="stake.md#0x1_stake_get_delegated_voter">stake::get_delegated_voter</a>(pool_address) == pool_address
}
</code></pre>



</details>

<a id="0x1_delegation_pool_observed_lockup_cycle"></a>

## Function `observed_lockup_cycle`

Return the index of current observed lockup cycle on delegation pool <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_observed_lockup_cycle">observed_lockup_cycle</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_observed_lockup_cycle">observed_lockup_cycle</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).observed_lockup_cycle.index
}
</code></pre>



</details>

<a id="0x1_delegation_pool_is_next_commission_percentage_effective"></a>

## Function `is_next_commission_percentage_effective`

Return whether the commission percentage for the next lockup cycle is effective.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_is_next_commission_percentage_effective">is_next_commission_percentage_effective</a>(pool_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_is_next_commission_percentage_effective">is_next_commission_percentage_effective</a>(pool_address: <b>address</b>): bool <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address) &&
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt;= <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address).effective_after_secs
}
</code></pre>



</details>

<a id="0x1_delegation_pool_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the operator commission percentage set on the delegation pool <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(
    pool_address: <b>address</b>
): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_is_next_commission_percentage_effective">is_next_commission_percentage_effective</a>(pool_address)) {
        <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle">operator_commission_percentage_next_lockup_cycle</a>(pool_address)
    } <b>else</b> {
        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).operator_commission_percentage
    }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle"></a>

## Function `operator_commission_percentage_next_lockup_cycle`

Return the operator commission percentage for the next lockup cycle.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle">operator_commission_percentage_next_lockup_cycle</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle">operator_commission_percentage_next_lockup_cycle</a>(
    pool_address: <b>address</b>
): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>if</b> (<b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address)) {
        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address).commission_percentage_next_lockup_cycle
    } <b>else</b> {
        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).operator_commission_percentage
    }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_shareholders_count_active_pool"></a>

## Function `shareholders_count_active_pool`

Return the number of delegators owning active stake within <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_shareholders_count_active_pool">shareholders_count_active_pool</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_shareholders_count_active_pool">shareholders_count_active_pool</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders_count">pool_u64::shareholders_count</a>(&<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).active_shares)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_delegation_pool_stake"></a>

## Function `get_delegation_pool_stake`

Return the stake amounts on <code>pool_address</code> in the different states:
(<code>active</code>,<code>inactive</code>,<code>pending_active</code>,<code>pending_inactive</code>)


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegation_pool_stake">get_delegation_pool_stake</a>(pool_address: <b>address</b>): (u64, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegation_pool_stake">get_delegation_pool_stake</a>(pool_address: <b>address</b>): (u64, u64, u64, u64) {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_pending_withdrawal"></a>

## Function `get_pending_withdrawal`

Return whether the given delegator has any withdrawable stake. If they recently requested to unlock
some stake and the stake pool's lockup cycle has not ended, their coins are not withdrawable yet.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_pending_withdrawal">get_pending_withdrawal</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_pending_withdrawal">get_pending_withdrawal</a>(
    pool_address: <b>address</b>,
    delegator_address: <b>address</b>
): (bool, u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>let</b> pool = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> (
        lockup_cycle_ended,
        _,
        pending_inactive,
        _,
        commission_pending_inactive
    ) = <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool);

    <b>let</b> (withdrawal_exists, withdrawal_olc) = <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool, delegator_address);
    <b>if</b> (!withdrawal_exists) {
        // <b>if</b> no pending withdrawal, there is neither inactive nor pending_inactive <a href="stake.md#0x1_stake">stake</a>
        (<b>false</b>, 0)
    } <b>else</b> {
        // delegator <b>has</b> either inactive or pending_inactive <a href="stake.md#0x1_stake">stake</a> due <b>to</b> automatic withdrawals
        <b>let</b> inactive_shares = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&pool.inactive_shares, withdrawal_olc);
        <b>if</b> (withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) {
            // <b>if</b> withdrawal's lockup cycle ended on delegation pool then it is inactive
            (<b>true</b>, <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(inactive_shares, delegator_address))
        } <b>else</b> {
            pending_inactive = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">pool_u64::shares_to_amount_with_total_coins</a>(
                inactive_shares,
                <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(inactive_shares, delegator_address),
                // exclude operator pending_inactive rewards not converted <b>to</b> shares yet
                pending_inactive - commission_pending_inactive
            );
            // <b>if</b> withdrawal's lockup cycle ended ONLY on <a href="stake.md#0x1_stake">stake</a> pool then it is also inactive
            (lockup_cycle_ended, pending_inactive)
        }
    }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_stake"></a>

## Function `get_stake`

Return total stake owned by <code>delegator_address</code> within delegation pool <code>pool_address</code>
in each of its individual states: (<code>active</code>,<code>inactive</code>,<code>pending_inactive</code>)


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_stake">get_stake</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): (u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_stake">get_stake</a>(
    pool_address: <b>address</b>,
    delegator_address: <b>address</b>
): (u64, u64, u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>let</b> pool = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> (
        lockup_cycle_ended,
        active,
        _,
        commission_active,
        commission_pending_inactive
    ) = <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool);

    <b>let</b> total_active_shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_total_shares">pool_u64::total_shares</a>(&pool.active_shares);
    <b>let</b> delegator_active_shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(&pool.active_shares, delegator_address);

    <b>let</b> (_, _, pending_active, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
    <b>if</b> (pending_active == 0) {
        // zero `pending_active` <a href="stake.md#0x1_stake">stake</a> indicates that either there are no `add_stake` fees or
        // previous epoch <b>has</b> ended and should identify shares owning these fees <b>as</b> released
        total_active_shares = total_active_shares - <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(&pool.active_shares, <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>);
        <b>if</b> (delegator_address == <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>) {
            delegator_active_shares = 0
        }
    };
    active = pool_u64::shares_to_amount_with_total_stats(
        &pool.active_shares,
        delegator_active_shares,
        // exclude operator active rewards not converted <b>to</b> shares yet
        active - commission_active,
        total_active_shares
    );

    // get state and <a href="stake.md#0x1_stake">stake</a> (0 <b>if</b> there is none) of the pending withdrawal
    <b>let</b> (withdrawal_inactive, withdrawal_stake) = <a href="delegation_pool.md#0x1_delegation_pool_get_pending_withdrawal">get_pending_withdrawal</a>(pool_address, delegator_address);
    // report non-active stakes accordingly <b>to</b> the state of the pending withdrawal
    <b>let</b> (inactive, pending_inactive) = <b>if</b> (withdrawal_inactive) (withdrawal_stake, 0) <b>else</b> (0, withdrawal_stake);

    // should also <b>include</b> commission rewards in case of the operator <a href="account.md#0x1_account">account</a>
    // operator rewards are actually used <b>to</b> buy shares which is introducing
    // some imprecision (received <a href="stake.md#0x1_stake">stake</a> would be slightly less)
    // but adding rewards onto the existing <a href="stake.md#0x1_stake">stake</a> is still a good approximation
    <b>if</b> (delegator_address == <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(get_operator(pool_address))) {
        active = active + commission_active;
        // in-flight pending_inactive commission can coexist <b>with</b> already inactive withdrawal
        <b>if</b> (lockup_cycle_ended) {
            inactive = inactive + commission_pending_inactive
        } <b>else</b> {
            pending_inactive = pending_inactive + commission_pending_inactive
        }
    };

    (active, inactive, pending_inactive)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_add_stake_fee"></a>

## Function `get_add_stake_fee`

Return refundable stake to be extracted from added <code>amount</code> at <code>add_stake</code> operation on pool <code>pool_address</code>.
If the validator produces rewards this epoch, added stake goes directly to <code>pending_active</code> and
does not earn rewards. However, all shares within a pool appreciate uniformly and when this epoch ends:
- either added shares are still <code>pending_active</code> and steal from rewards of existing <code>active</code> stake
- or have moved to <code>pending_inactive</code> and get full rewards (they displaced <code>active</code> stake at <code>unlock</code>)
To mitigate this, some of the added stake is extracted and fed back into the pool as placeholder
for the rewards the remaining stake would have earned if active:
extracted-fee = (amount - extracted-fee) * reward-rate% * (100% - operator-commission%)


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_add_stake_fee">get_add_stake_fee</a>(pool_address: <b>address</b>, amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_add_stake_fee">get_add_stake_fee</a>(
    pool_address: <b>address</b>,
    amount: u64
): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <b>if</b> (<a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(pool_address)) {
        <b>let</b> (rewards_rate, rewards_rate_denominator) = <a href="staking_config.md#0x1_staking_config_get_reward_rate">staking_config::get_reward_rate</a>(&<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());
        <b>if</b> (rewards_rate_denominator &gt; 0) {
            <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);

            rewards_rate = rewards_rate * (<a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a> - <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(pool_address));
            rewards_rate_denominator = rewards_rate_denominator * <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>;
            ((((amount <b>as</b> u128) * (rewards_rate <b>as</b> u128)) / ((rewards_rate <b>as</b> u128) + (rewards_rate_denominator <b>as</b> u128))) <b>as</b> u64)
        } <b>else</b> { 0 }
    } <b>else</b> { 0 }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_can_withdraw_pending_inactive"></a>

## Function `can_withdraw_pending_inactive`

Return whether <code>pending_inactive</code> stake can be directly withdrawn from
the delegation pool, implicitly its stake pool, in the special case
the validator had gone inactive before its lockup expired.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_can_withdraw_pending_inactive">can_withdraw_pending_inactive</a>(pool_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_can_withdraw_pending_inactive">can_withdraw_pending_inactive</a>(pool_address: <b>address</b>): bool {
    <a href="stake.md#0x1_stake_get_validator_state">stake::get_validator_state</a>(pool_address) == <a href="delegation_pool.md#0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a> &&
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt;= <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_voter_total_voting_power"></a>

## Function `calculate_and_update_voter_total_voting_power`

Return the total voting power of a delegator in a delegation pool. This function syncs DelegationPool to the
latest state.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voter_total_voting_power">calculate_and_update_voter_total_voting_power</a>(pool_address: <b>address</b>, voter: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voter_total_voting_power">calculate_and_update_voter_total_voting_power</a>(
    pool_address: <b>address</b>,
    voter: <b>address</b>
): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);
    // Delegation pool need <b>to</b> be synced <b>to</b> explain rewards(which could change the <a href="coin.md#0x1_coin">coin</a> amount) and
    // commission(which could cause share transfer).
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);
    <b>let</b> pool = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    <b>let</b> latest_delegated_votes = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, voter);
    <a href="delegation_pool.md#0x1_delegation_pool_calculate_total_voting_power">calculate_total_voting_power</a>(pool, latest_delegated_votes)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_remaining_voting_power"></a>

## Function `calculate_and_update_remaining_voting_power`

Return the remaining voting power of a delegator in a delegation pool on a proposal. This function syncs DelegationPool to the
latest state.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_remaining_voting_power">calculate_and_update_remaining_voting_power</a>(pool_address: <b>address</b>, voter_address: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_remaining_voting_power">calculate_and_update_remaining_voting_power</a>(
    pool_address: <b>address</b>,
    voter_address: <b>address</b>,
    proposal_id: u64
): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);
    // If the whole <a href="stake.md#0x1_stake">stake</a> pool <b>has</b> no <a href="voting.md#0x1_voting">voting</a> power(e.g. it <b>has</b> already voted before partial
    // governance <a href="voting.md#0x1_voting">voting</a> flag is enabled), the delegator also <b>has</b> no <a href="voting.md#0x1_voting">voting</a> power.
    <b>if</b> (<a href="aptos_governance.md#0x1_aptos_governance_get_remaining_voting_power">aptos_governance::get_remaining_voting_power</a>(pool_address, proposal_id) == 0) {
        <b>return</b> 0
    };

    <b>let</b> total_voting_power = <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voter_total_voting_power">calculate_and_update_voter_total_voting_power</a>(pool_address, voter_address);
    <b>let</b> governance_records = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    total_voting_power - <a href="delegation_pool.md#0x1_delegation_pool_get_used_voting_power">get_used_voting_power</a>(governance_records, voter_address, proposal_id)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegator_voter"></a>

## Function `calculate_and_update_delegator_voter`

Return the latest delegated voter of a delegator in a delegation pool. This function syncs DelegationPool to the
latest state.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter">calculate_and_update_delegator_voter</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter">calculate_and_update_delegator_voter</a>(
    pool_address: <b>address</b>,
    delegator_address: <b>address</b>
): <b>address</b> <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);
    <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(
        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address),
        <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address),
        delegator_address
    )
}
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_voting_delegation"></a>

## Function `calculate_and_update_voting_delegation`

Return the current state of a voting delegation of a delegator in a delegation pool.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voting_delegation">calculate_and_update_voting_delegation</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): (<b>address</b>, <b>address</b>, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voting_delegation">calculate_and_update_voting_delegation</a>(
    pool_address: <b>address</b>,
    delegator_address: <b>address</b>
): (<b>address</b>, <b>address</b>, u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);
    <b>let</b> vote_delegation = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(
        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address),
        <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address),
        delegator_address
    );

    (vote_delegation.voter, vote_delegation.pending_voter, vote_delegation.last_locked_until_secs)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_expected_stake_pool_address"></a>

## Function `get_expected_stake_pool_address`

Return the address of the stake pool to be created with the provided owner, and seed.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_expected_stake_pool_address">get_expected_stake_pool_address</a>(owner: <b>address</b>, delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_expected_stake_pool_address">get_expected_stake_pool_address</a>(owner: <b>address</b>, delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <b>address</b> {
    <b>let</b> seed = <a href="delegation_pool.md#0x1_delegation_pool_create_resource_account_seed">create_resource_account_seed</a>(delegation_pool_creation_seed);
    <a href="account.md#0x1_account_create_resource_address">account::create_resource_address</a>(&owner, seed)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_min_remaining_secs_for_commission_change"></a>

## Function `min_remaining_secs_for_commission_change`

Return the minimum remaining time in seconds for commission change, which is one fourth of the lockup duration.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_min_remaining_secs_for_commission_change">min_remaining_secs_for_commission_change</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_min_remaining_secs_for_commission_change">min_remaining_secs_for_commission_change</a>(): u64 {
    <b>let</b> config = <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();
    <a href="staking_config.md#0x1_staking_config_get_recurring_lockup_duration">staking_config::get_recurring_lockup_duration</a>(&config) / 4
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlisting_enabled"></a>

## Function `allowlisting_enabled`

Return whether allowlisting is enabled for the provided delegation pool.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address: <b>address</b>): bool {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(pool_address)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_delegator_allowlisted"></a>

## Function `delegator_allowlisted`

Return whether the provided delegator is allowlisted.
A delegator is allowlisted if:
- allowlisting is disabled on the pool
- delegator is part of the allowlist


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(
    pool_address: <b>address</b>,
    delegator_address: <b>address</b>,
): bool <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <b>if</b> (!<a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address)) { <b>return</b> <b>true</b> };
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(<b>freeze</b>(<a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address)), delegator_address)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_delegators_allowlist"></a>

## Function `get_delegators_allowlist`

Return allowlist or revert if allowlisting is not enabled for the provided delegation pool.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegators_allowlist">get_delegators_allowlist</a>(pool_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegators_allowlist">get_delegators_allowlist</a>(
    pool_address: <b>address</b>,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);

    <b>let</b> allowlist = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_for_each_ref">smart_table::for_each_ref</a>(<b>freeze</b>(<a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address)), |delegator, _v| {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> allowlist, *delegator);
    });
    allowlist
}
</code></pre>



</details>

<a id="0x1_delegation_pool_check_delegation_pool_management_permission"></a>

## Function `check_delegation_pool_management_permission`

Permissions


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">permissioned_signer::check_permission_exists</a>(s, DelegationPermission::DelegationPoolManagementPermission {}),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="delegation_pool.md#0x1_delegation_pool_ENO_DELEGATION_PERMISSION">ENO_DELEGATION_PERMISSION</a>),
    );
}
</code></pre>



</details>

<a id="0x1_delegation_pool_grant_delegation_pool_management_permission"></a>

## Function `grant_delegation_pool_management_permission`



<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_grant_delegation_pool_management_permission">grant_delegation_pool_management_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_grant_delegation_pool_management_permission">grant_delegation_pool_management_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">permissioned_signer::authorize_unlimited</a>(master, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>, DelegationPermission::DelegationPoolManagementPermission {})
}
</code></pre>



</details>

<a id="0x1_delegation_pool_check_stake_management_permission"></a>

## Function `check_stake_management_permission`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">permissioned_signer::check_permission_exists</a>(s, DelegationPermission::StakeManagementPermission {}),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="delegation_pool.md#0x1_delegation_pool_ENO_DELEGATION_PERMISSION">ENO_DELEGATION_PERMISSION</a>),
    );
}
</code></pre>



</details>

<a id="0x1_delegation_pool_grant_stake_management_permission"></a>

## Function `grant_stake_management_permission`



<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_grant_stake_management_permission">grant_stake_management_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_grant_stake_management_permission">grant_stake_management_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">permissioned_signer::authorize_unlimited</a>(master, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>, DelegationPermission::StakeManagementPermission {})
}
</code></pre>



</details>

<a id="0x1_delegation_pool_initialize_delegation_pool"></a>

## Function `initialize_delegation_pool`

Initialize a delegation pool of custom fixed <code>operator_commission_percentage</code>.
A resource account is created from <code>owner</code> signer and its supplied <code>delegation_pool_creation_seed</code>
to host the delegation pool resource and own the underlying stake pool.
Ownership over setting the operator/voter is granted to <code>owner</code> who has both roles initially.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_initialize_delegation_pool">initialize_delegation_pool</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator_commission_percentage: u64, delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_initialize_delegation_pool">initialize_delegation_pool</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    operator_commission_percentage: u64,
    delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(owner);
    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <b>assert</b>!(!<a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(owner_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>));
    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a> &lt;= <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>));

    // generate a seed <b>to</b> be used <b>to</b> create the resource <a href="account.md#0x1_account">account</a> hosting the delegation pool
    <b>let</b> seed = <a href="delegation_pool.md#0x1_delegation_pool_create_resource_account_seed">create_resource_account_seed</a>(delegation_pool_creation_seed);

    <b>let</b> (stake_pool_signer, stake_pool_signer_cap) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(owner, seed);
    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&stake_pool_signer);

    // stake_pool_signer will be owner of the <a href="stake.md#0x1_stake">stake</a> pool and have its `<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>`
    <b>let</b> pool_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&stake_pool_signer);
    <a href="stake.md#0x1_stake_initialize_stake_owner">stake::initialize_stake_owner</a>(&stake_pool_signer, 0, owner_address, owner_address);

    <b>let</b> inactive_shares = <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a>, <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>&gt;();
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(
        &<b>mut</b> inactive_shares,
        <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(0),
        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create_with_scaling_factor">pool_u64::create_with_scaling_factor</a>(<a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>)
    );

    <b>move_to</b>(&stake_pool_signer, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
        active_shares: <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create_with_scaling_factor">pool_u64::create_with_scaling_factor</a>(<a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>),
        observed_lockup_cycle: <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(0),
        inactive_shares,
        pending_withdrawals: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;<b>address</b>, <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a>&gt;(),
        stake_pool_signer_cap,
        total_coins_inactive: 0,
        operator_commission_percentage,
        add_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_AddStakeEvent">AddStakeEvent</a>&gt;(&stake_pool_signer),
        reactivate_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_ReactivateStakeEvent">ReactivateStakeEvent</a>&gt;(&stake_pool_signer),
        unlock_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_UnlockStakeEvent">UnlockStakeEvent</a>&gt;(&stake_pool_signer),
        withdraw_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_WithdrawStakeEvent">WithdrawStakeEvent</a>&gt;(&stake_pool_signer),
        distribute_commission_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">DistributeCommissionEvent</a>&gt;(&stake_pool_signer),
    });

    // save delegation pool ownership and resource <a href="account.md#0x1_account">account</a> <b>address</b> (inner <a href="stake.md#0x1_stake">stake</a> pool <b>address</b>) on `owner`
    <b>move_to</b>(owner, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a> { pool_address });

    // All delegation pool enable partial governance <a href="voting.md#0x1_voting">voting</a> by default once the feature flag is enabled.
    <a href="delegation_pool.md#0x1_delegation_pool_enable_partial_governance_voting">enable_partial_governance_voting</a>(pool_address);
}
</code></pre>



</details>

<a id="0x1_delegation_pool_beneficiary_for_operator"></a>

## Function `beneficiary_for_operator`

Return the beneficiary address of the operator.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(operator: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(operator: <b>address</b>): <b>address</b> <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator)) {
        <b>return</b> <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator).beneficiary_for_operator
    } <b>else</b> {
        operator
    }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_enable_partial_governance_voting"></a>

## Function `enable_partial_governance_voting`

Enable partial governance voting on a stake pool. The voter of this stake pool will be managed by this module.
The existing voter will be replaced. The function is permissionless.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_enable_partial_governance_voting">enable_partial_governance_voting</a>(pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_enable_partial_governance_voting">enable_partial_governance_voting</a>(
    pool_address: <b>address</b>,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation.
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a> = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> stake_pool_signer = <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>);
    // delegated_voter is managed by the <a href="stake.md#0x1_stake">stake</a> pool itself, which <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> is managed by <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>.
    // So <a href="voting.md#0x1_voting">voting</a> power of this <a href="stake.md#0x1_stake">stake</a> pool can only be used through this <b>module</b>.
    <a href="stake.md#0x1_stake_set_delegated_voter">stake::set_delegated_voter</a>(&stake_pool_signer, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&stake_pool_signer));

    <b>move_to</b>(&stake_pool_signer, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
        votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
        votes_per_proposal: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
        vote_delegation: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
        delegated_votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
        vote_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_VoteEvent">VoteEvent</a>&gt;(&stake_pool_signer),
        create_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_CreateProposalEvent">CreateProposalEvent</a>&gt;(&stake_pool_signer),
        delegate_voting_power_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPowerEvent">DelegateVotingPowerEvent</a>&gt;(&stake_pool_signer),
    });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_vote"></a>

## Function `vote`

Vote on a proposal with a voter's voting power. To successfully vote, the following conditions must be met:
1. The voting period of the proposal hasn't ended.
2. The delegation pool's lockup period ends after the voting period of the proposal.
3. The voter still has spare voting power on this proposal.
4. The delegation pool never votes on the proposal before enabling partial governance voting.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_vote">vote</a>(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, proposal_id: u64, voting_power: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_vote">vote</a>(
    voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    proposal_id: u64,
    voting_power: u64,
    should_pass: bool
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(voter);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation.
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> voter_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);
    <b>let</b> remaining_voting_power = <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_remaining_voting_power">calculate_and_update_remaining_voting_power</a>(
        pool_address,
        voter_address,
        proposal_id
    );
    <b>if</b> (voting_power &gt; remaining_voting_power) {
        voting_power = remaining_voting_power;
    };
    <a href="aptos_governance.md#0x1_aptos_governance_assert_proposal_expiration">aptos_governance::assert_proposal_expiration</a>(pool_address, proposal_id);
    <b>assert</b>!(voting_power &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ENO_VOTING_POWER">ENO_VOTING_POWER</a>));

    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    // Check a edge case during the transient period of enabling partial governance <a href="voting.md#0x1_voting">voting</a>.
    <a href="delegation_pool.md#0x1_delegation_pool_assert_and_update_proposal_used_voting_power">assert_and_update_proposal_used_voting_power</a>(governance_records, pool_address, proposal_id, voting_power);
    <b>let</b> used_voting_power = <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_used_voting_power">borrow_mut_used_voting_power</a>(governance_records, voter_address, proposal_id);
    *used_voting_power = *used_voting_power + voting_power;

    <b>let</b> pool_signer = <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address));
    <a href="aptos_governance.md#0x1_aptos_governance_partial_vote">aptos_governance::partial_vote</a>(&pool_signer, pool_address, proposal_id, voting_power, should_pass);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="delegation_pool.md#0x1_delegation_pool_Vote">Vote</a> {
                voter: voter_address,
                proposal_id,
                <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: pool_address,
                num_votes: voting_power,
                should_pass,
            }
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> governance_records.vote_events,
            <a href="delegation_pool.md#0x1_delegation_pool_VoteEvent">VoteEvent</a> {
                voter: voter_address,
                proposal_id,
                <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: pool_address,
                num_votes: voting_power,
                should_pass,
            }
        );
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_create_proposal"></a>

## Function `create_proposal`

A voter could create a governance proposal by this function. To successfully create a proposal, the voter's
voting power in THIS delegation pool must be not less than the minimum required voting power specified in
<code><a href="aptos_governance.md#0x1_aptos_governance">aptos_governance</a>.<b>move</b></code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_create_proposal">create_proposal</a>(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_create_proposal">create_proposal</a>(
    voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    is_multi_step_proposal: bool,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(voter);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);

    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> voter_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);
    <b>let</b> pool = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    <b>let</b> total_voting_power = <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegated_votes">calculate_and_update_delegated_votes</a>(pool, governance_records, voter_addr);
    <b>assert</b>!(
        total_voting_power &gt;= <a href="aptos_governance.md#0x1_aptos_governance_get_required_proposer_stake">aptos_governance::get_required_proposer_stake</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EINSUFFICIENT_PROPOSER_STAKE">EINSUFFICIENT_PROPOSER_STAKE</a>));
    <b>let</b> pool_signer = <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address));
    <b>let</b> proposal_id = <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2_impl">aptos_governance::create_proposal_v2_impl</a>(
        &pool_signer,
        pool_address,
        execution_hash,
        metadata_location,
        metadata_hash,
        is_multi_step_proposal,
    );

    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="delegation_pool.md#0x1_delegation_pool_CreateProposal">CreateProposal</a> {
                proposal_id,
                voter: voter_addr,
                <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: pool_address,
            }
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> governance_records.create_proposal_events,
            <a href="delegation_pool.md#0x1_delegation_pool_CreateProposalEvent">CreateProposalEvent</a> {
                proposal_id,
                voter: voter_addr,
                <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: pool_address,
            }
        );
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_owner_cap_exists"></a>

## Function `assert_owner_cap_exists`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>) {
    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(owner), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>));
}
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_delegation_pool_exists"></a>

## Function `assert_delegation_pool_exists`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address: <b>address</b>) {
    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_delegation_pool_exists">delegation_pool_exists</a>(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST">EDELEGATION_POOL_DOES_NOT_EXIST</a>));
}
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_min_active_balance"></a>

## Function `assert_min_active_balance`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>) {
    <b>let</b> balance = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(&pool.active_shares, delegator_address);
    <b>assert</b>!(balance &gt;= <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW">EDELEGATOR_ACTIVE_BALANCE_TOO_LOW</a>));
}
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_min_pending_inactive_balance"></a>

## Function `assert_min_pending_inactive_balance`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_min_pending_inactive_balance">assert_min_pending_inactive_balance</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_min_pending_inactive_balance">assert_min_pending_inactive_balance</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>) {
    <b>let</b> balance = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool), delegator_address);
    <b>assert</b>!(
        balance &gt;= <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW">EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW</a>)
    );
}
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_partial_governance_voting_enabled"></a>

## Function `assert_partial_governance_voting_enabled`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address: <b>address</b>) {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>assert</b>!(
        <a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED">EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED</a>)
    );
}
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_allowlisting_enabled"></a>

## Function `assert_allowlisting_enabled`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address: <b>address</b>) {
    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_ENABLED">EDELEGATORS_ALLOWLISTING_NOT_ENABLED</a>));
}
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_delegator_allowlisted"></a>

## Function `assert_delegator_allowlisted`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_delegator_allowlisted">assert_delegator_allowlisted</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_delegator_allowlisted">assert_delegator_allowlisted</a>(
    pool_address: <b>address</b>,
    delegator_address: <b>address</b>,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <b>assert</b>!(
        <a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address, delegator_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_NOT_ALLOWLISTED">EDELEGATOR_NOT_ALLOWLISTED</a>)
    );
}
</code></pre>



</details>

<a id="0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake"></a>

## Function `coins_to_redeem_to_ensure_min_stake`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake">coins_to_redeem_to_ensure_min_stake</a>(src_shares_pool: &<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake">coins_to_redeem_to_ensure_min_stake</a>(
    src_shares_pool: &<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>,
    shareholder: <b>address</b>,
    amount: u64,
): u64 {
    // find how many coins would be redeemed <b>if</b> supplying `amount`
    <b>let</b> redeemed_coins = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount">pool_u64::shares_to_amount</a>(
        src_shares_pool,
        <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(src_shares_pool, shareholder, amount)
    );
    // <b>if</b> balance drops under threshold then redeem it entirely
    <b>let</b> src_balance = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(src_shares_pool, shareholder);
    <b>if</b> (src_balance - redeemed_coins &lt; <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>) {
        amount = src_balance;
    };
    amount
}
</code></pre>



</details>

<a id="0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake"></a>

## Function `coins_to_transfer_to_ensure_min_stake`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(src_shares_pool: &<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, dst_shares_pool: &<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(
    src_shares_pool: &<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>,
    dst_shares_pool: &<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>,
    shareholder: <b>address</b>,
    amount: u64,
): u64 {
    // find how many coins would be redeemed from source <b>if</b> supplying `amount`
    <b>let</b> redeemed_coins = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount">pool_u64::shares_to_amount</a>(
        src_shares_pool,
        <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(src_shares_pool, shareholder, amount)
    );
    // <b>if</b> balance on destination would be less than threshold then redeem difference <b>to</b> threshold
    <b>let</b> dst_balance = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(dst_shares_pool, shareholder);
    <b>if</b> (dst_balance + redeemed_coins &lt; <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>) {
        // `redeemed_coins` &gt;= `amount` - 1 <b>as</b> redeem can lose at most 1 <a href="coin.md#0x1_coin">coin</a>
        amount = <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a> - dst_balance + 1;
    };
    // check <b>if</b> new `amount` drops balance on source under threshold and adjust
    <a href="delegation_pool.md#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake">coins_to_redeem_to_ensure_min_stake</a>(src_shares_pool, shareholder, amount)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_retrieve_stake_pool_owner"></a>

## Function `retrieve_stake_pool_owner`

Retrieves the shared resource account owning the stake pool in order
to forward a stake-management operation to this underlying pool.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&pool.stake_pool_signer_cap)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_pool_address"></a>

## Function `get_pool_address`

Get the address of delegation pool reference <code>pool</code>.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): <b>address</b> {
    <a href="account.md#0x1_account_get_signer_capability_address">account::get_signer_capability_address</a>(&pool.stake_pool_signer_cap)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_delegator_active_shares"></a>

## Function `get_delegator_active_shares`

Get the active share amount of the delegator.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator: <b>address</b>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator: <b>address</b>): u128 {
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(&pool.active_shares, delegator)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_delegator_pending_inactive_shares"></a>

## Function `get_delegator_pending_inactive_shares`

Get the pending inactive share amount of the delegator.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_pending_inactive_shares">get_delegator_pending_inactive_shares</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator: <b>address</b>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_pending_inactive_shares">get_delegator_pending_inactive_shares</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator: <b>address</b>): u128 {
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool), delegator)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_get_used_voting_power"></a>

## Function `get_used_voting_power`

Get the used voting power of a voter on a proposal.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_used_voting_power">get_used_voting_power</a>(governance_records: &<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, voter: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_used_voting_power">get_used_voting_power</a>(governance_records: &<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, voter: <b>address</b>, proposal_id: u64): u64 {
    <b>let</b> votes = &governance_records.votes;
    <b>let</b> key = <a href="delegation_pool.md#0x1_delegation_pool_VotingRecordKey">VotingRecordKey</a> {
        voter,
        proposal_id,
    };
    *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_with_default">smart_table::borrow_with_default</a>(votes, key, &0)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_create_resource_account_seed"></a>

## Function `create_resource_account_seed`

Create the seed to derive the resource account address.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_create_resource_account_seed">create_resource_account_seed</a>(delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_create_resource_account_seed">create_resource_account_seed</a>(
    delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> seed = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    // <b>include</b> <b>module</b> salt (before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> subseeds) <b>to</b> avoid conflicts <b>with</b> other modules creating resource accounts
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, <a href="delegation_pool.md#0x1_delegation_pool_MODULE_SALT">MODULE_SALT</a>);
    // <b>include</b> an additional salt in case the same resource <a href="account.md#0x1_account">account</a> <b>has</b> already been created
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, delegation_pool_creation_seed);
    seed
}
</code></pre>



</details>

<a id="0x1_delegation_pool_borrow_mut_used_voting_power"></a>

## Function `borrow_mut_used_voting_power`

Borrow the mutable used voting power of a voter on a proposal.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_used_voting_power">borrow_mut_used_voting_power</a>(governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, voter: <b>address</b>, proposal_id: u64): &<b>mut</b> u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_used_voting_power">borrow_mut_used_voting_power</a>(
    governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,
    voter: <b>address</b>,
    proposal_id: u64
): &<b>mut</b> u64 {
    <b>let</b> votes = &<b>mut</b> governance_records.votes;
    <b>let</b> key = <a href="delegation_pool.md#0x1_delegation_pool_VotingRecordKey">VotingRecordKey</a> {
        proposal_id,
        voter,
    };
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(votes, key, 0)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation"></a>

## Function `update_and_borrow_mut_delegator_vote_delegation`

Update VoteDelegation of a delegator to up-to-date then borrow_mut it.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, delegator: <b>address</b>): &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">delegation_pool::VoteDelegation</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(
    pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,
    delegator: <b>address</b>
): &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">VoteDelegation</a> {
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);
    <b>let</b> locked_until_secs = <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address);

    <b>let</b> vote_delegation_table = &<b>mut</b> governance_records.vote_delegation;
    // By default, a delegator's delegated voter is itself.
    // TODO: recycle storage when <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">VoteDelegation</a> equals <b>to</b> default value.
    <b>if</b> (!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(vote_delegation_table, delegator)) {
        <b>return</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(vote_delegation_table, delegator, <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">VoteDelegation</a> {
            voter: delegator,
            last_locked_until_secs: locked_until_secs,
            pending_voter: delegator,
        })
    };

    <b>let</b> vote_delegation = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(vote_delegation_table, delegator);
    // A lockup period <b>has</b> passed since last time `vote_delegation` was updated. Pending voter takes effect.
    <b>if</b> (vote_delegation.last_locked_until_secs &lt; locked_until_secs) {
        vote_delegation.voter = vote_delegation.pending_voter;
        vote_delegation.last_locked_until_secs = locked_until_secs;
    };
    vote_delegation
}
</code></pre>



</details>

<a id="0x1_delegation_pool_update_and_borrow_mut_delegated_votes"></a>

## Function `update_and_borrow_mut_delegated_votes`

Update DelegatedVotes of a voter to up-to-date then borrow_mut it.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, voter: <b>address</b>): &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">delegation_pool::DelegatedVotes</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(
    pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,
    voter: <b>address</b>
): &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a> {
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);
    <b>let</b> locked_until_secs = <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address);

    <b>let</b> delegated_votes_per_voter = &<b>mut</b> governance_records.delegated_votes;
    // By default, a delegator's voter is itself.
    // TODO: recycle storage when <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a> equals <b>to</b> default value.
    <b>if</b> (!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(delegated_votes_per_voter, voter)) {
        <b>let</b> active_shares = <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(pool, voter);
        <b>let</b> inactive_shares = <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_pending_inactive_shares">get_delegator_pending_inactive_shares</a>(pool, voter);
        <b>return</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(delegated_votes_per_voter, voter, <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a> {
            active_shares,
            pending_inactive_shares: inactive_shares,
            active_shares_next_lockup: active_shares,
            last_locked_until_secs: locked_until_secs,
        })
    };

    <b>let</b> delegated_votes = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(delegated_votes_per_voter, voter);
    // A lockup period <b>has</b> passed since last time `delegated_votes` was updated. Pending voter takes effect.
    <b>if</b> (delegated_votes.last_locked_until_secs &lt; locked_until_secs) {
        delegated_votes.active_shares = delegated_votes.active_shares_next_lockup;
        delegated_votes.pending_inactive_shares = 0;
        delegated_votes.last_locked_until_secs = locked_until_secs;
    };
    delegated_votes
}
</code></pre>



</details>

<a id="0x1_delegation_pool_olc_with_index"></a>

## Function `olc_with_index`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(index: u64): <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(index: u64): <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a> { index }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_total_voting_power"></a>

## Function `calculate_total_voting_power`

Given the amounts of shares in <code>active_shares</code> pool and <code>inactive_shares</code> pool, calculate the total voting
power, which equals to the sum of the coin amounts.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_total_voting_power">calculate_total_voting_power</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, latest_delegated_votes: &<a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">delegation_pool::DelegatedVotes</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_total_voting_power">calculate_total_voting_power</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, latest_delegated_votes: &<a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a>): u64 {
    <b>let</b> active_amount = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount">pool_u64::shares_to_amount</a>(
        &<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>.active_shares,
        latest_delegated_votes.active_shares);
    <b>let</b> pending_inactive_amount = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount">pool_u64::shares_to_amount</a>(
        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>),
        latest_delegated_votes.pending_inactive_shares);
    active_amount + pending_inactive_amount
}
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegator_voter_internal"></a>

## Function `calculate_and_update_delegator_voter_internal`

Update VoteDelegation of a delegator to up-to-date then return the latest voter.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, delegator: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(
    pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,
    delegator: <b>address</b>
): <b>address</b> {
    <b>let</b> vote_delegation = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(pool, governance_records, delegator);
    vote_delegation.voter
}
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegated_votes"></a>

## Function `calculate_and_update_delegated_votes`

Update DelegatedVotes of a voter to up-to-date then return the total voting power of this voter.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegated_votes">calculate_and_update_delegated_votes</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, voter: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegated_votes">calculate_and_update_delegated_votes</a>(
    pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,
    voter: <b>address</b>
): u64 {
    <b>let</b> delegated_votes = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, voter);
    <a href="delegation_pool.md#0x1_delegation_pool_calculate_total_voting_power">calculate_total_voting_power</a>(pool, delegated_votes)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_borrow_mut_delegators_allowlist"></a>

## Function `borrow_mut_delegators_allowlist`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address: <b>address</b>): &<b>mut</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<b>address</b>, bool&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address: <b>address</b>): &<b>mut</b> SmartTable&lt;<b>address</b>, bool&gt; {
    &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(pool_address).allowlist
}
</code></pre>



</details>

<a id="0x1_delegation_pool_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the underlying stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_operator">set_operator</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_operator">set_operator</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_operator: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(owner);
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    // ensure the <b>old</b> operator is paid its uncommitted commission rewards
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);
    <a href="stake.md#0x1_stake_set_operator">stake::set_operator</a>(&<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address)), new_operator);
}
</code></pre>



</details>

<a id="0x1_delegation_pool_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Allows an operator to change its beneficiary. Any existing unpaid commission rewards will be paid to the new
beneficiary. To ensure payment to the current beneficiary, one should first call <code>synchronize_delegation_pool</code>
before switching the beneficiary. An operator can set one beneficiary for delegation pools, not a separate
one for each pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(operator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_beneficiary: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(
    operator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_beneficiary: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(operator);
    // The beneficiay <b>address</b> of an operator is stored under the operator's <b>address</b>.
    // So, the operator does not need <b>to</b> be validated <b>with</b> respect <b>to</b> a staking pool.
    <b>let</b> operator_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator);
    <b>let</b> old_beneficiary = <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(operator_addr);
    <b>if</b> (<b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator_addr)) {
        <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator_addr).beneficiary_for_operator = new_beneficiary;
    } <b>else</b> {
        <b>move_to</b>(operator, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> { beneficiary_for_operator: new_beneficiary });
    };

    emit(<a href="delegation_pool.md#0x1_delegation_pool_SetBeneficiaryForOperator">SetBeneficiaryForOperator</a> {
        operator: operator_addr,
        old_beneficiary,
        new_beneficiary,
    });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_update_commission_percentage"></a>

## Function `update_commission_percentage`

Allows an owner to update the commission percentage for the operator of the underlying stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_commission_percentage">update_commission_percentage</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_commission_percentage">update_commission_percentage</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_commission_percentage: u64
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(owner);
    <b>assert</b>!(new_commission_percentage &lt;= <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>));
    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(owner_address);
    <b>assert</b>!(
        <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(pool_address) + <a href="delegation_pool.md#0x1_delegation_pool_MAX_COMMISSION_INCREASE">MAX_COMMISSION_INCREASE</a> &gt;= new_commission_percentage,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ETOO_LARGE_COMMISSION_INCREASE">ETOO_LARGE_COMMISSION_INCREASE</a>)
    );
    <b>assert</b>!(
        <a href="stake.md#0x1_stake_get_remaining_lockup_secs">stake::get_remaining_lockup_secs</a>(pool_address) &gt;= <a href="delegation_pool.md#0x1_delegation_pool_min_remaining_secs_for_commission_change">min_remaining_secs_for_commission_change</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_ETOO_LATE_COMMISSION_CHANGE">ETOO_LATE_COMMISSION_CHANGE</a>)
    );

    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation. this <b>ensures</b>:
    // (1) the operator is paid its uncommitted commission rewards <b>with</b> the <b>old</b> commission percentage, and
    // (2) <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> pending commission percentage change is applied before the new commission percentage is set.
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>if</b> (<b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address)) {
        <b>let</b> commission_percentage = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address);
        commission_percentage.commission_percentage_next_lockup_cycle = new_commission_percentage;
        commission_percentage.effective_after_secs = <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address);
    } <b>else</b> {
        <b>let</b> <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a> = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
        <b>let</b> pool_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>.stake_pool_signer_cap);
        <b>move_to</b>(&pool_signer, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
            commission_percentage_next_lockup_cycle: new_commission_percentage,
            effective_after_secs: <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address),
        });
    };

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_CommissionPercentageChange">CommissionPercentageChange</a> {
        pool_address,
        owner: owner_address,
        commission_percentage_next_lockup_cycle: new_commission_percentage,
    });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_set_delegated_voter"></a>

## Function `set_delegated_voter`

Deprecated. Use the partial governance voting flow instead.


<pre><code>#[deprecated]
<b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_delegated_voter">set_delegated_voter</a>(_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_voter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_delegated_voter">set_delegated_voter</a>(
    _owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _new_voter: <b>address</b>
) {
    <b>abort</b> <a href="delegation_pool.md#0x1_delegation_pool_ECAN_NO_LONGER_SET_DELEGATED_VOTER">ECAN_NO_LONGER_SET_DELEGATED_VOTER</a>
}
</code></pre>



</details>

<a id="0x1_delegation_pool_delegate_voting_power"></a>

## Function `delegate_voting_power`

Allows a delegator to delegate its voting power to a voter. If this delegator already has a delegated voter,
this change won't take effects until the next lockup period.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegate_voting_power">delegate_voting_power</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_voter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegate_voting_power">delegate_voting_power</a>(
    delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    new_voter: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(delegator);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);

    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> delegator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);
    <b>let</b> <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a> = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    <b>let</b> delegator_vote_delegation = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(
        <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>,
        governance_records,
        delegator_address
    );
    <b>let</b> pending_voter: <b>address</b> = delegator_vote_delegation.pending_voter;

    // No need <b>to</b> <b>update</b> <b>if</b> the voter doesn't really change.
    <b>if</b> (pending_voter != new_voter) {
        delegator_vote_delegation.pending_voter = new_voter;
        <b>let</b> active_shares = <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>, delegator_address);
        // &lt;active shares&gt; of &lt;pending voter of shareholder&gt; -= &lt;active_shares&gt;
        // &lt;active shares&gt; of &lt;new voter of shareholder&gt; += &lt;active_shares&gt;
        <b>let</b> pending_delegated_votes = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(
            <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>,
            governance_records,
            pending_voter
        );
        pending_delegated_votes.active_shares_next_lockup =
            pending_delegated_votes.active_shares_next_lockup - active_shares;

        <b>let</b> new_delegated_votes = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(
            <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>,
            governance_records,
            new_voter
        );
        new_delegated_votes.active_shares_next_lockup =
            new_delegated_votes.active_shares_next_lockup + active_shares;
    };

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPower">DelegateVotingPower</a> {
            pool_address,
            delegator: delegator_address,
            voter: new_voter,
        })
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> governance_records.delegate_voting_power_events, <a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPowerEvent">DelegateVotingPowerEvent</a> {
            pool_address,
            delegator: delegator_address,
            voter: new_voter,
        });
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_enable_delegators_allowlisting"></a>

## Function `enable_delegators_allowlisting`

Enable delegators allowlisting as the pool owner.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_enable_delegators_allowlisting">enable_delegators_allowlisting</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_enable_delegators_allowlisting">enable_delegators_allowlisting</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(owner);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_delegation_pool_allowlisting_enabled">features::delegation_pool_allowlisting_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED">EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED</a>)
    );

    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));
    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address)) { <b>return</b> };

    <b>let</b> pool_signer = <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address));
    <b>move_to</b>(&pool_signer, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> { allowlist: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>&lt;<b>address</b>, bool&gt;() });

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_EnableDelegatorsAllowlisting">EnableDelegatorsAllowlisting</a> { pool_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_disable_delegators_allowlisting"></a>

## Function `disable_delegators_allowlisting`

Disable delegators allowlisting as the pool owner. The existing allowlist will be emptied.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_disable_delegators_allowlisting">disable_delegators_allowlisting</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_disable_delegators_allowlisting">disable_delegators_allowlisting</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(owner);
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));
    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);

    <b>let</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> { allowlist } = <b>move_from</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(pool_address);
    // <b>if</b> the allowlist becomes too large, the owner can always remove some delegators
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_destroy">smart_table::destroy</a>(allowlist);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_DisableDelegatorsAllowlisting">DisableDelegatorsAllowlisting</a> { pool_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_delegator"></a>

## Function `allowlist_delegator`

Allowlist a delegator as the pool owner.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_allowlist_delegator">allowlist_delegator</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_allowlist_delegator">allowlist_delegator</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    delegator_address: <b>address</b>,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(owner);
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));
    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);

    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address, delegator_address)) { <b>return</b> };

    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(<a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address), delegator_address, <b>true</b>);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_AllowlistDelegator">AllowlistDelegator</a> { pool_address, delegator_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_remove_delegator_from_allowlist"></a>

## Function `remove_delegator_from_allowlist`

Remove a delegator from the allowlist as the pool owner, but do not unlock their stake.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_remove_delegator_from_allowlist">remove_delegator_from_allowlist</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_remove_delegator_from_allowlist">remove_delegator_from_allowlist</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    delegator_address: <b>address</b>,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(owner);
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));
    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);

    <b>if</b> (!<a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address, delegator_address)) { <b>return</b> };

    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_remove">smart_table::remove</a>(<a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address), delegator_address);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_RemoveDelegatorFromAllowlist">RemoveDelegatorFromAllowlist</a> { pool_address, delegator_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_evict_delegator"></a>

## Function `evict_delegator`

Evict a delegator that is not allowlisted by unlocking their entire stake.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_evict_delegator">evict_delegator</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_evict_delegator">evict_delegator</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    delegator_address: <b>address</b>,
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_delegation_pool_management_permission">check_delegation_pool_management_permission</a>(owner);
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));
    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);
    <b>assert</b>!(
        !<a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address, delegator_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_ECANNOT_EVICT_ALLOWLISTED_DELEGATOR">ECANNOT_EVICT_ALLOWLISTED_DELEGATOR</a>)
    );

    // synchronize pool in order <b>to</b> query latest balance of delegator
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> pool = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(pool, delegator_address) == 0) { <b>return</b> };

    <a href="delegation_pool.md#0x1_delegation_pool_unlock_internal">unlock_internal</a>(delegator_address, pool_address, <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(&pool.active_shares, delegator_address));

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_EvictDelegator">EvictDelegator</a> { pool_address, delegator_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins to the delegation pool <code>pool_address</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_add_stake">add_stake</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_add_stake">add_stake</a>(
    delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(delegator);
    // short-circuit <b>if</b> amount <b>to</b> add is 0 so no <a href="event.md#0x1_event">event</a> is emitted
    <b>if</b> (amount == 0) { <b>return</b> };

    <b>let</b> delegator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegator_allowlisted">assert_delegator_allowlisted</a>(pool_address, delegator_address);

    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    // fee <b>to</b> be charged for adding `amount` <a href="stake.md#0x1_stake">stake</a> on this delegation pool at this epoch
    <b>let</b> add_stake_fee = <a href="delegation_pool.md#0x1_delegation_pool_get_add_stake_fee">get_add_stake_fee</a>(pool_address, amount);

    <b>let</b> pool = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);

    // <a href="stake.md#0x1_stake">stake</a> the entire amount <b>to</b> the <a href="stake.md#0x1_stake">stake</a> pool
    <a href="aptos_account.md#0x1_aptos_account_transfer">aptos_account::transfer</a>(delegator, pool_address, amount);
    <a href="stake.md#0x1_stake_add_stake">stake::add_stake</a>(&<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool), amount);

    // but buy shares for delegator just for the remaining amount after fee
    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool, delegator_address, amount - add_stake_fee);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool, delegator_address);

    // grant temporary ownership over `add_stake` fees <b>to</b> a separate shareholder in order <b>to</b>:
    // - not mistake them for rewards <b>to</b> pay the operator from
    // - distribute them together <b>with</b> the `active` rewards when this epoch ends
    // in order <b>to</b> appreciate all shares on the active pool atomically
    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool, <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>, add_stake_fee);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="delegation_pool.md#0x1_delegation_pool_AddStake">AddStake</a> {
                pool_address,
                delegator_address,
                amount_added: amount,
                add_stake_fee,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> pool.add_stake_events,
            <a href="delegation_pool.md#0x1_delegation_pool_AddStakeEvent">AddStakeEvent</a> {
                pool_address,
                delegator_address,
                amount_added: amount,
                add_stake_fee,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_unlock"></a>

## Function `unlock`

Unlock <code>amount</code> from the active + pending_active stake of <code>delegator</code> or
at most how much active stake there is on the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock">unlock</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock">unlock</a>(
    delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(delegator);
    // short-circuit <b>if</b> amount <b>to</b> unlock is 0 so no <a href="event.md#0x1_event">event</a> is emitted
    <b>if</b> (amount == 0) { <b>return</b> };

    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> delegator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);
    <a href="delegation_pool.md#0x1_delegation_pool_unlock_internal">unlock_internal</a>(delegator_address, pool_address, amount);
}
</code></pre>



</details>

<a id="0x1_delegation_pool_unlock_internal"></a>

## Function `unlock_internal`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock_internal">unlock_internal</a>(delegator_address: <b>address</b>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock_internal">unlock_internal</a>(
    delegator_address: <b>address</b>,
    pool_address: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    <b>assert</b>!(delegator_address != <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ECANNOT_UNLOCK_NULL_SHAREHOLDER">ECANNOT_UNLOCK_NULL_SHAREHOLDER</a>));

    // fail unlock of more <a href="stake.md#0x1_stake">stake</a> than `active` on the <a href="stake.md#0x1_stake">stake</a> pool
    <b>let</b> (active, _, _, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
    <b>assert</b>!(amount &lt;= active, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK">ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK</a>));

    <b>let</b> pool = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    amount = <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(
        &pool.active_shares,
        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool),
        delegator_address,
        amount,
    );
    amount = <a href="delegation_pool.md#0x1_delegation_pool_redeem_active_shares">redeem_active_shares</a>(pool, delegator_address, amount);

    <a href="stake.md#0x1_stake_unlock">stake::unlock</a>(&<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool), amount);

    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(pool, delegator_address, amount);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_min_pending_inactive_balance">assert_min_pending_inactive_balance</a>(pool, delegator_address);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="delegation_pool.md#0x1_delegation_pool_UnlockStake">UnlockStake</a> {
                pool_address,
                delegator_address,
                amount_unlocked: amount,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> pool.unlock_stake_events,
            <a href="delegation_pool.md#0x1_delegation_pool_UnlockStakeEvent">UnlockStakeEvent</a> {
                pool_address,
                delegator_address,
                amount_unlocked: amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_reactivate_stake">reactivate_stake</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_reactivate_stake">reactivate_stake</a>(
    delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(delegator);
    // short-circuit <b>if</b> amount <b>to</b> reactivate is 0 so no <a href="event.md#0x1_event">event</a> is emitted
    <b>if</b> (amount == 0) { <b>return</b> };

    <b>let</b> delegator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegator_allowlisted">assert_delegator_allowlisted</a>(pool_address, delegator_address);

    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> pool = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    amount = <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(
        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool),
        &pool.active_shares,
        delegator_address,
        amount,
    );
    <b>let</b> observed_lockup_cycle = pool.observed_lockup_cycle;
    amount = <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(pool, delegator_address, amount, observed_lockup_cycle);

    <a href="stake.md#0x1_stake_reactivate_stake">stake::reactivate_stake</a>(&<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool), amount);

    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool, delegator_address, amount);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool, delegator_address);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStake">ReactivateStake</a> {
                pool_address,
                delegator_address,
                amount_reactivated: amount,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> pool.reactivate_stake_events,
            <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStakeEvent">ReactivateStakeEvent</a> {
                pool_address,
                delegator_address,
                amount_reactivated: amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of owned inactive stake from the delegation pool at <code>pool_address</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw">withdraw</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw">withdraw</a>(
    delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    pool_address: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_check_stake_management_permission">check_stake_management_permission</a>(delegator);
    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EWITHDRAW_ZERO_STAKE">EWITHDRAW_ZERO_STAKE</a>));
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);
    <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(<b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator), amount);
}
</code></pre>



</details>

<a id="0x1_delegation_pool_withdraw_internal"></a>

## Function `withdraw_internal`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(
    pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    delegator_address: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    // TODO: recycle storage when a delegator fully exits the delegation pool.
    // short-circuit <b>if</b> amount <b>to</b> withdraw is 0 so no <a href="event.md#0x1_event">event</a> is emitted
    <b>if</b> (amount == 0) { <b>return</b> };

    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);
    <b>let</b> (withdrawal_exists, withdrawal_olc) = <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool, delegator_address);
    // exit <b>if</b> no withdrawal or (it is pending and cannot withdraw pending_inactive <a href="stake.md#0x1_stake">stake</a> from <a href="stake.md#0x1_stake">stake</a> pool)
    <b>if</b> (!(
        withdrawal_exists &&
            (withdrawal_olc.index &lt; pool.observed_lockup_cycle.index || <a href="delegation_pool.md#0x1_delegation_pool_can_withdraw_pending_inactive">can_withdraw_pending_inactive</a>(pool_address))
    )) { <b>return</b> };

    <b>if</b> (withdrawal_olc.index == pool.observed_lockup_cycle.index) {
        amount = <a href="delegation_pool.md#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake">coins_to_redeem_to_ensure_min_stake</a>(
            <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool),
            delegator_address,
            amount,
        )
    };
    amount = <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(pool, delegator_address, amount, withdrawal_olc);

    <b>let</b> stake_pool_owner = &<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool);
    // <a href="stake.md#0x1_stake">stake</a> pool will inactivate entire pending_inactive <a href="stake.md#0x1_stake">stake</a> at `<a href="stake.md#0x1_stake_withdraw">stake::withdraw</a>` <b>to</b> make it withdrawable
    // however, bypassing the inactivation of excess <a href="stake.md#0x1_stake">stake</a> (inactivated but not withdrawn) <b>ensures</b>
    // the OLC is not advanced indefinitely on `unlock`-`withdraw` paired calls
    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_can_withdraw_pending_inactive">can_withdraw_pending_inactive</a>(pool_address)) {
        // get excess <a href="stake.md#0x1_stake">stake</a> before being entirely inactivated
        <b>let</b> (_, _, _, pending_inactive) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
        <b>if</b> (withdrawal_olc.index == pool.observed_lockup_cycle.index) {
            // `amount` less excess <b>if</b> withdrawing pending_inactive <a href="stake.md#0x1_stake">stake</a>
            pending_inactive = pending_inactive - amount
        };
        // escape excess <a href="stake.md#0x1_stake">stake</a> from inactivation
        <a href="stake.md#0x1_stake_reactivate_stake">stake::reactivate_stake</a>(stake_pool_owner, pending_inactive);
        <a href="stake.md#0x1_stake_withdraw">stake::withdraw</a>(stake_pool_owner, amount);
        // restore excess <a href="stake.md#0x1_stake">stake</a> <b>to</b> the pending_inactive state
        <a href="stake.md#0x1_stake_unlock">stake::unlock</a>(stake_pool_owner, pending_inactive);
    } <b>else</b> {
        // no excess <a href="stake.md#0x1_stake">stake</a> <b>if</b> `<a href="stake.md#0x1_stake_withdraw">stake::withdraw</a>` does not inactivate at all
        <a href="stake.md#0x1_stake_withdraw">stake::withdraw</a>(stake_pool_owner, amount);
    };
    <a href="aptos_account.md#0x1_aptos_account_transfer">aptos_account::transfer</a>(stake_pool_owner, delegator_address, amount);

    // commit withdrawal of possibly inactive <a href="stake.md#0x1_stake">stake</a> <b>to</b> the `total_coins_inactive`
    // known by the delegation pool in order <b>to</b> not mistake it for slashing at next synchronization
    <b>let</b> (_, inactive, _, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
    pool.total_coins_inactive = inactive;

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStake">WithdrawStake</a> {
                pool_address,
                delegator_address,
                amount_withdrawn: amount,
            },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> pool.withdraw_stake_events,
            <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStakeEvent">WithdrawStakeEvent</a> {
                pool_address,
                delegator_address,
                amount_withdrawn: amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_pending_withdrawal_exists"></a>

## Function `pending_withdrawal_exists`

Return the unique observed lockup cycle where delegator <code>delegator_address</code> may have
unlocking (or already unlocked) stake to be withdrawn from delegation pool <code>pool</code>.
A bool is returned to signal if a pending withdrawal exists at all.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>): (bool, <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>): (bool, <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a>) {
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&pool.pending_withdrawals, delegator_address)) {
        (<b>true</b>, *<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&pool.pending_withdrawals, delegator_address))
    } <b>else</b> {
        (<b>false</b>, <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(0))
    }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_pending_inactive_shares_pool_mut"></a>

## Function `pending_inactive_shares_pool_mut`

Return a mutable reference to the shares pool of <code>pending_inactive</code> stake on the
delegation pool, always the last item in <code>inactive_shares</code>.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): &<b>mut</b> <a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): &<b>mut</b> <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a> {
    <b>let</b> observed_lockup_cycle = pool.observed_lockup_cycle;
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> pool.inactive_shares, observed_lockup_cycle)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_pending_inactive_shares_pool"></a>

## Function `pending_inactive_shares_pool`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): &<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): &<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a> {
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&pool.inactive_shares, pool.observed_lockup_cycle)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_execute_pending_withdrawal"></a>

## Function `execute_pending_withdrawal`

Execute the pending withdrawal of <code>delegator_address</code> on delegation pool <code>pool</code>
if existing and already inactive to allow the creation of a new one.
<code>pending_inactive</code> stake would be left untouched even if withdrawable and should
be explicitly withdrawn by delegator


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_execute_pending_withdrawal">execute_pending_withdrawal</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_execute_pending_withdrawal">execute_pending_withdrawal</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    <b>let</b> (withdrawal_exists, withdrawal_olc) = <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool, delegator_address);
    <b>if</b> (withdrawal_exists && withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) {
        <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(pool, delegator_address, <a href="delegation_pool.md#0x1_delegation_pool_MAX_U64">MAX_U64</a>);
    }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_buy_in_active_shares"></a>

## Function `buy_in_active_shares`

Buy shares into the active pool on behalf of delegator <code>shareholder</code> who
deposited <code>coins_amount</code>. This function doesn't make any coin transfer.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, shareholder: <b>address</b>, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(
    pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    shareholder: <b>address</b>,
    coins_amount: u64,
): u128 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    <b>let</b> new_shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_amount_to_shares">pool_u64::amount_to_shares</a>(&pool.active_shares, coins_amount);
    // No need <b>to</b> buy 0 shares.
    <b>if</b> (new_shares == 0) { <b>return</b> 0 };

    // Always <b>update</b> governance records before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> change <b>to</b> the shares pool.
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);
    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address)) {
        <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_active_shares">update_governance_records_for_buy_in_active_shares</a>(pool, pool_address, new_shares, shareholder);
    };

    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(&<b>mut</b> pool.active_shares, shareholder, coins_amount);
    new_shares
}
</code></pre>



</details>

<a id="0x1_delegation_pool_buy_in_pending_inactive_shares"></a>

## Function `buy_in_pending_inactive_shares`

Buy shares into the pending_inactive pool on behalf of delegator <code>shareholder</code> who
redeemed <code>coins_amount</code> from the active pool to schedule it for unlocking.
If delegator's pending withdrawal exists and has been inactivated, execute it firstly
to ensure there is always only one withdrawal request.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, shareholder: <b>address</b>, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(
    pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    shareholder: <b>address</b>,
    coins_amount: u64,
): u128 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    <b>let</b> new_shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_amount_to_shares">pool_u64::amount_to_shares</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool), coins_amount);
    // never create a new pending withdrawal unless delegator owns some pending_inactive shares
    <b>if</b> (new_shares == 0) { <b>return</b> 0 };

    // Always <b>update</b> governance records before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> change <b>to</b> the shares pool.
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);
    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address)) {
        <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares">update_governance_records_for_buy_in_pending_inactive_shares</a>(pool, pool_address, new_shares, shareholder);
    };

    // cannot buy inactive shares, only pending_inactive at current lockup cycle
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool), shareholder, coins_amount);

    // execute the pending withdrawal <b>if</b> <b>exists</b> and is inactive before creating a new one
    <a href="delegation_pool.md#0x1_delegation_pool_execute_pending_withdrawal">execute_pending_withdrawal</a>(pool, shareholder);

    // save observed lockup cycle for the new pending withdrawal
    <b>let</b> observed_lockup_cycle = pool.observed_lockup_cycle;
    <b>assert</b>!(*<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut_with_default">table::borrow_mut_with_default</a>(
        &<b>mut</b> pool.pending_withdrawals,
        shareholder,
        observed_lockup_cycle
    ) == observed_lockup_cycle,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS">EPENDING_WITHDRAWAL_EXISTS</a>)
    );

    new_shares
}
</code></pre>



</details>

<a id="0x1_delegation_pool_amount_to_shares_to_redeem"></a>

## Function `amount_to_shares_to_redeem`

Convert <code>coins_amount</code> of coins to be redeemed from shares pool <code>shares_pool</code>
to the exact number of shares to redeem in order to achieve this.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(shares_pool: &<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(
    shares_pool: &<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>,
    shareholder: <b>address</b>,
    coins_amount: u64,
): u128 {
    <b>if</b> (coins_amount &gt;= <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(shares_pool, shareholder)) {
        // cap result at total shares of shareholder <b>to</b> pass `EINSUFFICIENT_SHARES` on subsequent redeem
        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(shares_pool, shareholder)
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_amount_to_shares">pool_u64::amount_to_shares</a>(shares_pool, coins_amount)
    }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_redeem_active_shares"></a>

## Function `redeem_active_shares`

Redeem shares from the active pool on behalf of delegator <code>shareholder</code> who
wants to unlock <code>coins_amount</code> of its active stake.
Extracted coins will be used to buy shares into the pending_inactive pool and
be available for withdrawal when current OLC ends.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_redeem_active_shares">redeem_active_shares</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, shareholder: <b>address</b>, coins_amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_redeem_active_shares">redeem_active_shares</a>(
    pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    shareholder: <b>address</b>,
    coins_amount: u64,
): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    <b>let</b> shares_to_redeem = <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(&pool.active_shares, shareholder, coins_amount);
    // silently exit <b>if</b> not a shareholder otherwise redeem would fail <b>with</b> `ESHAREHOLDER_NOT_FOUND`
    <b>if</b> (shares_to_redeem == 0) <b>return</b> 0;

    // Always <b>update</b> governance records before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> change <b>to</b> the shares pool.
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);
    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address)) {
        <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_active_shares">update_governanace_records_for_redeem_active_shares</a>(pool, pool_address, shares_to_redeem, shareholder);
    };

    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_redeem_shares">pool_u64::redeem_shares</a>(&<b>mut</b> pool.active_shares, shareholder, shares_to_redeem)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_redeem_inactive_shares"></a>

## Function `redeem_inactive_shares`

Redeem shares from the inactive pool at <code>lockup_cycle</code> < current OLC on behalf of
delegator <code>shareholder</code> who wants to withdraw <code>coins_amount</code> of its unlocked stake.
Redeem shares from the pending_inactive pool at <code>lockup_cycle</code> == current OLC on behalf of
delegator <code>shareholder</code> who wants to reactivate <code>coins_amount</code> of its unlocking stake.
For latter case, extracted coins will be used to buy shares into the active pool and
escape inactivation when current lockup ends.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, shareholder: <b>address</b>, coins_amount: u64, lockup_cycle: <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(
    pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,
    shareholder: <b>address</b>,
    coins_amount: u64,
    lockup_cycle: <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a>,
): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    <b>let</b> shares_to_redeem = <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&pool.inactive_shares, lockup_cycle),
        shareholder,
        coins_amount);
    // silently exit <b>if</b> not a shareholder otherwise redeem would fail <b>with</b> `ESHAREHOLDER_NOT_FOUND`
    <b>if</b> (shares_to_redeem == 0) <b>return</b> 0;

    // Always <b>update</b> governance records before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> change <b>to</b> the shares pool.
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);
    // Only redeem shares from the pending_inactive pool at `lockup_cycle` == current OLC.
    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address) && lockup_cycle.index == pool.observed_lockup_cycle.index) {
        <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares">update_governanace_records_for_redeem_pending_inactive_shares</a>(
            pool,
            pool_address,
            shares_to_redeem,
            shareholder
        );
    };

    <b>let</b> inactive_shares = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> pool.inactive_shares, lockup_cycle);
    // 1. reaching here means delegator owns inactive/pending_inactive shares at OLC `lockup_cycle`
    <b>let</b> redeemed_coins = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_redeem_shares">pool_u64::redeem_shares</a>(inactive_shares, shareholder, shares_to_redeem);

    // <b>if</b> entirely reactivated pending_inactive <a href="stake.md#0x1_stake">stake</a> or withdrawn inactive one,
    // re-enable unlocking for delegator by deleting this pending withdrawal
    <b>if</b> (<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(inactive_shares, shareholder) == 0) {
        // 2. a delegator owns inactive/pending_inactive shares only at the OLC of its pending withdrawal
        // 1 & 2: the pending withdrawal itself <b>has</b> been emptied of shares and can be safely deleted
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&<b>mut</b> pool.pending_withdrawals, shareholder);
    };
    // destroy inactive shares pool of past OLC <b>if</b> all its <a href="stake.md#0x1_stake">stake</a> <b>has</b> been withdrawn
    <b>if</b> (lockup_cycle.index &lt; pool.observed_lockup_cycle.index && total_coins(inactive_shares) == 0) {
        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_destroy_empty">pool_u64::destroy_empty</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&<b>mut</b> pool.inactive_shares, lockup_cycle));
    };

    redeemed_coins
}
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_stake_pool_drift"></a>

## Function `calculate_stake_pool_drift`

Calculate stake deviations between the delegation and stake pools in order to
capture the rewards earned in the meantime, resulted operator commission and
whether the lockup expired on the stake pool.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): (bool, u64, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): (bool, u64, u64, u64, u64) {
    <b>let</b> (active, inactive, pending_active, pending_inactive) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(<a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool));
    <b>assert</b>!(
        inactive &gt;= pool.total_coins_inactive,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC">ESLASHED_INACTIVE_STAKE_ON_PAST_OLC</a>)
    );
    // determine whether a new lockup cycle <b>has</b> been ended on the <a href="stake.md#0x1_stake">stake</a> pool and
    // inactivated SOME `pending_inactive` <a href="stake.md#0x1_stake">stake</a> which should stop earning rewards now,
    // thus requiring separation of the `pending_inactive` <a href="stake.md#0x1_stake">stake</a> on current observed lockup
    // and the future one on the newly started lockup
    <b>let</b> lockup_cycle_ended = inactive &gt; pool.total_coins_inactive;

    // actual coins on <a href="stake.md#0x1_stake">stake</a> pool belonging <b>to</b> the active shares pool
    active = active + pending_active;
    // actual coins on <a href="stake.md#0x1_stake">stake</a> pool belonging <b>to</b> the shares pool hosting `pending_inactive` <a href="stake.md#0x1_stake">stake</a>
    // at current observed lockup cycle, either pending: `pending_inactive` or already inactivated:
    <b>if</b> (lockup_cycle_ended) {
        // `inactive` on <a href="stake.md#0x1_stake">stake</a> pool = <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> previous `inactive` <a href="stake.md#0x1_stake">stake</a> +
        // <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> previous `pending_inactive` <a href="stake.md#0x1_stake">stake</a> and its rewards (both inactivated)
        pending_inactive = inactive - pool.total_coins_inactive
    };

    // on <a href="stake.md#0x1_stake">stake</a>-management operations, total coins on the <b>internal</b> shares pools and individual
    // stakes on the <a href="stake.md#0x1_stake">stake</a> pool are updated simultaneously, thus the only stakes becoming
    // unsynced are rewards and slashes routed exclusively <b>to</b>/out the <a href="stake.md#0x1_stake">stake</a> pool

    // operator `active` rewards not persisted yet <b>to</b> the active shares pool
    <b>let</b> pool_active = total_coins(&pool.active_shares);
    <b>let</b> commission_active = <b>if</b> (active &gt; pool_active) {
        <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(active - pool_active, pool.operator_commission_percentage, <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>)
    } <b>else</b> {
        // handle <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> slashing applied <b>to</b> `active` <a href="stake.md#0x1_stake">stake</a>
        0
    };
    // operator `pending_inactive` rewards not persisted yet <b>to</b> the pending_inactive shares pool
    <b>let</b> pool_pending_inactive = total_coins(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool));
    <b>let</b> commission_pending_inactive = <b>if</b> (pending_inactive &gt; pool_pending_inactive) {
        <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(
            pending_inactive - pool_pending_inactive,
            pool.operator_commission_percentage,
            <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>
        )
    } <b>else</b> {
        // handle <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> slashing applied <b>to</b> `pending_inactive` <a href="stake.md#0x1_stake">stake</a>
        0
    };

    (lockup_cycle_ended, active, pending_inactive, commission_active, commission_pending_inactive)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_synchronize_delegation_pool"></a>

## Function `synchronize_delegation_pool`

Synchronize delegation and stake pools: distribute yet-undetected rewards to the corresponding internal
shares pools, assign commission to operator and eventually prepare delegation pool for a new lockup cycle.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(
    pool_address: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>let</b> pool = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> (
        lockup_cycle_ended,
        active,
        pending_inactive,
        commission_active,
        commission_pending_inactive
    ) = <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool);

    // zero `pending_active` <a href="stake.md#0x1_stake">stake</a> indicates that either there are no `add_stake` fees or
    // previous epoch <b>has</b> ended and should release the shares owning the existing fees
    <b>let</b> (_, _, pending_active, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
    <b>if</b> (pending_active == 0) {
        // renounce ownership over the `add_stake` fees by redeeming all shares of
        // the special shareholder, implicitly their equivalent coins, out of the active shares pool
        <a href="delegation_pool.md#0x1_delegation_pool_redeem_active_shares">redeem_active_shares</a>(pool, <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>, <a href="delegation_pool.md#0x1_delegation_pool_MAX_U64">MAX_U64</a>);
    };

    // distribute rewards remaining after commission, <b>to</b> delegators (<b>to</b> already existing shares)
    // before buying shares for the operator for its entire commission fee
    // otherwise, operator's new shares would additionally appreciate from rewards it does not own

    // <b>update</b> total coins accumulated by `active` + `pending_active` shares
    // redeemed `add_stake` fees are restored and distributed <b>to</b> the rest of the pool <b>as</b> rewards
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_update_total_coins">pool_u64::update_total_coins</a>(&<b>mut</b> pool.active_shares, active - commission_active);
    // <b>update</b> total coins accumulated by `pending_inactive` shares at current observed lockup cycle
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_update_total_coins">pool_u64::update_total_coins</a>(
        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool),
        pending_inactive - commission_pending_inactive
    );

    // reward operator its commission out of uncommitted active rewards (`add_stake` fees already excluded)
    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool, <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(<a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address)), commission_active);
    // reward operator its commission out of uncommitted pending_inactive rewards
    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(
        pool,
        <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(<a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address)),
        commission_pending_inactive
    );

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> pool.distribute_commission_events,
        <a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">DistributeCommissionEvent</a> {
            pool_address,
            operator: <a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address),
            commission_active,
            commission_pending_inactive,
        },
    );

    emit(<a href="delegation_pool.md#0x1_delegation_pool_DistributeCommission">DistributeCommission</a> {
        pool_address,
        operator: <a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address),
        beneficiary: <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(<a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address)),
        commission_active,
        commission_pending_inactive,
    });

    // advance lockup cycle on delegation pool <b>if</b> already ended on <a href="stake.md#0x1_stake">stake</a> pool (AND <a href="stake.md#0x1_stake">stake</a> explicitly inactivated)
    <b>if</b> (lockup_cycle_ended) {
        // capture inactive coins over all ended lockup cycles (including this ending one)
        <b>let</b> (_, inactive, _, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
        pool.total_coins_inactive = inactive;

        // advance lockup cycle on the delegation pool
        pool.observed_lockup_cycle.index = pool.observed_lockup_cycle.index + 1;
        // start new lockup cycle <b>with</b> a fresh shares pool for `pending_inactive` <a href="stake.md#0x1_stake">stake</a>
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(
            &<b>mut</b> pool.inactive_shares,
            pool.observed_lockup_cycle,
            <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create_with_scaling_factor">pool_u64::create_with_scaling_factor</a>(<a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>)
        );
    };

    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_is_next_commission_percentage_effective">is_next_commission_percentage_effective</a>(pool_address)) {
        pool.operator_commission_percentage = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(
            pool_address
        ).commission_percentage_next_lockup_cycle;
    }
}
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_and_update_proposal_used_voting_power"></a>

## Function `assert_and_update_proposal_used_voting_power`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_and_update_proposal_used_voting_power">assert_and_update_proposal_used_voting_power</a>(governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, pool_address: <b>address</b>, proposal_id: u64, voting_power: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_and_update_proposal_used_voting_power">assert_and_update_proposal_used_voting_power</a>(
    governance_records: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, pool_address: <b>address</b>, proposal_id: u64, voting_power: u64
) {
    <b>let</b> stake_pool_remaining_voting_power = <a href="aptos_governance.md#0x1_aptos_governance_get_remaining_voting_power">aptos_governance::get_remaining_voting_power</a>(pool_address, proposal_id);
    <b>let</b> stake_pool_used_voting_power = <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">aptos_governance::get_voting_power</a>(
        pool_address
    ) - stake_pool_remaining_voting_power;
    <b>let</b> proposal_used_voting_power = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(
        &<b>mut</b> governance_records.votes_per_proposal,
        proposal_id,
        0
    );
    // A edge case: Before enabling partial governance <a href="voting.md#0x1_voting">voting</a> on a delegation pool, the delegation pool <b>has</b>
    // a voter which can vote <b>with</b> all <a href="voting.md#0x1_voting">voting</a> power of this delegation pool. If the voter votes on a proposal after
    // partial governance <a href="voting.md#0x1_voting">voting</a> flag is enabled, the delegation pool doesn't have enough <a href="voting.md#0x1_voting">voting</a> power on this
    // proposal for all the delegators. To be fair, no one can vote on this proposal through this delegation pool.
    // To detect this case, check <b>if</b> the <a href="stake.md#0x1_stake">stake</a> pool had used <a href="voting.md#0x1_voting">voting</a> power not through <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a> <b>module</b>.
    <b>assert</b>!(
        stake_pool_used_voting_power == *proposal_used_voting_power,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING">EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING</a>)
    );
    *proposal_used_voting_power = *proposal_used_voting_power + voting_power;
}
</code></pre>



</details>

<a id="0x1_delegation_pool_update_governance_records_for_buy_in_active_shares"></a>

## Function `update_governance_records_for_buy_in_active_shares`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_active_shares">update_governance_records_for_buy_in_active_shares</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, pool_address: <b>address</b>, new_shares: u128, shareholder: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_active_shares">update_governance_records_for_buy_in_active_shares</a>(
    pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, pool_address: <b>address</b>, new_shares: u128, shareholder: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    // &lt;active shares&gt; of &lt;shareholder&gt; += &lt;new_shares&gt; ----&gt;
    // &lt;active shares&gt; of &lt;current voter of shareholder&gt; += &lt;new_shares&gt;
    // &lt;active shares&gt; of &lt;next voter of shareholder&gt; += &lt;new_shares&gt;
    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    <b>let</b> vote_delegation = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(pool, governance_records, shareholder);
    <b>let</b> current_voter = vote_delegation.voter;
    <b>let</b> pending_voter = vote_delegation.pending_voter;
    <b>let</b> current_delegated_votes =
        <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, current_voter);
    current_delegated_votes.active_shares = current_delegated_votes.active_shares + new_shares;
    <b>if</b> (pending_voter == current_voter) {
        current_delegated_votes.active_shares_next_lockup =
            current_delegated_votes.active_shares_next_lockup + new_shares;
    } <b>else</b> {
        <b>let</b> pending_delegated_votes =
            <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, pending_voter);
        pending_delegated_votes.active_shares_next_lockup =
            pending_delegated_votes.active_shares_next_lockup + new_shares;
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares"></a>

## Function `update_governance_records_for_buy_in_pending_inactive_shares`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares">update_governance_records_for_buy_in_pending_inactive_shares</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, pool_address: <b>address</b>, new_shares: u128, shareholder: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares">update_governance_records_for_buy_in_pending_inactive_shares</a>(
    pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, pool_address: <b>address</b>, new_shares: u128, shareholder: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    // &lt;pending inactive shares&gt; of &lt;shareholder&gt; += &lt;new_shares&gt;   ----&gt;
    // &lt;pending inactive shares&gt; of &lt;current voter of shareholder&gt; += &lt;new_shares&gt;
    // no impact on &lt;pending inactive shares&gt; of &lt;next voter of shareholder&gt;
    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    <b>let</b> current_voter = <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(pool, governance_records, shareholder);
    <b>let</b> current_delegated_votes = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, current_voter);
    current_delegated_votes.pending_inactive_shares = current_delegated_votes.pending_inactive_shares + new_shares;
}
</code></pre>



</details>

<a id="0x1_delegation_pool_update_governanace_records_for_redeem_active_shares"></a>

## Function `update_governanace_records_for_redeem_active_shares`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_active_shares">update_governanace_records_for_redeem_active_shares</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, pool_address: <b>address</b>, shares_to_redeem: u128, shareholder: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_active_shares">update_governanace_records_for_redeem_active_shares</a>(
    pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, pool_address: <b>address</b>, shares_to_redeem: u128, shareholder: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    // &lt;active shares&gt; of &lt;shareholder&gt; -= &lt;shares_to_redeem&gt; ----&gt;
    // &lt;active shares&gt; of &lt;current voter of shareholder&gt; -= &lt;shares_to_redeem&gt;
    // &lt;active shares&gt; of &lt;next voter of shareholder&gt; -= &lt;shares_to_redeem&gt;
    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    <b>let</b> vote_delegation = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(
        pool,
        governance_records,
        shareholder
    );
    <b>let</b> current_voter = vote_delegation.voter;
    <b>let</b> pending_voter = vote_delegation.pending_voter;
    <b>let</b> current_delegated_votes = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, current_voter);
    current_delegated_votes.active_shares = current_delegated_votes.active_shares - shares_to_redeem;
    <b>if</b> (current_voter == pending_voter) {
        current_delegated_votes.active_shares_next_lockup =
            current_delegated_votes.active_shares_next_lockup - shares_to_redeem;
    } <b>else</b> {
        <b>let</b> pending_delegated_votes =
            <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, pending_voter);
        pending_delegated_votes.active_shares_next_lockup =
            pending_delegated_votes.active_shares_next_lockup - shares_to_redeem;
    };
}
</code></pre>



</details>

<a id="0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares"></a>

## Function `update_governanace_records_for_redeem_pending_inactive_shares`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares">update_governanace_records_for_redeem_pending_inactive_shares</a>(pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, pool_address: <b>address</b>, shares_to_redeem: u128, shareholder: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares">update_governanace_records_for_redeem_pending_inactive_shares</a>(
    pool: &<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, pool_address: <b>address</b>, shares_to_redeem: u128, shareholder: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> {
    // &lt;pending inactive shares&gt; of &lt;shareholder&gt; -= &lt;shares_to_redeem&gt;  ----&gt;
    // &lt;pending inactive shares&gt; of &lt;current voter of shareholder&gt; -= &lt;shares_to_redeem&gt;
    // no impact on &lt;pending inactive shares&gt; of &lt;next voter of shareholder&gt;
    <b>let</b> governance_records = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);
    <b>let</b> current_voter = <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(pool, governance_records, shareholder);
    <b>let</b> current_delegated_votes = <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, current_voter);
    current_delegated_votes.pending_inactive_shares = current_delegated_votes.pending_inactive_shares - shares_to_redeem;
}
</code></pre>



</details>

<a id="0x1_delegation_pool_multiply_then_divide"></a>

## Function `multiply_then_divide`

Deprecated, prefer math64::mul_div


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_multiply_then_divide">multiply_then_divide</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_multiply_then_divide">multiply_then_divide</a>(x: u64, y: u64, z: u64): u64 {
    <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(x, y, z)
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
<td>Every DelegationPool has only one corresponding StakePool stored at the same address.</td>
<td>Critical</td>
<td>Upon calling the initialize_delegation_pool function, a resource account is created from the "owner" signer to host the delegation pool resource and own the underlying stake pool.</td>
<td>Audited that the address of StakePool equals address of DelegationPool and the data invariant on the DelegationPool.</td>
</tr>

<tr>
<td>2</td>
<td>The signer capability within the delegation pool has an address equal to the address of the delegation pool.</td>
<td>Critical</td>
<td>The initialize_delegation_pool function moves the DelegationPool resource to the address associated with stake_pool_signer, which also possesses the signer capability.</td>
<td>Audited that the address of signer cap equals address of DelegationPool.</td>
</tr>

<tr>
<td>3</td>
<td>A delegator holds shares exclusively in one inactive shares pool, which could either be an already inactive pool or the pending_inactive pool.</td>
<td>High</td>
<td>The get_stake function returns the inactive stake owned by a delegator and checks which state the shares are in via the get_pending_withdrawal function.</td>
<td>Audited that either inactive or pending_inactive stake after invoking the get_stake function is zero and both are never non-zero.</td>
</tr>

<tr>
<td>4</td>
<td>The specific pool in which the delegator possesses inactive shares becomes designated as the pending withdrawal pool for that delegator.</td>
<td>Medium</td>
<td>The get_pending_withdrawal function checks if any pending withdrawal exists for a delegate address and if there is neither inactive nor pending_inactive stake, the pending_withdrawal_exists returns false.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>5</td>
<td>The existence of a pending withdrawal implies that it is associated with a pool where the delegator possesses inactive shares.</td>
<td>Medium</td>
<td>In the get_pending_withdrawal function, if withdrawal_exists is true, the function returns true and a non-zero amount</td>
<td>get_pending_withdrawal has been audited.</td>
</tr>

<tr>
<td>6</td>
<td>An inactive shares pool should have coins allocated to it; otherwise, it should become deleted.</td>
<td>Medium</td>
<td>The redeem_inactive_shares function has a check that destroys the inactive shares pool, given that it is empty.</td>
<td>shares pools have been audited.</td>
</tr>

<tr>
<td>7</td>
<td>The index of the pending withdrawal will not exceed the current OLC on DelegationPool.</td>
<td>High</td>
<td>The get_pending_withdrawal function has a check which ensures that withdrawal_olc.index < pool.observed_lockup_cycle.index.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>8</td>
<td>Slashing is not possible for inactive stakes.</td>
<td>Critical</td>
<td>The number of inactive staked coins must be greater than or equal to the total_coins_inactive of the pool.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>9</td>
<td>The delegator's active or pending inactive stake will always meet or exceed the minimum allowed value.</td>
<td>Medium</td>
<td>The add_stake, unlock and reactivate_stake functions ensure the active_shares or pending_inactive_shares balance for the delegator is greater than or equal to the MIN_COINS_ON_SHARES_POOL value.</td>
<td>Audited the comparison of active_shares or inactive_shares balance for the delegator with the MIN_COINS_ON_SHARES_POOL value.</td>
</tr>

<tr>
<td>10</td>
<td>The delegation pool exists at a given address.</td>
<td>Low</td>
<td>Functions that operate on the DelegationPool abort if there is no DelegationPool struct under the given pool_address.</td>
<td>Audited that there is no DelegationPool structure assigned to the pool_address given as a parameter.</td>
</tr>

<tr>
<td>11</td>
<td>The initialization of the delegation pool is contingent upon enabling the delegation pools feature.</td>
<td>Critical</td>
<td>The initialize_delegation_pool function should proceed if the DELEGATION_POOLS feature is enabled.</td>
<td>This has been audited.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
