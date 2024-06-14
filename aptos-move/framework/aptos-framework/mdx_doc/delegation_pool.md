
<a id="0x1_delegation_pool"></a>

# Module `0x1::delegation_pool`


Allow multiple delegators to participate in the same stake pool in order to collect the minimum
stake required to join the validator set. Delegators are rewarded out of the validator rewards
proportionally to their stake and provided the same stake&#45;management API as the stake pool owner.

The main accounting logic in the delegation pool contract handles the following:
1. Tracks how much stake each delegator owns, privately deposited as well as earned.
Accounting individual delegator stakes is achieved through the shares&#45;based pool defined at
<code>aptos_std::pool_u64</code>, hence delegators own shares rather than absolute stakes into the delegation pool.
2. Tracks rewards earned by the stake pool, implicitly by the delegation one, in the meantime
and distribute them accordingly.
3. Tracks lockup cycles on the stake pool in order to separate inactive stake (not earning rewards)
from pending_inactive stake (earning rewards) and allow its delegators to withdraw the former.
4. Tracks how much commission fee has to be paid to the operator out of incoming rewards before
distributing them to the internal pool_u64 pools.

In order to distinguish between stakes in different states and route rewards accordingly,
separate pool_u64 pools are used for individual stake states:
1. one of <code>active</code> &#43; <code>pending_active</code> stake
2. one of <code>inactive</code> stake FOR each past observed lockup cycle (OLC) on the stake pool
3. one of <code>pending_inactive</code> stake scheduled during this ongoing OLC

As stake&#45;state transitions and rewards are computed only at the stake pool level, the delegation pool
gets outdated. To mitigate this, at any interaction with the delegation pool, a process of synchronization
to the underlying stake pool is executed before the requested operation itself.

At synchronization:
&#45; stake deviations between the two pools are actually the rewards produced in the meantime.
&#45; the commission fee is extracted from the rewards, the remaining stake is distributed to the internal
pool_u64 pools and then the commission stake used to buy shares for operator.
&#45; if detecting that the lockup expired on the stake pool, the delegation pool will isolate its
pending_inactive stake (now inactive) and create a new pool_u64 to host future pending_inactive stake
scheduled this newly started lockup.
Detecting a lockup expiration on the stake pool resumes to detecting new inactive stake.

Accounting main invariants:
&#45; each stake&#45;management operation (add/unlock/reactivate/withdraw) and operator change triggers
the synchronization process before executing its own function.
&#45; each OLC maps to one or more real lockups on the stake pool, but not the opposite. Actually, only a real
lockup with &apos;activity&apos; (which inactivated some unlocking stake) triggers the creation of a new OLC.
&#45; unlocking and/or unlocked stake originating from different real lockups are never mixed together into
the same pool_u64. This invalidates the accounting of which rewards belong to whom.
&#45; no delegator can have unlocking and/or unlocked stake (pending withdrawals) in different OLCs. This ensures
delegators do not have to keep track of the OLCs when they unlocked. When creating a new pending withdrawal,
the existing one is executed (withdrawn) if is already inactive.
&#45; <code>add_stake</code> fees are always refunded, but only after the epoch when they have been charged ends.
&#45; withdrawing pending_inactive stake (when validator had gone inactive before its lockup expired)
does not inactivate any stake additional to the requested one to ensure OLC would not advance indefinitely.
&#45; the pending withdrawal exists at an OLC iff delegator owns some shares within the shares pool of that OLC.

Example flow:
<ol>
<li>A node operator creates a delegation pool by calling
<code>initialize_delegation_pool</code> and sets
its commission fee to 0% (for simplicity). A stake pool is created with no initial stake and owned by
a resource account controlled by the delegation pool.</li>
<li>Delegator A adds 100 stake which is converted to 100 shares into the active pool_u64</li>
<li>Operator joins the validator set as the stake pool has now the minimum stake</li>
<li>The stake pool earned rewards and now has 200 active stake. A&apos;s active shares are worth 200 coins as
the commission fee is 0%.</li>
<li></li>
<ol>
<li>A requests <code>unlock</code> for 100 stake</li>
<li>Synchronization detects 200 &#45; 100 active rewards which are entirely (0% commission) added to the active pool.</li>
<li>100 coins &#61; (100 &#42; 100) / 200 &#61; 50 shares are redeemed from the active pool and exchanged for 100 shares
into the pending_inactive one on A&apos;s behalf</li>
</ol>
<li>Delegator B adds 200 stake which is converted to (200 &#42; 50) / 100 &#61; 100 shares into the active pool</li>
<li>The stake pool earned rewards and now has 600 active and 200 pending_inactive stake.</li>
<li></li>
<ol>
<li>A requests <code>reactivate_stake</code> for 100 stake</li>
<li>
Synchronization detects 600 &#45; 300 active and 200 &#45; 100 pending_inactive rewards which are both entirely
distributed to their corresponding pools
</li>
<li>
100 coins &#61; (100 &#42; 100) / 200 &#61; 50 shares are redeemed from the pending_inactive pool and exchanged for
(100 &#42; 150) / 600 &#61; 25 shares into the active one on A&apos;s behalf
</li>
</ol>
<li>The lockup expires on the stake pool, inactivating the entire pending_inactive stake</li>
<li></li>
<ol>
<li>B requests <code>unlock</code> for 100 stake</li>
<li>
Synchronization detects no active or pending_inactive rewards, but 0 &#45;&gt; 100 inactive stake on the stake pool,
so it advances the observed lockup cycle and creates a pool_u64 for the new lockup, hence allowing previous
pending_inactive shares to be redeemed</li>
<li>
100 coins &#61; (100 &#42; 175) / 700 &#61; 25 shares are redeemed from the active pool and exchanged for 100 shares
into the new pending_inactive one on B&apos;s behalf
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
A&apos;s 50 shares &#61; (50 &#42; 100) / 50 &#61; 100 coins are redeemed from the (1st) inactive pool and 100 stake is
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="aptos_governance.md#0x1_aptos_governance">0x1::aptos_governance</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound">0x1::pool_u64_unbound</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_delegation_pool_DelegationPoolOwnership"></a>

## Resource `DelegationPoolOwnership`

Capability that represents ownership over privileged operations on the underlying stake pool.


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a> <b>has</b> store, key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> <b>has</b> key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_VotingRecordKey">VotingRecordKey</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">VoteDelegation</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> <b>has</b> key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> <b>has</b> key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> <b>has</b> key<br /></code></pre>



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

Tracks a delegation pool&apos;s allowlist of delegators.
If allowlisting is enabled, existing delegators are not implicitly allowlisted and they can be individually
evicted later by the pool owner.


<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> <b>has</b> key<br /></code></pre>



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

<a id="0x1_delegation_pool_AddStake"></a>

## Struct `AddStake`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_AddStake">AddStake</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_AddStakeEvent">AddStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStake">ReactivateStake</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStakeEvent">ReactivateStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_UnlockStake">UnlockStake</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_UnlockStakeEvent">UnlockStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStake">WithdrawStake</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStakeEvent">WithdrawStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">DistributeCommissionEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DistributeCommission">DistributeCommission</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_Vote">Vote</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_VoteEvent">VoteEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_CreateProposal">CreateProposal</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_CreateProposalEvent">CreateProposalEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPower">DelegateVotingPower</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPowerEvent">DelegateVotingPowerEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_SetBeneficiaryForOperator">SetBeneficiaryForOperator</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_CommissionPercentageChange">CommissionPercentageChange</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_EnableDelegatorsAllowlisting">EnableDelegatorsAllowlisting</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DisableDelegatorsAllowlisting">DisableDelegatorsAllowlisting</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_AllowlistDelegator">AllowlistDelegator</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_RemoveDelegatorFromAllowlist">RemoveDelegatorFromAllowlist</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_EvictDelegator">EvictDelegator</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MAX_U64">MAX_U64</a>: u64 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_delegation_pool_EDEPRECATED_FUNCTION"></a>

Function is deprecated.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x1_delegation_pool_EDISABLED_FUNCTION"></a>

The function is disabled or hasn&apos;t been enabled.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDISABLED_FUNCTION">EDISABLED_FUNCTION</a>: u64 &#61; 13;<br /></code></pre>



<a id="0x1_delegation_pool_ENOT_OPERATOR"></a>

The account is not the operator of the stake pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ENOT_OPERATOR">ENOT_OPERATOR</a>: u64 &#61; 18;<br /></code></pre>



<a id="0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS"></a>

Account is already owning a delegation pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_delegation_pool_EOWNER_CAP_NOT_FOUND"></a>

Delegation pool owner capability does not exist at the provided account.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_delegation_pool_EINSUFFICIENT_PROPOSER_STAKE"></a>

The voter does not have sufficient stake to create a proposal.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EINSUFFICIENT_PROPOSER_STAKE">EINSUFFICIENT_PROPOSER_STAKE</a>: u64 &#61; 15;<br /></code></pre>



<a id="0x1_delegation_pool_ENO_VOTING_POWER"></a>

The voter does not have any voting power on this proposal.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ENO_VOTING_POWER">ENO_VOTING_POWER</a>: u64 &#61; 16;<br /></code></pre>



<a id="0x1_delegation_pool_EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING"></a>

The stake pool has already voted on the proposal before enabling partial governance voting on this delegation pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING">EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING</a>: u64 &#61; 17;<br /></code></pre>



<a id="0x1_delegation_pool_ECANNOT_EVICT_ALLOWLISTED_DELEGATOR"></a>

Cannot evict an allowlisted delegator, should remove them from the allowlist first.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ECANNOT_EVICT_ALLOWLISTED_DELEGATOR">ECANNOT_EVICT_ALLOWLISTED_DELEGATOR</a>: u64 &#61; 26;<br /></code></pre>



<a id="0x1_delegation_pool_ECANNOT_UNLOCK_NULL_SHAREHOLDER"></a>

Cannot unlock the accumulated active stake of NULL_SHAREHOLDER(0x0).


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ECANNOT_UNLOCK_NULL_SHAREHOLDER">ECANNOT_UNLOCK_NULL_SHAREHOLDER</a>: u64 &#61; 27;<br /></code></pre>



<a id="0x1_delegation_pool_ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED"></a>

Changing operator commission rate in delegation pool is not supported.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED">ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED</a>: u64 &#61; 22;<br /></code></pre>



<a id="0x1_delegation_pool_EDELEGATION_POOLS_DISABLED"></a>

Creating delegation pools is not enabled yet.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOLS_DISABLED">EDELEGATION_POOLS_DISABLED</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST"></a>

Delegation pool does not exist at the provided pool address.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST">EDELEGATION_POOL_DOES_NOT_EXIST</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_ENABLED"></a>

Delegators allowlisting should be enabled to perform this operation.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_ENABLED">EDELEGATORS_ALLOWLISTING_NOT_ENABLED</a>: u64 &#61; 24;<br /></code></pre>



<a id="0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED"></a>

Delegators allowlisting is not supported.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED">EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED</a>: u64 &#61; 23;<br /></code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW"></a>

Delegator&apos;s active balance cannot be less than <code><a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a></code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW">EDELEGATOR_ACTIVE_BALANCE_TOO_LOW</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_NOT_ALLOWLISTED"></a>

Cannot add/reactivate stake unless being allowlisted by the pool owner.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_NOT_ALLOWLISTED">EDELEGATOR_NOT_ALLOWLISTED</a>: u64 &#61; 25;<br /></code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW"></a>

Delegator&apos;s pending_inactive balance cannot be less than <code><a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a></code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW">EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE"></a>

Commission percentage has to be between 0 and <code><a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a></code> &#45; 100%.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK"></a>

There is not enough <code>active</code> stake on the stake pool to <code>unlock</code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK">ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_delegation_pool_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED"></a>

Changing beneficiaries for operators is not supported.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED">EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED</a>: u64 &#61; 19;<br /></code></pre>



<a id="0x1_delegation_pool_EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED"></a>

Partial governance voting hasn&apos;t been enabled on this delegation pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED">EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED</a>: u64 &#61; 14;<br /></code></pre>



<a id="0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS"></a>

There is a pending withdrawal to be executed before <code>unlock</code>ing any new stake.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS">EPENDING_WITHDRAWAL_EXISTS</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC"></a>

Slashing (if implemented) should not be applied to already <code>inactive</code> stake.
Not only it invalidates the accounting of past observed lockup cycles (OLC),
but is also unfair to delegators whose stake has been inactive before validator started misbehaving.
Additionally, the inactive stake does not count on the voting power of validator.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC">ESLASHED_INACTIVE_STAKE_ON_PAST_OLC</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_delegation_pool_ETOO_LARGE_COMMISSION_INCREASE"></a>

Commission percentage increase is too large.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ETOO_LARGE_COMMISSION_INCREASE">ETOO_LARGE_COMMISSION_INCREASE</a>: u64 &#61; 20;<br /></code></pre>



<a id="0x1_delegation_pool_ETOO_LATE_COMMISSION_CHANGE"></a>

Commission percentage change is too late in this lockup period, and should be done at least a quarter (1/4) of the lockup duration before the lockup cycle ends.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ETOO_LATE_COMMISSION_CHANGE">ETOO_LATE_COMMISSION_CHANGE</a>: u64 &#61; 21;<br /></code></pre>



<a id="0x1_delegation_pool_EWITHDRAW_ZERO_STAKE"></a>

Cannot request to withdraw zero stake.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EWITHDRAW_ZERO_STAKE">EWITHDRAW_ZERO_STAKE</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x1_delegation_pool_MAX_COMMISSION_INCREASE"></a>

Maximum commission percentage increase per lockup cycle. 10% is represented as 1000.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MAX_COMMISSION_INCREASE">MAX_COMMISSION_INCREASE</a>: u64 &#61; 1000;<br /></code></pre>



<a id="0x1_delegation_pool_MAX_FEE"></a>

Maximum operator percentage fee(of double digit precision): 22.85% is represented as 2285


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>: u64 &#61; 10000;<br /></code></pre>



<a id="0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL"></a>

Minimum coins to exist on a shares pool at all times.
Enforced per delegator for both active and pending_inactive pools.
This constraint ensures the share price cannot overly increase and lead to
substantial losses when buying shares (can lose at most 1 share which may
be worth a lot if current share price is high).
This constraint is not enforced on inactive pools as they only allow redeems
(can lose at most 1 coin regardless of current share price).


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>: u64 &#61; 1000000000;<br /></code></pre>



<a id="0x1_delegation_pool_MODULE_SALT"></a>



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MODULE_SALT">MODULE_SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 100, 101, 108, 101, 103, 97, 116, 105, 111, 110, 95, 112, 111, 111, 108];<br /></code></pre>



<a id="0x1_delegation_pool_NULL_SHAREHOLDER"></a>

Special shareholder temporarily owning the <code>add_stake</code> fees charged during this epoch.
On each <code>add_stake</code> operation any resulted fee is used to buy active shares for this shareholder.
First synchronization after this epoch ends will distribute accumulated fees to the rest of the pool as refunds.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>: <b>address</b> &#61; 0x0;<br /></code></pre>



<a id="0x1_delegation_pool_SHARES_SCALING_FACTOR"></a>

Scaling factor of shares pools used within the delegation pool


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>: u64 &#61; 10000000000000000;<br /></code></pre>



<a id="0x1_delegation_pool_owner_cap_exists"></a>

## Function `owner_cap_exists`

Return whether supplied address <code>addr</code> is owner of a delegation pool.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(addr: <b>address</b>): bool &#123;<br />    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>&gt;(addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_owned_pool_address"></a>

## Function `get_owned_pool_address`

Return address of the delegation pool owned by <code>owner</code> or fail if there is none.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(owner: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(owner: <b>address</b>): <b>address</b> <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner);<br />    <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>&gt;(owner).pool_address<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_delegation_pool_exists"></a>

## Function `delegation_pool_exists`

Return whether a delegation pool exists at supplied address <code>addr</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegation_pool_exists">delegation_pool_exists</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegation_pool_exists">delegation_pool_exists</a>(addr: <b>address</b>): bool &#123;<br />    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_partial_governance_voting_enabled"></a>

## Function `partial_governance_voting_enabled`

Return whether a delegation pool has already enabled partial governance voting.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address: <b>address</b>): bool &#123;<br />    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address) &amp;&amp; <a href="stake.md#0x1_stake_get_delegated_voter">stake::get_delegated_voter</a>(pool_address) &#61;&#61; pool_address<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_observed_lockup_cycle"></a>

## Function `observed_lockup_cycle`

Return the index of current observed lockup cycle on delegation pool <code>pool_address</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_observed_lockup_cycle">observed_lockup_cycle</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_observed_lockup_cycle">observed_lockup_cycle</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).observed_lockup_cycle.index<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_is_next_commission_percentage_effective"></a>

