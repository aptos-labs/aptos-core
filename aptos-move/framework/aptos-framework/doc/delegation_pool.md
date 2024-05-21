
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


<pre><code>use 0x1::account;
use 0x1::aptos_account;
use 0x1::aptos_coin;
use 0x1::aptos_governance;
use 0x1::coin;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::pool_u64_unbound;
use 0x1::signer;
use 0x1::smart_table;
use 0x1::stake;
use 0x1::staking_config;
use 0x1::table;
use 0x1::table_with_length;
use 0x1::timestamp;
use 0x1::vector;
</code></pre>



<a id="0x1_delegation_pool_DelegationPoolOwnership"></a>

## Resource `DelegationPoolOwnership`

Capability that represents ownership over privileged operations on the underlying stake pool.


<pre><code>struct DelegationPoolOwnership has store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: address</code>
</dt>
<dd>
 equal to address of the resource account owning the stake pool
</dd>
</dl>


</details>

<a id="0x1_delegation_pool_ObservedLockupCycle"></a>

## Struct `ObservedLockupCycle`



<pre><code>struct ObservedLockupCycle has copy, drop, store
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



<pre><code>struct DelegationPool has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active_shares: pool_u64_unbound::Pool</code>
</dt>
<dd>

</dd>
<dt>
<code>observed_lockup_cycle: delegation_pool::ObservedLockupCycle</code>
</dt>
<dd>

</dd>
<dt>
<code>inactive_shares: table::Table&lt;delegation_pool::ObservedLockupCycle, pool_u64_unbound::Pool&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_withdrawals: table::Table&lt;address, delegation_pool::ObservedLockupCycle&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool_signer_cap: account::SignerCapability</code>
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
<code>add_stake_events: event::EventHandle&lt;delegation_pool::AddStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reactivate_stake_events: event::EventHandle&lt;delegation_pool::ReactivateStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_stake_events: event::EventHandle&lt;delegation_pool::UnlockStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_stake_events: event::EventHandle&lt;delegation_pool::WithdrawStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>distribute_commission_events: event::EventHandle&lt;delegation_pool::DistributeCommissionEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_VotingRecordKey"></a>

## Struct `VotingRecordKey`



<pre><code>struct VotingRecordKey has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: address</code>
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


<pre><code>struct VoteDelegation has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_voter: address</code>
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


<pre><code>struct DelegatedVotes has copy, drop, store
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


<pre><code>struct GovernanceRecords has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>votes: smart_table::SmartTable&lt;delegation_pool::VotingRecordKey, u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>votes_per_proposal: smart_table::SmartTable&lt;u64, u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_delegation: smart_table::SmartTable&lt;address, delegation_pool::VoteDelegation&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delegated_votes: smart_table::SmartTable&lt;address, delegation_pool::DelegatedVotes&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: event::EventHandle&lt;delegation_pool::VoteEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_proposal_events: event::EventHandle&lt;delegation_pool::CreateProposalEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delegate_voting_power_events: event::EventHandle&lt;delegation_pool::DelegateVotingPowerEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_BeneficiaryForOperator"></a>

## Resource `BeneficiaryForOperator`



<pre><code>struct BeneficiaryForOperator has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>beneficiary_for_operator: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_NextCommissionPercentage"></a>

## Resource `NextCommissionPercentage`



<pre><code>struct NextCommissionPercentage has key
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


<pre><code>struct DelegationPoolAllowlisting has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allowlist: smart_table::SmartTable&lt;address, bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_AddStake"></a>

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
<code>delegator_address: address</code>
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
<code>delegator_address: address</code>
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
<code>delegator_address: address</code>
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
<code>delegator_address: address</code>
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
<code>delegator_address: address</code>
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
<code>delegator_address: address</code>
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
<code>delegator_address: address</code>
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
<code>delegator_address: address</code>
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



<pre><code>&#35;[event]
struct DistributeCommissionEvent has drop, store
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
<code>operator: address</code>
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



<pre><code>&#35;[event]
struct DistributeCommission has drop, store
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
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>beneficiary: address</code>
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



<pre><code>&#35;[event]
struct Vote has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>delegation_pool: address</code>
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



<pre><code>struct VoteEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>delegation_pool: address</code>
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



<pre><code>&#35;[event]
struct CreateProposal has drop, store
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
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>delegation_pool: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_CreateProposalEvent"></a>

## Struct `CreateProposalEvent`



<pre><code>struct CreateProposalEvent has drop, store
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
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>delegation_pool: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DelegateVotingPower"></a>

## Struct `DelegateVotingPower`



<pre><code>&#35;[event]
struct DelegateVotingPower has drop, store
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
<code>delegator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_DelegateVotingPowerEvent"></a>

## Struct `DelegateVotingPowerEvent`



<pre><code>struct DelegateVotingPowerEvent has drop, store
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
<code>delegator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_SetBeneficiaryForOperator"></a>

## Struct `SetBeneficiaryForOperator`



<pre><code>&#35;[event]
struct SetBeneficiaryForOperator has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_beneficiary: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_beneficiary: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_CommissionPercentageChange"></a>

## Struct `CommissionPercentageChange`



<pre><code>&#35;[event]
struct CommissionPercentageChange has drop, store
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
<code>owner: address</code>
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



<pre><code>&#35;[event]
struct EnableDelegatorsAllowlisting has drop, store
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

<a id="0x1_delegation_pool_DisableDelegatorsAllowlisting"></a>

## Struct `DisableDelegatorsAllowlisting`



<pre><code>&#35;[event]
struct DisableDelegatorsAllowlisting has drop, store
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

<a id="0x1_delegation_pool_AllowlistDelegator"></a>

## Struct `AllowlistDelegator`



<pre><code>&#35;[event]
struct AllowlistDelegator has drop, store
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
<code>delegator_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_RemoveDelegatorFromAllowlist"></a>

## Struct `RemoveDelegatorFromAllowlist`



<pre><code>&#35;[event]
struct RemoveDelegatorFromAllowlist has drop, store
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
<code>delegator_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_EvictDelegator"></a>

## Struct `EvictDelegator`



<pre><code>&#35;[event]
struct EvictDelegator has drop, store
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
<code>delegator_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_delegation_pool_MAX_U64"></a>



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_delegation_pool_EDEPRECATED_FUNCTION"></a>

Function is deprecated.


<pre><code>const EDEPRECATED_FUNCTION: u64 &#61; 12;
</code></pre>



<a id="0x1_delegation_pool_EDISABLED_FUNCTION"></a>

The function is disabled or hasn't been enabled.


<pre><code>const EDISABLED_FUNCTION: u64 &#61; 13;
</code></pre>



<a id="0x1_delegation_pool_ENOT_OPERATOR"></a>

The account is not the operator of the stake pool.


<pre><code>const ENOT_OPERATOR: u64 &#61; 18;
</code></pre>



<a id="0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS"></a>

Account is already owning a delegation pool.


<pre><code>const EOWNER_CAP_ALREADY_EXISTS: u64 &#61; 2;
</code></pre>



<a id="0x1_delegation_pool_EOWNER_CAP_NOT_FOUND"></a>

Delegation pool owner capability does not exist at the provided account.


<pre><code>const EOWNER_CAP_NOT_FOUND: u64 &#61; 1;
</code></pre>



<a id="0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code>const VALIDATOR_STATUS_INACTIVE: u64 &#61; 4;
</code></pre>



<a id="0x1_delegation_pool_EINSUFFICIENT_PROPOSER_STAKE"></a>

The voter does not have sufficient stake to create a proposal.


<pre><code>const EINSUFFICIENT_PROPOSER_STAKE: u64 &#61; 15;
</code></pre>



<a id="0x1_delegation_pool_ENO_VOTING_POWER"></a>

The voter does not have any voting power on this proposal.


<pre><code>const ENO_VOTING_POWER: u64 &#61; 16;
</code></pre>



<a id="0x1_delegation_pool_EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING"></a>

The stake pool has already voted on the proposal before enabling partial governance voting on this delegation pool.


<pre><code>const EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING: u64 &#61; 17;
</code></pre>



<a id="0x1_delegation_pool_ECANNOT_EVICT_ALLOWLISTED_DELEGATOR"></a>

Cannot evict an allowlisted delegator, should remove them from the allowlist first.


<pre><code>const ECANNOT_EVICT_ALLOWLISTED_DELEGATOR: u64 &#61; 26;
</code></pre>



<a id="0x1_delegation_pool_ECANNOT_UNLOCK_NULL_SHAREHOLDER"></a>

Cannot unlock the accumulated active stake of NULL_SHAREHOLDER(0x0).


<pre><code>const ECANNOT_UNLOCK_NULL_SHAREHOLDER: u64 &#61; 27;
</code></pre>



<a id="0x1_delegation_pool_ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED"></a>

Changing operator commission rate in delegation pool is not supported.


<pre><code>const ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED: u64 &#61; 22;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATION_POOLS_DISABLED"></a>

Creating delegation pools is not enabled yet.


<pre><code>const EDELEGATION_POOLS_DISABLED: u64 &#61; 10;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST"></a>

Delegation pool does not exist at the provided pool address.


<pre><code>const EDELEGATION_POOL_DOES_NOT_EXIST: u64 &#61; 3;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_ENABLED"></a>

Delegators allowlisting should be enabled to perform this operation.


<pre><code>const EDELEGATORS_ALLOWLISTING_NOT_ENABLED: u64 &#61; 24;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED"></a>

Delegators allowlisting is not supported.


<pre><code>const EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED: u64 &#61; 23;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW"></a>

Delegator's active balance cannot be less than <code>MIN_COINS_ON_SHARES_POOL</code>.


<pre><code>const EDELEGATOR_ACTIVE_BALANCE_TOO_LOW: u64 &#61; 8;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_NOT_ALLOWLISTED"></a>

Cannot add/reactivate stake unless being allowlisted by the pool owner.


<pre><code>const EDELEGATOR_NOT_ALLOWLISTED: u64 &#61; 25;
</code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW"></a>

Delegator's pending_inactive balance cannot be less than <code>MIN_COINS_ON_SHARES_POOL</code>.


<pre><code>const EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW: u64 &#61; 9;
</code></pre>



<a id="0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE"></a>

Commission percentage has to be between 0 and <code>MAX_FEE</code> - 100%.


<pre><code>const EINVALID_COMMISSION_PERCENTAGE: u64 &#61; 5;
</code></pre>



<a id="0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK"></a>

There is not enough <code>active</code> stake on the stake pool to <code>unlock</code>.


<pre><code>const ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK: u64 &#61; 6;
</code></pre>



<a id="0x1_delegation_pool_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED"></a>

Changing beneficiaries for operators is not supported.


<pre><code>const EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED: u64 &#61; 19;
</code></pre>



<a id="0x1_delegation_pool_EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED"></a>

Partial governance voting hasn't been enabled on this delegation pool.


<pre><code>const EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED: u64 &#61; 14;
</code></pre>



<a id="0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS"></a>

There is a pending withdrawal to be executed before <code>unlock</code>ing any new stake.


<pre><code>const EPENDING_WITHDRAWAL_EXISTS: u64 &#61; 4;
</code></pre>



<a id="0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC"></a>

Slashing (if implemented) should not be applied to already <code>inactive</code> stake.
Not only it invalidates the accounting of past observed lockup cycles (OLC),
but is also unfair to delegators whose stake has been inactive before validator started misbehaving.
Additionally, the inactive stake does not count on the voting power of validator.


<pre><code>const ESLASHED_INACTIVE_STAKE_ON_PAST_OLC: u64 &#61; 7;
</code></pre>



<a id="0x1_delegation_pool_ETOO_LARGE_COMMISSION_INCREASE"></a>

Commission percentage increase is too large.


<pre><code>const ETOO_LARGE_COMMISSION_INCREASE: u64 &#61; 20;
</code></pre>



<a id="0x1_delegation_pool_ETOO_LATE_COMMISSION_CHANGE"></a>

Commission percentage change is too late in this lockup period, and should be done at least a quarter (1/4) of the lockup duration before the lockup cycle ends.


<pre><code>const ETOO_LATE_COMMISSION_CHANGE: u64 &#61; 21;
</code></pre>



<a id="0x1_delegation_pool_EWITHDRAW_ZERO_STAKE"></a>

Cannot request to withdraw zero stake.


<pre><code>const EWITHDRAW_ZERO_STAKE: u64 &#61; 11;
</code></pre>



<a id="0x1_delegation_pool_MAX_COMMISSION_INCREASE"></a>

Maximum commission percentage increase per lockup cycle. 10% is represented as 1000.


<pre><code>const MAX_COMMISSION_INCREASE: u64 &#61; 1000;
</code></pre>



<a id="0x1_delegation_pool_MAX_FEE"></a>