## Function `is_next_commission_percentage_effective`

Return whether the commission percentage for the next lockup cycle is effective.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_is_next_commission_percentage_effective">is_next_commission_percentage_effective</a>(pool_address: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_is_next_commission_percentage_effective">is_next_commission_percentage_effective</a>(pool_address: <b>address</b>): bool <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address) &amp;&amp;<br />        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt;&#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address).effective_after_secs<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the operator commission percentage set on the delegation pool <code>pool_address</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(<br />    pool_address: <b>address</b><br />): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_is_next_commission_percentage_effective">is_next_commission_percentage_effective</a>(pool_address)) &#123;<br />        <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle">operator_commission_percentage_next_lockup_cycle</a>(pool_address)<br />    &#125; <b>else</b> &#123;<br />        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).operator_commission_percentage<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle"></a>

## Function `operator_commission_percentage_next_lockup_cycle`

Return the operator commission percentage for the next lockup cycle.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle">operator_commission_percentage_next_lockup_cycle</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle">operator_commission_percentage_next_lockup_cycle</a>(<br />    pool_address: <b>address</b><br />): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <b>if</b> (<b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address)) &#123;<br />        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address).commission_percentage_next_lockup_cycle<br />    &#125; <b>else</b> &#123;<br />        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).operator_commission_percentage<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_shareholders_count_active_pool"></a>

## Function `shareholders_count_active_pool`

Return the number of delegators owning active stake within <code>pool_address</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_shareholders_count_active_pool">shareholders_count_active_pool</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_shareholders_count_active_pool">shareholders_count_active_pool</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders_count">pool_u64::shareholders_count</a>(&amp;<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).active_shares)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_delegation_pool_stake"></a>

## Function `get_delegation_pool_stake`

Return the stake amounts on <code>pool_address</code> in the different states:
(<code>active</code>,<code>inactive</code>,<code>pending_active</code>,<code>pending_inactive</code>)


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegation_pool_stake">get_delegation_pool_stake</a>(pool_address: <b>address</b>): (u64, u64, u64, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegation_pool_stake">get_delegation_pool_stake</a>(pool_address: <b>address</b>): (u64, u64, u64, u64) &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_pending_withdrawal"></a>

## Function `get_pending_withdrawal`

Return whether the given delegator has any withdrawable stake. If they recently requested to unlock
some stake and the stake pool&apos;s lockup cycle has not ended, their coins are not withdrawable yet.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_pending_withdrawal">get_pending_withdrawal</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): (bool, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_pending_withdrawal">get_pending_withdrawal</a>(<br />    pool_address: <b>address</b>,<br />    delegator_address: <b>address</b><br />): (bool, u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <b>let</b> pool &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    <b>let</b> (<br />        lockup_cycle_ended,<br />        _,<br />        pending_inactive,<br />        _,<br />        commission_pending_inactive<br />    ) &#61; <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool);<br /><br />    <b>let</b> (withdrawal_exists, withdrawal_olc) &#61; <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool, delegator_address);<br />    <b>if</b> (!withdrawal_exists) &#123;<br />        // <b>if</b> no pending withdrawal, there is neither inactive nor pending_inactive <a href="stake.md#0x1_stake">stake</a>
        (<b>false</b>, 0)<br />    &#125; <b>else</b> &#123;<br />        // delegator <b>has</b> either inactive or pending_inactive <a href="stake.md#0x1_stake">stake</a> due <b>to</b> automatic withdrawals<br />        <b>let</b> inactive_shares &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&amp;pool.inactive_shares, withdrawal_olc);<br />        <b>if</b> (withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) &#123;<br />            // <b>if</b> withdrawal&apos;s lockup cycle ended on delegation pool then it is inactive
            (<b>true</b>, <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(inactive_shares, delegator_address))<br />        &#125; <b>else</b> &#123;<br />            pending_inactive &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">pool_u64::shares_to_amount_with_total_coins</a>(<br />                inactive_shares,<br />                <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(inactive_shares, delegator_address),<br />                // exclude operator pending_inactive rewards not converted <b>to</b> shares yet<br />                pending_inactive &#45; commission_pending_inactive<br />            );<br />            // <b>if</b> withdrawal&apos;s lockup cycle ended ONLY on <a href="stake.md#0x1_stake">stake</a> pool then it is also inactive
            (lockup_cycle_ended, pending_inactive)<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_stake"></a>

## Function `get_stake`

Return total stake owned by <code>delegator_address</code> within delegation pool <code>pool_address</code>
in each of its individual states: (<code>active</code>,<code>inactive</code>,<code>pending_inactive</code>)


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_stake">get_stake</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): (u64, u64, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_stake">get_stake</a>(<br />    pool_address: <b>address</b>,<br />    delegator_address: <b>address</b><br />): (u64, u64, u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <b>let</b> pool &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    <b>let</b> (<br />        lockup_cycle_ended,<br />        active,<br />        _,<br />        commission_active,<br />        commission_pending_inactive<br />    ) &#61; <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool);<br /><br />    <b>let</b> total_active_shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_total_shares">pool_u64::total_shares</a>(&amp;pool.active_shares);<br />    <b>let</b> delegator_active_shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(&amp;pool.active_shares, delegator_address);<br /><br />    <b>let</b> (_, _, pending_active, _) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);<br />    <b>if</b> (pending_active &#61;&#61; 0) &#123;<br />        // zero `pending_active` <a href="stake.md#0x1_stake">stake</a> indicates that either there are no `add_stake` fees or<br />        // previous epoch <b>has</b> ended and should identify shares owning these fees <b>as</b> released<br />        total_active_shares &#61; total_active_shares &#45; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(&amp;pool.active_shares, <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>);<br />        <b>if</b> (delegator_address &#61;&#61; <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>) &#123;<br />            delegator_active_shares &#61; 0<br />        &#125;<br />    &#125;;<br />    active &#61; pool_u64::shares_to_amount_with_total_stats(<br />        &amp;pool.active_shares,<br />        delegator_active_shares,<br />        // exclude operator active rewards not converted <b>to</b> shares yet<br />        active &#45; commission_active,<br />        total_active_shares<br />    );<br /><br />    // get state and <a href="stake.md#0x1_stake">stake</a> (0 <b>if</b> there is none) of the pending withdrawal<br />    <b>let</b> (withdrawal_inactive, withdrawal_stake) &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_pending_withdrawal">get_pending_withdrawal</a>(pool_address, delegator_address);<br />    // report non&#45;active stakes accordingly <b>to</b> the state of the pending withdrawal<br />    <b>let</b> (inactive, pending_inactive) &#61; <b>if</b> (withdrawal_inactive) (withdrawal_stake, 0) <b>else</b> (0, withdrawal_stake);<br /><br />    // should also <b>include</b> commission rewards in case of the operator <a href="account.md#0x1_account">account</a><br />    // operator rewards are actually used <b>to</b> buy shares which is introducing<br />    // some imprecision (received <a href="stake.md#0x1_stake">stake</a> would be slightly less)<br />    // but adding rewards onto the existing <a href="stake.md#0x1_stake">stake</a> is still a good approximation<br />    <b>if</b> (delegator_address &#61;&#61; <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(get_operator(pool_address))) &#123;<br />        active &#61; active &#43; commission_active;<br />        // in&#45;flight pending_inactive commission can coexist <b>with</b> already inactive withdrawal<br />        <b>if</b> (lockup_cycle_ended) &#123;<br />            inactive &#61; inactive &#43; commission_pending_inactive<br />        &#125; <b>else</b> &#123;<br />            pending_inactive &#61; pending_inactive &#43; commission_pending_inactive<br />        &#125;<br />    &#125;;<br /><br />    (active, inactive, pending_inactive)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_add_stake_fee"></a>

## Function `get_add_stake_fee`

Return refundable stake to be extracted from added <code>amount</code> at <code>add_stake</code> operation on pool <code>pool_address</code>.
If the validator produces rewards this epoch, added stake goes directly to <code>pending_active</code> and
does not earn rewards. However, all shares within a pool appreciate uniformly and when this epoch ends:
&#45; either added shares are still <code>pending_active</code> and steal from rewards of existing <code>active</code> stake
&#45; or have moved to <code>pending_inactive</code> and get full rewards (they displaced <code>active</code> stake at <code>unlock</code>)
To mitigate this, some of the added stake is extracted and fed back into the pool as placeholder
for the rewards the remaining stake would have earned if active:
extracted&#45;fee &#61; (amount &#45; extracted&#45;fee) &#42; reward&#45;rate% &#42; (100% &#45; operator&#45;commission%)


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_add_stake_fee">get_add_stake_fee</a>(pool_address: <b>address</b>, amount: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_add_stake_fee">get_add_stake_fee</a>(<br />    pool_address: <b>address</b>,<br />    amount: u64<br />): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <b>if</b> (<a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(pool_address)) &#123;<br />        <b>let</b> (rewards_rate, rewards_rate_denominator) &#61; <a href="staking_config.md#0x1_staking_config_get_reward_rate">staking_config::get_reward_rate</a>(&amp;<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());<br />        <b>if</b> (rewards_rate_denominator &gt; 0) &#123;<br />            <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br /><br />            rewards_rate &#61; rewards_rate &#42; (<a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a> &#45; <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(pool_address));<br />            rewards_rate_denominator &#61; rewards_rate_denominator &#42; <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>;<br />            ((((amount <b>as</b> u128) &#42; (rewards_rate <b>as</b> u128)) / ((rewards_rate <b>as</b> u128) &#43; (rewards_rate_denominator <b>as</b> u128))) <b>as</b> u64)<br />        &#125; <b>else</b> &#123; 0 &#125;<br />    &#125; <b>else</b> &#123; 0 &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_can_withdraw_pending_inactive"></a>

## Function `can_withdraw_pending_inactive`

Return whether <code>pending_inactive</code> stake can be directly withdrawn from
the delegation pool, implicitly its stake pool, in the special case
the validator had gone inactive before its lockup expired.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_can_withdraw_pending_inactive">can_withdraw_pending_inactive</a>(pool_address: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_can_withdraw_pending_inactive">can_withdraw_pending_inactive</a>(pool_address: <b>address</b>): bool &#123;<br />    <a href="stake.md#0x1_stake_get_validator_state">stake::get_validator_state</a>(pool_address) &#61;&#61; <a href="delegation_pool.md#0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a> &amp;&amp;<br />        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt;&#61; <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_voter_total_voting_power"></a>

## Function `calculate_and_update_voter_total_voting_power`

Return the total voting power of a delegator in a delegation pool. This function syncs DelegationPool to the
latest state.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voter_total_voting_power">calculate_and_update_voter_total_voting_power</a>(pool_address: <b>address</b>, voter: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voter_total_voting_power">calculate_and_update_voter_total_voting_power</a>(<br />    pool_address: <b>address</b>,<br />    voter: <b>address</b><br />): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);<br />    // Delegation pool need <b>to</b> be synced <b>to</b> explain rewards(which could change the <a href="coin.md#0x1_coin">coin</a> amount) and<br />    // commission(which could cause share transfer).<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br />    <b>let</b> pool &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    <b>let</b> latest_delegated_votes &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, voter);<br />    <a href="delegation_pool.md#0x1_delegation_pool_calculate_total_voting_power">calculate_total_voting_power</a>(pool, latest_delegated_votes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_remaining_voting_power"></a>

## Function `calculate_and_update_remaining_voting_power`

Return the remaining voting power of a delegator in a delegation pool on a proposal. This function syncs DelegationPool to the
latest state.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_remaining_voting_power">calculate_and_update_remaining_voting_power</a>(pool_address: <b>address</b>, voter_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_remaining_voting_power">calculate_and_update_remaining_voting_power</a>(<br />    pool_address: <b>address</b>,<br />    voter_address: <b>address</b>,<br />    proposal_id: u64<br />): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);<br />    // If the whole <a href="stake.md#0x1_stake">stake</a> pool <b>has</b> no <a href="voting.md#0x1_voting">voting</a> power(e.g. it <b>has</b> already voted before partial<br />    // governance <a href="voting.md#0x1_voting">voting</a> flag is enabled), the delegator also <b>has</b> no <a href="voting.md#0x1_voting">voting</a> power.<br />    <b>if</b> (<a href="aptos_governance.md#0x1_aptos_governance_get_remaining_voting_power">aptos_governance::get_remaining_voting_power</a>(pool_address, proposal_id) &#61;&#61; 0) &#123;<br />        <b>return</b> 0<br />    &#125;;<br /><br />    <b>let</b> total_voting_power &#61; <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voter_total_voting_power">calculate_and_update_voter_total_voting_power</a>(pool_address, voter_address);<br />    <b>let</b> governance_records &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    total_voting_power &#45; <a href="delegation_pool.md#0x1_delegation_pool_get_used_voting_power">get_used_voting_power</a>(governance_records, voter_address, proposal_id)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegator_voter"></a>

## Function `calculate_and_update_delegator_voter`

Return the latest delegated voter of a delegator in a delegation pool. This function syncs DelegationPool to the
latest state.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter">calculate_and_update_delegator_voter</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter">calculate_and_update_delegator_voter</a>(<br />    pool_address: <b>address</b>,<br />    delegator_address: <b>address</b><br />): <b>address</b> <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);<br />    <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(<br />        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address),<br />        <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address),<br />        delegator_address<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_voting_delegation"></a>

## Function `calculate_and_update_voting_delegation`

Return the current state of a voting delegation of a delegator in a delegation pool.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voting_delegation">calculate_and_update_voting_delegation</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): (<b>address</b>, <b>address</b>, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_voting_delegation">calculate_and_update_voting_delegation</a>(<br />    pool_address: <b>address</b>,<br />    delegator_address: <b>address</b><br />): (<b>address</b>, <b>address</b>, u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);<br />    <b>let</b> vote_delegation &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(<br />        <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address),<br />        <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address),<br />        delegator_address<br />    );<br /><br />    (vote_delegation.voter, vote_delegation.pending_voter, vote_delegation.last_locked_until_secs)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_expected_stake_pool_address"></a>

## Function `get_expected_stake_pool_address`

Return the address of the stake pool to be created with the provided owner, and seed.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_expected_stake_pool_address">get_expected_stake_pool_address</a>(owner: <b>address</b>, delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_expected_stake_pool_address">get_expected_stake_pool_address</a>(owner: <b>address</b>, delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br />): <b>address</b> &#123;<br />    <b>let</b> seed &#61; <a href="delegation_pool.md#0x1_delegation_pool_create_resource_account_seed">create_resource_account_seed</a>(delegation_pool_creation_seed);<br />    <a href="account.md#0x1_account_create_resource_address">account::create_resource_address</a>(&amp;owner, seed)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_min_remaining_secs_for_commission_change"></a>

## Function `min_remaining_secs_for_commission_change`

Return the minimum remaining time in seconds for commission change, which is one fourth of the lockup duration.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_min_remaining_secs_for_commission_change">min_remaining_secs_for_commission_change</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_min_remaining_secs_for_commission_change">min_remaining_secs_for_commission_change</a>(): u64 &#123;<br />    <b>let</b> config &#61; <a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>();<br />    <a href="staking_config.md#0x1_staking_config_get_recurring_lockup_duration">staking_config::get_recurring_lockup_duration</a>(&amp;config) / 4<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_allowlisting_enabled"></a>

## Function `allowlisting_enabled`

Return whether allowlisting is enabled for the provided delegation pool.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address: <b>address</b>): bool &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(pool_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_delegator_allowlisted"></a>

## Function `delegator_allowlisted`

Return whether the provided delegator is allowlisted.
A delegator is allowlisted if:
&#45; allowlisting is disabled on the pool
&#45; delegator is part of the allowlist


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(<br />    pool_address: <b>address</b>,<br />    delegator_address: <b>address</b>,<br />): bool <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    <b>if</b> (!<a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address)) &#123; <b>return</b> <b>true</b> &#125;;<br />    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(<b>freeze</b>(<a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address)), delegator_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_delegators_allowlist"></a>

## Function `get_delegators_allowlist`

Return allowlist or revert if allowlisting is not enabled for the provided delegation pool.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegators_allowlist">get_delegators_allowlist</a>(pool_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegators_allowlist">get_delegators_allowlist</a>(<br />    pool_address: <b>address</b>,<br />): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);<br /><br />    <b>let</b> allowlist &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_for_each_ref">smart_table::for_each_ref</a>(<b>freeze</b>(<a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address)), &#124;delegator, _v&#124; &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> allowlist, &#42;delegator);<br />    &#125;);<br />    allowlist<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_initialize_delegation_pool"></a>

## Function `initialize_delegation_pool`

Initialize a delegation pool of custom fixed <code>operator_commission_percentage</code>.
A resource account is created from <code>owner</code> signer and its supplied <code>delegation_pool_creation_seed</code>
to host the delegation pool resource and own the underlying stake pool.
Ownership over setting the operator/voter is granted to <code>owner</code> who has both roles initially.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_initialize_delegation_pool">initialize_delegation_pool</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator_commission_percentage: u64, delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_initialize_delegation_pool">initialize_delegation_pool</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    operator_commission_percentage: u64,<br />    delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_delegation_pools_enabled">features::delegation_pools_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOLS_DISABLED">EDELEGATION_POOLS_DISABLED</a>));<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>assert</b>!(!<a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(owner_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>));<br />    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a> &lt;&#61; <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>));<br /><br />    // generate a seed <b>to</b> be used <b>to</b> create the resource <a href="account.md#0x1_account">account</a> hosting the delegation pool<br />    <b>let</b> seed &#61; <a href="delegation_pool.md#0x1_delegation_pool_create_resource_account_seed">create_resource_account_seed</a>(delegation_pool_creation_seed);<br /><br />    <b>let</b> (stake_pool_signer, stake_pool_signer_cap) &#61; <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(owner, seed);<br />    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&amp;stake_pool_signer);<br /><br />    // stake_pool_signer will be owner of the <a href="stake.md#0x1_stake">stake</a> pool and have its `<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>`<br />    <b>let</b> pool_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;stake_pool_signer);<br />    <a href="stake.md#0x1_stake_initialize_stake_owner">stake::initialize_stake_owner</a>(&amp;stake_pool_signer, 0, owner_address, owner_address);<br /><br />    <b>let</b> inactive_shares &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a>, <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>&gt;();<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(<br />        &amp;<b>mut</b> inactive_shares,<br />        <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(0),<br />        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create_with_scaling_factor">pool_u64::create_with_scaling_factor</a>(<a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>)<br />    );<br /><br />    <b>move_to</b>(&amp;stake_pool_signer, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> &#123;<br />        active_shares: <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create_with_scaling_factor">pool_u64::create_with_scaling_factor</a>(<a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>),<br />        observed_lockup_cycle: <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(0),<br />        inactive_shares,<br />        pending_withdrawals: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;<b>address</b>, <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a>&gt;(),<br />        stake_pool_signer_cap,<br />        total_coins_inactive: 0,<br />        operator_commission_percentage,<br />        add_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_AddStakeEvent">AddStakeEvent</a>&gt;(&amp;stake_pool_signer),<br />        reactivate_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_ReactivateStakeEvent">ReactivateStakeEvent</a>&gt;(&amp;stake_pool_signer),<br />        unlock_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_UnlockStakeEvent">UnlockStakeEvent</a>&gt;(&amp;stake_pool_signer),<br />        withdraw_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_WithdrawStakeEvent">WithdrawStakeEvent</a>&gt;(&amp;stake_pool_signer),<br />        distribute_commission_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">DistributeCommissionEvent</a>&gt;(&amp;stake_pool_signer),<br />    &#125;);<br /><br />    // save delegation pool ownership and resource <a href="account.md#0x1_account">account</a> <b>address</b> (inner <a href="stake.md#0x1_stake">stake</a> pool <b>address</b>) on `owner`<br />    <b>move_to</b>(owner, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a> &#123; pool_address &#125;);<br /><br />    // All delegation pool enable partial governance <a href="voting.md#0x1_voting">voting</a> by default once the feature flag is enabled.<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_partial_governance_voting_enabled">features::partial_governance_voting_enabled</a>(<br />    ) &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_delegation_pool_partial_governance_voting_enabled">features::delegation_pool_partial_governance_voting_enabled</a>()) &#123;<br />        <a href="delegation_pool.md#0x1_delegation_pool_enable_partial_governance_voting">enable_partial_governance_voting</a>(pool_address);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_beneficiary_for_operator"></a>

## Function `beneficiary_for_operator`

Return the beneficiary address of the operator.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(operator: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(operator: <b>address</b>): <b>address</b> <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator)) &#123;<br />        <b>return</b> <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator).beneficiary_for_operator<br />    &#125; <b>else</b> &#123;<br />        operator<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_enable_partial_governance_voting"></a>

## Function `enable_partial_governance_voting`

Enable partial governance voting on a stake pool. The voter of this stake pool will be managed by this module.
The existing voter will be replaced. The function is permissionless.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_enable_partial_governance_voting">enable_partial_governance_voting</a>(pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_enable_partial_governance_voting">enable_partial_governance_voting</a>(<br />    pool_address: <b>address</b>,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_partial_governance_voting_enabled">features::partial_governance_voting_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDISABLED_FUNCTION">EDISABLED_FUNCTION</a>));<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_delegation_pool_partial_governance_voting_enabled">features::delegation_pool_partial_governance_voting_enabled</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDISABLED_FUNCTION">EDISABLED_FUNCTION</a>)<br />    );<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation.<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    <b>let</b> <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a> &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    <b>let</b> stake_pool_signer &#61; <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>);<br />    // delegated_voter is managed by the <a href="stake.md#0x1_stake">stake</a> pool itself, which <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> capability is managed by <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>.<br />    // So <a href="voting.md#0x1_voting">voting</a> power of this <a href="stake.md#0x1_stake">stake</a> pool can only be used through this <b>module</b>.<br />    <a href="stake.md#0x1_stake_set_delegated_voter">stake::set_delegated_voter</a>(&amp;stake_pool_signer, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;stake_pool_signer));<br /><br />    <b>move_to</b>(&amp;stake_pool_signer, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />        votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),<br />        votes_per_proposal: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),<br />        vote_delegation: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),<br />        delegated_votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),<br />        vote_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_VoteEvent">VoteEvent</a>&gt;(&amp;stake_pool_signer),<br />        create_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_CreateProposalEvent">CreateProposalEvent</a>&gt;(&amp;stake_pool_signer),<br />        delegate_voting_power_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPowerEvent">DelegateVotingPowerEvent</a>&gt;(&amp;stake_pool_signer),<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_vote"></a>

## Function `vote`

Vote on a proposal with a voter&apos;s voting power. To successfully vote, the following conditions must be met:
1. The voting period of the proposal hasn&apos;t ended.
2. The delegation pool&apos;s lockup period ends after the voting period of the proposal.
3. The voter still has spare voting power on this proposal.
4. The delegation pool never votes on the proposal before enabling partial governance voting.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_vote">vote</a>(voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, proposal_id: u64, voting_power: u64, should_pass: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_vote">vote</a>(<br />    voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    proposal_id: u64,<br />    voting_power: u64,<br />    should_pass: bool<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);<br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation.<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    <b>let</b> voter_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);<br />    <b>let</b> remaining_voting_power &#61; <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_remaining_voting_power">calculate_and_update_remaining_voting_power</a>(<br />        pool_address,<br />        voter_address,<br />        proposal_id<br />    );<br />    <b>if</b> (voting_power &gt; remaining_voting_power) &#123;<br />        voting_power &#61; remaining_voting_power;<br />    &#125;;<br />    <b>assert</b>!(voting_power &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ENO_VOTING_POWER">ENO_VOTING_POWER</a>));<br /><br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    // Check a edge case during the transient period of enabling partial governance <a href="voting.md#0x1_voting">voting</a>.<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_and_update_proposal_used_voting_power">assert_and_update_proposal_used_voting_power</a>(governance_records, pool_address, proposal_id, voting_power);<br />    <b>let</b> used_voting_power &#61; <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_used_voting_power">borrow_mut_used_voting_power</a>(governance_records, voter_address, proposal_id);<br />    &#42;used_voting_power &#61; &#42;used_voting_power &#43; voting_power;<br /><br />    <b>let</b> pool_signer &#61; <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address));<br />    <a href="aptos_governance.md#0x1_aptos_governance_partial_vote">aptos_governance::partial_vote</a>(&amp;pool_signer, pool_address, proposal_id, voting_power, should_pass);<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool_Vote">Vote</a> &#123;<br />                voter: voter_address,<br />                proposal_id,<br />                <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: pool_address,<br />                num_votes: voting_power,<br />                should_pass,<br />            &#125;<br />        );<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> governance_records.vote_events,<br />        <a href="delegation_pool.md#0x1_delegation_pool_VoteEvent">VoteEvent</a> &#123;<br />            voter: voter_address,<br />            proposal_id,<br />            <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: pool_address,<br />            num_votes: voting_power,<br />            should_pass,<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_create_proposal"></a>