Maximum operator percentage fee(of double digit precision): 22.85% is represented as 2285


<pre><code>const MAX_FEE: u64 &#61; 10000;
</code></pre>



<a id="0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL"></a>

Minimum coins to exist on a shares pool at all times.
Enforced per delegator for both active and pending_inactive pools.
This constraint ensures the share price cannot overly increase and lead to
substantial losses when buying shares (can lose at most 1 share which may
be worth a lot if current share price is high).
This constraint is not enforced on inactive pools as they only allow redeems
(can lose at most 1 coin regardless of current share price).


<pre><code>const MIN_COINS_ON_SHARES_POOL: u64 &#61; 1000000000;
</code></pre>



<a id="0x1_delegation_pool_MODULE_SALT"></a>



<pre><code>const MODULE_SALT: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 100, 101, 108, 101, 103, 97, 116, 105, 111, 110, 95, 112, 111, 111, 108];
</code></pre>



<a id="0x1_delegation_pool_NULL_SHAREHOLDER"></a>

Special shareholder temporarily owning the <code>add_stake</code> fees charged during this epoch.
On each <code>add_stake</code> operation any resulted fee is used to buy active shares for this shareholder.
First synchronization after this epoch ends will distribute accumulated fees to the rest of the pool as refunds.


<pre><code>const NULL_SHAREHOLDER: address &#61; 0x0;
</code></pre>



<a id="0x1_delegation_pool_SHARES_SCALING_FACTOR"></a>

Scaling factor of shares pools used within the delegation pool


<pre><code>const SHARES_SCALING_FACTOR: u64 &#61; 10000000000000000;
</code></pre>



<a id="0x1_delegation_pool_owner_cap_exists"></a>

## Function `owner_cap_exists`

Return whether supplied address <code>addr</code> is owner of a delegation pool.