## Function `create_proposal`

A voter could create a governance proposal by this function. To successfully create a proposal, the voter&apos;s
voting power in THIS delegation pool must be not less than the minimum required voting power specified in
<code><a href="aptos_governance.md#0x1_aptos_governance">aptos_governance</a>.<b>move</b></code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_create_proposal">create_proposal</a>(voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_create_proposal">create_proposal</a>(<br />    voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    is_multi_step_proposal: bool,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);<br /><br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    <b>let</b> voter_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);<br />    <b>let</b> pool &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    <b>let</b> total_voting_power &#61; <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegated_votes">calculate_and_update_delegated_votes</a>(pool, governance_records, voter_addr);<br />    <b>assert</b>!(<br />        total_voting_power &gt;&#61; <a href="aptos_governance.md#0x1_aptos_governance_get_required_proposer_stake">aptos_governance::get_required_proposer_stake</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EINSUFFICIENT_PROPOSER_STAKE">EINSUFFICIENT_PROPOSER_STAKE</a>));<br />    <b>let</b> pool_signer &#61; <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address));<br />    <b>let</b> proposal_id &#61; <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2_impl">aptos_governance::create_proposal_v2_impl</a>(<br />        &amp;pool_signer,<br />        pool_address,<br />        execution_hash,<br />        metadata_location,<br />        metadata_hash,<br />        is_multi_step_proposal,<br />    );<br /><br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool_CreateProposal">CreateProposal</a> &#123;<br />                proposal_id,<br />                voter: voter_addr,<br />                <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: pool_address,<br />            &#125;<br />        );<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> governance_records.create_proposal_events,<br />        <a href="delegation_pool.md#0x1_delegation_pool_CreateProposalEvent">CreateProposalEvent</a> &#123;<br />            proposal_id,<br />            voter: voter_addr,<br />            <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: pool_address,<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_assert_owner_cap_exists"></a>

## Function `assert_owner_cap_exists`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_owner_cap_exists">assert_owner_cap_exists</a>(owner: <b>address</b>) &#123;<br />    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(owner), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_assert_delegation_pool_exists"></a>

## Function `assert_delegation_pool_exists`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address: <b>address</b>) &#123;<br />    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_delegation_pool_exists">delegation_pool_exists</a>(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST">EDELEGATION_POOL_DOES_NOT_EXIST</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_assert_min_active_balance"></a>

## Function `assert_min_active_balance`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>) &#123;<br />    <b>let</b> balance &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(&amp;pool.active_shares, delegator_address);<br />    <b>assert</b>!(balance &gt;&#61; <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW">EDELEGATOR_ACTIVE_BALANCE_TOO_LOW</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_assert_min_pending_inactive_balance"></a>

## Function `assert_min_pending_inactive_balance`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_min_pending_inactive_balance">assert_min_pending_inactive_balance</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_min_pending_inactive_balance">assert_min_pending_inactive_balance</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>) &#123;<br />    <b>let</b> balance &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool), delegator_address);<br />    <b>assert</b>!(<br />        balance &gt;&#61; <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW">EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW</a>)<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_assert_partial_governance_voting_enabled"></a>

## Function `assert_partial_governance_voting_enabled`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address: <b>address</b>) &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <b>assert</b>!(<br />        <a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED">EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED</a>)<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_assert_allowlisting_enabled"></a>

## Function `assert_allowlisting_enabled`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address: <b>address</b>) &#123;<br />    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_ENABLED">EDELEGATORS_ALLOWLISTING_NOT_ENABLED</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_assert_delegator_allowlisted"></a>

## Function `assert_delegator_allowlisted`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_delegator_allowlisted">assert_delegator_allowlisted</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_delegator_allowlisted">assert_delegator_allowlisted</a>(<br />    pool_address: <b>address</b>,<br />    delegator_address: <b>address</b>,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    <b>assert</b>!(<br />        <a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address, delegator_address),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_NOT_ALLOWLISTED">EDELEGATOR_NOT_ALLOWLISTED</a>)<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake"></a>

## Function `coins_to_redeem_to_ensure_min_stake`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake">coins_to_redeem_to_ensure_min_stake</a>(src_shares_pool: &amp;<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, amount: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake">coins_to_redeem_to_ensure_min_stake</a>(<br />    src_shares_pool: &amp;<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>,<br />    shareholder: <b>address</b>,<br />    amount: u64,<br />): u64 &#123;<br />    // find how many coins would be redeemed <b>if</b> supplying `amount`<br />    <b>let</b> redeemed_coins &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount">pool_u64::shares_to_amount</a>(<br />        src_shares_pool,<br />        <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(src_shares_pool, shareholder, amount)<br />    );<br />    // <b>if</b> balance drops under threshold then redeem it entirely<br />    <b>let</b> src_balance &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(src_shares_pool, shareholder);<br />    <b>if</b> (src_balance &#45; redeemed_coins &lt; <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>) &#123;<br />        amount &#61; src_balance;<br />    &#125;;<br />    amount<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake"></a>

## Function `coins_to_transfer_to_ensure_min_stake`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(src_shares_pool: &amp;<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, dst_shares_pool: &amp;<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, amount: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(<br />    src_shares_pool: &amp;<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>,<br />    dst_shares_pool: &amp;<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>,<br />    shareholder: <b>address</b>,<br />    amount: u64,<br />): u64 &#123;<br />    // find how many coins would be redeemed from source <b>if</b> supplying `amount`<br />    <b>let</b> redeemed_coins &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount">pool_u64::shares_to_amount</a>(<br />        src_shares_pool,<br />        <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(src_shares_pool, shareholder, amount)<br />    );<br />    // <b>if</b> balance on destination would be less than threshold then redeem difference <b>to</b> threshold<br />    <b>let</b> dst_balance &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(dst_shares_pool, shareholder);<br />    <b>if</b> (dst_balance &#43; redeemed_coins &lt; <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>) &#123;<br />        // `redeemed_coins` &gt;&#61; `amount` &#45; 1 <b>as</b> redeem can lose at most 1 <a href="coin.md#0x1_coin">coin</a><br />        amount &#61; <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a> &#45; dst_balance &#43; 1;<br />    &#125;;<br />    // check <b>if</b> new `amount` drops balance on source under threshold and adjust<br />    <a href="delegation_pool.md#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake">coins_to_redeem_to_ensure_min_stake</a>(src_shares_pool, shareholder, amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_retrieve_stake_pool_owner"></a>

## Function `retrieve_stake_pool_owner`

Retrieves the shared resource account owning the stake pool in order
to forward a stake&#45;management operation to this underlying pool.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#123;<br />    <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&amp;pool.stake_pool_signer_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_pool_address"></a>

## Function `get_pool_address`

Get the address of delegation pool reference <code>pool</code>.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): <b>address</b> &#123;<br />    <a href="account.md#0x1_account_get_signer_capability_address">account::get_signer_capability_address</a>(&amp;pool.stake_pool_signer_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_delegator_active_shares"></a>

## Function `get_delegator_active_shares`

Get the active share amount of the delegator.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator: <b>address</b>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator: <b>address</b>): u128 &#123;<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(&amp;pool.active_shares, delegator)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_delegator_pending_inactive_shares"></a>

## Function `get_delegator_pending_inactive_shares`

Get the pending inactive share amount of the delegator.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_pending_inactive_shares">get_delegator_pending_inactive_shares</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator: <b>address</b>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_pending_inactive_shares">get_delegator_pending_inactive_shares</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator: <b>address</b>): u128 &#123;<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool), delegator)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_get_used_voting_power"></a>

## Function `get_used_voting_power`

Get the used voting power of a voter on a proposal.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_used_voting_power">get_used_voting_power</a>(governance_records: &amp;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, voter: <b>address</b>, proposal_id: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_used_voting_power">get_used_voting_power</a>(governance_records: &amp;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, voter: <b>address</b>, proposal_id: u64): u64 &#123;<br />    <b>let</b> votes &#61; &amp;governance_records.votes;<br />    <b>let</b> key &#61; <a href="delegation_pool.md#0x1_delegation_pool_VotingRecordKey">VotingRecordKey</a> &#123;<br />        voter,<br />        proposal_id,<br />    &#125;;<br />    &#42;<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_with_default">smart_table::borrow_with_default</a>(votes, key, &amp;0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_create_resource_account_seed"></a>

## Function `create_resource_account_seed`

Create the seed to derive the resource account address.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_create_resource_account_seed">create_resource_account_seed</a>(delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_create_resource_account_seed">create_resource_account_seed</a>(<br />    delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <b>let</b> seed &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();<br />    // <b>include</b> <b>module</b> salt (before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> subseeds) <b>to</b> avoid conflicts <b>with</b> other modules creating resource accounts<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, <a href="delegation_pool.md#0x1_delegation_pool_MODULE_SALT">MODULE_SALT</a>);<br />    // <b>include</b> an additional salt in case the same resource <a href="account.md#0x1_account">account</a> <b>has</b> already been created<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, delegation_pool_creation_seed);<br />    seed<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_borrow_mut_used_voting_power"></a>

## Function `borrow_mut_used_voting_power`

Borrow the mutable used voting power of a voter on a proposal.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_used_voting_power">borrow_mut_used_voting_power</a>(governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, voter: <b>address</b>, proposal_id: u64): &amp;<b>mut</b> u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_used_voting_power">borrow_mut_used_voting_power</a>(<br />    governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,<br />    voter: <b>address</b>,<br />    proposal_id: u64<br />): &amp;<b>mut</b> u64 &#123;<br />    <b>let</b> votes &#61; &amp;<b>mut</b> governance_records.votes;<br />    <b>let</b> key &#61; <a href="delegation_pool.md#0x1_delegation_pool_VotingRecordKey">VotingRecordKey</a> &#123;<br />        proposal_id,<br />        voter,<br />    &#125;;<br />    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(votes, key, 0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation"></a>

## Function `update_and_borrow_mut_delegator_vote_delegation`

Update VoteDelegation of a delegator to up&#45;to&#45;date then borrow_mut it.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, delegator: <b>address</b>): &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">delegation_pool::VoteDelegation</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(<br />    pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,<br />    delegator: <b>address</b><br />): &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">VoteDelegation</a> &#123;<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);<br />    <b>let</b> locked_until_secs &#61; <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address);<br /><br />    <b>let</b> vote_delegation_table &#61; &amp;<b>mut</b> governance_records.vote_delegation;<br />    // By default, a delegator&apos;s delegated voter is itself.<br />    // TODO: recycle storage when <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">VoteDelegation</a> equals <b>to</b> default value.<br />    <b>if</b> (!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(vote_delegation_table, delegator)) &#123;<br />        <b>return</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(vote_delegation_table, delegator, <a href="delegation_pool.md#0x1_delegation_pool_VoteDelegation">VoteDelegation</a> &#123;<br />            voter: delegator,<br />            last_locked_until_secs: locked_until_secs,<br />            pending_voter: delegator,<br />        &#125;)<br />    &#125;;<br /><br />    <b>let</b> vote_delegation &#61; <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(vote_delegation_table, delegator);<br />    // A lockup period <b>has</b> passed since last time `vote_delegation` was updated. Pending voter takes effect.<br />    <b>if</b> (vote_delegation.last_locked_until_secs &lt; locked_until_secs) &#123;<br />        vote_delegation.voter &#61; vote_delegation.pending_voter;<br />        vote_delegation.last_locked_until_secs &#61; locked_until_secs;<br />    &#125;;<br />    vote_delegation<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_update_and_borrow_mut_delegated_votes"></a>

## Function `update_and_borrow_mut_delegated_votes`

Update DelegatedVotes of a voter to up&#45;to&#45;date then borrow_mut it.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, voter: <b>address</b>): &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">delegation_pool::DelegatedVotes</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(<br />    pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,<br />    voter: <b>address</b><br />): &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a> &#123;<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);<br />    <b>let</b> locked_until_secs &#61; <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address);<br /><br />    <b>let</b> delegated_votes_per_voter &#61; &amp;<b>mut</b> governance_records.delegated_votes;<br />    // By default, a delegator&apos;s voter is itself.<br />    // TODO: recycle storage when <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a> equals <b>to</b> default value.<br />    <b>if</b> (!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(delegated_votes_per_voter, voter)) &#123;<br />        <b>let</b> active_shares &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(pool, voter);<br />        <b>let</b> inactive_shares &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_pending_inactive_shares">get_delegator_pending_inactive_shares</a>(pool, voter);<br />        <b>return</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(delegated_votes_per_voter, voter, <a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a> &#123;<br />            active_shares,<br />            pending_inactive_shares: inactive_shares,<br />            active_shares_next_lockup: active_shares,<br />            last_locked_until_secs: locked_until_secs,<br />        &#125;)<br />    &#125;;<br /><br />    <b>let</b> delegated_votes &#61; <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(delegated_votes_per_voter, voter);<br />    // A lockup period <b>has</b> passed since last time `delegated_votes` was updated. Pending voter takes effect.<br />    <b>if</b> (delegated_votes.last_locked_until_secs &lt; locked_until_secs) &#123;<br />        delegated_votes.active_shares &#61; delegated_votes.active_shares_next_lockup;<br />        delegated_votes.pending_inactive_shares &#61; 0;<br />        delegated_votes.last_locked_until_secs &#61; locked_until_secs;<br />    &#125;;<br />    delegated_votes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_olc_with_index"></a>

## Function `olc_with_index`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(index: u64): <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(index: u64): <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a> &#123; index &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_total_voting_power"></a>

## Function `calculate_total_voting_power`

Given the amounts of shares in <code>active_shares</code> pool and <code>inactive_shares</code> pool, calculate the total voting
power, which equals to the sum of the coin amounts.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_total_voting_power">calculate_total_voting_power</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, latest_delegated_votes: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">delegation_pool::DelegatedVotes</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_total_voting_power">calculate_total_voting_power</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, latest_delegated_votes: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegatedVotes">DelegatedVotes</a>): u64 &#123;<br />    <b>let</b> active_amount &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount">pool_u64::shares_to_amount</a>(<br />        &amp;<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>.active_shares,<br />        latest_delegated_votes.active_shares);<br />    <b>let</b> pending_inactive_amount &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount">pool_u64::shares_to_amount</a>(<br />        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>),<br />        latest_delegated_votes.pending_inactive_shares);<br />    active_amount &#43; pending_inactive_amount<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegator_voter_internal"></a>

## Function `calculate_and_update_delegator_voter_internal`

Update VoteDelegation of a delegator to up&#45;to&#45;date then return the latest voter.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, delegator: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(<br />    pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,<br />    delegator: <b>address</b><br />): <b>address</b> &#123;<br />    <b>let</b> vote_delegation &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(pool, governance_records, delegator);<br />    vote_delegation.voter<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegated_votes"></a>

## Function `calculate_and_update_delegated_votes`

Update DelegatedVotes of a voter to up&#45;to&#45;date then return the total voting power of this voter.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegated_votes">calculate_and_update_delegated_votes</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, voter: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegated_votes">calculate_and_update_delegated_votes</a>(<br />    pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>,<br />    voter: <b>address</b><br />): u64 &#123;<br />    <b>let</b> delegated_votes &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, voter);<br />    <a href="delegation_pool.md#0x1_delegation_pool_calculate_total_voting_power">calculate_total_voting_power</a>(pool, delegated_votes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_borrow_mut_delegators_allowlist"></a>

## Function `borrow_mut_delegators_allowlist`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address: <b>address</b>): &amp;<b>mut</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<b>address</b>, bool&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(<br />    pool_address: <b>address</b><br />): &amp;<b>mut</b> SmartTable&lt;<b>address</b>, bool&gt; <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(pool_address).allowlist<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the underlying stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_operator">set_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_operator">set_operator</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    new_operator: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));<br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation<br />    // ensure the <b>old</b> operator is paid its uncommitted commission rewards<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br />    <a href="stake.md#0x1_stake_set_operator">stake::set_operator</a>(&amp;<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address)), new_operator);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Allows an operator to change its beneficiary. Any existing unpaid commission rewards will be paid to the new
beneficiary. To ensure payment to the current beneficiary, one should first call <code>synchronize_delegation_pool</code>
before switching the beneficiary. An operator can set one beneficiary for delegation pools, not a separate
one for each pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_beneficiary: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(<br />    operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    new_beneficiary: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operator_beneficiary_change_enabled">features::operator_beneficiary_change_enabled</a>(), std::error::invalid_state(<br />        <a href="delegation_pool.md#0x1_delegation_pool_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED">EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED</a><br />    ));<br />    // The beneficiay <b>address</b> of an operator is stored under the operator&apos;s <b>address</b>.<br />    // So, the operator does not need <b>to</b> be validated <b>with</b> respect <b>to</b> a staking pool.<br />    <b>let</b> operator_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator);<br />    <b>let</b> old_beneficiary &#61; <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(operator_addr);<br />    <b>if</b> (<b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator_addr)) &#123;<br />        <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator_addr).beneficiary_for_operator &#61; new_beneficiary;<br />    &#125; <b>else</b> &#123;<br />        <b>move_to</b>(operator, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123; beneficiary_for_operator: new_beneficiary &#125;);<br />    &#125;;<br /><br />    emit(<a href="delegation_pool.md#0x1_delegation_pool_SetBeneficiaryForOperator">SetBeneficiaryForOperator</a> &#123;<br />        operator: operator_addr,<br />        old_beneficiary,<br />        new_beneficiary,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_update_commission_percentage"></a>

## Function `update_commission_percentage`

Allows an owner to update the commission percentage for the operator of the underlying stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_commission_percentage">update_commission_percentage</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_commission_percentage: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_commission_percentage">update_commission_percentage</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    new_commission_percentage: u64<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_commission_change_delegation_pool_enabled">features::commission_change_delegation_pool_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<br />        <a href="delegation_pool.md#0x1_delegation_pool_ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED">ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED</a><br />    ));<br />    <b>assert</b>!(new_commission_percentage &lt;&#61; <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>));<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(owner_address);<br />    <b>assert</b>!(<br />        <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(pool_address) &#43; <a href="delegation_pool.md#0x1_delegation_pool_MAX_COMMISSION_INCREASE">MAX_COMMISSION_INCREASE</a> &gt;&#61; new_commission_percentage,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ETOO_LARGE_COMMISSION_INCREASE">ETOO_LARGE_COMMISSION_INCREASE</a>)<br />    );<br />    <b>assert</b>!(<br />        <a href="stake.md#0x1_stake_get_remaining_lockup_secs">stake::get_remaining_lockup_secs</a>(pool_address) &gt;&#61; <a href="delegation_pool.md#0x1_delegation_pool_min_remaining_secs_for_commission_change">min_remaining_secs_for_commission_change</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_ETOO_LATE_COMMISSION_CHANGE">ETOO_LATE_COMMISSION_CHANGE</a>)<br />    );<br /><br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation. this <b>ensures</b>:<br />    // (1) the operator is paid its uncommitted commission rewards <b>with</b> the <b>old</b> commission percentage, and<br />    // (2) <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> pending commission percentage change is applied before the new commission percentage is set.<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    <b>if</b> (<b>exists</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address)) &#123;<br />        <b>let</b> commission_percentage &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(pool_address);<br />        commission_percentage.commission_percentage_next_lockup_cycle &#61; new_commission_percentage;<br />        commission_percentage.effective_after_secs &#61; <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a> &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />        <b>let</b> pool_signer &#61; <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&amp;<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>.stake_pool_signer_cap);<br />        <b>move_to</b>(&amp;pool_signer, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />            commission_percentage_next_lockup_cycle: new_commission_percentage,<br />            effective_after_secs: <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(pool_address),<br />        &#125;);<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_CommissionPercentageChange">CommissionPercentageChange</a> &#123;<br />        pool_address,<br />        owner: owner_address,<br />        commission_percentage_next_lockup_cycle: new_commission_percentage,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_set_delegated_voter"></a>

## Function `set_delegated_voter`

Allows an owner to change the delegated voter of the underlying stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_delegated_voter">set_delegated_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_delegated_voter">set_delegated_voter</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    new_voter: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    // No one can change delegated_voter once the partial governance <a href="voting.md#0x1_voting">voting</a> feature is enabled.<br />    <b>assert</b>!(<br />        !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_delegation_pool_partial_governance_voting_enabled">features::delegation_pool_partial_governance_voting_enabled</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>)<br />    );<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));<br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br />    <a href="stake.md#0x1_stake_set_delegated_voter">stake::set_delegated_voter</a>(&amp;<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address)), new_voter);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_delegate_voting_power"></a>

## Function `delegate_voting_power`

Allows a delegator to delegate its voting power to a voter. If this delegator already has a delegated voter,
this change won&apos;t take effects until the next lockup period.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegate_voting_power">delegate_voting_power</a>(delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_delegate_voting_power">delegate_voting_power</a>(<br />    delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    new_voter: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_partial_governance_voting_enabled">assert_partial_governance_voting_enabled</a>(pool_address);<br /><br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    <b>let</b> delegator_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);<br />    <b>let</b> <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a> &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    <b>let</b> delegator_vote_delegation &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(<br />        <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>,<br />        governance_records,<br />        delegator_address<br />    );<br />    <b>let</b> pending_voter: <b>address</b> &#61; delegator_vote_delegation.pending_voter;<br /><br />    // No need <b>to</b> <b>update</b> <b>if</b> the voter doesn&apos;t really change.<br />    <b>if</b> (pending_voter !&#61; new_voter) &#123;<br />        delegator_vote_delegation.pending_voter &#61; new_voter;<br />        <b>let</b> active_shares &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(<a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>, delegator_address);<br />        // &lt;active shares&gt; of &lt;pending voter of shareholder&gt; &#45;&#61; &lt;active_shares&gt;<br />        // &lt;active shares&gt; of &lt;new voter of shareholder&gt; &#43;&#61; &lt;active_shares&gt;<br />        <b>let</b> pending_delegated_votes &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>,<br />            governance_records,<br />            pending_voter<br />        );<br />        pending_delegated_votes.active_shares_next_lockup &#61;<br />            pending_delegated_votes.active_shares_next_lockup &#45; active_shares;<br /><br />        <b>let</b> new_delegated_votes &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a>,<br />            governance_records,<br />            new_voter<br />        );<br />        new_delegated_votes.active_shares_next_lockup &#61;<br />            new_delegated_votes.active_shares_next_lockup &#43; active_shares;<br />    &#125;;<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPower">DelegateVotingPower</a> &#123;<br />            pool_address,<br />            delegator: delegator_address,<br />            voter: new_voter,<br />        &#125;)<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&amp;<b>mut</b> governance_records.delegate_voting_power_events, <a href="delegation_pool.md#0x1_delegation_pool_DelegateVotingPowerEvent">DelegateVotingPowerEvent</a> &#123;<br />        pool_address,<br />        delegator: delegator_address,<br />        voter: new_voter,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_enable_delegators_allowlisting"></a>

## Function `enable_delegators_allowlisting`

Enable delegators allowlisting as the pool owner.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_enable_delegators_allowlisting">enable_delegators_allowlisting</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_enable_delegators_allowlisting">enable_delegators_allowlisting</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> &#123;<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_delegation_pool_allowlisting_enabled">features::delegation_pool_allowlisting_enabled</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED">EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED</a>)<br />    );<br /><br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));<br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_allowlisting_enabled">allowlisting_enabled</a>(pool_address)) &#123; <b>return</b> &#125;;<br /><br />    <b>let</b> pool_signer &#61; <a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address));<br />    <b>move_to</b>(&amp;pool_signer, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123; allowlist: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>&lt;<b>address</b>, bool&gt;() &#125;);<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_EnableDelegatorsAllowlisting">EnableDelegatorsAllowlisting</a> &#123; pool_address &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_disable_delegators_allowlisting"></a>

## Function `disable_delegators_allowlisting`

Disable delegators allowlisting as the pool owner. The existing allowlist will be emptied.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_disable_delegators_allowlisting">disable_delegators_allowlisting</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_disable_delegators_allowlisting">disable_delegators_allowlisting</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);<br /><br />    <b>let</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123; allowlist &#125; &#61; <b>move_from</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(pool_address);<br />    // <b>if</b> the allowlist becomes too large, the owner can always remove some delegators<br />    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_destroy">smart_table::destroy</a>(allowlist);<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_DisableDelegatorsAllowlisting">DisableDelegatorsAllowlisting</a> &#123; pool_address &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_delegator"></a>

## Function `allowlist_delegator`

Allowlist a delegator as the pool owner.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_allowlist_delegator">allowlist_delegator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegator_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_allowlist_delegator">allowlist_delegator</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    delegator_address: <b>address</b>,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);<br /><br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address, delegator_address)) &#123; <b>return</b> &#125;;<br /><br />    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(<a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address), delegator_address, <b>true</b>);<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_AllowlistDelegator">AllowlistDelegator</a> &#123; pool_address, delegator_address &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_remove_delegator_from_allowlist"></a>

## Function `remove_delegator_from_allowlist`

Remove a delegator from the allowlist as the pool owner, but do not unlock their stake.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_remove_delegator_from_allowlist">remove_delegator_from_allowlist</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegator_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_remove_delegator_from_allowlist">remove_delegator_from_allowlist</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    delegator_address: <b>address</b>,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);<br /><br />    <b>if</b> (!<a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address, delegator_address)) &#123; <b>return</b> &#125;;<br /><br />    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_remove">smart_table::remove</a>(<a href="delegation_pool.md#0x1_delegation_pool_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(pool_address), delegator_address);<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_RemoveDelegatorFromAllowlist">RemoveDelegatorFromAllowlist</a> &#123; pool_address, delegator_address &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_evict_delegator"></a>

## Function `evict_delegator`