<pre><code>&#35;[view]
public fun owner_cap_exists(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun owner_cap_exists(addr: address): bool &#123;
    exists&lt;DelegationPoolOwnership&gt;(addr)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_owned_pool_address"></a>

## Function `get_owned_pool_address`

Return address of the delegation pool owned by <code>owner</code> or fail if there is none.


<pre><code>&#35;[view]
public fun get_owned_pool_address(owner: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_owned_pool_address(owner: address): address acquires DelegationPoolOwnership &#123;
    assert_owner_cap_exists(owner);
    borrow_global&lt;DelegationPoolOwnership&gt;(owner).pool_address
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_delegation_pool_exists"></a>

## Function `delegation_pool_exists`

Return whether a delegation pool exists at supplied address <code>addr</code>.


<pre><code>&#35;[view]
public fun delegation_pool_exists(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegation_pool_exists(addr: address): bool &#123;
    exists&lt;DelegationPool&gt;(addr)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_partial_governance_voting_enabled"></a>

## Function `partial_governance_voting_enabled`

Return whether a delegation pool has already enabled partial governance voting.


<pre><code>&#35;[view]
public fun partial_governance_voting_enabled(pool_address: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun partial_governance_voting_enabled(pool_address: address): bool &#123;
    exists&lt;GovernanceRecords&gt;(pool_address) &amp;&amp; stake::get_delegated_voter(pool_address) &#61;&#61; pool_address
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_observed_lockup_cycle"></a>

## Function `observed_lockup_cycle`

Return the index of current observed lockup cycle on delegation pool <code>pool_address</code>.


<pre><code>&#35;[view]
public fun observed_lockup_cycle(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun observed_lockup_cycle(pool_address: address): u64 acquires DelegationPool &#123;
    assert_delegation_pool_exists(pool_address);
    borrow_global&lt;DelegationPool&gt;(pool_address).observed_lockup_cycle.index
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_is_next_commission_percentage_effective"></a>

## Function `is_next_commission_percentage_effective`

Return whether the commission percentage for the next lockup cycle is effective.


<pre><code>&#35;[view]
public fun is_next_commission_percentage_effective(pool_address: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_next_commission_percentage_effective(pool_address: address): bool acquires NextCommissionPercentage &#123;
    exists&lt;NextCommissionPercentage&gt;(pool_address) &amp;&amp;
        timestamp::now_seconds() &gt;&#61; borrow_global&lt;NextCommissionPercentage&gt;(pool_address).effective_after_secs
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the operator commission percentage set on the delegation pool <code>pool_address</code>.


<pre><code>&#35;[view]
public fun operator_commission_percentage(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator_commission_percentage(
    pool_address: address
): u64 acquires DelegationPool, NextCommissionPercentage &#123;
    assert_delegation_pool_exists(pool_address);
    if (is_next_commission_percentage_effective(pool_address)) &#123;
        operator_commission_percentage_next_lockup_cycle(pool_address)
    &#125; else &#123;
        borrow_global&lt;DelegationPool&gt;(pool_address).operator_commission_percentage
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle"></a>

## Function `operator_commission_percentage_next_lockup_cycle`

Return the operator commission percentage for the next lockup cycle.


<pre><code>&#35;[view]
public fun operator_commission_percentage_next_lockup_cycle(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator_commission_percentage_next_lockup_cycle(
    pool_address: address
): u64 acquires DelegationPool, NextCommissionPercentage &#123;
    assert_delegation_pool_exists(pool_address);
    if (exists&lt;NextCommissionPercentage&gt;(pool_address)) &#123;
        borrow_global&lt;NextCommissionPercentage&gt;(pool_address).commission_percentage_next_lockup_cycle
    &#125; else &#123;
        borrow_global&lt;DelegationPool&gt;(pool_address).operator_commission_percentage
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_shareholders_count_active_pool"></a>

## Function `shareholders_count_active_pool`

Return the number of delegators owning active stake within <code>pool_address</code>.


<pre><code>&#35;[view]
public fun shareholders_count_active_pool(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shareholders_count_active_pool(pool_address: address): u64 acquires DelegationPool &#123;
    assert_delegation_pool_exists(pool_address);
    pool_u64::shareholders_count(&amp;borrow_global&lt;DelegationPool&gt;(pool_address).active_shares)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_delegation_pool_stake"></a>

## Function `get_delegation_pool_stake`

Return the stake amounts on <code>pool_address</code> in the different states:
(<code>active</code>,<code>inactive</code>,<code>pending_active</code>,<code>pending_inactive</code>)


<pre><code>&#35;[view]
public fun get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64) &#123;
    assert_delegation_pool_exists(pool_address);
    stake::get_stake(pool_address)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_pending_withdrawal"></a>

## Function `get_pending_withdrawal`

Return whether the given delegator has any withdrawable stake. If they recently requested to unlock
some stake and the stake pool's lockup cycle has not ended, their coins are not withdrawable yet.


<pre><code>&#35;[view]
public fun get_pending_withdrawal(pool_address: address, delegator_address: address): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_pending_withdrawal(
    pool_address: address,
    delegator_address: address
): (bool, u64) acquires DelegationPool &#123;
    assert_delegation_pool_exists(pool_address);
    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);
    let (
        lockup_cycle_ended,
        _,
        pending_inactive,
        _,
        commission_pending_inactive
    ) &#61; calculate_stake_pool_drift(pool);

    let (withdrawal_exists, withdrawal_olc) &#61; pending_withdrawal_exists(pool, delegator_address);
    if (!withdrawal_exists) &#123;
        // if no pending withdrawal, there is neither inactive nor pending_inactive stake
        (false, 0)
    &#125; else &#123;
        // delegator has either inactive or pending_inactive stake due to automatic withdrawals
        let inactive_shares &#61; table::borrow(&amp;pool.inactive_shares, withdrawal_olc);
        if (withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) &#123;
            // if withdrawal&apos;s lockup cycle ended on delegation pool then it is inactive
            (true, pool_u64::balance(inactive_shares, delegator_address))
        &#125; else &#123;
            pending_inactive &#61; pool_u64::shares_to_amount_with_total_coins(
                inactive_shares,
                pool_u64::shares(inactive_shares, delegator_address),
                // exclude operator pending_inactive rewards not converted to shares yet
                pending_inactive &#45; commission_pending_inactive
            );
            // if withdrawal&apos;s lockup cycle ended ONLY on stake pool then it is also inactive
            (lockup_cycle_ended, pending_inactive)
        &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_stake"></a>

## Function `get_stake`

Return total stake owned by <code>delegator_address</code> within delegation pool <code>pool_address</code>
in each of its individual states: (<code>active</code>,<code>inactive</code>,<code>pending_inactive</code>)


<pre><code>&#35;[view]
public fun get_stake(pool_address: address, delegator_address: address): (u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_stake(
    pool_address: address,
    delegator_address: address
): (u64, u64, u64) acquires DelegationPool, BeneficiaryForOperator &#123;
    assert_delegation_pool_exists(pool_address);
    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);
    let (
        lockup_cycle_ended,
        active,
        _,
        commission_active,
        commission_pending_inactive
    ) &#61; calculate_stake_pool_drift(pool);

    let total_active_shares &#61; pool_u64::total_shares(&amp;pool.active_shares);
    let delegator_active_shares &#61; pool_u64::shares(&amp;pool.active_shares, delegator_address);

    let (_, _, pending_active, _) &#61; stake::get_stake(pool_address);
    if (pending_active &#61;&#61; 0) &#123;
        // zero `pending_active` stake indicates that either there are no `add_stake` fees or
        // previous epoch has ended and should identify shares owning these fees as released
        total_active_shares &#61; total_active_shares &#45; pool_u64::shares(&amp;pool.active_shares, NULL_SHAREHOLDER);
        if (delegator_address &#61;&#61; NULL_SHAREHOLDER) &#123;
            delegator_active_shares &#61; 0
        &#125;
    &#125;;
    active &#61; pool_u64::shares_to_amount_with_total_stats(
        &amp;pool.active_shares,
        delegator_active_shares,
        // exclude operator active rewards not converted to shares yet
        active &#45; commission_active,
        total_active_shares
    );

    // get state and stake (0 if there is none) of the pending withdrawal
    let (withdrawal_inactive, withdrawal_stake) &#61; get_pending_withdrawal(pool_address, delegator_address);
    // report non&#45;active stakes accordingly to the state of the pending withdrawal
    let (inactive, pending_inactive) &#61; if (withdrawal_inactive) (withdrawal_stake, 0) else (0, withdrawal_stake);

    // should also include commission rewards in case of the operator account
    // operator rewards are actually used to buy shares which is introducing
    // some imprecision (received stake would be slightly less)
    // but adding rewards onto the existing stake is still a good approximation
    if (delegator_address &#61;&#61; beneficiary_for_operator(get_operator(pool_address))) &#123;
        active &#61; active &#43; commission_active;
        // in&#45;flight pending_inactive commission can coexist with already inactive withdrawal
        if (lockup_cycle_ended) &#123;
            inactive &#61; inactive &#43; commission_pending_inactive
        &#125; else &#123;
            pending_inactive &#61; pending_inactive &#43; commission_pending_inactive
        &#125;
    &#125;;

    (active, inactive, pending_inactive)
&#125;
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


<pre><code>&#35;[view]
public fun get_add_stake_fee(pool_address: address, amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_add_stake_fee(
    pool_address: address,
    amount: u64
): u64 acquires DelegationPool, NextCommissionPercentage &#123;
    if (stake::is_current_epoch_validator(pool_address)) &#123;
        let (rewards_rate, rewards_rate_denominator) &#61; staking_config::get_reward_rate(&amp;staking_config::get());
        if (rewards_rate_denominator &gt; 0) &#123;
            assert_delegation_pool_exists(pool_address);

            rewards_rate &#61; rewards_rate &#42; (MAX_FEE &#45; operator_commission_percentage(pool_address));
            rewards_rate_denominator &#61; rewards_rate_denominator &#42; MAX_FEE;
            ((((amount as u128) &#42; (rewards_rate as u128)) / ((rewards_rate as u128) &#43; (rewards_rate_denominator as u128))) as u64)
        &#125; else &#123; 0 &#125;
    &#125; else &#123; 0 &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_can_withdraw_pending_inactive"></a>

## Function `can_withdraw_pending_inactive`

Return whether <code>pending_inactive</code> stake can be directly withdrawn from
the delegation pool, implicitly its stake pool, in the special case
the validator had gone inactive before its lockup expired.


<pre><code>&#35;[view]
public fun can_withdraw_pending_inactive(pool_address: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_withdraw_pending_inactive(pool_address: address): bool &#123;
    stake::get_validator_state(pool_address) &#61;&#61; VALIDATOR_STATUS_INACTIVE &amp;&amp;
        timestamp::now_seconds() &gt;&#61; stake::get_lockup_secs(pool_address)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_voter_total_voting_power"></a>

## Function `calculate_and_update_voter_total_voting_power`

Return the total voting power of a delegator in a delegation pool. This function syncs DelegationPool to the
latest state.


<pre><code>&#35;[view]
public fun calculate_and_update_voter_total_voting_power(pool_address: address, voter: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun calculate_and_update_voter_total_voting_power(
    pool_address: address,
    voter: address
): u64 acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert_partial_governance_voting_enabled(pool_address);
    // Delegation pool need to be synced to explain rewards(which could change the coin amount) and
    // commission(which could cause share transfer).
    synchronize_delegation_pool(pool_address);
    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);
    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);
    let latest_delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, voter);
    calculate_total_voting_power(pool, latest_delegated_votes)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_remaining_voting_power"></a>

## Function `calculate_and_update_remaining_voting_power`

Return the remaining voting power of a delegator in a delegation pool on a proposal. This function syncs DelegationPool to the
latest state.


<pre><code>&#35;[view]
public fun calculate_and_update_remaining_voting_power(pool_address: address, voter_address: address, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun calculate_and_update_remaining_voting_power(
    pool_address: address,
    voter_address: address,
    proposal_id: u64
): u64 acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert_partial_governance_voting_enabled(pool_address);
    // If the whole stake pool has no voting power(e.g. it has already voted before partial
    // governance voting flag is enabled), the delegator also has no voting power.
    if (aptos_governance::get_remaining_voting_power(pool_address, proposal_id) &#61;&#61; 0) &#123;
        return 0
    &#125;;

    let total_voting_power &#61; calculate_and_update_voter_total_voting_power(pool_address, voter_address);
    let governance_records &#61; borrow_global&lt;GovernanceRecords&gt;(pool_address);
    total_voting_power &#45; get_used_voting_power(governance_records, voter_address, proposal_id)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegator_voter"></a>

## Function `calculate_and_update_delegator_voter`

Return the latest delegated voter of a delegator in a delegation pool. This function syncs DelegationPool to the
latest state.


<pre><code>&#35;[view]
public fun calculate_and_update_delegator_voter(pool_address: address, delegator_address: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun calculate_and_update_delegator_voter(
    pool_address: address,
    delegator_address: address
): address acquires DelegationPool, GovernanceRecords &#123;
    assert_partial_governance_voting_enabled(pool_address);
    calculate_and_update_delegator_voter_internal(
        borrow_global&lt;DelegationPool&gt;(pool_address),
        borrow_global_mut&lt;GovernanceRecords&gt;(pool_address),
        delegator_address
    )
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_voting_delegation"></a>

## Function `calculate_and_update_voting_delegation`

Return the current state of a voting delegation of a delegator in a delegation pool.


<pre><code>&#35;[view]
public fun calculate_and_update_voting_delegation(pool_address: address, delegator_address: address): (address, address, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun calculate_and_update_voting_delegation(
    pool_address: address,
    delegator_address: address
): (address, address, u64) acquires DelegationPool, GovernanceRecords &#123;
    assert_partial_governance_voting_enabled(pool_address);
    let vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(
        borrow_global&lt;DelegationPool&gt;(pool_address),
        borrow_global_mut&lt;GovernanceRecords&gt;(pool_address),
        delegator_address
    );

    (vote_delegation.voter, vote_delegation.pending_voter, vote_delegation.last_locked_until_secs)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_expected_stake_pool_address"></a>

## Function `get_expected_stake_pool_address`

Return the address of the stake pool to be created with the provided owner, and seed.


<pre><code>&#35;[view]
public fun get_expected_stake_pool_address(owner: address, delegation_pool_creation_seed: vector&lt;u8&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_expected_stake_pool_address(owner: address, delegation_pool_creation_seed: vector&lt;u8&gt;
): address &#123;
    let seed &#61; create_resource_account_seed(delegation_pool_creation_seed);
    account::create_resource_address(&amp;owner, seed)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_min_remaining_secs_for_commission_change"></a>

## Function `min_remaining_secs_for_commission_change`

Return the minimum remaining time in seconds for commission change, which is one fourth of the lockup duration.


<pre><code>&#35;[view]
public fun min_remaining_secs_for_commission_change(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun min_remaining_secs_for_commission_change(): u64 &#123;
    let config &#61; staking_config::get();
    staking_config::get_recurring_lockup_duration(&amp;config) / 4
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlisting_enabled"></a>

## Function `allowlisting_enabled`

Return whether allowlisting is enabled for the provided delegation pool.


<pre><code>&#35;[view]
public fun allowlisting_enabled(pool_address: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun allowlisting_enabled(pool_address: address): bool &#123;
    assert_delegation_pool_exists(pool_address);
    exists&lt;DelegationPoolAllowlisting&gt;(pool_address)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_delegator_allowlisted"></a>

## Function `delegator_allowlisted`

Return whether the provided delegator is allowlisted.
A delegator is allowlisted if:
- allowlisting is disabled on the pool
- delegator is part of the allowlist


<pre><code>&#35;[view]
public fun delegator_allowlisted(pool_address: address, delegator_address: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegator_allowlisted(
    pool_address: address,
    delegator_address: address,
): bool acquires DelegationPoolAllowlisting &#123;
    if (!allowlisting_enabled(pool_address)) &#123; return true &#125;;
    smart_table::contains(freeze(borrow_mut_delegators_allowlist(pool_address)), delegator_address)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_delegators_allowlist"></a>

## Function `get_delegators_allowlist`

Return allowlist or revert if allowlisting is not enabled for the provided delegation pool.


<pre><code>&#35;[view]
public fun get_delegators_allowlist(pool_address: address): vector&lt;address&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegators_allowlist(
    pool_address: address,
): vector&lt;address&gt; acquires DelegationPoolAllowlisting &#123;
    assert_allowlisting_enabled(pool_address);

    let allowlist &#61; vector[];
    smart_table::for_each_ref(freeze(borrow_mut_delegators_allowlist(pool_address)), &#124;delegator, _v&#124; &#123;
        vector::push_back(&amp;mut allowlist, &#42;delegator);
    &#125;);
    allowlist
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_initialize_delegation_pool"></a>

## Function `initialize_delegation_pool`

Initialize a delegation pool of custom fixed <code>operator_commission_percentage</code>.
A resource account is created from <code>owner</code> signer and its supplied <code>delegation_pool_creation_seed</code>
to host the delegation pool resource and own the underlying stake pool.
Ownership over setting the operator/voter is granted to <code>owner</code> who has both roles initially.


<pre><code>public entry fun initialize_delegation_pool(owner: &amp;signer, operator_commission_percentage: u64, delegation_pool_creation_seed: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize_delegation_pool(
    owner: &amp;signer,
    operator_commission_percentage: u64,
    delegation_pool_creation_seed: vector&lt;u8&gt;,
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert!(features::delegation_pools_enabled(), error::invalid_state(EDELEGATION_POOLS_DISABLED));
    let owner_address &#61; signer::address_of(owner);
    assert!(!owner_cap_exists(owner_address), error::already_exists(EOWNER_CAP_ALREADY_EXISTS));
    assert!(operator_commission_percentage &lt;&#61; MAX_FEE, error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE));

    // generate a seed to be used to create the resource account hosting the delegation pool
    let seed &#61; create_resource_account_seed(delegation_pool_creation_seed);

    let (stake_pool_signer, stake_pool_signer_cap) &#61; account::create_resource_account(owner, seed);
    coin::register&lt;AptosCoin&gt;(&amp;stake_pool_signer);

    // stake_pool_signer will be owner of the stake pool and have its `stake::OwnerCapability`
    let pool_address &#61; signer::address_of(&amp;stake_pool_signer);
    stake::initialize_stake_owner(&amp;stake_pool_signer, 0, owner_address, owner_address);

    let inactive_shares &#61; table::new&lt;ObservedLockupCycle, pool_u64::Pool&gt;();
    table::add(
        &amp;mut inactive_shares,
        olc_with_index(0),
        pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR)
    );

    move_to(&amp;stake_pool_signer, DelegationPool &#123;
        active_shares: pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR),
        observed_lockup_cycle: olc_with_index(0),
        inactive_shares,
        pending_withdrawals: table::new&lt;address, ObservedLockupCycle&gt;(),
        stake_pool_signer_cap,
        total_coins_inactive: 0,
        operator_commission_percentage,
        add_stake_events: account::new_event_handle&lt;AddStakeEvent&gt;(&amp;stake_pool_signer),
        reactivate_stake_events: account::new_event_handle&lt;ReactivateStakeEvent&gt;(&amp;stake_pool_signer),
        unlock_stake_events: account::new_event_handle&lt;UnlockStakeEvent&gt;(&amp;stake_pool_signer),
        withdraw_stake_events: account::new_event_handle&lt;WithdrawStakeEvent&gt;(&amp;stake_pool_signer),
        distribute_commission_events: account::new_event_handle&lt;DistributeCommissionEvent&gt;(&amp;stake_pool_signer),
    &#125;);

    // save delegation pool ownership and resource account address (inner stake pool address) on `owner`
    move_to(owner, DelegationPoolOwnership &#123; pool_address &#125;);

    // All delegation pool enable partial governance voting by default once the feature flag is enabled.
    if (features::partial_governance_voting_enabled(
    ) &amp;&amp; features::delegation_pool_partial_governance_voting_enabled()) &#123;
        enable_partial_governance_voting(pool_address);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_beneficiary_for_operator"></a>

## Function `beneficiary_for_operator`

Return the beneficiary address of the operator.


<pre><code>&#35;[view]
public fun beneficiary_for_operator(operator: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun beneficiary_for_operator(operator: address): address acquires BeneficiaryForOperator &#123;
    if (exists&lt;BeneficiaryForOperator&gt;(operator)) &#123;
        return borrow_global&lt;BeneficiaryForOperator&gt;(operator).beneficiary_for_operator
    &#125; else &#123;
        operator
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_enable_partial_governance_voting"></a>

## Function `enable_partial_governance_voting`

Enable partial governance voting on a stake pool. The voter of this stake pool will be managed by this module.
The existing voter will be replaced. The function is permissionless.


<pre><code>public entry fun enable_partial_governance_voting(pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun enable_partial_governance_voting(
    pool_address: address,
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert!(features::partial_governance_voting_enabled(), error::invalid_state(EDISABLED_FUNCTION));
    assert!(
        features::delegation_pool_partial_governance_voting_enabled(),
        error::invalid_state(EDISABLED_FUNCTION)
    );
    assert_delegation_pool_exists(pool_address);
    // synchronize delegation and stake pools before any user operation.
    synchronize_delegation_pool(pool_address);

    let delegation_pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);
    let stake_pool_signer &#61; retrieve_stake_pool_owner(delegation_pool);
    // delegated_voter is managed by the stake pool itself, which signer capability is managed by DelegationPool.
    // So voting power of this stake pool can only be used through this module.
    stake::set_delegated_voter(&amp;stake_pool_signer, signer::address_of(&amp;stake_pool_signer));

    move_to(&amp;stake_pool_signer, GovernanceRecords &#123;
        votes: smart_table::new(),
        votes_per_proposal: smart_table::new(),
        vote_delegation: smart_table::new(),
        delegated_votes: smart_table::new(),
        vote_events: account::new_event_handle&lt;VoteEvent&gt;(&amp;stake_pool_signer),
        create_proposal_events: account::new_event_handle&lt;CreateProposalEvent&gt;(&amp;stake_pool_signer),
        delegate_voting_power_events: account::new_event_handle&lt;DelegateVotingPowerEvent&gt;(&amp;stake_pool_signer),
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_vote"></a>

## Function `vote`

Vote on a proposal with a voter's voting power. To successfully vote, the following conditions must be met:
1. The voting period of the proposal hasn't ended.
2. The delegation pool's lockup period ends after the voting period of the proposal.
3. The voter still has spare voting power on this proposal.
4. The delegation pool never votes on the proposal before enabling partial governance voting.


<pre><code>public entry fun vote(voter: &amp;signer, pool_address: address, proposal_id: u64, voting_power: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote(
    voter: &amp;signer,
    pool_address: address,
    proposal_id: u64,
    voting_power: u64,
    should_pass: bool
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert_partial_governance_voting_enabled(pool_address);
    // synchronize delegation and stake pools before any user operation.
    synchronize_delegation_pool(pool_address);

    let voter_address &#61; signer::address_of(voter);
    let remaining_voting_power &#61; calculate_and_update_remaining_voting_power(
        pool_address,
        voter_address,
        proposal_id
    );
    if (voting_power &gt; remaining_voting_power) &#123;
        voting_power &#61; remaining_voting_power;
    &#125;;
    assert!(voting_power &gt; 0, error::invalid_argument(ENO_VOTING_POWER));

    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);
    // Check a edge case during the transient period of enabling partial governance voting.
    assert_and_update_proposal_used_voting_power(governance_records, pool_address, proposal_id, voting_power);
    let used_voting_power &#61; borrow_mut_used_voting_power(governance_records, voter_address, proposal_id);
    &#42;used_voting_power &#61; &#42;used_voting_power &#43; voting_power;

    let pool_signer &#61; retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address));
    aptos_governance::partial_vote(&amp;pool_signer, pool_address, proposal_id, voting_power, should_pass);

    if (features::module_event_migration_enabled()) &#123;
        event::emit(
            Vote &#123;
                voter: voter_address,
                proposal_id,
                delegation_pool: pool_address,
                num_votes: voting_power,
                should_pass,
            &#125;
        );
    &#125;;

    event::emit_event(
        &amp;mut governance_records.vote_events,
        VoteEvent &#123;
            voter: voter_address,
            proposal_id,
            delegation_pool: pool_address,
            num_votes: voting_power,
            should_pass,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_create_proposal"></a>

## Function `create_proposal`

A voter could create a governance proposal by this function. To successfully create a proposal, the voter's
voting power in THIS delegation pool must be not less than the minimum required voting power specified in
<code>aptos_governance.move</code>.


<pre><code>public entry fun create_proposal(voter: &amp;signer, pool_address: address, execution_hash: vector&lt;u8&gt;, metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;, is_multi_step_proposal: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_proposal(
    voter: &amp;signer,
    pool_address: address,
    execution_hash: vector&lt;u8&gt;,
    metadata_location: vector&lt;u8&gt;,
    metadata_hash: vector&lt;u8&gt;,
    is_multi_step_proposal: bool,
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert_partial_governance_voting_enabled(pool_address);

    // synchronize delegation and stake pools before any user operation
    synchronize_delegation_pool(pool_address);

    let voter_addr &#61; signer::address_of(voter);
    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);
    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);
    let total_voting_power &#61; calculate_and_update_delegated_votes(pool, governance_records, voter_addr);
    assert!(
        total_voting_power &gt;&#61; aptos_governance::get_required_proposer_stake(),
        error::invalid_argument(EINSUFFICIENT_PROPOSER_STAKE));
    let pool_signer &#61; retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address));
    let proposal_id &#61; aptos_governance::create_proposal_v2_impl(
        &amp;pool_signer,
        pool_address,
        execution_hash,
        metadata_location,
        metadata_hash,
        is_multi_step_proposal,
    );

    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);

    if (features::module_event_migration_enabled()) &#123;
        event::emit(
            CreateProposal &#123;
                proposal_id,
                voter: voter_addr,
                delegation_pool: pool_address,
            &#125;
        );
    &#125;;

    event::emit_event(
        &amp;mut governance_records.create_proposal_events,
        CreateProposalEvent &#123;
            proposal_id,
            voter: voter_addr,
            delegation_pool: pool_address,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_owner_cap_exists"></a>

## Function `assert_owner_cap_exists`



<pre><code>fun assert_owner_cap_exists(owner: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_owner_cap_exists(owner: address) &#123;
    assert!(owner_cap_exists(owner), error::not_found(EOWNER_CAP_NOT_FOUND));
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_delegation_pool_exists"></a>

## Function `assert_delegation_pool_exists`



<pre><code>fun assert_delegation_pool_exists(pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_delegation_pool_exists(pool_address: address) &#123;
    assert!(delegation_pool_exists(pool_address), error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST));
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_min_active_balance"></a>

## Function `assert_min_active_balance`



<pre><code>fun assert_min_active_balance(pool: &amp;delegation_pool::DelegationPool, delegator_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_min_active_balance(pool: &amp;DelegationPool, delegator_address: address) &#123;
    let balance &#61; pool_u64::balance(&amp;pool.active_shares, delegator_address);
    assert!(balance &gt;&#61; MIN_COINS_ON_SHARES_POOL, error::invalid_argument(EDELEGATOR_ACTIVE_BALANCE_TOO_LOW));
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_min_pending_inactive_balance"></a>

## Function `assert_min_pending_inactive_balance`



<pre><code>fun assert_min_pending_inactive_balance(pool: &amp;delegation_pool::DelegationPool, delegator_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_min_pending_inactive_balance(pool: &amp;DelegationPool, delegator_address: address) &#123;
    let balance &#61; pool_u64::balance(pending_inactive_shares_pool(pool), delegator_address);
    assert!(
        balance &gt;&#61; MIN_COINS_ON_SHARES_POOL,
        error::invalid_argument(EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW)
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_partial_governance_voting_enabled"></a>

## Function `assert_partial_governance_voting_enabled`



<pre><code>fun assert_partial_governance_voting_enabled(pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_partial_governance_voting_enabled(pool_address: address) &#123;
    assert_delegation_pool_exists(pool_address);
    assert!(
        partial_governance_voting_enabled(pool_address),
        error::invalid_state(EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED)
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_allowlisting_enabled"></a>

## Function `assert_allowlisting_enabled`



<pre><code>fun assert_allowlisting_enabled(pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_allowlisting_enabled(pool_address: address) &#123;
    assert!(allowlisting_enabled(pool_address), error::invalid_state(EDELEGATORS_ALLOWLISTING_NOT_ENABLED));
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_delegator_allowlisted"></a>

## Function `assert_delegator_allowlisted`



<pre><code>fun assert_delegator_allowlisted(pool_address: address, delegator_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_delegator_allowlisted(
    pool_address: address,
    delegator_address: address,
) acquires DelegationPoolAllowlisting &#123;
    assert!(
        delegator_allowlisted(pool_address, delegator_address),
        error::permission_denied(EDELEGATOR_NOT_ALLOWLISTED)
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake"></a>

## Function `coins_to_redeem_to_ensure_min_stake`



<pre><code>fun coins_to_redeem_to_ensure_min_stake(src_shares_pool: &amp;pool_u64_unbound::Pool, shareholder: address, amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun coins_to_redeem_to_ensure_min_stake(
    src_shares_pool: &amp;pool_u64::Pool,
    shareholder: address,
    amount: u64,
): u64 &#123;
    // find how many coins would be redeemed if supplying `amount`
    let redeemed_coins &#61; pool_u64::shares_to_amount(
        src_shares_pool,
        amount_to_shares_to_redeem(src_shares_pool, shareholder, amount)
    );
    // if balance drops under threshold then redeem it entirely
    let src_balance &#61; pool_u64::balance(src_shares_pool, shareholder);
    if (src_balance &#45; redeemed_coins &lt; MIN_COINS_ON_SHARES_POOL) &#123;
        amount &#61; src_balance;
    &#125;;
    amount
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake"></a>

## Function `coins_to_transfer_to_ensure_min_stake`



<pre><code>fun coins_to_transfer_to_ensure_min_stake(src_shares_pool: &amp;pool_u64_unbound::Pool, dst_shares_pool: &amp;pool_u64_unbound::Pool, shareholder: address, amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun coins_to_transfer_to_ensure_min_stake(
    src_shares_pool: &amp;pool_u64::Pool,
    dst_shares_pool: &amp;pool_u64::Pool,
    shareholder: address,
    amount: u64,
): u64 &#123;
    // find how many coins would be redeemed from source if supplying `amount`
    let redeemed_coins &#61; pool_u64::shares_to_amount(
        src_shares_pool,
        amount_to_shares_to_redeem(src_shares_pool, shareholder, amount)
    );
    // if balance on destination would be less than threshold then redeem difference to threshold
    let dst_balance &#61; pool_u64::balance(dst_shares_pool, shareholder);
    if (dst_balance &#43; redeemed_coins &lt; MIN_COINS_ON_SHARES_POOL) &#123;
        // `redeemed_coins` &gt;&#61; `amount` &#45; 1 as redeem can lose at most 1 coin
        amount &#61; MIN_COINS_ON_SHARES_POOL &#45; dst_balance &#43; 1;
    &#125;;
    // check if new `amount` drops balance on source under threshold and adjust
    coins_to_redeem_to_ensure_min_stake(src_shares_pool, shareholder, amount)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_retrieve_stake_pool_owner"></a>

## Function `retrieve_stake_pool_owner`

Retrieves the shared resource account owning the stake pool in order
to forward a stake-management operation to this underlying pool.


<pre><code>fun retrieve_stake_pool_owner(pool: &amp;delegation_pool::DelegationPool): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun retrieve_stake_pool_owner(pool: &amp;DelegationPool): signer &#123;
    account::create_signer_with_capability(&amp;pool.stake_pool_signer_cap)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_pool_address"></a>

## Function `get_pool_address`

Get the address of delegation pool reference <code>pool</code>.


<pre><code>fun get_pool_address(pool: &amp;delegation_pool::DelegationPool): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_pool_address(pool: &amp;DelegationPool): address &#123;
    account::get_signer_capability_address(&amp;pool.stake_pool_signer_cap)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_delegator_active_shares"></a>

## Function `get_delegator_active_shares`

Get the active share amount of the delegator.


<pre><code>fun get_delegator_active_shares(pool: &amp;delegation_pool::DelegationPool, delegator: address): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_delegator_active_shares(pool: &amp;DelegationPool, delegator: address): u128 &#123;
    pool_u64::shares(&amp;pool.active_shares, delegator)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_delegator_pending_inactive_shares"></a>

## Function `get_delegator_pending_inactive_shares`

Get the pending inactive share amount of the delegator.


<pre><code>fun get_delegator_pending_inactive_shares(pool: &amp;delegation_pool::DelegationPool, delegator: address): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_delegator_pending_inactive_shares(pool: &amp;DelegationPool, delegator: address): u128 &#123;
    pool_u64::shares(pending_inactive_shares_pool(pool), delegator)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_get_used_voting_power"></a>

## Function `get_used_voting_power`

Get the used voting power of a voter on a proposal.


<pre><code>fun get_used_voting_power(governance_records: &amp;delegation_pool::GovernanceRecords, voter: address, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_used_voting_power(governance_records: &amp;GovernanceRecords, voter: address, proposal_id: u64): u64 &#123;
    let votes &#61; &amp;governance_records.votes;
    let key &#61; VotingRecordKey &#123;
        voter,
        proposal_id,
    &#125;;
    &#42;smart_table::borrow_with_default(votes, key, &amp;0)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_create_resource_account_seed"></a>

## Function `create_resource_account_seed`

Create the seed to derive the resource account address.


<pre><code>fun create_resource_account_seed(delegation_pool_creation_seed: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_resource_account_seed(
    delegation_pool_creation_seed: vector&lt;u8&gt;,
): vector&lt;u8&gt; &#123;
    let seed &#61; vector::empty&lt;u8&gt;();
    // include module salt (before any subseeds) to avoid conflicts with other modules creating resource accounts
    vector::append(&amp;mut seed, MODULE_SALT);
    // include an additional salt in case the same resource account has already been created
    vector::append(&amp;mut seed, delegation_pool_creation_seed);
    seed
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_borrow_mut_used_voting_power"></a>

## Function `borrow_mut_used_voting_power`

Borrow the mutable used voting power of a voter on a proposal.


<pre><code>fun borrow_mut_used_voting_power(governance_records: &amp;mut delegation_pool::GovernanceRecords, voter: address, proposal_id: u64): &amp;mut u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_mut_used_voting_power(
    governance_records: &amp;mut GovernanceRecords,
    voter: address,
    proposal_id: u64
): &amp;mut u64 &#123;
    let votes &#61; &amp;mut governance_records.votes;
    let key &#61; VotingRecordKey &#123;
        proposal_id,
        voter,
    &#125;;
    smart_table::borrow_mut_with_default(votes, key, 0)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation"></a>

## Function `update_and_borrow_mut_delegator_vote_delegation`

Update VoteDelegation of a delegator to up-to-date then borrow_mut it.


<pre><code>fun update_and_borrow_mut_delegator_vote_delegation(pool: &amp;delegation_pool::DelegationPool, governance_records: &amp;mut delegation_pool::GovernanceRecords, delegator: address): &amp;mut delegation_pool::VoteDelegation
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_and_borrow_mut_delegator_vote_delegation(
    pool: &amp;DelegationPool,
    governance_records: &amp;mut GovernanceRecords,
    delegator: address
): &amp;mut VoteDelegation &#123;
    let pool_address &#61; get_pool_address(pool);
    let locked_until_secs &#61; stake::get_lockup_secs(pool_address);

    let vote_delegation_table &#61; &amp;mut governance_records.vote_delegation;
    // By default, a delegator&apos;s delegated voter is itself.
    // TODO: recycle storage when VoteDelegation equals to default value.
    if (!smart_table::contains(vote_delegation_table, delegator)) &#123;
        return smart_table::borrow_mut_with_default(vote_delegation_table, delegator, VoteDelegation &#123;
            voter: delegator,
            last_locked_until_secs: locked_until_secs,
            pending_voter: delegator,
        &#125;)
    &#125;;

    let vote_delegation &#61; smart_table::borrow_mut(vote_delegation_table, delegator);
    // A lockup period has passed since last time `vote_delegation` was updated. Pending voter takes effect.
    if (vote_delegation.last_locked_until_secs &lt; locked_until_secs) &#123;
        vote_delegation.voter &#61; vote_delegation.pending_voter;
        vote_delegation.last_locked_until_secs &#61; locked_until_secs;
    &#125;;
    vote_delegation
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_update_and_borrow_mut_delegated_votes"></a>

## Function `update_and_borrow_mut_delegated_votes`

Update DelegatedVotes of a voter to up-to-date then borrow_mut it.


<pre><code>fun update_and_borrow_mut_delegated_votes(pool: &amp;delegation_pool::DelegationPool, governance_records: &amp;mut delegation_pool::GovernanceRecords, voter: address): &amp;mut delegation_pool::DelegatedVotes
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_and_borrow_mut_delegated_votes(
    pool: &amp;DelegationPool,
    governance_records: &amp;mut GovernanceRecords,
    voter: address
): &amp;mut DelegatedVotes &#123;
    let pool_address &#61; get_pool_address(pool);
    let locked_until_secs &#61; stake::get_lockup_secs(pool_address);

    let delegated_votes_per_voter &#61; &amp;mut governance_records.delegated_votes;
    // By default, a delegator&apos;s voter is itself.
    // TODO: recycle storage when DelegatedVotes equals to default value.
    if (!smart_table::contains(delegated_votes_per_voter, voter)) &#123;
        let active_shares &#61; get_delegator_active_shares(pool, voter);
        let inactive_shares &#61; get_delegator_pending_inactive_shares(pool, voter);
        return smart_table::borrow_mut_with_default(delegated_votes_per_voter, voter, DelegatedVotes &#123;
            active_shares,
            pending_inactive_shares: inactive_shares,
            active_shares_next_lockup: active_shares,
            last_locked_until_secs: locked_until_secs,
        &#125;)
    &#125;;

    let delegated_votes &#61; smart_table::borrow_mut(delegated_votes_per_voter, voter);
    // A lockup period has passed since last time `delegated_votes` was updated. Pending voter takes effect.
    if (delegated_votes.last_locked_until_secs &lt; locked_until_secs) &#123;
        delegated_votes.active_shares &#61; delegated_votes.active_shares_next_lockup;
        delegated_votes.pending_inactive_shares &#61; 0;
        delegated_votes.last_locked_until_secs &#61; locked_until_secs;
    &#125;;
    delegated_votes
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_olc_with_index"></a>

## Function `olc_with_index`



<pre><code>fun olc_with_index(index: u64): delegation_pool::ObservedLockupCycle
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun olc_with_index(index: u64): ObservedLockupCycle &#123;
    ObservedLockupCycle &#123; index &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_total_voting_power"></a>

## Function `calculate_total_voting_power`

Given the amounts of shares in <code>active_shares</code> pool and <code>inactive_shares</code> pool, calculate the total voting
power, which equals to the sum of the coin amounts.


<pre><code>fun calculate_total_voting_power(delegation_pool: &amp;delegation_pool::DelegationPool, latest_delegated_votes: &amp;delegation_pool::DelegatedVotes): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_total_voting_power(delegation_pool: &amp;DelegationPool, latest_delegated_votes: &amp;DelegatedVotes): u64 &#123;
    let active_amount &#61; pool_u64::shares_to_amount(
        &amp;delegation_pool.active_shares,
        latest_delegated_votes.active_shares);
    let pending_inactive_amount &#61; pool_u64::shares_to_amount(
        pending_inactive_shares_pool(delegation_pool),
        latest_delegated_votes.pending_inactive_shares);
    active_amount &#43; pending_inactive_amount
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegator_voter_internal"></a>

## Function `calculate_and_update_delegator_voter_internal`

Update VoteDelegation of a delegator to up-to-date then return the latest voter.


<pre><code>fun calculate_and_update_delegator_voter_internal(pool: &amp;delegation_pool::DelegationPool, governance_records: &amp;mut delegation_pool::GovernanceRecords, delegator: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_and_update_delegator_voter_internal(
    pool: &amp;DelegationPool,
    governance_records: &amp;mut GovernanceRecords,
    delegator: address
): address &#123;
    let vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(pool, governance_records, delegator);
    vote_delegation.voter
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegated_votes"></a>

## Function `calculate_and_update_delegated_votes`

Update DelegatedVotes of a voter to up-to-date then return the total voting power of this voter.


<pre><code>fun calculate_and_update_delegated_votes(pool: &amp;delegation_pool::DelegationPool, governance_records: &amp;mut delegation_pool::GovernanceRecords, voter: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_and_update_delegated_votes(
    pool: &amp;DelegationPool,
    governance_records: &amp;mut GovernanceRecords,
    voter: address
): u64 &#123;
    let delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, voter);
    calculate_total_voting_power(pool, delegated_votes)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_borrow_mut_delegators_allowlist"></a>

## Function `borrow_mut_delegators_allowlist`



<pre><code>fun borrow_mut_delegators_allowlist(pool_address: address): &amp;mut smart_table::SmartTable&lt;address, bool&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_mut_delegators_allowlist(
    pool_address: address
): &amp;mut SmartTable&lt;address, bool&gt; acquires DelegationPoolAllowlisting &#123;
    &amp;mut borrow_global_mut&lt;DelegationPoolAllowlisting&gt;(pool_address).allowlist
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the underlying stake pool.


<pre><code>public entry fun set_operator(owner: &amp;signer, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_operator(
    owner: &amp;signer,
    new_operator: address
) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));
    // synchronize delegation and stake pools before any user operation
    // ensure the old operator is paid its uncommitted commission rewards
    synchronize_delegation_pool(pool_address);
    stake::set_operator(&amp;retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address)), new_operator);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Allows an operator to change its beneficiary. Any existing unpaid commission rewards will be paid to the new
beneficiary. To ensure payment to the current beneficiary, one should first call <code>synchronize_delegation_pool</code>
before switching the beneficiary. An operator can set one beneficiary for delegation pools, not a separate
one for each pool.


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary_for_operator(
    operator: &amp;signer,
    new_beneficiary: address
) acquires BeneficiaryForOperator &#123;
    assert!(features::operator_beneficiary_change_enabled(), std::error::invalid_state(
        EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED
    ));
    // The beneficiay address of an operator is stored under the operator&apos;s address.
    // So, the operator does not need to be validated with respect to a staking pool.
    let operator_addr &#61; signer::address_of(operator);
    let old_beneficiary &#61; beneficiary_for_operator(operator_addr);
    if (exists&lt;BeneficiaryForOperator&gt;(operator_addr)) &#123;
        borrow_global_mut&lt;BeneficiaryForOperator&gt;(operator_addr).beneficiary_for_operator &#61; new_beneficiary;
    &#125; else &#123;
        move_to(operator, BeneficiaryForOperator &#123; beneficiary_for_operator: new_beneficiary &#125;);
    &#125;;

    emit(SetBeneficiaryForOperator &#123;
        operator: operator_addr,
        old_beneficiary,
        new_beneficiary,
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_update_commission_percentage"></a>

## Function `update_commission_percentage`

Allows an owner to update the commission percentage for the operator of the underlying stake pool.


<pre><code>public entry fun update_commission_percentage(owner: &amp;signer, new_commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_commission_percentage(
    owner: &amp;signer,
    new_commission_percentage: u64
) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert!(features::commission_change_delegation_pool_enabled(), error::invalid_state(
        ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED
    ));
    assert!(new_commission_percentage &lt;&#61; MAX_FEE, error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE));
    let owner_address &#61; signer::address_of(owner);
    let pool_address &#61; get_owned_pool_address(owner_address);
    assert!(
        operator_commission_percentage(pool_address) &#43; MAX_COMMISSION_INCREASE &gt;&#61; new_commission_percentage,
        error::invalid_argument(ETOO_LARGE_COMMISSION_INCREASE)
    );
    assert!(
        stake::get_remaining_lockup_secs(pool_address) &gt;&#61; min_remaining_secs_for_commission_change(),
        error::invalid_state(ETOO_LATE_COMMISSION_CHANGE)
    );

    // synchronize delegation and stake pools before any user operation. this ensures:
    // (1) the operator is paid its uncommitted commission rewards with the old commission percentage, and
    // (2) any pending commission percentage change is applied before the new commission percentage is set.
    synchronize_delegation_pool(pool_address);

    if (exists&lt;NextCommissionPercentage&gt;(pool_address)) &#123;
        let commission_percentage &#61; borrow_global_mut&lt;NextCommissionPercentage&gt;(pool_address);
        commission_percentage.commission_percentage_next_lockup_cycle &#61; new_commission_percentage;
        commission_percentage.effective_after_secs &#61; stake::get_lockup_secs(pool_address);
    &#125; else &#123;
        let delegation_pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);
        let pool_signer &#61; account::create_signer_with_capability(&amp;delegation_pool.stake_pool_signer_cap);
        move_to(&amp;pool_signer, NextCommissionPercentage &#123;
            commission_percentage_next_lockup_cycle: new_commission_percentage,
            effective_after_secs: stake::get_lockup_secs(pool_address),
        &#125;);
    &#125;;

    event::emit(CommissionPercentageChange &#123;
        pool_address,
        owner: owner_address,
        commission_percentage_next_lockup_cycle: new_commission_percentage,
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_set_delegated_voter"></a>

## Function `set_delegated_voter`

Allows an owner to change the delegated voter of the underlying stake pool.


<pre><code>public entry fun set_delegated_voter(owner: &amp;signer, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_delegated_voter(
    owner: &amp;signer,
    new_voter: address
) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    // No one can change delegated_voter once the partial governance voting feature is enabled.
    assert!(
        !features::delegation_pool_partial_governance_voting_enabled(),
        error::invalid_state(EDEPRECATED_FUNCTION)
    );
    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));
    // synchronize delegation and stake pools before any user operation
    synchronize_delegation_pool(pool_address);
    stake::set_delegated_voter(&amp;retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address)), new_voter);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_delegate_voting_power"></a>

## Function `delegate_voting_power`

Allows a delegator to delegate its voting power to a voter. If this delegator already has a delegated voter,
this change won't take effects until the next lockup period.


<pre><code>public entry fun delegate_voting_power(delegator: &amp;signer, pool_address: address, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun delegate_voting_power(
    delegator: &amp;signer,
    pool_address: address,
    new_voter: address
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert_partial_governance_voting_enabled(pool_address);

    // synchronize delegation and stake pools before any user operation
    synchronize_delegation_pool(pool_address);

    let delegator_address &#61; signer::address_of(delegator);
    let delegation_pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);
    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);
    let delegator_vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(
        delegation_pool,
        governance_records,
        delegator_address
    );
    let pending_voter: address &#61; delegator_vote_delegation.pending_voter;

    // No need to update if the voter doesn&apos;t really change.
    if (pending_voter !&#61; new_voter) &#123;
        delegator_vote_delegation.pending_voter &#61; new_voter;
        let active_shares &#61; get_delegator_active_shares(delegation_pool, delegator_address);
        // &lt;active shares&gt; of &lt;pending voter of shareholder&gt; &#45;&#61; &lt;active_shares&gt;
        // &lt;active shares&gt; of &lt;new voter of shareholder&gt; &#43;&#61; &lt;active_shares&gt;
        let pending_delegated_votes &#61; update_and_borrow_mut_delegated_votes(
            delegation_pool,
            governance_records,
            pending_voter
        );
        pending_delegated_votes.active_shares_next_lockup &#61;
            pending_delegated_votes.active_shares_next_lockup &#45; active_shares;

        let new_delegated_votes &#61; update_and_borrow_mut_delegated_votes(
            delegation_pool,
            governance_records,
            new_voter
        );
        new_delegated_votes.active_shares_next_lockup &#61;
            new_delegated_votes.active_shares_next_lockup &#43; active_shares;
    &#125;;

    if (features::module_event_migration_enabled()) &#123;
        event::emit(DelegateVotingPower &#123;
            pool_address,
            delegator: delegator_address,
            voter: new_voter,
        &#125;)
    &#125;;

    event::emit_event(&amp;mut governance_records.delegate_voting_power_events, DelegateVotingPowerEvent &#123;
        pool_address,
        delegator: delegator_address,
        voter: new_voter,
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_enable_delegators_allowlisting"></a>

## Function `enable_delegators_allowlisting`

Enable delegators allowlisting as the pool owner.


<pre><code>public entry fun enable_delegators_allowlisting(owner: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun enable_delegators_allowlisting(
    owner: &amp;signer,
) acquires DelegationPoolOwnership, DelegationPool &#123;
    assert!(
        features::delegation_pool_allowlisting_enabled(),
        error::invalid_state(EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED)
    );

    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));
    if (allowlisting_enabled(pool_address)) &#123; return &#125;;

    let pool_signer &#61; retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address));
    move_to(&amp;pool_signer, DelegationPoolAllowlisting &#123; allowlist: smart_table::new&lt;address, bool&gt;() &#125;);

    event::emit(EnableDelegatorsAllowlisting &#123; pool_address &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_disable_delegators_allowlisting"></a>

## Function `disable_delegators_allowlisting`

Disable delegators allowlisting as the pool owner. The existing allowlist will be emptied.


<pre><code>public entry fun disable_delegators_allowlisting(owner: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun disable_delegators_allowlisting(
    owner: &amp;signer,
) acquires DelegationPoolOwnership, DelegationPoolAllowlisting &#123;
    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));
    assert_allowlisting_enabled(pool_address);

    let DelegationPoolAllowlisting &#123; allowlist &#125; &#61; move_from&lt;DelegationPoolAllowlisting&gt;(pool_address);
    // if the allowlist becomes too large, the owner can always remove some delegators
    smart_table::destroy(allowlist);

    event::emit(DisableDelegatorsAllowlisting &#123; pool_address &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_delegator"></a>

## Function `allowlist_delegator`

Allowlist a delegator as the pool owner.


<pre><code>public entry fun allowlist_delegator(owner: &amp;signer, delegator_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun allowlist_delegator(
    owner: &amp;signer,
    delegator_address: address,
) acquires DelegationPoolOwnership, DelegationPoolAllowlisting &#123;
    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));
    assert_allowlisting_enabled(pool_address);

    if (delegator_allowlisted(pool_address, delegator_address)) &#123; return &#125;;

    smart_table::add(borrow_mut_delegators_allowlist(pool_address), delegator_address, true);

    event::emit(AllowlistDelegator &#123; pool_address, delegator_address &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_remove_delegator_from_allowlist"></a>

## Function `remove_delegator_from_allowlist`

Remove a delegator from the allowlist as the pool owner, but do not unlock their stake.


<pre><code>public entry fun remove_delegator_from_allowlist(owner: &amp;signer, delegator_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun remove_delegator_from_allowlist(
    owner: &amp;signer,
    delegator_address: address,
) acquires DelegationPoolOwnership, DelegationPoolAllowlisting &#123;
    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));
    assert_allowlisting_enabled(pool_address);

    if (!delegator_allowlisted(pool_address, delegator_address)) &#123; return &#125;;

    smart_table::remove(borrow_mut_delegators_allowlist(pool_address), delegator_address);

    event::emit(RemoveDelegatorFromAllowlist &#123; pool_address, delegator_address &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_evict_delegator"></a>

## Function `evict_delegator`

Evict a delegator that is not allowlisted by unlocking their entire stake.


<pre><code>public entry fun evict_delegator(owner: &amp;signer, delegator_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun evict_delegator(
    owner: &amp;signer,
    delegator_address: address,
) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage, DelegationPoolAllowlisting &#123;
    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));
    assert_allowlisting_enabled(pool_address);
    assert!(
        !delegator_allowlisted(pool_address, delegator_address),
        error::invalid_state(ECANNOT_EVICT_ALLOWLISTED_DELEGATOR)
    );

    // synchronize pool in order to query latest balance of delegator
    synchronize_delegation_pool(pool_address);

    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);
    if (get_delegator_active_shares(pool, delegator_address) &#61;&#61; 0) &#123; return &#125;;

    unlock_internal(delegator_address, pool_address, pool_u64::balance(&amp;pool.active_shares, delegator_address));

    event::emit(EvictDelegator &#123; pool_address, delegator_address &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins to the delegation pool <code>pool_address</code>.


<pre><code>public entry fun add_stake(delegator: &amp;signer, pool_address: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_stake(
    delegator: &amp;signer,
    pool_address: address,
    amount: u64
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage, DelegationPoolAllowlisting &#123;
    // short&#45;circuit if amount to add is 0 so no event is emitted
    if (amount &#61;&#61; 0) &#123; return &#125;;

    let delegator_address &#61; signer::address_of(delegator);
    assert_delegator_allowlisted(pool_address, delegator_address);

    // synchronize delegation and stake pools before any user operation
    synchronize_delegation_pool(pool_address);

    // fee to be charged for adding `amount` stake on this delegation pool at this epoch
    let add_stake_fee &#61; get_add_stake_fee(pool_address, amount);

    let pool &#61; borrow_global_mut&lt;DelegationPool&gt;(pool_address);

    // stake the entire amount to the stake pool
    aptos_account::transfer(delegator, pool_address, amount);
    stake::add_stake(&amp;retrieve_stake_pool_owner(pool), amount);

    // but buy shares for delegator just for the remaining amount after fee
    buy_in_active_shares(pool, delegator_address, amount &#45; add_stake_fee);
    assert_min_active_balance(pool, delegator_address);

    // grant temporary ownership over `add_stake` fees to a separate shareholder in order to:
    // &#45; not mistake them for rewards to pay the operator from
    // &#45; distribute them together with the `active` rewards when this epoch ends
    // in order to appreciate all shares on the active pool atomically
    buy_in_active_shares(pool, NULL_SHAREHOLDER, add_stake_fee);

    if (features::module_event_migration_enabled()) &#123;
        event::emit(
            AddStake &#123;
                pool_address,
                delegator_address,
                amount_added: amount,
                add_stake_fee,
            &#125;,
        );
    &#125;;

    event::emit_event(
        &amp;mut pool.add_stake_events,
        AddStakeEvent &#123;
            pool_address,
            delegator_address,
            amount_added: amount,
            add_stake_fee,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_unlock"></a>

## Function `unlock`

Unlock <code>amount</code> from the active + pending_active stake of <code>delegator</code> or
at most how much active stake there is on the stake pool.


<pre><code>public entry fun unlock(delegator: &amp;signer, pool_address: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock(
    delegator: &amp;signer,
    pool_address: address,
    amount: u64
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    // short&#45;circuit if amount to unlock is 0 so no event is emitted
    if (amount &#61;&#61; 0) &#123; return &#125;;

    // synchronize delegation and stake pools before any user operation
    synchronize_delegation_pool(pool_address);

    let delegator_address &#61; signer::address_of(delegator);
    unlock_internal(delegator_address, pool_address, amount);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_unlock_internal"></a>

## Function `unlock_internal`



<pre><code>fun unlock_internal(delegator_address: address, pool_address: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun unlock_internal(
    delegator_address: address,
    pool_address: address,
    amount: u64
) acquires DelegationPool, GovernanceRecords &#123;
    assert!(delegator_address !&#61; NULL_SHAREHOLDER, error::invalid_argument(ECANNOT_UNLOCK_NULL_SHAREHOLDER));

    // fail unlock of more stake than `active` on the stake pool
    let (active, _, _, _) &#61; stake::get_stake(pool_address);
    assert!(amount &lt;&#61; active, error::invalid_argument(ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK));

    let pool &#61; borrow_global_mut&lt;DelegationPool&gt;(pool_address);
    amount &#61; coins_to_transfer_to_ensure_min_stake(
        &amp;pool.active_shares,
        pending_inactive_shares_pool(pool),
        delegator_address,
        amount,
    );
    amount &#61; redeem_active_shares(pool, delegator_address, amount);

    stake::unlock(&amp;retrieve_stake_pool_owner(pool), amount);

    buy_in_pending_inactive_shares(pool, delegator_address, amount);
    assert_min_pending_inactive_balance(pool, delegator_address);

    if (features::module_event_migration_enabled()) &#123;
        event::emit(
            UnlockStake &#123;
                pool_address,
                delegator_address,
                amount_unlocked: amount,
            &#125;,
        );
    &#125;;

    event::emit_event(
        &amp;mut pool.unlock_stake_events,
        UnlockStakeEvent &#123;
            pool_address,
            delegator_address,
            amount_unlocked: amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code>public entry fun reactivate_stake(delegator: &amp;signer, pool_address: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reactivate_stake(
    delegator: &amp;signer,
    pool_address: address,
    amount: u64
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage, DelegationPoolAllowlisting &#123;
    // short&#45;circuit if amount to reactivate is 0 so no event is emitted
    if (amount &#61;&#61; 0) &#123; return &#125;;

    let delegator_address &#61; signer::address_of(delegator);
    assert_delegator_allowlisted(pool_address, delegator_address);

    // synchronize delegation and stake pools before any user operation
    synchronize_delegation_pool(pool_address);

    let pool &#61; borrow_global_mut&lt;DelegationPool&gt;(pool_address);
    amount &#61; coins_to_transfer_to_ensure_min_stake(
        pending_inactive_shares_pool(pool),
        &amp;pool.active_shares,
        delegator_address,
        amount,
    );
    let observed_lockup_cycle &#61; pool.observed_lockup_cycle;
    amount &#61; redeem_inactive_shares(pool, delegator_address, amount, observed_lockup_cycle);

    stake::reactivate_stake(&amp;retrieve_stake_pool_owner(pool), amount);

    buy_in_active_shares(pool, delegator_address, amount);
    assert_min_active_balance(pool, delegator_address);

    if (features::module_event_migration_enabled()) &#123;
        event::emit(
            ReactivateStake &#123;
                pool_address,
                delegator_address,
                amount_reactivated: amount,
            &#125;,
        );
    &#125;;

    event::emit_event(
        &amp;mut pool.reactivate_stake_events,
        ReactivateStakeEvent &#123;
            pool_address,
            delegator_address,
            amount_reactivated: amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of owned inactive stake from the delegation pool at <code>pool_address</code>.


<pre><code>public entry fun withdraw(delegator: &amp;signer, pool_address: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun withdraw(
    delegator: &amp;signer,
    pool_address: address,
    amount: u64
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert!(amount &gt; 0, error::invalid_argument(EWITHDRAW_ZERO_STAKE));
    // synchronize delegation and stake pools before any user operation
    synchronize_delegation_pool(pool_address);
    withdraw_internal(borrow_global_mut&lt;DelegationPool&gt;(pool_address), signer::address_of(delegator), amount);
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_withdraw_internal"></a>

## Function `withdraw_internal`



<pre><code>fun withdraw_internal(pool: &amp;mut delegation_pool::DelegationPool, delegator_address: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun withdraw_internal(
    pool: &amp;mut DelegationPool,
    delegator_address: address,
    amount: u64
) acquires GovernanceRecords &#123;
    // TODO: recycle storage when a delegator fully exits the delegation pool.
    // short&#45;circuit if amount to withdraw is 0 so no event is emitted
    if (amount &#61;&#61; 0) &#123; return &#125;;

    let pool_address &#61; get_pool_address(pool);
    let (withdrawal_exists, withdrawal_olc) &#61; pending_withdrawal_exists(pool, delegator_address);
    // exit if no withdrawal or (it is pending and cannot withdraw pending_inactive stake from stake pool)
    if (!(
        withdrawal_exists &amp;&amp;
            (withdrawal_olc.index &lt; pool.observed_lockup_cycle.index &#124;&#124; can_withdraw_pending_inactive(pool_address))
    )) &#123; return &#125;;

    if (withdrawal_olc.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;
        amount &#61; coins_to_redeem_to_ensure_min_stake(
            pending_inactive_shares_pool(pool),
            delegator_address,
            amount,
        )
    &#125;;
    amount &#61; redeem_inactive_shares(pool, delegator_address, amount, withdrawal_olc);

    let stake_pool_owner &#61; &amp;retrieve_stake_pool_owner(pool);
    // stake pool will inactivate entire pending_inactive stake at `stake::withdraw` to make it withdrawable
    // however, bypassing the inactivation of excess stake (inactivated but not withdrawn) ensures
    // the OLC is not advanced indefinitely on `unlock`&#45;`withdraw` paired calls
    if (can_withdraw_pending_inactive(pool_address)) &#123;
        // get excess stake before being entirely inactivated
        let (_, _, _, pending_inactive) &#61; stake::get_stake(pool_address);
        if (withdrawal_olc.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;
            // `amount` less excess if withdrawing pending_inactive stake
            pending_inactive &#61; pending_inactive &#45; amount
        &#125;;
        // escape excess stake from inactivation
        stake::reactivate_stake(stake_pool_owner, pending_inactive);
        stake::withdraw(stake_pool_owner, amount);
        // restore excess stake to the pending_inactive state
        stake::unlock(stake_pool_owner, pending_inactive);
    &#125; else &#123;
        // no excess stake if `stake::withdraw` does not inactivate at all
        stake::withdraw(stake_pool_owner, amount);
    &#125;;
    aptos_account::transfer(stake_pool_owner, delegator_address, amount);

    // commit withdrawal of possibly inactive stake to the `total_coins_inactive`
    // known by the delegation pool in order to not mistake it for slashing at next synchronization
    let (_, inactive, _, _) &#61; stake::get_stake(pool_address);
    pool.total_coins_inactive &#61; inactive;

    if (features::module_event_migration_enabled()) &#123;
        event::emit(
            WithdrawStake &#123;
                pool_address,
                delegator_address,
                amount_withdrawn: amount,
            &#125;,
        );
    &#125;;

    event::emit_event(
        &amp;mut pool.withdraw_stake_events,
        WithdrawStakeEvent &#123;
            pool_address,
            delegator_address,
            amount_withdrawn: amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_pending_withdrawal_exists"></a>

## Function `pending_withdrawal_exists`

Return the unique observed lockup cycle where delegator <code>delegator_address</code> may have
unlocking (or already unlocked) stake to be withdrawn from delegation pool <code>pool</code>.
A bool is returned to signal if a pending withdrawal exists at all.


<pre><code>fun pending_withdrawal_exists(pool: &amp;delegation_pool::DelegationPool, delegator_address: address): (bool, delegation_pool::ObservedLockupCycle)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun pending_withdrawal_exists(pool: &amp;DelegationPool, delegator_address: address): (bool, ObservedLockupCycle) &#123;
    if (table::contains(&amp;pool.pending_withdrawals, delegator_address)) &#123;
        (true, &#42;table::borrow(&amp;pool.pending_withdrawals, delegator_address))
    &#125; else &#123;
        (false, olc_with_index(0))
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_pending_inactive_shares_pool_mut"></a>

## Function `pending_inactive_shares_pool_mut`

Return a mutable reference to the shares pool of <code>pending_inactive</code> stake on the
delegation pool, always the last item in <code>inactive_shares</code>.


<pre><code>fun pending_inactive_shares_pool_mut(pool: &amp;mut delegation_pool::DelegationPool): &amp;mut pool_u64_unbound::Pool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun pending_inactive_shares_pool_mut(pool: &amp;mut DelegationPool): &amp;mut pool_u64::Pool &#123;
    let observed_lockup_cycle &#61; pool.observed_lockup_cycle;
    table::borrow_mut(&amp;mut pool.inactive_shares, observed_lockup_cycle)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_pending_inactive_shares_pool"></a>

## Function `pending_inactive_shares_pool`



<pre><code>fun pending_inactive_shares_pool(pool: &amp;delegation_pool::DelegationPool): &amp;pool_u64_unbound::Pool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun pending_inactive_shares_pool(pool: &amp;DelegationPool): &amp;pool_u64::Pool &#123;
    table::borrow(&amp;pool.inactive_shares, pool.observed_lockup_cycle)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_execute_pending_withdrawal"></a>

## Function `execute_pending_withdrawal`

Execute the pending withdrawal of <code>delegator_address</code> on delegation pool <code>pool</code>
if existing and already inactive to allow the creation of a new one.
<code>pending_inactive</code> stake would be left untouched even if withdrawable and should
be explicitly withdrawn by delegator


<pre><code>fun execute_pending_withdrawal(pool: &amp;mut delegation_pool::DelegationPool, delegator_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun execute_pending_withdrawal(pool: &amp;mut DelegationPool, delegator_address: address) acquires GovernanceRecords &#123;
    let (withdrawal_exists, withdrawal_olc) &#61; pending_withdrawal_exists(pool, delegator_address);
    if (withdrawal_exists &amp;&amp; withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) &#123;
        withdraw_internal(pool, delegator_address, MAX_U64);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_buy_in_active_shares"></a>

## Function `buy_in_active_shares`

Buy shares into the active pool on behalf of delegator <code>shareholder</code> who
deposited <code>coins_amount</code>. This function doesn't make any coin transfer.


<pre><code>fun buy_in_active_shares(pool: &amp;mut delegation_pool::DelegationPool, shareholder: address, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun buy_in_active_shares(
    pool: &amp;mut DelegationPool,
    shareholder: address,
    coins_amount: u64,
): u128 acquires GovernanceRecords &#123;
    let new_shares &#61; pool_u64::amount_to_shares(&amp;pool.active_shares, coins_amount);
    // No need to buy 0 shares.
    if (new_shares &#61;&#61; 0) &#123; return 0 &#125;;

    // Always update governance records before any change to the shares pool.
    let pool_address &#61; get_pool_address(pool);
    if (partial_governance_voting_enabled(pool_address)) &#123;
        update_governance_records_for_buy_in_active_shares(pool, pool_address, new_shares, shareholder);
    &#125;;

    pool_u64::buy_in(&amp;mut pool.active_shares, shareholder, coins_amount);
    new_shares
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_buy_in_pending_inactive_shares"></a>

## Function `buy_in_pending_inactive_shares`

Buy shares into the pending_inactive pool on behalf of delegator <code>shareholder</code> who
redeemed <code>coins_amount</code> from the active pool to schedule it for unlocking.
If delegator's pending withdrawal exists and has been inactivated, execute it firstly
to ensure there is always only one withdrawal request.


<pre><code>fun buy_in_pending_inactive_shares(pool: &amp;mut delegation_pool::DelegationPool, shareholder: address, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun buy_in_pending_inactive_shares(
    pool: &amp;mut DelegationPool,
    shareholder: address,
    coins_amount: u64,
): u128 acquires GovernanceRecords &#123;
    let new_shares &#61; pool_u64::amount_to_shares(pending_inactive_shares_pool(pool), coins_amount);
    // never create a new pending withdrawal unless delegator owns some pending_inactive shares
    if (new_shares &#61;&#61; 0) &#123; return 0 &#125;;

    // Always update governance records before any change to the shares pool.
    let pool_address &#61; get_pool_address(pool);
    if (partial_governance_voting_enabled(pool_address)) &#123;
        update_governance_records_for_buy_in_pending_inactive_shares(pool, pool_address, new_shares, shareholder);
    &#125;;

    // cannot buy inactive shares, only pending_inactive at current lockup cycle
    pool_u64::buy_in(pending_inactive_shares_pool_mut(pool), shareholder, coins_amount);

    // execute the pending withdrawal if exists and is inactive before creating a new one
    execute_pending_withdrawal(pool, shareholder);

    // save observed lockup cycle for the new pending withdrawal
    let observed_lockup_cycle &#61; pool.observed_lockup_cycle;
    assert!(&#42;table::borrow_mut_with_default(
        &amp;mut pool.pending_withdrawals,
        shareholder,
        observed_lockup_cycle
    ) &#61;&#61; observed_lockup_cycle,
        error::invalid_state(EPENDING_WITHDRAWAL_EXISTS)
    );

    new_shares
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_amount_to_shares_to_redeem"></a>

## Function `amount_to_shares_to_redeem`

Convert <code>coins_amount</code> of coins to be redeemed from shares pool <code>shares_pool</code>
to the exact number of shares to redeem in order to achieve this.


<pre><code>fun amount_to_shares_to_redeem(shares_pool: &amp;pool_u64_unbound::Pool, shareholder: address, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun amount_to_shares_to_redeem(
    shares_pool: &amp;pool_u64::Pool,
    shareholder: address,
    coins_amount: u64,
): u128 &#123;
    if (coins_amount &gt;&#61; pool_u64::balance(shares_pool, shareholder)) &#123;
        // cap result at total shares of shareholder to pass `EINSUFFICIENT_SHARES` on subsequent redeem
        pool_u64::shares(shares_pool, shareholder)
    &#125; else &#123;
        pool_u64::amount_to_shares(shares_pool, coins_amount)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_redeem_active_shares"></a>

## Function `redeem_active_shares`

Redeem shares from the active pool on behalf of delegator <code>shareholder</code> who
wants to unlock <code>coins_amount</code> of its active stake.
Extracted coins will be used to buy shares into the pending_inactive pool and
be available for withdrawal when current OLC ends.


<pre><code>fun redeem_active_shares(pool: &amp;mut delegation_pool::DelegationPool, shareholder: address, coins_amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun redeem_active_shares(
    pool: &amp;mut DelegationPool,
    shareholder: address,
    coins_amount: u64,
): u64 acquires GovernanceRecords &#123;
    let shares_to_redeem &#61; amount_to_shares_to_redeem(&amp;pool.active_shares, shareholder, coins_amount);
    // silently exit if not a shareholder otherwise redeem would fail with `ESHAREHOLDER_NOT_FOUND`
    if (shares_to_redeem &#61;&#61; 0) return 0;

    // Always update governance records before any change to the shares pool.
    let pool_address &#61; get_pool_address(pool);
    if (partial_governance_voting_enabled(pool_address)) &#123;
        update_governanace_records_for_redeem_active_shares(pool, pool_address, shares_to_redeem, shareholder);
    &#125;;

    pool_u64::redeem_shares(&amp;mut pool.active_shares, shareholder, shares_to_redeem)
&#125;
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


<pre><code>fun redeem_inactive_shares(pool: &amp;mut delegation_pool::DelegationPool, shareholder: address, coins_amount: u64, lockup_cycle: delegation_pool::ObservedLockupCycle): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun redeem_inactive_shares(
    pool: &amp;mut DelegationPool,
    shareholder: address,
    coins_amount: u64,
    lockup_cycle: ObservedLockupCycle,
): u64 acquires GovernanceRecords &#123;
    let shares_to_redeem &#61; amount_to_shares_to_redeem(
        table::borrow(&amp;pool.inactive_shares, lockup_cycle),
        shareholder,
        coins_amount);
    // silently exit if not a shareholder otherwise redeem would fail with `ESHAREHOLDER_NOT_FOUND`
    if (shares_to_redeem &#61;&#61; 0) return 0;

    // Always update governance records before any change to the shares pool.
    let pool_address &#61; get_pool_address(pool);
    // Only redeem shares from the pending_inactive pool at `lockup_cycle` &#61;&#61; current OLC.
    if (partial_governance_voting_enabled(pool_address) &amp;&amp; lockup_cycle.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;
        update_governanace_records_for_redeem_pending_inactive_shares(
            pool,
            pool_address,
            shares_to_redeem,
            shareholder
        );
    &#125;;

    let inactive_shares &#61; table::borrow_mut(&amp;mut pool.inactive_shares, lockup_cycle);
    // 1. reaching here means delegator owns inactive/pending_inactive shares at OLC `lockup_cycle`
    let redeemed_coins &#61; pool_u64::redeem_shares(inactive_shares, shareholder, shares_to_redeem);

    // if entirely reactivated pending_inactive stake or withdrawn inactive one,
    // re&#45;enable unlocking for delegator by deleting this pending withdrawal
    if (pool_u64::shares(inactive_shares, shareholder) &#61;&#61; 0) &#123;
        // 2. a delegator owns inactive/pending_inactive shares only at the OLC of its pending withdrawal
        // 1 &amp; 2: the pending withdrawal itself has been emptied of shares and can be safely deleted
        table::remove(&amp;mut pool.pending_withdrawals, shareholder);
    &#125;;
    // destroy inactive shares pool of past OLC if all its stake has been withdrawn
    if (lockup_cycle.index &lt; pool.observed_lockup_cycle.index &amp;&amp; total_coins(inactive_shares) &#61;&#61; 0) &#123;
        pool_u64::destroy_empty(table::remove(&amp;mut pool.inactive_shares, lockup_cycle));
    &#125;;

    redeemed_coins
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_calculate_stake_pool_drift"></a>

## Function `calculate_stake_pool_drift`

Calculate stake deviations between the delegation and stake pools in order to
capture the rewards earned in the meantime, resulted operator commission and
whether the lockup expired on the stake pool.


<pre><code>fun calculate_stake_pool_drift(pool: &amp;delegation_pool::DelegationPool): (bool, u64, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_stake_pool_drift(pool: &amp;DelegationPool): (bool, u64, u64, u64, u64) &#123;
    let (active, inactive, pending_active, pending_inactive) &#61; stake::get_stake(get_pool_address(pool));
    assert!(
        inactive &gt;&#61; pool.total_coins_inactive,
        error::invalid_state(ESLASHED_INACTIVE_STAKE_ON_PAST_OLC)
    );
    // determine whether a new lockup cycle has been ended on the stake pool and
    // inactivated SOME `pending_inactive` stake which should stop earning rewards now,
    // thus requiring separation of the `pending_inactive` stake on current observed lockup
    // and the future one on the newly started lockup
    let lockup_cycle_ended &#61; inactive &gt; pool.total_coins_inactive;

    // actual coins on stake pool belonging to the active shares pool
    active &#61; active &#43; pending_active;
    // actual coins on stake pool belonging to the shares pool hosting `pending_inactive` stake
    // at current observed lockup cycle, either pending: `pending_inactive` or already inactivated:
    if (lockup_cycle_ended) &#123;
        // `inactive` on stake pool &#61; any previous `inactive` stake &#43;
        // any previous `pending_inactive` stake and its rewards (both inactivated)
        pending_inactive &#61; inactive &#45; pool.total_coins_inactive
    &#125;;

    // on stake&#45;management operations, total coins on the internal shares pools and individual
    // stakes on the stake pool are updated simultaneously, thus the only stakes becoming
    // unsynced are rewards and slashes routed exclusively to/out the stake pool

    // operator `active` rewards not persisted yet to the active shares pool
    let pool_active &#61; total_coins(&amp;pool.active_shares);
    let commission_active &#61; if (active &gt; pool_active) &#123;
        math64::mul_div(active &#45; pool_active, pool.operator_commission_percentage, MAX_FEE)
    &#125; else &#123;
        // handle any slashing applied to `active` stake
        0
    &#125;;
    // operator `pending_inactive` rewards not persisted yet to the pending_inactive shares pool
    let pool_pending_inactive &#61; total_coins(pending_inactive_shares_pool(pool));
    let commission_pending_inactive &#61; if (pending_inactive &gt; pool_pending_inactive) &#123;
        math64::mul_div(
            pending_inactive &#45; pool_pending_inactive,
            pool.operator_commission_percentage,
            MAX_FEE
        )
    &#125; else &#123;
        // handle any slashing applied to `pending_inactive` stake
        0
    &#125;;

    (lockup_cycle_ended, active, pending_inactive, commission_active, commission_pending_inactive)
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_synchronize_delegation_pool"></a>

## Function `synchronize_delegation_pool`

Synchronize delegation and stake pools: distribute yet-undetected rewards to the corresponding internal
shares pools, assign commission to operator and eventually prepare delegation pool for a new lockup cycle.


<pre><code>public entry fun synchronize_delegation_pool(pool_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun synchronize_delegation_pool(
    pool_address: address
) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;
    assert_delegation_pool_exists(pool_address);
    let pool &#61; borrow_global_mut&lt;DelegationPool&gt;(pool_address);
    let (
        lockup_cycle_ended,
        active,
        pending_inactive,
        commission_active,
        commission_pending_inactive
    ) &#61; calculate_stake_pool_drift(pool);

    // zero `pending_active` stake indicates that either there are no `add_stake` fees or
    // previous epoch has ended and should release the shares owning the existing fees
    let (_, _, pending_active, _) &#61; stake::get_stake(pool_address);
    if (pending_active &#61;&#61; 0) &#123;
        // renounce ownership over the `add_stake` fees by redeeming all shares of
        // the special shareholder, implicitly their equivalent coins, out of the active shares pool
        redeem_active_shares(pool, NULL_SHAREHOLDER, MAX_U64);
    &#125;;

    // distribute rewards remaining after commission, to delegators (to already existing shares)
    // before buying shares for the operator for its entire commission fee
    // otherwise, operator&apos;s new shares would additionally appreciate from rewards it does not own

    // update total coins accumulated by `active` &#43; `pending_active` shares
    // redeemed `add_stake` fees are restored and distributed to the rest of the pool as rewards
    pool_u64::update_total_coins(&amp;mut pool.active_shares, active &#45; commission_active);
    // update total coins accumulated by `pending_inactive` shares at current observed lockup cycle
    pool_u64::update_total_coins(
        pending_inactive_shares_pool_mut(pool),
        pending_inactive &#45; commission_pending_inactive
    );

    // reward operator its commission out of uncommitted active rewards (`add_stake` fees already excluded)
    buy_in_active_shares(pool, beneficiary_for_operator(stake::get_operator(pool_address)), commission_active);
    // reward operator its commission out of uncommitted pending_inactive rewards
    buy_in_pending_inactive_shares(
        pool,
        beneficiary_for_operator(stake::get_operator(pool_address)),
        commission_pending_inactive
    );

    event::emit_event(
        &amp;mut pool.distribute_commission_events,
        DistributeCommissionEvent &#123;
            pool_address,
            operator: stake::get_operator(pool_address),
            commission_active,
            commission_pending_inactive,
        &#125;,
    );

    if (features::operator_beneficiary_change_enabled()) &#123;
        emit(DistributeCommission &#123;
            pool_address,
            operator: stake::get_operator(pool_address),
            beneficiary: beneficiary_for_operator(stake::get_operator(pool_address)),
            commission_active,
            commission_pending_inactive,
        &#125;)
    &#125;;

    // advance lockup cycle on delegation pool if already ended on stake pool (AND stake explicitly inactivated)
    if (lockup_cycle_ended) &#123;
        // capture inactive coins over all ended lockup cycles (including this ending one)
        let (_, inactive, _, _) &#61; stake::get_stake(pool_address);
        pool.total_coins_inactive &#61; inactive;

        // advance lockup cycle on the delegation pool
        pool.observed_lockup_cycle.index &#61; pool.observed_lockup_cycle.index &#43; 1;
        // start new lockup cycle with a fresh shares pool for `pending_inactive` stake
        table::add(
            &amp;mut pool.inactive_shares,
            pool.observed_lockup_cycle,
            pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR)
        );
    &#125;;

    if (is_next_commission_percentage_effective(pool_address)) &#123;
        pool.operator_commission_percentage &#61; borrow_global&lt;NextCommissionPercentage&gt;(
            pool_address
        ).commission_percentage_next_lockup_cycle;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_assert_and_update_proposal_used_voting_power"></a>

## Function `assert_and_update_proposal_used_voting_power`



<pre><code>fun assert_and_update_proposal_used_voting_power(governance_records: &amp;mut delegation_pool::GovernanceRecords, pool_address: address, proposal_id: u64, voting_power: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_and_update_proposal_used_voting_power(
    governance_records: &amp;mut GovernanceRecords, pool_address: address, proposal_id: u64, voting_power: u64
) &#123;
    let stake_pool_remaining_voting_power &#61; aptos_governance::get_remaining_voting_power(pool_address, proposal_id);
    let stake_pool_used_voting_power &#61; aptos_governance::get_voting_power(
        pool_address
    ) &#45; stake_pool_remaining_voting_power;
    let proposal_used_voting_power &#61; smart_table::borrow_mut_with_default(
        &amp;mut governance_records.votes_per_proposal,
        proposal_id,
        0
    );
    // A edge case: Before enabling partial governance voting on a delegation pool, the delegation pool has
    // a voter which can vote with all voting power of this delegation pool. If the voter votes on a proposal after
    // partial governance voting flag is enabled, the delegation pool doesn&apos;t have enough voting power on this
    // proposal for all the delegators. To be fair, no one can vote on this proposal through this delegation pool.
    // To detect this case, check if the stake pool had used voting power not through delegation_pool module.
    assert!(
        stake_pool_used_voting_power &#61;&#61; &#42;proposal_used_voting_power,
        error::invalid_argument(EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING)
    );
    &#42;proposal_used_voting_power &#61; &#42;proposal_used_voting_power &#43; voting_power;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_update_governance_records_for_buy_in_active_shares"></a>

## Function `update_governance_records_for_buy_in_active_shares`



<pre><code>fun update_governance_records_for_buy_in_active_shares(pool: &amp;delegation_pool::DelegationPool, pool_address: address, new_shares: u128, shareholder: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_governance_records_for_buy_in_active_shares(
    pool: &amp;DelegationPool, pool_address: address, new_shares: u128, shareholder: address
) acquires GovernanceRecords &#123;
    // &lt;active shares&gt; of &lt;shareholder&gt; &#43;&#61; &lt;new_shares&gt; &#45;&#45;&#45;&#45;&gt;
    // &lt;active shares&gt; of &lt;current voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;
    // &lt;active shares&gt; of &lt;next voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;
    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);
    let vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(pool, governance_records, shareholder);
    let current_voter &#61; vote_delegation.voter;
    let pending_voter &#61; vote_delegation.pending_voter;
    let current_delegated_votes &#61;
        update_and_borrow_mut_delegated_votes(pool, governance_records, current_voter);
    current_delegated_votes.active_shares &#61; current_delegated_votes.active_shares &#43; new_shares;
    if (pending_voter &#61;&#61; current_voter) &#123;
        current_delegated_votes.active_shares_next_lockup &#61;
            current_delegated_votes.active_shares_next_lockup &#43; new_shares;
    &#125; else &#123;
        let pending_delegated_votes &#61;
            update_and_borrow_mut_delegated_votes(pool, governance_records, pending_voter);
        pending_delegated_votes.active_shares_next_lockup &#61;
            pending_delegated_votes.active_shares_next_lockup &#43; new_shares;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares"></a>

## Function `update_governance_records_for_buy_in_pending_inactive_shares`



<pre><code>fun update_governance_records_for_buy_in_pending_inactive_shares(pool: &amp;delegation_pool::DelegationPool, pool_address: address, new_shares: u128, shareholder: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_governance_records_for_buy_in_pending_inactive_shares(
    pool: &amp;DelegationPool, pool_address: address, new_shares: u128, shareholder: address
) acquires GovernanceRecords &#123;
    // &lt;pending inactive shares&gt; of &lt;shareholder&gt; &#43;&#61; &lt;new_shares&gt;   &#45;&#45;&#45;&#45;&gt;
    // &lt;pending inactive shares&gt; of &lt;current voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;
    // no impact on &lt;pending inactive shares&gt; of &lt;next voter of shareholder&gt;
    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);
    let current_voter &#61; calculate_and_update_delegator_voter_internal(pool, governance_records, shareholder);
    let current_delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, current_voter);
    current_delegated_votes.pending_inactive_shares &#61; current_delegated_votes.pending_inactive_shares &#43; new_shares;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_update_governanace_records_for_redeem_active_shares"></a>

## Function `update_governanace_records_for_redeem_active_shares`



<pre><code>fun update_governanace_records_for_redeem_active_shares(pool: &amp;delegation_pool::DelegationPool, pool_address: address, shares_to_redeem: u128, shareholder: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_governanace_records_for_redeem_active_shares(
    pool: &amp;DelegationPool, pool_address: address, shares_to_redeem: u128, shareholder: address
) acquires GovernanceRecords &#123;
    // &lt;active shares&gt; of &lt;shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt; &#45;&#45;&#45;&#45;&gt;
    // &lt;active shares&gt; of &lt;current voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;
    // &lt;active shares&gt; of &lt;next voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;
    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);
    let vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(
        pool,
        governance_records,
        shareholder
    );
    let current_voter &#61; vote_delegation.voter;
    let pending_voter &#61; vote_delegation.pending_voter;
    let current_delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, current_voter);
    current_delegated_votes.active_shares &#61; current_delegated_votes.active_shares &#45; shares_to_redeem;
    if (current_voter &#61;&#61; pending_voter) &#123;
        current_delegated_votes.active_shares_next_lockup &#61;
            current_delegated_votes.active_shares_next_lockup &#45; shares_to_redeem;
    &#125; else &#123;
        let pending_delegated_votes &#61;
            update_and_borrow_mut_delegated_votes(pool, governance_records, pending_voter);
        pending_delegated_votes.active_shares_next_lockup &#61;
            pending_delegated_votes.active_shares_next_lockup &#45; shares_to_redeem;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares"></a>

## Function `update_governanace_records_for_redeem_pending_inactive_shares`



<pre><code>fun update_governanace_records_for_redeem_pending_inactive_shares(pool: &amp;delegation_pool::DelegationPool, pool_address: address, shares_to_redeem: u128, shareholder: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_governanace_records_for_redeem_pending_inactive_shares(
    pool: &amp;DelegationPool, pool_address: address, shares_to_redeem: u128, shareholder: address
) acquires GovernanceRecords &#123;
    // &lt;pending inactive shares&gt; of &lt;shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;  &#45;&#45;&#45;&#45;&gt;
    // &lt;pending inactive shares&gt; of &lt;current voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;
    // no impact on &lt;pending inactive shares&gt; of &lt;next voter of shareholder&gt;
    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);
    let current_voter &#61; calculate_and_update_delegator_voter_internal(pool, governance_records, shareholder);
    let current_delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, current_voter);
    current_delegated_votes.pending_inactive_shares &#61; current_delegated_votes.pending_inactive_shares &#45; shares_to_redeem;
&#125;
</code></pre>



</details>

<a id="0x1_delegation_pool_multiply_then_divide"></a>

## Function `multiply_then_divide`

Deprecated, prefer math64::mul_div


<pre><code>&#35;[deprecated]
public fun multiply_then_divide(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multiply_then_divide(x: u64, y: u64, z: u64): u64 &#123;
    math64::mul_div(x, y, z)
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


<pre><code>pragma verify&#61;false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