Evict a delegator that is not allowlisted by unlocking their entire stake.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_evict_delegator">evict_delegator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegator_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_evict_delegator">evict_delegator</a>(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    delegator_address: <b>address</b>,<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(pool_address);<br />    <b>assert</b>!(<br />        !<a href="delegation_pool.md#0x1_delegation_pool_delegator_allowlisted">delegator_allowlisted</a>(pool_address, delegator_address),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_ECANNOT_EVICT_ALLOWLISTED_DELEGATOR">ECANNOT_EVICT_ALLOWLISTED_DELEGATOR</a>)<br />    );<br /><br />    // synchronize pool in order <b>to</b> query latest balance of delegator<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    <b>let</b> pool &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_get_delegator_active_shares">get_delegator_active_shares</a>(pool, delegator_address) &#61;&#61; 0) &#123; <b>return</b> &#125;;<br /><br />    <a href="delegation_pool.md#0x1_delegation_pool_unlock_internal">unlock_internal</a>(delegator_address, pool_address, <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(&amp;pool.active_shares, delegator_address));<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool.md#0x1_delegation_pool_EvictDelegator">EvictDelegator</a> &#123; pool_address, delegator_address &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins to the delegation pool <code>pool_address</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_add_stake">add_stake</a>(delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_add_stake">add_stake</a>(<br />    delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    amount: u64<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    // short&#45;circuit <b>if</b> amount <b>to</b> add is 0 so no <a href="event.md#0x1_event">event</a> is emitted<br />    <b>if</b> (amount &#61;&#61; 0) &#123; <b>return</b> &#125;;<br /><br />    <b>let</b> delegator_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegator_allowlisted">assert_delegator_allowlisted</a>(pool_address, delegator_address);<br /><br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    // fee <b>to</b> be charged for adding `amount` <a href="stake.md#0x1_stake">stake</a> on this delegation pool at this epoch<br />    <b>let</b> add_stake_fee &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_add_stake_fee">get_add_stake_fee</a>(pool_address, amount);<br /><br />    <b>let</b> pool &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br /><br />    // <a href="stake.md#0x1_stake">stake</a> the entire amount <b>to</b> the <a href="stake.md#0x1_stake">stake</a> pool<br />    <a href="aptos_account.md#0x1_aptos_account_transfer">aptos_account::transfer</a>(delegator, pool_address, amount);<br />    <a href="stake.md#0x1_stake_add_stake">stake::add_stake</a>(&amp;<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool), amount);<br /><br />    // but buy shares for delegator just for the remaining amount after fee<br />    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool, delegator_address, amount &#45; add_stake_fee);<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool, delegator_address);<br /><br />    // grant temporary ownership over `add_stake` fees <b>to</b> a separate shareholder in order <b>to</b>:<br />    // &#45; not mistake them for rewards <b>to</b> pay the operator from<br />    // &#45; distribute them together <b>with</b> the `active` rewards when this epoch ends<br />    // in order <b>to</b> appreciate all shares on the active pool atomically<br />    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool, <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>, add_stake_fee);<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool_AddStake">AddStake</a> &#123;<br />                pool_address,<br />                delegator_address,<br />                amount_added: amount,<br />                add_stake_fee,<br />            &#125;,<br />        );<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> pool.add_stake_events,<br />        <a href="delegation_pool.md#0x1_delegation_pool_AddStakeEvent">AddStakeEvent</a> &#123;<br />            pool_address,<br />            delegator_address,<br />            amount_added: amount,<br />            add_stake_fee,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_unlock"></a>

## Function `unlock`

Unlock <code>amount</code> from the active &#43; pending_active stake of <code>delegator</code> or
at most how much active stake there is on the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock">unlock</a>(delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock">unlock</a>(<br />    delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    amount: u64<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    // short&#45;circuit <b>if</b> amount <b>to</b> unlock is 0 so no <a href="event.md#0x1_event">event</a> is emitted<br />    <b>if</b> (amount &#61;&#61; 0) &#123; <b>return</b> &#125;;<br /><br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    <b>let</b> delegator_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);<br />    <a href="delegation_pool.md#0x1_delegation_pool_unlock_internal">unlock_internal</a>(delegator_address, pool_address, amount);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_unlock_internal"></a>

## Function `unlock_internal`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock_internal">unlock_internal</a>(delegator_address: <b>address</b>, pool_address: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock_internal">unlock_internal</a>(<br />    delegator_address: <b>address</b>,<br />    pool_address: <b>address</b>,<br />    amount: u64<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    <b>assert</b>!(delegator_address !&#61; <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ECANNOT_UNLOCK_NULL_SHAREHOLDER">ECANNOT_UNLOCK_NULL_SHAREHOLDER</a>));<br /><br />    // fail unlock of more <a href="stake.md#0x1_stake">stake</a> than `active` on the <a href="stake.md#0x1_stake">stake</a> pool<br />    <b>let</b> (active, _, _, _) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);<br />    <b>assert</b>!(amount &lt;&#61; active, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK">ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK</a>));<br /><br />    <b>let</b> pool &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    amount &#61; <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(<br />        &amp;pool.active_shares,<br />        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool),<br />        delegator_address,<br />        amount,<br />    );<br />    amount &#61; <a href="delegation_pool.md#0x1_delegation_pool_redeem_active_shares">redeem_active_shares</a>(pool, delegator_address, amount);<br /><br />    <a href="stake.md#0x1_stake_unlock">stake::unlock</a>(&amp;<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool), amount);<br /><br />    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(pool, delegator_address, amount);<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_min_pending_inactive_balance">assert_min_pending_inactive_balance</a>(pool, delegator_address);<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool_UnlockStake">UnlockStake</a> &#123;<br />                pool_address,<br />                delegator_address,<br />                amount_unlocked: amount,<br />            &#125;,<br />        );<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> pool.unlock_stake_events,<br />        <a href="delegation_pool.md#0x1_delegation_pool_UnlockStakeEvent">UnlockStakeEvent</a> &#123;<br />            pool_address,<br />            delegator_address,<br />            amount_unlocked: amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_reactivate_stake">reactivate_stake</a>(delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_reactivate_stake">reactivate_stake</a>(<br />    delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    amount: u64<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> &#123;<br />    // short&#45;circuit <b>if</b> amount <b>to</b> reactivate is 0 so no <a href="event.md#0x1_event">event</a> is emitted<br />    <b>if</b> (amount &#61;&#61; 0) &#123; <b>return</b> &#125;;<br /><br />    <b>let</b> delegator_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegator_allowlisted">assert_delegator_allowlisted</a>(pool_address, delegator_address);<br /><br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br /><br />    <b>let</b> pool &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    amount &#61; <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(<br />        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool),<br />        &amp;pool.active_shares,<br />        delegator_address,<br />        amount,<br />    );<br />    <b>let</b> observed_lockup_cycle &#61; pool.observed_lockup_cycle;<br />    amount &#61; <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(pool, delegator_address, amount, observed_lockup_cycle);<br /><br />    <a href="stake.md#0x1_stake_reactivate_stake">stake::reactivate_stake</a>(&amp;<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool), amount);<br /><br />    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool, delegator_address, amount);<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool, delegator_address);<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStake">ReactivateStake</a> &#123;<br />                pool_address,<br />                delegator_address,<br />                amount_reactivated: amount,<br />            &#125;,<br />        );<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> pool.reactivate_stake_events,<br />        <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStakeEvent">ReactivateStakeEvent</a> &#123;<br />            pool_address,<br />            delegator_address,<br />            amount_reactivated: amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of owned inactive stake from the delegation pool at <code>pool_address</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw">withdraw</a>(delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw">withdraw</a>(<br />    delegator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    pool_address: <b>address</b>,<br />    amount: u64<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EWITHDRAW_ZERO_STAKE">EWITHDRAW_ZERO_STAKE</a>));<br />    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation<br />    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);<br />    <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(<b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator), amount);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_withdraw_internal"></a>

## Function `withdraw_internal`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(<br />    pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    delegator_address: <b>address</b>,<br />    amount: u64<br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    // TODO: recycle storage when a delegator fully exits the delegation pool.<br />    // short&#45;circuit <b>if</b> amount <b>to</b> withdraw is 0 so no <a href="event.md#0x1_event">event</a> is emitted<br />    <b>if</b> (amount &#61;&#61; 0) &#123; <b>return</b> &#125;;<br /><br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);<br />    <b>let</b> (withdrawal_exists, withdrawal_olc) &#61; <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool, delegator_address);<br />    // exit <b>if</b> no withdrawal or (it is pending and cannot withdraw pending_inactive <a href="stake.md#0x1_stake">stake</a> from <a href="stake.md#0x1_stake">stake</a> pool)<br />    <b>if</b> (!(<br />        withdrawal_exists &amp;&amp;<br />            (withdrawal_olc.index &lt; pool.observed_lockup_cycle.index &#124;&#124; <a href="delegation_pool.md#0x1_delegation_pool_can_withdraw_pending_inactive">can_withdraw_pending_inactive</a>(pool_address))<br />    )) &#123; <b>return</b> &#125;;<br /><br />    <b>if</b> (withdrawal_olc.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;<br />        amount &#61; <a href="delegation_pool.md#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake">coins_to_redeem_to_ensure_min_stake</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool),<br />            delegator_address,<br />            amount,<br />        )<br />    &#125;;<br />    amount &#61; <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(pool, delegator_address, amount, withdrawal_olc);<br /><br />    <b>let</b> stake_pool_owner &#61; &amp;<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool);<br />    // <a href="stake.md#0x1_stake">stake</a> pool will inactivate entire pending_inactive <a href="stake.md#0x1_stake">stake</a> at `<a href="stake.md#0x1_stake_withdraw">stake::withdraw</a>` <b>to</b> make it withdrawable<br />    // however, bypassing the inactivation of excess <a href="stake.md#0x1_stake">stake</a> (inactivated but not withdrawn) <b>ensures</b><br />    // the OLC is not advanced indefinitely on `unlock`&#45;`withdraw` paired calls<br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_can_withdraw_pending_inactive">can_withdraw_pending_inactive</a>(pool_address)) &#123;<br />        // get excess <a href="stake.md#0x1_stake">stake</a> before being entirely inactivated<br />        <b>let</b> (_, _, _, pending_inactive) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);<br />        <b>if</b> (withdrawal_olc.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;<br />            // `amount` less excess <b>if</b> withdrawing pending_inactive <a href="stake.md#0x1_stake">stake</a><br />            pending_inactive &#61; pending_inactive &#45; amount<br />        &#125;;<br />        // escape excess <a href="stake.md#0x1_stake">stake</a> from inactivation<br />        <a href="stake.md#0x1_stake_reactivate_stake">stake::reactivate_stake</a>(stake_pool_owner, pending_inactive);<br />        <a href="stake.md#0x1_stake_withdraw">stake::withdraw</a>(stake_pool_owner, amount);<br />        // restore excess <a href="stake.md#0x1_stake">stake</a> <b>to</b> the pending_inactive state<br />        <a href="stake.md#0x1_stake_unlock">stake::unlock</a>(stake_pool_owner, pending_inactive);<br />    &#125; <b>else</b> &#123;<br />        // no excess <a href="stake.md#0x1_stake">stake</a> <b>if</b> `<a href="stake.md#0x1_stake_withdraw">stake::withdraw</a>` does not inactivate at all<br />        <a href="stake.md#0x1_stake_withdraw">stake::withdraw</a>(stake_pool_owner, amount);<br />    &#125;;<br />    <a href="aptos_account.md#0x1_aptos_account_transfer">aptos_account::transfer</a>(stake_pool_owner, delegator_address, amount);<br /><br />    // commit withdrawal of possibly inactive <a href="stake.md#0x1_stake">stake</a> <b>to</b> the `total_coins_inactive`<br />    // known by the delegation pool in order <b>to</b> not mistake it for slashing at next synchronization<br />    <b>let</b> (_, inactive, _, _) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);<br />    pool.total_coins_inactive &#61; inactive;<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_module_event_migration_enabled">features::module_event_migration_enabled</a>()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStake">WithdrawStake</a> &#123;<br />                pool_address,<br />                delegator_address,<br />                amount_withdrawn: amount,<br />            &#125;,<br />        );<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> pool.withdraw_stake_events,<br />        <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStakeEvent">WithdrawStakeEvent</a> &#123;<br />            pool_address,<br />            delegator_address,<br />            amount_withdrawn: amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_pending_withdrawal_exists"></a>

## Function `pending_withdrawal_exists`

Return the unique observed lockup cycle where delegator <code>delegator_address</code> may have
unlocking (or already unlocked) stake to be withdrawn from delegation pool <code>pool</code>.
A bool is returned to signal if a pending withdrawal exists at all.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>): (bool, <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>): (bool, <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a>) &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;pool.pending_withdrawals, delegator_address)) &#123;<br />        (<b>true</b>, &#42;<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&amp;pool.pending_withdrawals, delegator_address))<br />    &#125; <b>else</b> &#123;<br />        (<b>false</b>, <a href="delegation_pool.md#0x1_delegation_pool_olc_with_index">olc_with_index</a>(0))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_pending_inactive_shares_pool_mut"></a>

## Function `pending_inactive_shares_pool_mut`

Return a mutable reference to the shares pool of <code>pending_inactive</code> stake on the
delegation pool, always the last item in <code>inactive_shares</code>.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): &amp;<b>mut</b> <a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): &amp;<b>mut</b> <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a> &#123;<br />    <b>let</b> observed_lockup_cycle &#61; pool.observed_lockup_cycle;<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> pool.inactive_shares, observed_lockup_cycle)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_pending_inactive_shares_pool"></a>

## Function `pending_inactive_shares_pool`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): &amp;<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): &amp;<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a> &#123;<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&amp;pool.inactive_shares, pool.observed_lockup_cycle)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_execute_pending_withdrawal"></a>

## Function `execute_pending_withdrawal`

Execute the pending withdrawal of <code>delegator_address</code> on delegation pool <code>pool</code>
if existing and already inactive to allow the creation of a new one.
<code>pending_inactive</code> stake would be left untouched even if withdrawable and should
be explicitly withdrawn by delegator


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_execute_pending_withdrawal">execute_pending_withdrawal</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_execute_pending_withdrawal">execute_pending_withdrawal</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    <b>let</b> (withdrawal_exists, withdrawal_olc) &#61; <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool, delegator_address);<br />    <b>if</b> (withdrawal_exists &amp;&amp; withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) &#123;<br />        <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(pool, delegator_address, <a href="delegation_pool.md#0x1_delegation_pool_MAX_U64">MAX_U64</a>);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_buy_in_active_shares"></a>

## Function `buy_in_active_shares`

Buy shares into the active pool on behalf of delegator <code>shareholder</code> who
deposited <code>coins_amount</code>. This function doesn&apos;t make any coin transfer.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, shareholder: <b>address</b>, coins_amount: u64): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(<br />    pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    shareholder: <b>address</b>,<br />    coins_amount: u64,<br />): u128 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    <b>let</b> new_shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_amount_to_shares">pool_u64::amount_to_shares</a>(&amp;pool.active_shares, coins_amount);<br />    // No need <b>to</b> buy 0 shares.<br />    <b>if</b> (new_shares &#61;&#61; 0) &#123; <b>return</b> 0 &#125;;<br /><br />    // Always <b>update</b> governance records before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> change <b>to</b> the shares pool.<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);<br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address)) &#123;<br />        <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_active_shares">update_governance_records_for_buy_in_active_shares</a>(pool, pool_address, new_shares, shareholder);<br />    &#125;;<br /><br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(&amp;<b>mut</b> pool.active_shares, shareholder, coins_amount);<br />    new_shares<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_buy_in_pending_inactive_shares"></a>

## Function `buy_in_pending_inactive_shares`

Buy shares into the pending_inactive pool on behalf of delegator <code>shareholder</code> who
redeemed <code>coins_amount</code> from the active pool to schedule it for unlocking.
If delegator&apos;s pending withdrawal exists and has been inactivated, execute it firstly
to ensure there is always only one withdrawal request.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, shareholder: <b>address</b>, coins_amount: u64): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(<br />    pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    shareholder: <b>address</b>,<br />    coins_amount: u64,<br />): u128 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    <b>let</b> new_shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_amount_to_shares">pool_u64::amount_to_shares</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool), coins_amount);<br />    // never create a new pending withdrawal unless delegator owns some pending_inactive shares<br />    <b>if</b> (new_shares &#61;&#61; 0) &#123; <b>return</b> 0 &#125;;<br /><br />    // Always <b>update</b> governance records before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> change <b>to</b> the shares pool.<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);<br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address)) &#123;<br />        <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares">update_governance_records_for_buy_in_pending_inactive_shares</a>(pool, pool_address, new_shares, shareholder);<br />    &#125;;<br /><br />    // cannot buy inactive shares, only pending_inactive at current lockup cycle<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool), shareholder, coins_amount);<br /><br />    // execute the pending withdrawal <b>if</b> <b>exists</b> and is inactive before creating a new one<br />    <a href="delegation_pool.md#0x1_delegation_pool_execute_pending_withdrawal">execute_pending_withdrawal</a>(pool, shareholder);<br /><br />    // save observed lockup cycle for the new pending withdrawal<br />    <b>let</b> observed_lockup_cycle &#61; pool.observed_lockup_cycle;<br />    <b>assert</b>!(&#42;<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut_with_default">table::borrow_mut_with_default</a>(<br />        &amp;<b>mut</b> pool.pending_withdrawals,<br />        shareholder,<br />        observed_lockup_cycle<br />    ) &#61;&#61; observed_lockup_cycle,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS">EPENDING_WITHDRAWAL_EXISTS</a>)<br />    );<br /><br />    new_shares<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_amount_to_shares_to_redeem"></a>

## Function `amount_to_shares_to_redeem`

Convert <code>coins_amount</code> of coins to be redeemed from shares pool <code>shares_pool</code>
to the exact number of shares to redeem in order to achieve this.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(shares_pool: &amp;<a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(<br />    shares_pool: &amp;<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>,<br />    shareholder: <b>address</b>,<br />    coins_amount: u64,<br />): u128 &#123;<br />    <b>if</b> (coins_amount &gt;&#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(shares_pool, shareholder)) &#123;<br />        // cap result at total shares of shareholder <b>to</b> pass `EINSUFFICIENT_SHARES` on subsequent redeem<br />        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(shares_pool, shareholder)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_amount_to_shares">pool_u64::amount_to_shares</a>(shares_pool, coins_amount)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_redeem_active_shares"></a>

## Function `redeem_active_shares`

Redeem shares from the active pool on behalf of delegator <code>shareholder</code> who
wants to unlock <code>coins_amount</code> of its active stake.
Extracted coins will be used to buy shares into the pending_inactive pool and
be available for withdrawal when current OLC ends.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_redeem_active_shares">redeem_active_shares</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, shareholder: <b>address</b>, coins_amount: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_redeem_active_shares">redeem_active_shares</a>(<br />    pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    shareholder: <b>address</b>,<br />    coins_amount: u64,<br />): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    <b>let</b> shares_to_redeem &#61; <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(&amp;pool.active_shares, shareholder, coins_amount);<br />    // silently exit <b>if</b> not a shareholder otherwise redeem would fail <b>with</b> `ESHAREHOLDER_NOT_FOUND`<br />    <b>if</b> (shares_to_redeem &#61;&#61; 0) <b>return</b> 0;<br /><br />    // Always <b>update</b> governance records before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> change <b>to</b> the shares pool.<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);<br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address)) &#123;<br />        <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_active_shares">update_governanace_records_for_redeem_active_shares</a>(pool, pool_address, shares_to_redeem, shareholder);<br />    &#125;;<br /><br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_redeem_shares">pool_u64::redeem_shares</a>(&amp;<b>mut</b> pool.active_shares, shareholder, shares_to_redeem)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_redeem_inactive_shares"></a>

## Function `redeem_inactive_shares`

Redeem shares from the inactive pool at <code>lockup_cycle</code> &lt; current OLC on behalf of
delegator <code>shareholder</code> who wants to withdraw <code>coins_amount</code> of its unlocked stake.
Redeem shares from the pending_inactive pool at <code>lockup_cycle</code> &#61;&#61; current OLC on behalf of
delegator <code>shareholder</code> who wants to reactivate <code>coins_amount</code> of its unlocking stake.
For latter case, extracted coins will be used to buy shares into the active pool and
escape inactivation when current lockup ends.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, shareholder: <b>address</b>, coins_amount: u64, lockup_cycle: <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">delegation_pool::ObservedLockupCycle</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(<br />    pool: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>,<br />    shareholder: <b>address</b>,<br />    coins_amount: u64,<br />    lockup_cycle: <a href="delegation_pool.md#0x1_delegation_pool_ObservedLockupCycle">ObservedLockupCycle</a>,<br />): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    <b>let</b> shares_to_redeem &#61; <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(<br />        <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&amp;pool.inactive_shares, lockup_cycle),<br />        shareholder,<br />        coins_amount);<br />    // silently exit <b>if</b> not a shareholder otherwise redeem would fail <b>with</b> `ESHAREHOLDER_NOT_FOUND`<br />    <b>if</b> (shares_to_redeem &#61;&#61; 0) <b>return</b> 0;<br /><br />    // Always <b>update</b> governance records before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> change <b>to</b> the shares pool.<br />    <b>let</b> pool_address &#61; <a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool);<br />    // Only redeem shares from the pending_inactive pool at `lockup_cycle` &#61;&#61; current OLC.<br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(pool_address) &amp;&amp; lockup_cycle.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;<br />        <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares">update_governanace_records_for_redeem_pending_inactive_shares</a>(<br />            pool,<br />            pool_address,<br />            shares_to_redeem,<br />            shareholder<br />        );<br />    &#125;;<br /><br />    <b>let</b> inactive_shares &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> pool.inactive_shares, lockup_cycle);<br />    // 1. reaching here means delegator owns inactive/pending_inactive shares at OLC `lockup_cycle`<br />    <b>let</b> redeemed_coins &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_redeem_shares">pool_u64::redeem_shares</a>(inactive_shares, shareholder, shares_to_redeem);<br /><br />    // <b>if</b> entirely reactivated pending_inactive <a href="stake.md#0x1_stake">stake</a> or withdrawn inactive one,<br />    // re&#45;enable unlocking for delegator by deleting this pending withdrawal<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(inactive_shares, shareholder) &#61;&#61; 0) &#123;<br />        // 2. a delegator owns inactive/pending_inactive shares only at the OLC of its pending withdrawal<br />        // 1 &amp; 2: the pending withdrawal itself <b>has</b> been emptied of shares and can be safely deleted<br />        <a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&amp;<b>mut</b> pool.pending_withdrawals, shareholder);<br />    &#125;;<br />    // destroy inactive shares pool of past OLC <b>if</b> all its <a href="stake.md#0x1_stake">stake</a> <b>has</b> been withdrawn<br />    <b>if</b> (lockup_cycle.index &lt; pool.observed_lockup_cycle.index &amp;&amp; total_coins(inactive_shares) &#61;&#61; 0) &#123;<br />        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_destroy_empty">pool_u64::destroy_empty</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&amp;<b>mut</b> pool.inactive_shares, lockup_cycle));<br />    &#125;;<br /><br />    redeemed_coins<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_stake_pool_drift"></a>

## Function `calculate_stake_pool_drift`

Calculate stake deviations between the delegation and stake pools in order to
capture the rewards earned in the meantime, resulted operator commission and
whether the lockup expired on the stake pool.


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>): (bool, u64, u64, u64, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>): (bool, u64, u64, u64, u64) &#123;<br />    <b>let</b> (active, inactive, pending_active, pending_inactive) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(<a href="delegation_pool.md#0x1_delegation_pool_get_pool_address">get_pool_address</a>(pool));<br />    <b>assert</b>!(<br />        inactive &gt;&#61; pool.total_coins_inactive,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC">ESLASHED_INACTIVE_STAKE_ON_PAST_OLC</a>)<br />    );<br />    // determine whether a new lockup cycle <b>has</b> been ended on the <a href="stake.md#0x1_stake">stake</a> pool and<br />    // inactivated SOME `pending_inactive` <a href="stake.md#0x1_stake">stake</a> which should stop earning rewards now,<br />    // thus requiring separation of the `pending_inactive` <a href="stake.md#0x1_stake">stake</a> on current observed lockup<br />    // and the future one on the newly started lockup<br />    <b>let</b> lockup_cycle_ended &#61; inactive &gt; pool.total_coins_inactive;<br /><br />    // actual coins on <a href="stake.md#0x1_stake">stake</a> pool belonging <b>to</b> the active shares pool<br />    active &#61; active &#43; pending_active;<br />    // actual coins on <a href="stake.md#0x1_stake">stake</a> pool belonging <b>to</b> the shares pool hosting `pending_inactive` <a href="stake.md#0x1_stake">stake</a><br />    // at current observed lockup cycle, either pending: `pending_inactive` or already inactivated:<br />    <b>if</b> (lockup_cycle_ended) &#123;<br />        // `inactive` on <a href="stake.md#0x1_stake">stake</a> pool &#61; <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> previous `inactive` <a href="stake.md#0x1_stake">stake</a> &#43;<br />        // <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> previous `pending_inactive` <a href="stake.md#0x1_stake">stake</a> and its rewards (both inactivated)<br />        pending_inactive &#61; inactive &#45; pool.total_coins_inactive<br />    &#125;;<br /><br />    // on <a href="stake.md#0x1_stake">stake</a>&#45;management operations, total coins on the <b>internal</b> shares pools and individual<br />    // stakes on the <a href="stake.md#0x1_stake">stake</a> pool are updated simultaneously, thus the only stakes becoming<br />    // unsynced are rewards and slashes routed exclusively <b>to</b>/out the <a href="stake.md#0x1_stake">stake</a> pool<br /><br />    // operator `active` rewards not persisted yet <b>to</b> the active shares pool<br />    <b>let</b> pool_active &#61; total_coins(&amp;pool.active_shares);<br />    <b>let</b> commission_active &#61; <b>if</b> (active &gt; pool_active) &#123;<br />        <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(active &#45; pool_active, pool.operator_commission_percentage, <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>)<br />    &#125; <b>else</b> &#123;<br />        // handle <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> slashing applied <b>to</b> `active` <a href="stake.md#0x1_stake">stake</a><br />        0<br />    &#125;;<br />    // operator `pending_inactive` rewards not persisted yet <b>to</b> the pending_inactive shares pool<br />    <b>let</b> pool_pending_inactive &#61; total_coins(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool));<br />    <b>let</b> commission_pending_inactive &#61; <b>if</b> (pending_inactive &gt; pool_pending_inactive) &#123;<br />        <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(<br />            pending_inactive &#45; pool_pending_inactive,<br />            pool.operator_commission_percentage,<br />            <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a><br />        )<br />    &#125; <b>else</b> &#123;<br />        // handle <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> slashing applied <b>to</b> `pending_inactive` <a href="stake.md#0x1_stake">stake</a><br />        0<br />    &#125;;<br /><br />    (lockup_cycle_ended, active, pending_inactive, commission_active, commission_pending_inactive)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_synchronize_delegation_pool"></a>

## Function `synchronize_delegation_pool`

Synchronize delegation and stake pools: distribute yet&#45;undetected rewards to the corresponding internal
shares pools, assign commission to operator and eventually prepare delegation pool for a new lockup cycle.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(<br />    pool_address: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, <a href="delegation_pool.md#0x1_delegation_pool_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a> &#123;<br />    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);<br />    <b>let</b> pool &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);<br />    <b>let</b> (<br />        lockup_cycle_ended,<br />        active,<br />        pending_inactive,<br />        commission_active,<br />        commission_pending_inactive<br />    ) &#61; <a href="delegation_pool.md#0x1_delegation_pool_calculate_stake_pool_drift">calculate_stake_pool_drift</a>(pool);<br /><br />    // zero `pending_active` <a href="stake.md#0x1_stake">stake</a> indicates that either there are no `add_stake` fees or<br />    // previous epoch <b>has</b> ended and should release the shares owning the existing fees<br />    <b>let</b> (_, _, pending_active, _) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);<br />    <b>if</b> (pending_active &#61;&#61; 0) &#123;<br />        // renounce ownership over the `add_stake` fees by redeeming all shares of<br />        // the special shareholder, implicitly their equivalent coins, out of the active shares pool<br />        <a href="delegation_pool.md#0x1_delegation_pool_redeem_active_shares">redeem_active_shares</a>(pool, <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>, <a href="delegation_pool.md#0x1_delegation_pool_MAX_U64">MAX_U64</a>);<br />    &#125;;<br /><br />    // distribute rewards remaining after commission, <b>to</b> delegators (<b>to</b> already existing shares)<br />    // before buying shares for the operator for its entire commission fee<br />    // otherwise, operator&apos;s new shares would additionally appreciate from rewards it does not own<br /><br />    // <b>update</b> total coins accumulated by `active` &#43; `pending_active` shares<br />    // redeemed `add_stake` fees are restored and distributed <b>to</b> the rest of the pool <b>as</b> rewards<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_update_total_coins">pool_u64::update_total_coins</a>(&amp;<b>mut</b> pool.active_shares, active &#45; commission_active);<br />    // <b>update</b> total coins accumulated by `pending_inactive` shares at current observed lockup cycle<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_update_total_coins">pool_u64::update_total_coins</a>(<br />        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool),<br />        pending_inactive &#45; commission_pending_inactive<br />    );<br /><br />    // reward operator its commission out of uncommitted active rewards (`add_stake` fees already excluded)<br />    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_active_shares">buy_in_active_shares</a>(pool, <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(<a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address)), commission_active);<br />    // reward operator its commission out of uncommitted pending_inactive rewards<br />    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(<br />        pool,<br />        <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(<a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address)),<br />        commission_pending_inactive<br />    );<br /><br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> pool.distribute_commission_events,<br />        <a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">DistributeCommissionEvent</a> &#123;<br />            pool_address,<br />            operator: <a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address),<br />            commission_active,<br />            commission_pending_inactive,<br />        &#125;,<br />    );<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operator_beneficiary_change_enabled">features::operator_beneficiary_change_enabled</a>()) &#123;<br />        emit(<a href="delegation_pool.md#0x1_delegation_pool_DistributeCommission">DistributeCommission</a> &#123;<br />            pool_address,<br />            operator: <a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address),<br />            beneficiary: <a href="delegation_pool.md#0x1_delegation_pool_beneficiary_for_operator">beneficiary_for_operator</a>(<a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address)),<br />            commission_active,<br />            commission_pending_inactive,<br />        &#125;)<br />    &#125;;<br /><br />    // advance lockup cycle on delegation pool <b>if</b> already ended on <a href="stake.md#0x1_stake">stake</a> pool (AND <a href="stake.md#0x1_stake">stake</a> explicitly inactivated)<br />    <b>if</b> (lockup_cycle_ended) &#123;<br />        // capture inactive coins over all ended lockup cycles (including this ending one)<br />        <b>let</b> (_, inactive, _, _) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);<br />        pool.total_coins_inactive &#61; inactive;<br /><br />        // advance lockup cycle on the delegation pool<br />        pool.observed_lockup_cycle.index &#61; pool.observed_lockup_cycle.index &#43; 1;<br />        // start new lockup cycle <b>with</b> a fresh shares pool for `pending_inactive` <a href="stake.md#0x1_stake">stake</a><br />        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(<br />            &amp;<b>mut</b> pool.inactive_shares,<br />            pool.observed_lockup_cycle,<br />            <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create_with_scaling_factor">pool_u64::create_with_scaling_factor</a>(<a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>)<br />        );<br />    &#125;;<br /><br />    <b>if</b> (<a href="delegation_pool.md#0x1_delegation_pool_is_next_commission_percentage_effective">is_next_commission_percentage_effective</a>(pool_address)) &#123;<br />        pool.operator_commission_percentage &#61; <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_NextCommissionPercentage">NextCommissionPercentage</a>&gt;(<br />            pool_address<br />        ).commission_percentage_next_lockup_cycle;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_assert_and_update_proposal_used_voting_power"></a>

## Function `assert_and_update_proposal_used_voting_power`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_and_update_proposal_used_voting_power">assert_and_update_proposal_used_voting_power</a>(governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">delegation_pool::GovernanceRecords</a>, pool_address: <b>address</b>, proposal_id: u64, voting_power: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_assert_and_update_proposal_used_voting_power">assert_and_update_proposal_used_voting_power</a>(<br />    governance_records: &amp;<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>, pool_address: <b>address</b>, proposal_id: u64, voting_power: u64<br />) &#123;<br />    <b>let</b> stake_pool_remaining_voting_power &#61; <a href="aptos_governance.md#0x1_aptos_governance_get_remaining_voting_power">aptos_governance::get_remaining_voting_power</a>(pool_address, proposal_id);<br />    <b>let</b> stake_pool_used_voting_power &#61; <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">aptos_governance::get_voting_power</a>(<br />        pool_address<br />    ) &#45; stake_pool_remaining_voting_power;<br />    <b>let</b> proposal_used_voting_power &#61; <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(<br />        &amp;<b>mut</b> governance_records.votes_per_proposal,<br />        proposal_id,<br />        0<br />    );<br />    // A edge case: Before enabling partial governance <a href="voting.md#0x1_voting">voting</a> on a delegation pool, the delegation pool <b>has</b><br />    // a voter which can vote <b>with</b> all <a href="voting.md#0x1_voting">voting</a> power of this delegation pool. If the voter votes on a proposal after<br />    // partial governance <a href="voting.md#0x1_voting">voting</a> flag is enabled, the delegation pool doesn&apos;t have enough <a href="voting.md#0x1_voting">voting</a> power on this<br />    // proposal for all the delegators. To be fair, no one can vote on this proposal through this delegation pool.<br />    // To detect this case, check <b>if</b> the <a href="stake.md#0x1_stake">stake</a> pool had used <a href="voting.md#0x1_voting">voting</a> power not through <a href="delegation_pool.md#0x1_delegation_pool">delegation_pool</a> <b>module</b>.<br />    <b>assert</b>!(<br />        stake_pool_used_voting_power &#61;&#61; &#42;proposal_used_voting_power,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING">EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING</a>)<br />    );<br />    &#42;proposal_used_voting_power &#61; &#42;proposal_used_voting_power &#43; voting_power;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_update_governance_records_for_buy_in_active_shares"></a>

## Function `update_governance_records_for_buy_in_active_shares`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_active_shares">update_governance_records_for_buy_in_active_shares</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, pool_address: <b>address</b>, new_shares: u128, shareholder: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_active_shares">update_governance_records_for_buy_in_active_shares</a>(<br />    pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, pool_address: <b>address</b>, new_shares: u128, shareholder: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    // &lt;active shares&gt; of &lt;shareholder&gt; &#43;&#61; &lt;new_shares&gt; &#45;&#45;&#45;&#45;&gt;<br />    // &lt;active shares&gt; of &lt;current voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;<br />    // &lt;active shares&gt; of &lt;next voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;<br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    <b>let</b> vote_delegation &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(pool, governance_records, shareholder);<br />    <b>let</b> current_voter &#61; vote_delegation.voter;<br />    <b>let</b> pending_voter &#61; vote_delegation.pending_voter;<br />    <b>let</b> current_delegated_votes &#61;<br />        <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, current_voter);<br />    current_delegated_votes.active_shares &#61; current_delegated_votes.active_shares &#43; new_shares;<br />    <b>if</b> (pending_voter &#61;&#61; current_voter) &#123;<br />        current_delegated_votes.active_shares_next_lockup &#61;<br />            current_delegated_votes.active_shares_next_lockup &#43; new_shares;<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> pending_delegated_votes &#61;<br />            <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, pending_voter);<br />        pending_delegated_votes.active_shares_next_lockup &#61;<br />            pending_delegated_votes.active_shares_next_lockup &#43; new_shares;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares"></a>

## Function `update_governance_records_for_buy_in_pending_inactive_shares`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares">update_governance_records_for_buy_in_pending_inactive_shares</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, pool_address: <b>address</b>, new_shares: u128, shareholder: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares">update_governance_records_for_buy_in_pending_inactive_shares</a>(<br />    pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, pool_address: <b>address</b>, new_shares: u128, shareholder: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    // &lt;pending inactive shares&gt; of &lt;shareholder&gt; &#43;&#61; &lt;new_shares&gt;   &#45;&#45;&#45;&#45;&gt;<br />    // &lt;pending inactive shares&gt; of &lt;current voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;<br />    // no impact on &lt;pending inactive shares&gt; of &lt;next voter of shareholder&gt;<br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    <b>let</b> current_voter &#61; <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(pool, governance_records, shareholder);<br />    <b>let</b> current_delegated_votes &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, current_voter);<br />    current_delegated_votes.pending_inactive_shares &#61; current_delegated_votes.pending_inactive_shares &#43; new_shares;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_update_governanace_records_for_redeem_active_shares"></a>

## Function `update_governanace_records_for_redeem_active_shares`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_active_shares">update_governanace_records_for_redeem_active_shares</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, pool_address: <b>address</b>, shares_to_redeem: u128, shareholder: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_active_shares">update_governanace_records_for_redeem_active_shares</a>(<br />    pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, pool_address: <b>address</b>, shares_to_redeem: u128, shareholder: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    // &lt;active shares&gt; of &lt;shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt; &#45;&#45;&#45;&#45;&gt;<br />    // &lt;active shares&gt; of &lt;current voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;<br />    // &lt;active shares&gt; of &lt;next voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;<br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    <b>let</b> vote_delegation &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation">update_and_borrow_mut_delegator_vote_delegation</a>(<br />        pool,<br />        governance_records,<br />        shareholder<br />    );<br />    <b>let</b> current_voter &#61; vote_delegation.voter;<br />    <b>let</b> pending_voter &#61; vote_delegation.pending_voter;<br />    <b>let</b> current_delegated_votes &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, current_voter);<br />    current_delegated_votes.active_shares &#61; current_delegated_votes.active_shares &#45; shares_to_redeem;<br />    <b>if</b> (current_voter &#61;&#61; pending_voter) &#123;<br />        current_delegated_votes.active_shares_next_lockup &#61;<br />            current_delegated_votes.active_shares_next_lockup &#45; shares_to_redeem;<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> pending_delegated_votes &#61;<br />            <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, pending_voter);<br />        pending_delegated_votes.active_shares_next_lockup &#61;<br />            pending_delegated_votes.active_shares_next_lockup &#45; shares_to_redeem;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares"></a>

## Function `update_governanace_records_for_redeem_pending_inactive_shares`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares">update_governanace_records_for_redeem_pending_inactive_shares</a>(pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, pool_address: <b>address</b>, shares_to_redeem: u128, shareholder: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares">update_governanace_records_for_redeem_pending_inactive_shares</a>(<br />    pool: &amp;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, pool_address: <b>address</b>, shares_to_redeem: u128, shareholder: <b>address</b><br />) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a> &#123;<br />    // &lt;pending inactive shares&gt; of &lt;shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;  &#45;&#45;&#45;&#45;&gt;<br />    // &lt;pending inactive shares&gt; of &lt;current voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;<br />    // no impact on &lt;pending inactive shares&gt; of &lt;next voter of shareholder&gt;<br />    <b>let</b> governance_records &#61; <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_GovernanceRecords">GovernanceRecords</a>&gt;(pool_address);<br />    <b>let</b> current_voter &#61; <a href="delegation_pool.md#0x1_delegation_pool_calculate_and_update_delegator_voter_internal">calculate_and_update_delegator_voter_internal</a>(pool, governance_records, shareholder);<br />    <b>let</b> current_delegated_votes &#61; <a href="delegation_pool.md#0x1_delegation_pool_update_and_borrow_mut_delegated_votes">update_and_borrow_mut_delegated_votes</a>(pool, governance_records, current_voter);<br />    current_delegated_votes.pending_inactive_shares &#61; current_delegated_votes.pending_inactive_shares &#45; shares_to_redeem;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_delegation_pool_multiply_then_divide"></a>

## Function `multiply_then_divide`

Deprecated, prefer math64::mul_div


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_multiply_then_divide">multiply_then_divide</a>(x: u64, y: u64, z: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_multiply_then_divide">multiply_then_divide</a>(x: u64, y: u64, z: u64): u64 &#123;<br />    <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(x, y, z)<br />&#125;<br /></code></pre>



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
<td>Upon calling the initialize_delegation_pool function, a resource account is created from the &quot;owner&quot; signer to host the delegation pool resource and own the underlying stake pool.</td>
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
<td>Audited that either inactive or pending_inactive stake after invoking the get_stake function is zero and both are never non&#45;zero.</td>
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
<td>In the get_pending_withdrawal function, if withdrawal_exists is true, the function returns true and a non&#45;zero amount</td>
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
<td>The delegator&apos;s active or pending inactive stake will always meet or exceed the minimum allowed value.</td>
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


<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
