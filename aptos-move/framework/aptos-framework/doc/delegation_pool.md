
<a id="0x1_delegation_pool"></a>

# Module `0x1::delegation_pool`

<br/>Allow multiple delegators to participate in the same stake pool in order to collect the minimum<br/>stake required to join the validator set. Delegators are rewarded out of the validator rewards<br/>proportionally to their stake and provided the same stake&#45;management API as the stake pool owner.<br/><br/>The main accounting logic in the delegation pool contract handles the following:<br/>1. Tracks how much stake each delegator owns, privately deposited as well as earned.<br/>Accounting individual delegator stakes is achieved through the shares&#45;based pool defined at
&lt;code&gt;aptos_std::pool_u64&lt;/code&gt;, hence delegators own shares rather than absolute stakes into the delegation pool.<br/>2. Tracks rewards earned by the stake pool, implicitly by the delegation one, in the meantime<br/>and distribute them accordingly.<br/>3. Tracks lockup cycles on the stake pool in order to separate inactive stake (not earning rewards)<br/>from pending_inactive stake (earning rewards) and allow its delegators to withdraw the former.<br/>4. Tracks how much commission fee has to be paid to the operator out of incoming rewards before<br/>distributing them to the internal pool_u64 pools.<br/><br/>In order to distinguish between stakes in different states and route rewards accordingly,<br/>separate pool_u64 pools are used for individual stake states:<br/>1. one of &lt;code&gt;active&lt;/code&gt; &#43; &lt;code&gt;pending_active&lt;/code&gt; stake<br/>2. one of &lt;code&gt;inactive&lt;/code&gt; stake FOR each past observed lockup cycle (OLC) on the stake pool<br/>3. one of &lt;code&gt;pending_inactive&lt;/code&gt; stake scheduled during this ongoing OLC<br/><br/>As stake&#45;state transitions and rewards are computed only at the stake pool level, the delegation pool<br/>gets outdated. To mitigate this, at any interaction with the delegation pool, a process of synchronization<br/>to the underlying stake pool is executed before the requested operation itself.<br/><br/>At synchronization:<br/> &#45; stake deviations between the two pools are actually the rewards produced in the meantime.<br/> &#45; the commission fee is extracted from the rewards, the remaining stake is distributed to the internal<br/>pool_u64 pools and then the commission stake used to buy shares for operator.<br/> &#45; if detecting that the lockup expired on the stake pool, the delegation pool will isolate its<br/>pending_inactive stake (now inactive) and create a new pool_u64 to host future pending_inactive stake<br/>scheduled this newly started lockup.<br/>Detecting a lockup expiration on the stake pool resumes to detecting new inactive stake.<br/><br/>Accounting main invariants:<br/> &#45; each stake&#45;management operation (add/unlock/reactivate/withdraw) and operator change triggers<br/>the synchronization process before executing its own function.<br/> &#45; each OLC maps to one or more real lockups on the stake pool, but not the opposite. Actually, only a real<br/>lockup with &apos;activity&apos; (which inactivated some unlocking stake) triggers the creation of a new OLC.<br/> &#45; unlocking and/or unlocked stake originating from different real lockups are never mixed together into<br/>the same pool_u64. This invalidates the accounting of which rewards belong to whom.<br/> &#45; no delegator can have unlocking and/or unlocked stake (pending withdrawals) in different OLCs. This ensures<br/>delegators do not have to keep track of the OLCs when they unlocked. When creating a new pending withdrawal,<br/>the existing one is executed (withdrawn) if is already inactive.<br/> &#45; &lt;code&gt;add_stake&lt;/code&gt; fees are always refunded, but only after the epoch when they have been charged ends.<br/> &#45; withdrawing pending_inactive stake (when validator had gone inactive before its lockup expired)<br/>does not inactivate any stake additional to the requested one to ensure OLC would not advance indefinitely.<br/> &#45; the pending withdrawal exists at an OLC iff delegator owns some shares within the shares pool of that OLC.<br/><br/>Example flow:<br/>&lt;ol&gt;<br/>&lt;li&gt;A node operator creates a delegation pool by calling
&lt;code&gt;initialize_delegation_pool&lt;/code&gt; and sets<br/>its commission fee to 0% (for simplicity). A stake pool is created with no initial stake and owned by<br/>a resource account controlled by the delegation pool.&lt;/li&gt;<br/>&lt;li&gt;Delegator A adds 100 stake which is converted to 100 shares into the active pool_u64&lt;/li&gt;<br/>&lt;li&gt;Operator joins the validator set as the stake pool has now the minimum stake&lt;/li&gt;<br/>&lt;li&gt;The stake pool earned rewards and now has 200 active stake. A&apos;s active shares are worth 200 coins as<br/>the commission fee is 0%.&lt;/li&gt;<br/>&lt;li&gt;&lt;/li&gt;<br/>&lt;ol&gt;<br/>    &lt;li&gt;A requests &lt;code&gt;unlock&lt;/code&gt; for 100 stake&lt;/li&gt;<br/>    &lt;li&gt;Synchronization detects 200 &#45; 100 active rewards which are entirely (0% commission) added to the active pool.&lt;/li&gt;<br/>    &lt;li&gt;100 coins &#61; (100 &#42; 100) / 200 &#61; 50 shares are redeemed from the active pool and exchanged for 100 shares<br/>into the pending_inactive one on A&apos;s behalf&lt;/li&gt;<br/>&lt;/ol&gt;<br/>&lt;li&gt;Delegator B adds 200 stake which is converted to (200 &#42; 50) / 100 &#61; 100 shares into the active pool&lt;/li&gt;<br/>&lt;li&gt;The stake pool earned rewards and now has 600 active and 200 pending_inactive stake.&lt;/li&gt;<br/>&lt;li&gt;&lt;/li&gt;<br/>&lt;ol&gt;<br/>    &lt;li&gt;A requests &lt;code&gt;reactivate_stake&lt;/code&gt; for 100 stake&lt;/li&gt;<br/>    &lt;li&gt;<br/>    Synchronization detects 600 &#45; 300 active and 200 &#45; 100 pending_inactive rewards which are both entirely<br/>    distributed to their corresponding pools
&lt;/li&gt;<br/>    &lt;li&gt;<br/>    100 coins &#61; (100 &#42; 100) / 200 &#61; 50 shares are redeemed from the pending_inactive pool and exchanged for
(100 &#42; 150) / 600 &#61; 25 shares into the active one on A&apos;s behalf
&lt;/li&gt;<br/>&lt;/ol&gt;<br/>&lt;li&gt;The lockup expires on the stake pool, inactivating the entire pending_inactive stake&lt;/li&gt;<br/>&lt;li&gt;&lt;/li&gt;<br/>&lt;ol&gt;<br/>    &lt;li&gt;B requests &lt;code&gt;unlock&lt;/code&gt; for 100 stake&lt;/li&gt;<br/>    &lt;li&gt;<br/>    Synchronization detects no active or pending_inactive rewards, but 0 &#45;&gt; 100 inactive stake on the stake pool,<br/>    so it advances the observed lockup cycle and creates a pool_u64 for the new lockup, hence allowing previous<br/>    pending_inactive shares to be redeemed&lt;/li&gt;<br/>    &lt;li&gt;<br/>    100 coins &#61; (100 &#42; 175) / 700 &#61; 25 shares are redeemed from the active pool and exchanged for 100 shares<br/>    into the new pending_inactive one on B&apos;s behalf
&lt;/li&gt;<br/>&lt;/ol&gt;<br/>&lt;li&gt;The stake pool earned rewards and now has some pending_inactive rewards.&lt;/li&gt;<br/>&lt;li&gt;&lt;/li&gt;<br/>&lt;ol&gt;<br/>    &lt;li&gt;A requests &lt;code&gt;withdraw&lt;/code&gt; for its entire inactive stake&lt;/li&gt;<br/>    &lt;li&gt;<br/>    Synchronization detects no new inactive stake, but some pending_inactive rewards which are distributed<br/>    to the (2nd) pending_inactive pool
&lt;/li&gt;<br/>    &lt;li&gt;<br/>    A&apos;s 50 shares &#61; (50 &#42; 100) / 50 &#61; 100 coins are redeemed from the (1st) inactive pool and 100 stake is<br/>    transferred to A
&lt;/li&gt;<br/>&lt;/ol&gt;<br/>&lt;/ol&gt;<br/>


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


<pre><code>use 0x1::account;<br/>use 0x1::aptos_account;<br/>use 0x1::aptos_coin;<br/>use 0x1::aptos_governance;<br/>use 0x1::coin;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::pool_u64_unbound;<br/>use 0x1::signer;<br/>use 0x1::smart_table;<br/>use 0x1::stake;<br/>use 0x1::staking_config;<br/>use 0x1::table;<br/>use 0x1::table_with_length;<br/>use 0x1::timestamp;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_delegation_pool_DelegationPoolOwnership"></a>

## Resource `DelegationPoolOwnership`

Capability that represents ownership over privileged operations on the underlying stake pool.


<pre><code>struct DelegationPoolOwnership has store, key<br/></code></pre>



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



<pre><code>struct ObservedLockupCycle has copy, drop, store<br/></code></pre>



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



<pre><code>struct DelegationPool has key<br/></code></pre>



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



<pre><code>struct VotingRecordKey has copy, drop, store<br/></code></pre>



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


<pre><code>struct VoteDelegation has copy, drop, store<br/></code></pre>



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


<pre><code>struct DelegatedVotes has copy, drop, store<br/></code></pre>



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

Track governance information of a delegation(e.g. voter delegation/voting power calculation).<br/> This struct should be stored in the delegation pool resource account.


<pre><code>struct GovernanceRecords has key<br/></code></pre>



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



<pre><code>struct BeneficiaryForOperator has key<br/></code></pre>



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



<pre><code>struct NextCommissionPercentage has key<br/></code></pre>



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

Tracks a delegation pool&apos;s allowlist of delegators.<br/> If allowlisting is enabled, existing delegators are not implicitly allowlisted and they can be individually<br/> evicted later by the pool owner.


<pre><code>struct DelegationPoolAllowlisting has key<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct DistributeCommissionEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct DistributeCommission has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct Vote has drop, store<br/></code></pre>



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



<pre><code>struct VoteEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct CreateProposal has drop, store<br/></code></pre>



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



<pre><code>struct CreateProposalEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct DelegateVotingPower has drop, store<br/></code></pre>



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



<pre><code>struct DelegateVotingPowerEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct SetBeneficiaryForOperator has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct CommissionPercentageChange has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct EnableDelegatorsAllowlisting has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct DisableDelegatorsAllowlisting has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct AllowlistDelegator has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct RemoveDelegatorFromAllowlist has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct EvictDelegator has drop, store<br/></code></pre>



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



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_delegation_pool_EDEPRECATED_FUNCTION"></a>

Function is deprecated.


<pre><code>const EDEPRECATED_FUNCTION: u64 &#61; 12;<br/></code></pre>



<a id="0x1_delegation_pool_EDISABLED_FUNCTION"></a>

The function is disabled or hasn&apos;t been enabled.


<pre><code>const EDISABLED_FUNCTION: u64 &#61; 13;<br/></code></pre>



<a id="0x1_delegation_pool_ENOT_OPERATOR"></a>

The account is not the operator of the stake pool.


<pre><code>const ENOT_OPERATOR: u64 &#61; 18;<br/></code></pre>



<a id="0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS"></a>

Account is already owning a delegation pool.


<pre><code>const EOWNER_CAP_ALREADY_EXISTS: u64 &#61; 2;<br/></code></pre>



<a id="0x1_delegation_pool_EOWNER_CAP_NOT_FOUND"></a>

Delegation pool owner capability does not exist at the provided account.


<pre><code>const EOWNER_CAP_NOT_FOUND: u64 &#61; 1;<br/></code></pre>



<a id="0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code>const VALIDATOR_STATUS_INACTIVE: u64 &#61; 4;<br/></code></pre>



<a id="0x1_delegation_pool_EINSUFFICIENT_PROPOSER_STAKE"></a>

The voter does not have sufficient stake to create a proposal.


<pre><code>const EINSUFFICIENT_PROPOSER_STAKE: u64 &#61; 15;<br/></code></pre>



<a id="0x1_delegation_pool_ENO_VOTING_POWER"></a>

The voter does not have any voting power on this proposal.


<pre><code>const ENO_VOTING_POWER: u64 &#61; 16;<br/></code></pre>



<a id="0x1_delegation_pool_EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING"></a>

The stake pool has already voted on the proposal before enabling partial governance voting on this delegation pool.


<pre><code>const EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING: u64 &#61; 17;<br/></code></pre>



<a id="0x1_delegation_pool_ECANNOT_EVICT_ALLOWLISTED_DELEGATOR"></a>

Cannot evict an allowlisted delegator, should remove them from the allowlist first.


<pre><code>const ECANNOT_EVICT_ALLOWLISTED_DELEGATOR: u64 &#61; 26;<br/></code></pre>



<a id="0x1_delegation_pool_ECANNOT_UNLOCK_NULL_SHAREHOLDER"></a>

Cannot unlock the accumulated active stake of NULL_SHAREHOLDER(0x0).


<pre><code>const ECANNOT_UNLOCK_NULL_SHAREHOLDER: u64 &#61; 27;<br/></code></pre>



<a id="0x1_delegation_pool_ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED"></a>

Changing operator commission rate in delegation pool is not supported.


<pre><code>const ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED: u64 &#61; 22;<br/></code></pre>



<a id="0x1_delegation_pool_EDELEGATION_POOLS_DISABLED"></a>

Creating delegation pools is not enabled yet.


<pre><code>const EDELEGATION_POOLS_DISABLED: u64 &#61; 10;<br/></code></pre>



<a id="0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST"></a>

Delegation pool does not exist at the provided pool address.


<pre><code>const EDELEGATION_POOL_DOES_NOT_EXIST: u64 &#61; 3;<br/></code></pre>



<a id="0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_ENABLED"></a>

Delegators allowlisting should be enabled to perform this operation.


<pre><code>const EDELEGATORS_ALLOWLISTING_NOT_ENABLED: u64 &#61; 24;<br/></code></pre>



<a id="0x1_delegation_pool_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED"></a>

Delegators allowlisting is not supported.


<pre><code>const EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED: u64 &#61; 23;<br/></code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW"></a>

Delegator&apos;s active balance cannot be less than <code>MIN_COINS_ON_SHARES_POOL</code>.


<pre><code>const EDELEGATOR_ACTIVE_BALANCE_TOO_LOW: u64 &#61; 8;<br/></code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_NOT_ALLOWLISTED"></a>

Cannot add/reactivate stake unless being allowlisted by the pool owner.


<pre><code>const EDELEGATOR_NOT_ALLOWLISTED: u64 &#61; 25;<br/></code></pre>



<a id="0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW"></a>

Delegator&apos;s pending_inactive balance cannot be less than <code>MIN_COINS_ON_SHARES_POOL</code>.


<pre><code>const EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW: u64 &#61; 9;<br/></code></pre>



<a id="0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE"></a>

Commission percentage has to be between 0 and <code>MAX_FEE</code> &#45; 100%.


<pre><code>const EINVALID_COMMISSION_PERCENTAGE: u64 &#61; 5;<br/></code></pre>



<a id="0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK"></a>

There is not enough <code>active</code> stake on the stake pool to <code>unlock</code>.


<pre><code>const ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK: u64 &#61; 6;<br/></code></pre>



<a id="0x1_delegation_pool_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED"></a>

Changing beneficiaries for operators is not supported.


<pre><code>const EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED: u64 &#61; 19;<br/></code></pre>



<a id="0x1_delegation_pool_EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED"></a>

Partial governance voting hasn&apos;t been enabled on this delegation pool.


<pre><code>const EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED: u64 &#61; 14;<br/></code></pre>



<a id="0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS"></a>

There is a pending withdrawal to be executed before <code>unlock</code>ing any new stake.


<pre><code>const EPENDING_WITHDRAWAL_EXISTS: u64 &#61; 4;<br/></code></pre>



<a id="0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC"></a>

Slashing (if implemented) should not be applied to already <code>inactive</code> stake.<br/> Not only it invalidates the accounting of past observed lockup cycles (OLC),<br/> but is also unfair to delegators whose stake has been inactive before validator started misbehaving.<br/> Additionally, the inactive stake does not count on the voting power of validator.


<pre><code>const ESLASHED_INACTIVE_STAKE_ON_PAST_OLC: u64 &#61; 7;<br/></code></pre>



<a id="0x1_delegation_pool_ETOO_LARGE_COMMISSION_INCREASE"></a>

Commission percentage increase is too large.


<pre><code>const ETOO_LARGE_COMMISSION_INCREASE: u64 &#61; 20;<br/></code></pre>



<a id="0x1_delegation_pool_ETOO_LATE_COMMISSION_CHANGE"></a>

Commission percentage change is too late in this lockup period, and should be done at least a quarter (1/4) of the lockup duration before the lockup cycle ends.


<pre><code>const ETOO_LATE_COMMISSION_CHANGE: u64 &#61; 21;<br/></code></pre>



<a id="0x1_delegation_pool_EWITHDRAW_ZERO_STAKE"></a>

Cannot request to withdraw zero stake.


<pre><code>const EWITHDRAW_ZERO_STAKE: u64 &#61; 11;<br/></code></pre>



<a id="0x1_delegation_pool_MAX_COMMISSION_INCREASE"></a>

Maximum commission percentage increase per lockup cycle. 10% is represented as 1000.


<pre><code>const MAX_COMMISSION_INCREASE: u64 &#61; 1000;<br/></code></pre>



<a id="0x1_delegation_pool_MAX_FEE"></a>

Maximum operator percentage fee(of double digit precision): 22.85% is represented as 2285


<pre><code>const MAX_FEE: u64 &#61; 10000;<br/></code></pre>



<a id="0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL"></a>

Minimum coins to exist on a shares pool at all times.<br/> Enforced per delegator for both active and pending_inactive pools.<br/> This constraint ensures the share price cannot overly increase and lead to<br/> substantial losses when buying shares (can lose at most 1 share which may<br/> be worth a lot if current share price is high).<br/> This constraint is not enforced on inactive pools as they only allow redeems
(can lose at most 1 coin regardless of current share price).


<pre><code>const MIN_COINS_ON_SHARES_POOL: u64 &#61; 1000000000;<br/></code></pre>



<a id="0x1_delegation_pool_MODULE_SALT"></a>



<pre><code>const MODULE_SALT: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 100, 101, 108, 101, 103, 97, 116, 105, 111, 110, 95, 112, 111, 111, 108];<br/></code></pre>



<a id="0x1_delegation_pool_NULL_SHAREHOLDER"></a>

Special shareholder temporarily owning the <code>add_stake</code> fees charged during this epoch.<br/> On each <code>add_stake</code> operation any resulted fee is used to buy active shares for this shareholder.<br/> First synchronization after this epoch ends will distribute accumulated fees to the rest of the pool as refunds.


<pre><code>const NULL_SHAREHOLDER: address &#61; 0x0;<br/></code></pre>



<a id="0x1_delegation_pool_SHARES_SCALING_FACTOR"></a>

Scaling factor of shares pools used within the delegation pool


<pre><code>const SHARES_SCALING_FACTOR: u64 &#61; 10000000000000000;<br/></code></pre>



<a id="0x1_delegation_pool_owner_cap_exists"></a>

## Function `owner_cap_exists`

Return whether supplied address <code>addr</code> is owner of a delegation pool.


<pre><code>&#35;[view]<br/>public fun owner_cap_exists(addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun owner_cap_exists(addr: address): bool &#123;<br/>    exists&lt;DelegationPoolOwnership&gt;(addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_owned_pool_address"></a>

## Function `get_owned_pool_address`

Return address of the delegation pool owned by <code>owner</code> or fail if there is none.


<pre><code>&#35;[view]<br/>public fun get_owned_pool_address(owner: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_owned_pool_address(owner: address): address acquires DelegationPoolOwnership &#123;<br/>    assert_owner_cap_exists(owner);<br/>    borrow_global&lt;DelegationPoolOwnership&gt;(owner).pool_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_delegation_pool_exists"></a>

## Function `delegation_pool_exists`

Return whether a delegation pool exists at supplied address <code>addr</code>.


<pre><code>&#35;[view]<br/>public fun delegation_pool_exists(addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegation_pool_exists(addr: address): bool &#123;<br/>    exists&lt;DelegationPool&gt;(addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_partial_governance_voting_enabled"></a>

## Function `partial_governance_voting_enabled`

Return whether a delegation pool has already enabled partial governance voting.


<pre><code>&#35;[view]<br/>public fun partial_governance_voting_enabled(pool_address: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun partial_governance_voting_enabled(pool_address: address): bool &#123;<br/>    exists&lt;GovernanceRecords&gt;(pool_address) &amp;&amp; stake::get_delegated_voter(pool_address) &#61;&#61; pool_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_observed_lockup_cycle"></a>

## Function `observed_lockup_cycle`

Return the index of current observed lockup cycle on delegation pool <code>pool_address</code>.


<pre><code>&#35;[view]<br/>public fun observed_lockup_cycle(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun observed_lockup_cycle(pool_address: address): u64 acquires DelegationPool &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    borrow_global&lt;DelegationPool&gt;(pool_address).observed_lockup_cycle.index<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_is_next_commission_percentage_effective"></a>

## Function `is_next_commission_percentage_effective`

Return whether the commission percentage for the next lockup cycle is effective.


<pre><code>&#35;[view]<br/>public fun is_next_commission_percentage_effective(pool_address: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_next_commission_percentage_effective(pool_address: address): bool acquires NextCommissionPercentage &#123;<br/>    exists&lt;NextCommissionPercentage&gt;(pool_address) &amp;&amp;<br/>        timestamp::now_seconds() &gt;&#61; borrow_global&lt;NextCommissionPercentage&gt;(pool_address).effective_after_secs<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the operator commission percentage set on the delegation pool <code>pool_address</code>.


<pre><code>&#35;[view]<br/>public fun operator_commission_percentage(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator_commission_percentage(<br/>    pool_address: address<br/>): u64 acquires DelegationPool, NextCommissionPercentage &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    if (is_next_commission_percentage_effective(pool_address)) &#123;<br/>        operator_commission_percentage_next_lockup_cycle(pool_address)<br/>    &#125; else &#123;<br/>        borrow_global&lt;DelegationPool&gt;(pool_address).operator_commission_percentage<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_operator_commission_percentage_next_lockup_cycle"></a>

## Function `operator_commission_percentage_next_lockup_cycle`

Return the operator commission percentage for the next lockup cycle.


<pre><code>&#35;[view]<br/>public fun operator_commission_percentage_next_lockup_cycle(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator_commission_percentage_next_lockup_cycle(<br/>    pool_address: address<br/>): u64 acquires DelegationPool, NextCommissionPercentage &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    if (exists&lt;NextCommissionPercentage&gt;(pool_address)) &#123;<br/>        borrow_global&lt;NextCommissionPercentage&gt;(pool_address).commission_percentage_next_lockup_cycle<br/>    &#125; else &#123;<br/>        borrow_global&lt;DelegationPool&gt;(pool_address).operator_commission_percentage<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_shareholders_count_active_pool"></a>

## Function `shareholders_count_active_pool`

Return the number of delegators owning active stake within <code>pool_address</code>.


<pre><code>&#35;[view]<br/>public fun shareholders_count_active_pool(pool_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shareholders_count_active_pool(pool_address: address): u64 acquires DelegationPool &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    pool_u64::shareholders_count(&amp;borrow_global&lt;DelegationPool&gt;(pool_address).active_shares)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_delegation_pool_stake"></a>

## Function `get_delegation_pool_stake`

Return the stake amounts on <code>pool_address</code> in the different states:<br/> (<code>active</code>,<code>inactive</code>,<code>pending_active</code>,<code>pending_inactive</code>)


<pre><code>&#35;[view]<br/>public fun get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64) &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    stake::get_stake(pool_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_pending_withdrawal"></a>

## Function `get_pending_withdrawal`

Return whether the given delegator has any withdrawable stake. If they recently requested to unlock<br/> some stake and the stake pool&apos;s lockup cycle has not ended, their coins are not withdrawable yet.


<pre><code>&#35;[view]<br/>public fun get_pending_withdrawal(pool_address: address, delegator_address: address): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_pending_withdrawal(<br/>    pool_address: address,<br/>    delegator_address: address<br/>): (bool, u64) acquires DelegationPool &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);<br/>    let (<br/>        lockup_cycle_ended,<br/>        _,<br/>        pending_inactive,<br/>        _,<br/>        commission_pending_inactive<br/>    ) &#61; calculate_stake_pool_drift(pool);<br/><br/>    let (withdrawal_exists, withdrawal_olc) &#61; pending_withdrawal_exists(pool, delegator_address);<br/>    if (!withdrawal_exists) &#123;<br/>        // if no pending withdrawal, there is neither inactive nor pending_inactive stake
        (false, 0)<br/>    &#125; else &#123;<br/>        // delegator has either inactive or pending_inactive stake due to automatic withdrawals<br/>        let inactive_shares &#61; table::borrow(&amp;pool.inactive_shares, withdrawal_olc);<br/>        if (withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) &#123;<br/>            // if withdrawal&apos;s lockup cycle ended on delegation pool then it is inactive
            (true, pool_u64::balance(inactive_shares, delegator_address))<br/>        &#125; else &#123;<br/>            pending_inactive &#61; pool_u64::shares_to_amount_with_total_coins(<br/>                inactive_shares,<br/>                pool_u64::shares(inactive_shares, delegator_address),<br/>                // exclude operator pending_inactive rewards not converted to shares yet<br/>                pending_inactive &#45; commission_pending_inactive<br/>            );<br/>            // if withdrawal&apos;s lockup cycle ended ONLY on stake pool then it is also inactive
            (lockup_cycle_ended, pending_inactive)<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_stake"></a>

## Function `get_stake`

Return total stake owned by <code>delegator_address</code> within delegation pool <code>pool_address</code><br/> in each of its individual states: (<code>active</code>,<code>inactive</code>,<code>pending_inactive</code>)


<pre><code>&#35;[view]<br/>public fun get_stake(pool_address: address, delegator_address: address): (u64, u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_stake(<br/>    pool_address: address,<br/>    delegator_address: address<br/>): (u64, u64, u64) acquires DelegationPool, BeneficiaryForOperator &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);<br/>    let (<br/>        lockup_cycle_ended,<br/>        active,<br/>        _,<br/>        commission_active,<br/>        commission_pending_inactive<br/>    ) &#61; calculate_stake_pool_drift(pool);<br/><br/>    let total_active_shares &#61; pool_u64::total_shares(&amp;pool.active_shares);<br/>    let delegator_active_shares &#61; pool_u64::shares(&amp;pool.active_shares, delegator_address);<br/><br/>    let (_, _, pending_active, _) &#61; stake::get_stake(pool_address);<br/>    if (pending_active &#61;&#61; 0) &#123;<br/>        // zero `pending_active` stake indicates that either there are no `add_stake` fees or<br/>        // previous epoch has ended and should identify shares owning these fees as released<br/>        total_active_shares &#61; total_active_shares &#45; pool_u64::shares(&amp;pool.active_shares, NULL_SHAREHOLDER);<br/>        if (delegator_address &#61;&#61; NULL_SHAREHOLDER) &#123;<br/>            delegator_active_shares &#61; 0<br/>        &#125;<br/>    &#125;;<br/>    active &#61; pool_u64::shares_to_amount_with_total_stats(<br/>        &amp;pool.active_shares,<br/>        delegator_active_shares,<br/>        // exclude operator active rewards not converted to shares yet<br/>        active &#45; commission_active,<br/>        total_active_shares<br/>    );<br/><br/>    // get state and stake (0 if there is none) of the pending withdrawal<br/>    let (withdrawal_inactive, withdrawal_stake) &#61; get_pending_withdrawal(pool_address, delegator_address);<br/>    // report non&#45;active stakes accordingly to the state of the pending withdrawal<br/>    let (inactive, pending_inactive) &#61; if (withdrawal_inactive) (withdrawal_stake, 0) else (0, withdrawal_stake);<br/><br/>    // should also include commission rewards in case of the operator account<br/>    // operator rewards are actually used to buy shares which is introducing<br/>    // some imprecision (received stake would be slightly less)<br/>    // but adding rewards onto the existing stake is still a good approximation<br/>    if (delegator_address &#61;&#61; beneficiary_for_operator(get_operator(pool_address))) &#123;<br/>        active &#61; active &#43; commission_active;<br/>        // in&#45;flight pending_inactive commission can coexist with already inactive withdrawal<br/>        if (lockup_cycle_ended) &#123;<br/>            inactive &#61; inactive &#43; commission_pending_inactive<br/>        &#125; else &#123;<br/>            pending_inactive &#61; pending_inactive &#43; commission_pending_inactive<br/>        &#125;<br/>    &#125;;<br/><br/>    (active, inactive, pending_inactive)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_add_stake_fee"></a>

## Function `get_add_stake_fee`

Return refundable stake to be extracted from added <code>amount</code> at <code>add_stake</code> operation on pool <code>pool_address</code>.<br/> If the validator produces rewards this epoch, added stake goes directly to <code>pending_active</code> and<br/> does not earn rewards. However, all shares within a pool appreciate uniformly and when this epoch ends:<br/> &#45; either added shares are still <code>pending_active</code> and steal from rewards of existing <code>active</code> stake<br/> &#45; or have moved to <code>pending_inactive</code> and get full rewards (they displaced <code>active</code> stake at <code>unlock</code>)<br/> To mitigate this, some of the added stake is extracted and fed back into the pool as placeholder<br/> for the rewards the remaining stake would have earned if active:<br/> extracted&#45;fee &#61; (amount &#45; extracted&#45;fee) &#42; reward&#45;rate% &#42; (100% &#45; operator&#45;commission%)


<pre><code>&#35;[view]<br/>public fun get_add_stake_fee(pool_address: address, amount: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_add_stake_fee(<br/>    pool_address: address,<br/>    amount: u64<br/>): u64 acquires DelegationPool, NextCommissionPercentage &#123;<br/>    if (stake::is_current_epoch_validator(pool_address)) &#123;<br/>        let (rewards_rate, rewards_rate_denominator) &#61; staking_config::get_reward_rate(&amp;staking_config::get());<br/>        if (rewards_rate_denominator &gt; 0) &#123;<br/>            assert_delegation_pool_exists(pool_address);<br/><br/>            rewards_rate &#61; rewards_rate &#42; (MAX_FEE &#45; operator_commission_percentage(pool_address));<br/>            rewards_rate_denominator &#61; rewards_rate_denominator &#42; MAX_FEE;<br/>            ((((amount as u128) &#42; (rewards_rate as u128)) / ((rewards_rate as u128) &#43; (rewards_rate_denominator as u128))) as u64)<br/>        &#125; else &#123; 0 &#125;<br/>    &#125; else &#123; 0 &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_can_withdraw_pending_inactive"></a>

## Function `can_withdraw_pending_inactive`

Return whether <code>pending_inactive</code> stake can be directly withdrawn from<br/> the delegation pool, implicitly its stake pool, in the special case<br/> the validator had gone inactive before its lockup expired.


<pre><code>&#35;[view]<br/>public fun can_withdraw_pending_inactive(pool_address: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_withdraw_pending_inactive(pool_address: address): bool &#123;<br/>    stake::get_validator_state(pool_address) &#61;&#61; VALIDATOR_STATUS_INACTIVE &amp;&amp;<br/>        timestamp::now_seconds() &gt;&#61; stake::get_lockup_secs(pool_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_voter_total_voting_power"></a>

## Function `calculate_and_update_voter_total_voting_power`

Return the total voting power of a delegator in a delegation pool. This function syncs DelegationPool to the<br/> latest state.


<pre><code>&#35;[view]<br/>public fun calculate_and_update_voter_total_voting_power(pool_address: address, voter: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun calculate_and_update_voter_total_voting_power(<br/>    pool_address: address,<br/>    voter: address<br/>): u64 acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert_partial_governance_voting_enabled(pool_address);<br/>    // Delegation pool need to be synced to explain rewards(which could change the coin amount) and<br/>    // commission(which could cause share transfer).<br/>    synchronize_delegation_pool(pool_address);<br/>    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);<br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/>    let latest_delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, voter);<br/>    calculate_total_voting_power(pool, latest_delegated_votes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_remaining_voting_power"></a>

## Function `calculate_and_update_remaining_voting_power`

Return the remaining voting power of a delegator in a delegation pool on a proposal. This function syncs DelegationPool to the<br/> latest state.


<pre><code>&#35;[view]<br/>public fun calculate_and_update_remaining_voting_power(pool_address: address, voter_address: address, proposal_id: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun calculate_and_update_remaining_voting_power(<br/>    pool_address: address,<br/>    voter_address: address,<br/>    proposal_id: u64<br/>): u64 acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert_partial_governance_voting_enabled(pool_address);<br/>    // If the whole stake pool has no voting power(e.g. it has already voted before partial<br/>    // governance voting flag is enabled), the delegator also has no voting power.<br/>    if (aptos_governance::get_remaining_voting_power(pool_address, proposal_id) &#61;&#61; 0) &#123;<br/>        return 0<br/>    &#125;;<br/><br/>    let total_voting_power &#61; calculate_and_update_voter_total_voting_power(pool_address, voter_address);<br/>    let governance_records &#61; borrow_global&lt;GovernanceRecords&gt;(pool_address);<br/>    total_voting_power &#45; get_used_voting_power(governance_records, voter_address, proposal_id)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegator_voter"></a>

## Function `calculate_and_update_delegator_voter`

Return the latest delegated voter of a delegator in a delegation pool. This function syncs DelegationPool to the<br/> latest state.


<pre><code>&#35;[view]<br/>public fun calculate_and_update_delegator_voter(pool_address: address, delegator_address: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun calculate_and_update_delegator_voter(<br/>    pool_address: address,<br/>    delegator_address: address<br/>): address acquires DelegationPool, GovernanceRecords &#123;<br/>    assert_partial_governance_voting_enabled(pool_address);<br/>    calculate_and_update_delegator_voter_internal(<br/>        borrow_global&lt;DelegationPool&gt;(pool_address),<br/>        borrow_global_mut&lt;GovernanceRecords&gt;(pool_address),<br/>        delegator_address<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_voting_delegation"></a>

## Function `calculate_and_update_voting_delegation`

Return the current state of a voting delegation of a delegator in a delegation pool.


<pre><code>&#35;[view]<br/>public fun calculate_and_update_voting_delegation(pool_address: address, delegator_address: address): (address, address, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun calculate_and_update_voting_delegation(<br/>    pool_address: address,<br/>    delegator_address: address<br/>): (address, address, u64) acquires DelegationPool, GovernanceRecords &#123;<br/>    assert_partial_governance_voting_enabled(pool_address);<br/>    let vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(<br/>        borrow_global&lt;DelegationPool&gt;(pool_address),<br/>        borrow_global_mut&lt;GovernanceRecords&gt;(pool_address),<br/>        delegator_address<br/>    );<br/><br/>    (vote_delegation.voter, vote_delegation.pending_voter, vote_delegation.last_locked_until_secs)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_expected_stake_pool_address"></a>

## Function `get_expected_stake_pool_address`

Return the address of the stake pool to be created with the provided owner, and seed.


<pre><code>&#35;[view]<br/>public fun get_expected_stake_pool_address(owner: address, delegation_pool_creation_seed: vector&lt;u8&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_expected_stake_pool_address(owner: address, delegation_pool_creation_seed: vector&lt;u8&gt;<br/>): address &#123;<br/>    let seed &#61; create_resource_account_seed(delegation_pool_creation_seed);<br/>    account::create_resource_address(&amp;owner, seed)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_min_remaining_secs_for_commission_change"></a>

## Function `min_remaining_secs_for_commission_change`

Return the minimum remaining time in seconds for commission change, which is one fourth of the lockup duration.


<pre><code>&#35;[view]<br/>public fun min_remaining_secs_for_commission_change(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun min_remaining_secs_for_commission_change(): u64 &#123;<br/>    let config &#61; staking_config::get();<br/>    staking_config::get_recurring_lockup_duration(&amp;config) / 4<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_allowlisting_enabled"></a>

## Function `allowlisting_enabled`

Return whether allowlisting is enabled for the provided delegation pool.


<pre><code>&#35;[view]<br/>public fun allowlisting_enabled(pool_address: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun allowlisting_enabled(pool_address: address): bool &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    exists&lt;DelegationPoolAllowlisting&gt;(pool_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_delegator_allowlisted"></a>

## Function `delegator_allowlisted`

Return whether the provided delegator is allowlisted.<br/> A delegator is allowlisted if:<br/> &#45; allowlisting is disabled on the pool<br/> &#45; delegator is part of the allowlist


<pre><code>&#35;[view]<br/>public fun delegator_allowlisted(pool_address: address, delegator_address: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegator_allowlisted(<br/>    pool_address: address,<br/>    delegator_address: address,<br/>): bool acquires DelegationPoolAllowlisting &#123;<br/>    if (!allowlisting_enabled(pool_address)) &#123; return true &#125;;<br/>    smart_table::contains(freeze(borrow_mut_delegators_allowlist(pool_address)), delegator_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_delegators_allowlist"></a>

## Function `get_delegators_allowlist`

Return allowlist or revert if allowlisting is not enabled for the provided delegation pool.


<pre><code>&#35;[view]<br/>public fun get_delegators_allowlist(pool_address: address): vector&lt;address&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegators_allowlist(<br/>    pool_address: address,<br/>): vector&lt;address&gt; acquires DelegationPoolAllowlisting &#123;<br/>    assert_allowlisting_enabled(pool_address);<br/><br/>    let allowlist &#61; vector[];<br/>    smart_table::for_each_ref(freeze(borrow_mut_delegators_allowlist(pool_address)), &#124;delegator, _v&#124; &#123;<br/>        vector::push_back(&amp;mut allowlist, &#42;delegator);<br/>    &#125;);<br/>    allowlist<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_initialize_delegation_pool"></a>

## Function `initialize_delegation_pool`

Initialize a delegation pool of custom fixed <code>operator_commission_percentage</code>.<br/> A resource account is created from <code>owner</code> signer and its supplied <code>delegation_pool_creation_seed</code><br/> to host the delegation pool resource and own the underlying stake pool.<br/> Ownership over setting the operator/voter is granted to <code>owner</code> who has both roles initially.


<pre><code>public entry fun initialize_delegation_pool(owner: &amp;signer, operator_commission_percentage: u64, delegation_pool_creation_seed: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize_delegation_pool(<br/>    owner: &amp;signer,<br/>    operator_commission_percentage: u64,<br/>    delegation_pool_creation_seed: vector&lt;u8&gt;,<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert!(features::delegation_pools_enabled(), error::invalid_state(EDELEGATION_POOLS_DISABLED));<br/>    let owner_address &#61; signer::address_of(owner);<br/>    assert!(!owner_cap_exists(owner_address), error::already_exists(EOWNER_CAP_ALREADY_EXISTS));<br/>    assert!(operator_commission_percentage &lt;&#61; MAX_FEE, error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE));<br/><br/>    // generate a seed to be used to create the resource account hosting the delegation pool<br/>    let seed &#61; create_resource_account_seed(delegation_pool_creation_seed);<br/><br/>    let (stake_pool_signer, stake_pool_signer_cap) &#61; account::create_resource_account(owner, seed);<br/>    coin::register&lt;AptosCoin&gt;(&amp;stake_pool_signer);<br/><br/>    // stake_pool_signer will be owner of the stake pool and have its `stake::OwnerCapability`<br/>    let pool_address &#61; signer::address_of(&amp;stake_pool_signer);<br/>    stake::initialize_stake_owner(&amp;stake_pool_signer, 0, owner_address, owner_address);<br/><br/>    let inactive_shares &#61; table::new&lt;ObservedLockupCycle, pool_u64::Pool&gt;();<br/>    table::add(<br/>        &amp;mut inactive_shares,<br/>        olc_with_index(0),<br/>        pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR)<br/>    );<br/><br/>    move_to(&amp;stake_pool_signer, DelegationPool &#123;<br/>        active_shares: pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR),<br/>        observed_lockup_cycle: olc_with_index(0),<br/>        inactive_shares,<br/>        pending_withdrawals: table::new&lt;address, ObservedLockupCycle&gt;(),<br/>        stake_pool_signer_cap,<br/>        total_coins_inactive: 0,<br/>        operator_commission_percentage,<br/>        add_stake_events: account::new_event_handle&lt;AddStakeEvent&gt;(&amp;stake_pool_signer),<br/>        reactivate_stake_events: account::new_event_handle&lt;ReactivateStakeEvent&gt;(&amp;stake_pool_signer),<br/>        unlock_stake_events: account::new_event_handle&lt;UnlockStakeEvent&gt;(&amp;stake_pool_signer),<br/>        withdraw_stake_events: account::new_event_handle&lt;WithdrawStakeEvent&gt;(&amp;stake_pool_signer),<br/>        distribute_commission_events: account::new_event_handle&lt;DistributeCommissionEvent&gt;(&amp;stake_pool_signer),<br/>    &#125;);<br/><br/>    // save delegation pool ownership and resource account address (inner stake pool address) on `owner`<br/>    move_to(owner, DelegationPoolOwnership &#123; pool_address &#125;);<br/><br/>    // All delegation pool enable partial governance voting by default once the feature flag is enabled.<br/>    if (features::partial_governance_voting_enabled(<br/>    ) &amp;&amp; features::delegation_pool_partial_governance_voting_enabled()) &#123;<br/>        enable_partial_governance_voting(pool_address);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_beneficiary_for_operator"></a>

## Function `beneficiary_for_operator`

Return the beneficiary address of the operator.


<pre><code>&#35;[view]<br/>public fun beneficiary_for_operator(operator: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun beneficiary_for_operator(operator: address): address acquires BeneficiaryForOperator &#123;<br/>    if (exists&lt;BeneficiaryForOperator&gt;(operator)) &#123;<br/>        return borrow_global&lt;BeneficiaryForOperator&gt;(operator).beneficiary_for_operator<br/>    &#125; else &#123;<br/>        operator<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_enable_partial_governance_voting"></a>

## Function `enable_partial_governance_voting`

Enable partial governance voting on a stake pool. The voter of this stake pool will be managed by this module.<br/> The existing voter will be replaced. The function is permissionless.


<pre><code>public entry fun enable_partial_governance_voting(pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun enable_partial_governance_voting(<br/>    pool_address: address,<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert!(features::partial_governance_voting_enabled(), error::invalid_state(EDISABLED_FUNCTION));<br/>    assert!(<br/>        features::delegation_pool_partial_governance_voting_enabled(),<br/>        error::invalid_state(EDISABLED_FUNCTION)<br/>    );<br/>    assert_delegation_pool_exists(pool_address);<br/>    // synchronize delegation and stake pools before any user operation.<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    let delegation_pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);<br/>    let stake_pool_signer &#61; retrieve_stake_pool_owner(delegation_pool);<br/>    // delegated_voter is managed by the stake pool itself, which signer capability is managed by DelegationPool.<br/>    // So voting power of this stake pool can only be used through this module.<br/>    stake::set_delegated_voter(&amp;stake_pool_signer, signer::address_of(&amp;stake_pool_signer));<br/><br/>    move_to(&amp;stake_pool_signer, GovernanceRecords &#123;<br/>        votes: smart_table::new(),<br/>        votes_per_proposal: smart_table::new(),<br/>        vote_delegation: smart_table::new(),<br/>        delegated_votes: smart_table::new(),<br/>        vote_events: account::new_event_handle&lt;VoteEvent&gt;(&amp;stake_pool_signer),<br/>        create_proposal_events: account::new_event_handle&lt;CreateProposalEvent&gt;(&amp;stake_pool_signer),<br/>        delegate_voting_power_events: account::new_event_handle&lt;DelegateVotingPowerEvent&gt;(&amp;stake_pool_signer),<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_vote"></a>

## Function `vote`

Vote on a proposal with a voter&apos;s voting power. To successfully vote, the following conditions must be met:<br/> 1. The voting period of the proposal hasn&apos;t ended.<br/> 2. The delegation pool&apos;s lockup period ends after the voting period of the proposal.<br/> 3. The voter still has spare voting power on this proposal.<br/> 4. The delegation pool never votes on the proposal before enabling partial governance voting.


<pre><code>public entry fun vote(voter: &amp;signer, pool_address: address, proposal_id: u64, voting_power: u64, should_pass: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote(<br/>    voter: &amp;signer,<br/>    pool_address: address,<br/>    proposal_id: u64,<br/>    voting_power: u64,<br/>    should_pass: bool<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert_partial_governance_voting_enabled(pool_address);<br/>    // synchronize delegation and stake pools before any user operation.<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    let voter_address &#61; signer::address_of(voter);<br/>    let remaining_voting_power &#61; calculate_and_update_remaining_voting_power(<br/>        pool_address,<br/>        voter_address,<br/>        proposal_id<br/>    );<br/>    if (voting_power &gt; remaining_voting_power) &#123;<br/>        voting_power &#61; remaining_voting_power;<br/>    &#125;;<br/>    assert!(voting_power &gt; 0, error::invalid_argument(ENO_VOTING_POWER));<br/><br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/>    // Check a edge case during the transient period of enabling partial governance voting.<br/>    assert_and_update_proposal_used_voting_power(governance_records, pool_address, proposal_id, voting_power);<br/>    let used_voting_power &#61; borrow_mut_used_voting_power(governance_records, voter_address, proposal_id);<br/>    &#42;used_voting_power &#61; &#42;used_voting_power &#43; voting_power;<br/><br/>    let pool_signer &#61; retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address));<br/>    aptos_governance::partial_vote(&amp;pool_signer, pool_address, proposal_id, voting_power, should_pass);<br/><br/>    if (features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            Vote &#123;<br/>                voter: voter_address,<br/>                proposal_id,<br/>                delegation_pool: pool_address,<br/>                num_votes: voting_power,<br/>                should_pass,<br/>            &#125;<br/>        );<br/>    &#125;;<br/><br/>    event::emit_event(<br/>        &amp;mut governance_records.vote_events,<br/>        VoteEvent &#123;<br/>            voter: voter_address,<br/>            proposal_id,<br/>            delegation_pool: pool_address,<br/>            num_votes: voting_power,<br/>            should_pass,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_create_proposal"></a>

## Function `create_proposal`

A voter could create a governance proposal by this function. To successfully create a proposal, the voter&apos;s<br/> voting power in THIS delegation pool must be not less than the minimum required voting power specified in<br/> <code>aptos_governance.move</code>.


<pre><code>public entry fun create_proposal(voter: &amp;signer, pool_address: address, execution_hash: vector&lt;u8&gt;, metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;, is_multi_step_proposal: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_proposal(<br/>    voter: &amp;signer,<br/>    pool_address: address,<br/>    execution_hash: vector&lt;u8&gt;,<br/>    metadata_location: vector&lt;u8&gt;,<br/>    metadata_hash: vector&lt;u8&gt;,<br/>    is_multi_step_proposal: bool,<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert_partial_governance_voting_enabled(pool_address);<br/><br/>    // synchronize delegation and stake pools before any user operation<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    let voter_addr &#61; signer::address_of(voter);<br/>    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);<br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/>    let total_voting_power &#61; calculate_and_update_delegated_votes(pool, governance_records, voter_addr);<br/>    assert!(<br/>        total_voting_power &gt;&#61; aptos_governance::get_required_proposer_stake(),<br/>        error::invalid_argument(EINSUFFICIENT_PROPOSER_STAKE));<br/>    let pool_signer &#61; retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address));<br/>    let proposal_id &#61; aptos_governance::create_proposal_v2_impl(<br/>        &amp;pool_signer,<br/>        pool_address,<br/>        execution_hash,<br/>        metadata_location,<br/>        metadata_hash,<br/>        is_multi_step_proposal,<br/>    );<br/><br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/><br/>    if (features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            CreateProposal &#123;<br/>                proposal_id,<br/>                voter: voter_addr,<br/>                delegation_pool: pool_address,<br/>            &#125;<br/>        );<br/>    &#125;;<br/><br/>    event::emit_event(<br/>        &amp;mut governance_records.create_proposal_events,<br/>        CreateProposalEvent &#123;<br/>            proposal_id,<br/>            voter: voter_addr,<br/>            delegation_pool: pool_address,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_assert_owner_cap_exists"></a>

## Function `assert_owner_cap_exists`



<pre><code>fun assert_owner_cap_exists(owner: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_owner_cap_exists(owner: address) &#123;<br/>    assert!(owner_cap_exists(owner), error::not_found(EOWNER_CAP_NOT_FOUND));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_assert_delegation_pool_exists"></a>

## Function `assert_delegation_pool_exists`



<pre><code>fun assert_delegation_pool_exists(pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_delegation_pool_exists(pool_address: address) &#123;<br/>    assert!(delegation_pool_exists(pool_address), error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_assert_min_active_balance"></a>

## Function `assert_min_active_balance`



<pre><code>fun assert_min_active_balance(pool: &amp;delegation_pool::DelegationPool, delegator_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_min_active_balance(pool: &amp;DelegationPool, delegator_address: address) &#123;<br/>    let balance &#61; pool_u64::balance(&amp;pool.active_shares, delegator_address);<br/>    assert!(balance &gt;&#61; MIN_COINS_ON_SHARES_POOL, error::invalid_argument(EDELEGATOR_ACTIVE_BALANCE_TOO_LOW));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_assert_min_pending_inactive_balance"></a>

## Function `assert_min_pending_inactive_balance`



<pre><code>fun assert_min_pending_inactive_balance(pool: &amp;delegation_pool::DelegationPool, delegator_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_min_pending_inactive_balance(pool: &amp;DelegationPool, delegator_address: address) &#123;<br/>    let balance &#61; pool_u64::balance(pending_inactive_shares_pool(pool), delegator_address);<br/>    assert!(<br/>        balance &gt;&#61; MIN_COINS_ON_SHARES_POOL,<br/>        error::invalid_argument(EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW)<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_assert_partial_governance_voting_enabled"></a>

## Function `assert_partial_governance_voting_enabled`



<pre><code>fun assert_partial_governance_voting_enabled(pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_partial_governance_voting_enabled(pool_address: address) &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    assert!(<br/>        partial_governance_voting_enabled(pool_address),<br/>        error::invalid_state(EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED)<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_assert_allowlisting_enabled"></a>

## Function `assert_allowlisting_enabled`



<pre><code>fun assert_allowlisting_enabled(pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_allowlisting_enabled(pool_address: address) &#123;<br/>    assert!(allowlisting_enabled(pool_address), error::invalid_state(EDELEGATORS_ALLOWLISTING_NOT_ENABLED));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_assert_delegator_allowlisted"></a>

## Function `assert_delegator_allowlisted`



<pre><code>fun assert_delegator_allowlisted(pool_address: address, delegator_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_delegator_allowlisted(<br/>    pool_address: address,<br/>    delegator_address: address,<br/>) acquires DelegationPoolAllowlisting &#123;<br/>    assert!(<br/>        delegator_allowlisted(pool_address, delegator_address),<br/>        error::permission_denied(EDELEGATOR_NOT_ALLOWLISTED)<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake"></a>

## Function `coins_to_redeem_to_ensure_min_stake`



<pre><code>fun coins_to_redeem_to_ensure_min_stake(src_shares_pool: &amp;pool_u64_unbound::Pool, shareholder: address, amount: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun coins_to_redeem_to_ensure_min_stake(<br/>    src_shares_pool: &amp;pool_u64::Pool,<br/>    shareholder: address,<br/>    amount: u64,<br/>): u64 &#123;<br/>    // find how many coins would be redeemed if supplying `amount`<br/>    let redeemed_coins &#61; pool_u64::shares_to_amount(<br/>        src_shares_pool,<br/>        amount_to_shares_to_redeem(src_shares_pool, shareholder, amount)<br/>    );<br/>    // if balance drops under threshold then redeem it entirely<br/>    let src_balance &#61; pool_u64::balance(src_shares_pool, shareholder);<br/>    if (src_balance &#45; redeemed_coins &lt; MIN_COINS_ON_SHARES_POOL) &#123;<br/>        amount &#61; src_balance;<br/>    &#125;;<br/>    amount<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake"></a>

## Function `coins_to_transfer_to_ensure_min_stake`



<pre><code>fun coins_to_transfer_to_ensure_min_stake(src_shares_pool: &amp;pool_u64_unbound::Pool, dst_shares_pool: &amp;pool_u64_unbound::Pool, shareholder: address, amount: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun coins_to_transfer_to_ensure_min_stake(<br/>    src_shares_pool: &amp;pool_u64::Pool,<br/>    dst_shares_pool: &amp;pool_u64::Pool,<br/>    shareholder: address,<br/>    amount: u64,<br/>): u64 &#123;<br/>    // find how many coins would be redeemed from source if supplying `amount`<br/>    let redeemed_coins &#61; pool_u64::shares_to_amount(<br/>        src_shares_pool,<br/>        amount_to_shares_to_redeem(src_shares_pool, shareholder, amount)<br/>    );<br/>    // if balance on destination would be less than threshold then redeem difference to threshold<br/>    let dst_balance &#61; pool_u64::balance(dst_shares_pool, shareholder);<br/>    if (dst_balance &#43; redeemed_coins &lt; MIN_COINS_ON_SHARES_POOL) &#123;<br/>        // `redeemed_coins` &gt;&#61; `amount` &#45; 1 as redeem can lose at most 1 coin<br/>        amount &#61; MIN_COINS_ON_SHARES_POOL &#45; dst_balance &#43; 1;<br/>    &#125;;<br/>    // check if new `amount` drops balance on source under threshold and adjust<br/>    coins_to_redeem_to_ensure_min_stake(src_shares_pool, shareholder, amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_retrieve_stake_pool_owner"></a>

## Function `retrieve_stake_pool_owner`

Retrieves the shared resource account owning the stake pool in order<br/> to forward a stake&#45;management operation to this underlying pool.


<pre><code>fun retrieve_stake_pool_owner(pool: &amp;delegation_pool::DelegationPool): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun retrieve_stake_pool_owner(pool: &amp;DelegationPool): signer &#123;<br/>    account::create_signer_with_capability(&amp;pool.stake_pool_signer_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_pool_address"></a>

## Function `get_pool_address`

Get the address of delegation pool reference <code>pool</code>.


<pre><code>fun get_pool_address(pool: &amp;delegation_pool::DelegationPool): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_pool_address(pool: &amp;DelegationPool): address &#123;<br/>    account::get_signer_capability_address(&amp;pool.stake_pool_signer_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_delegator_active_shares"></a>

## Function `get_delegator_active_shares`

Get the active share amount of the delegator.


<pre><code>fun get_delegator_active_shares(pool: &amp;delegation_pool::DelegationPool, delegator: address): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_delegator_active_shares(pool: &amp;DelegationPool, delegator: address): u128 &#123;<br/>    pool_u64::shares(&amp;pool.active_shares, delegator)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_delegator_pending_inactive_shares"></a>

## Function `get_delegator_pending_inactive_shares`

Get the pending inactive share amount of the delegator.


<pre><code>fun get_delegator_pending_inactive_shares(pool: &amp;delegation_pool::DelegationPool, delegator: address): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_delegator_pending_inactive_shares(pool: &amp;DelegationPool, delegator: address): u128 &#123;<br/>    pool_u64::shares(pending_inactive_shares_pool(pool), delegator)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_get_used_voting_power"></a>

## Function `get_used_voting_power`

Get the used voting power of a voter on a proposal.


<pre><code>fun get_used_voting_power(governance_records: &amp;delegation_pool::GovernanceRecords, voter: address, proposal_id: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_used_voting_power(governance_records: &amp;GovernanceRecords, voter: address, proposal_id: u64): u64 &#123;<br/>    let votes &#61; &amp;governance_records.votes;<br/>    let key &#61; VotingRecordKey &#123;<br/>        voter,<br/>        proposal_id,<br/>    &#125;;<br/>    &#42;smart_table::borrow_with_default(votes, key, &amp;0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_create_resource_account_seed"></a>

## Function `create_resource_account_seed`

Create the seed to derive the resource account address.


<pre><code>fun create_resource_account_seed(delegation_pool_creation_seed: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_resource_account_seed(<br/>    delegation_pool_creation_seed: vector&lt;u8&gt;,<br/>): vector&lt;u8&gt; &#123;<br/>    let seed &#61; vector::empty&lt;u8&gt;();<br/>    // include module salt (before any subseeds) to avoid conflicts with other modules creating resource accounts<br/>    vector::append(&amp;mut seed, MODULE_SALT);<br/>    // include an additional salt in case the same resource account has already been created<br/>    vector::append(&amp;mut seed, delegation_pool_creation_seed);<br/>    seed<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_borrow_mut_used_voting_power"></a>

## Function `borrow_mut_used_voting_power`

Borrow the mutable used voting power of a voter on a proposal.


<pre><code>fun borrow_mut_used_voting_power(governance_records: &amp;mut delegation_pool::GovernanceRecords, voter: address, proposal_id: u64): &amp;mut u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_mut_used_voting_power(<br/>    governance_records: &amp;mut GovernanceRecords,<br/>    voter: address,<br/>    proposal_id: u64<br/>): &amp;mut u64 &#123;<br/>    let votes &#61; &amp;mut governance_records.votes;<br/>    let key &#61; VotingRecordKey &#123;<br/>        proposal_id,<br/>        voter,<br/>    &#125;;<br/>    smart_table::borrow_mut_with_default(votes, key, 0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_update_and_borrow_mut_delegator_vote_delegation"></a>

## Function `update_and_borrow_mut_delegator_vote_delegation`

Update VoteDelegation of a delegator to up&#45;to&#45;date then borrow_mut it.


<pre><code>fun update_and_borrow_mut_delegator_vote_delegation(pool: &amp;delegation_pool::DelegationPool, governance_records: &amp;mut delegation_pool::GovernanceRecords, delegator: address): &amp;mut delegation_pool::VoteDelegation<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_and_borrow_mut_delegator_vote_delegation(<br/>    pool: &amp;DelegationPool,<br/>    governance_records: &amp;mut GovernanceRecords,<br/>    delegator: address<br/>): &amp;mut VoteDelegation &#123;<br/>    let pool_address &#61; get_pool_address(pool);<br/>    let locked_until_secs &#61; stake::get_lockup_secs(pool_address);<br/><br/>    let vote_delegation_table &#61; &amp;mut governance_records.vote_delegation;<br/>    // By default, a delegator&apos;s delegated voter is itself.<br/>    // TODO: recycle storage when VoteDelegation equals to default value.<br/>    if (!smart_table::contains(vote_delegation_table, delegator)) &#123;<br/>        return smart_table::borrow_mut_with_default(vote_delegation_table, delegator, VoteDelegation &#123;<br/>            voter: delegator,<br/>            last_locked_until_secs: locked_until_secs,<br/>            pending_voter: delegator,<br/>        &#125;)<br/>    &#125;;<br/><br/>    let vote_delegation &#61; smart_table::borrow_mut(vote_delegation_table, delegator);<br/>    // A lockup period has passed since last time `vote_delegation` was updated. Pending voter takes effect.<br/>    if (vote_delegation.last_locked_until_secs &lt; locked_until_secs) &#123;<br/>        vote_delegation.voter &#61; vote_delegation.pending_voter;<br/>        vote_delegation.last_locked_until_secs &#61; locked_until_secs;<br/>    &#125;;<br/>    vote_delegation<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_update_and_borrow_mut_delegated_votes"></a>

## Function `update_and_borrow_mut_delegated_votes`

Update DelegatedVotes of a voter to up&#45;to&#45;date then borrow_mut it.


<pre><code>fun update_and_borrow_mut_delegated_votes(pool: &amp;delegation_pool::DelegationPool, governance_records: &amp;mut delegation_pool::GovernanceRecords, voter: address): &amp;mut delegation_pool::DelegatedVotes<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_and_borrow_mut_delegated_votes(<br/>    pool: &amp;DelegationPool,<br/>    governance_records: &amp;mut GovernanceRecords,<br/>    voter: address<br/>): &amp;mut DelegatedVotes &#123;<br/>    let pool_address &#61; get_pool_address(pool);<br/>    let locked_until_secs &#61; stake::get_lockup_secs(pool_address);<br/><br/>    let delegated_votes_per_voter &#61; &amp;mut governance_records.delegated_votes;<br/>    // By default, a delegator&apos;s voter is itself.<br/>    // TODO: recycle storage when DelegatedVotes equals to default value.<br/>    if (!smart_table::contains(delegated_votes_per_voter, voter)) &#123;<br/>        let active_shares &#61; get_delegator_active_shares(pool, voter);<br/>        let inactive_shares &#61; get_delegator_pending_inactive_shares(pool, voter);<br/>        return smart_table::borrow_mut_with_default(delegated_votes_per_voter, voter, DelegatedVotes &#123;<br/>            active_shares,<br/>            pending_inactive_shares: inactive_shares,<br/>            active_shares_next_lockup: active_shares,<br/>            last_locked_until_secs: locked_until_secs,<br/>        &#125;)<br/>    &#125;;<br/><br/>    let delegated_votes &#61; smart_table::borrow_mut(delegated_votes_per_voter, voter);<br/>    // A lockup period has passed since last time `delegated_votes` was updated. Pending voter takes effect.<br/>    if (delegated_votes.last_locked_until_secs &lt; locked_until_secs) &#123;<br/>        delegated_votes.active_shares &#61; delegated_votes.active_shares_next_lockup;<br/>        delegated_votes.pending_inactive_shares &#61; 0;<br/>        delegated_votes.last_locked_until_secs &#61; locked_until_secs;<br/>    &#125;;<br/>    delegated_votes<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_olc_with_index"></a>

## Function `olc_with_index`



<pre><code>fun olc_with_index(index: u64): delegation_pool::ObservedLockupCycle<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun olc_with_index(index: u64): ObservedLockupCycle &#123;<br/>    ObservedLockupCycle &#123; index &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_total_voting_power"></a>

## Function `calculate_total_voting_power`

Given the amounts of shares in <code>active_shares</code> pool and <code>inactive_shares</code> pool, calculate the total voting<br/> power, which equals to the sum of the coin amounts.


<pre><code>fun calculate_total_voting_power(delegation_pool: &amp;delegation_pool::DelegationPool, latest_delegated_votes: &amp;delegation_pool::DelegatedVotes): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_total_voting_power(delegation_pool: &amp;DelegationPool, latest_delegated_votes: &amp;DelegatedVotes): u64 &#123;<br/>    let active_amount &#61; pool_u64::shares_to_amount(<br/>        &amp;delegation_pool.active_shares,<br/>        latest_delegated_votes.active_shares);<br/>    let pending_inactive_amount &#61; pool_u64::shares_to_amount(<br/>        pending_inactive_shares_pool(delegation_pool),<br/>        latest_delegated_votes.pending_inactive_shares);<br/>    active_amount &#43; pending_inactive_amount<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegator_voter_internal"></a>

## Function `calculate_and_update_delegator_voter_internal`

Update VoteDelegation of a delegator to up&#45;to&#45;date then return the latest voter.


<pre><code>fun calculate_and_update_delegator_voter_internal(pool: &amp;delegation_pool::DelegationPool, governance_records: &amp;mut delegation_pool::GovernanceRecords, delegator: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_and_update_delegator_voter_internal(<br/>    pool: &amp;DelegationPool,<br/>    governance_records: &amp;mut GovernanceRecords,<br/>    delegator: address<br/>): address &#123;<br/>    let vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(pool, governance_records, delegator);<br/>    vote_delegation.voter<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_and_update_delegated_votes"></a>

## Function `calculate_and_update_delegated_votes`

Update DelegatedVotes of a voter to up&#45;to&#45;date then return the total voting power of this voter.


<pre><code>fun calculate_and_update_delegated_votes(pool: &amp;delegation_pool::DelegationPool, governance_records: &amp;mut delegation_pool::GovernanceRecords, voter: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_and_update_delegated_votes(<br/>    pool: &amp;DelegationPool,<br/>    governance_records: &amp;mut GovernanceRecords,<br/>    voter: address<br/>): u64 &#123;<br/>    let delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, voter);<br/>    calculate_total_voting_power(pool, delegated_votes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_borrow_mut_delegators_allowlist"></a>

## Function `borrow_mut_delegators_allowlist`



<pre><code>fun borrow_mut_delegators_allowlist(pool_address: address): &amp;mut smart_table::SmartTable&lt;address, bool&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_mut_delegators_allowlist(<br/>    pool_address: address<br/>): &amp;mut SmartTable&lt;address, bool&gt; acquires DelegationPoolAllowlisting &#123;<br/>    &amp;mut borrow_global_mut&lt;DelegationPoolAllowlisting&gt;(pool_address).allowlist<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the underlying stake pool.


<pre><code>public entry fun set_operator(owner: &amp;signer, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_operator(<br/>    owner: &amp;signer,<br/>    new_operator: address<br/>) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));<br/>    // synchronize delegation and stake pools before any user operation<br/>    // ensure the old operator is paid its uncommitted commission rewards<br/>    synchronize_delegation_pool(pool_address);<br/>    stake::set_operator(&amp;retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address)), new_operator);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Allows an operator to change its beneficiary. Any existing unpaid commission rewards will be paid to the new<br/> beneficiary. To ensure payment to the current beneficiary, one should first call <code>synchronize_delegation_pool</code><br/> before switching the beneficiary. An operator can set one beneficiary for delegation pools, not a separate<br/> one for each pool.


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary_for_operator(<br/>    operator: &amp;signer,<br/>    new_beneficiary: address<br/>) acquires BeneficiaryForOperator &#123;<br/>    assert!(features::operator_beneficiary_change_enabled(), std::error::invalid_state(<br/>        EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED<br/>    ));<br/>    // The beneficiay address of an operator is stored under the operator&apos;s address.<br/>    // So, the operator does not need to be validated with respect to a staking pool.<br/>    let operator_addr &#61; signer::address_of(operator);<br/>    let old_beneficiary &#61; beneficiary_for_operator(operator_addr);<br/>    if (exists&lt;BeneficiaryForOperator&gt;(operator_addr)) &#123;<br/>        borrow_global_mut&lt;BeneficiaryForOperator&gt;(operator_addr).beneficiary_for_operator &#61; new_beneficiary;<br/>    &#125; else &#123;<br/>        move_to(operator, BeneficiaryForOperator &#123; beneficiary_for_operator: new_beneficiary &#125;);<br/>    &#125;;<br/><br/>    emit(SetBeneficiaryForOperator &#123;<br/>        operator: operator_addr,<br/>        old_beneficiary,<br/>        new_beneficiary,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_update_commission_percentage"></a>

## Function `update_commission_percentage`

Allows an owner to update the commission percentage for the operator of the underlying stake pool.


<pre><code>public entry fun update_commission_percentage(owner: &amp;signer, new_commission_percentage: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_commission_percentage(<br/>    owner: &amp;signer,<br/>    new_commission_percentage: u64<br/>) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert!(features::commission_change_delegation_pool_enabled(), error::invalid_state(<br/>        ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED<br/>    ));<br/>    assert!(new_commission_percentage &lt;&#61; MAX_FEE, error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE));<br/>    let owner_address &#61; signer::address_of(owner);<br/>    let pool_address &#61; get_owned_pool_address(owner_address);<br/>    assert!(<br/>        operator_commission_percentage(pool_address) &#43; MAX_COMMISSION_INCREASE &gt;&#61; new_commission_percentage,<br/>        error::invalid_argument(ETOO_LARGE_COMMISSION_INCREASE)<br/>    );<br/>    assert!(<br/>        stake::get_remaining_lockup_secs(pool_address) &gt;&#61; min_remaining_secs_for_commission_change(),<br/>        error::invalid_state(ETOO_LATE_COMMISSION_CHANGE)<br/>    );<br/><br/>    // synchronize delegation and stake pools before any user operation. this ensures:<br/>    // (1) the operator is paid its uncommitted commission rewards with the old commission percentage, and<br/>    // (2) any pending commission percentage change is applied before the new commission percentage is set.<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    if (exists&lt;NextCommissionPercentage&gt;(pool_address)) &#123;<br/>        let commission_percentage &#61; borrow_global_mut&lt;NextCommissionPercentage&gt;(pool_address);<br/>        commission_percentage.commission_percentage_next_lockup_cycle &#61; new_commission_percentage;<br/>        commission_percentage.effective_after_secs &#61; stake::get_lockup_secs(pool_address);<br/>    &#125; else &#123;<br/>        let delegation_pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);<br/>        let pool_signer &#61; account::create_signer_with_capability(&amp;delegation_pool.stake_pool_signer_cap);<br/>        move_to(&amp;pool_signer, NextCommissionPercentage &#123;<br/>            commission_percentage_next_lockup_cycle: new_commission_percentage,<br/>            effective_after_secs: stake::get_lockup_secs(pool_address),<br/>        &#125;);<br/>    &#125;;<br/><br/>    event::emit(CommissionPercentageChange &#123;<br/>        pool_address,<br/>        owner: owner_address,<br/>        commission_percentage_next_lockup_cycle: new_commission_percentage,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_set_delegated_voter"></a>

## Function `set_delegated_voter`

Allows an owner to change the delegated voter of the underlying stake pool.


<pre><code>public entry fun set_delegated_voter(owner: &amp;signer, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_delegated_voter(<br/>    owner: &amp;signer,<br/>    new_voter: address<br/>) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    // No one can change delegated_voter once the partial governance voting feature is enabled.<br/>    assert!(<br/>        !features::delegation_pool_partial_governance_voting_enabled(),<br/>        error::invalid_state(EDEPRECATED_FUNCTION)<br/>    );<br/>    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));<br/>    // synchronize delegation and stake pools before any user operation<br/>    synchronize_delegation_pool(pool_address);<br/>    stake::set_delegated_voter(&amp;retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address)), new_voter);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_delegate_voting_power"></a>

## Function `delegate_voting_power`

Allows a delegator to delegate its voting power to a voter. If this delegator already has a delegated voter,<br/> this change won&apos;t take effects until the next lockup period.


<pre><code>public entry fun delegate_voting_power(delegator: &amp;signer, pool_address: address, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun delegate_voting_power(<br/>    delegator: &amp;signer,<br/>    pool_address: address,<br/>    new_voter: address<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert_partial_governance_voting_enabled(pool_address);<br/><br/>    // synchronize delegation and stake pools before any user operation<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    let delegator_address &#61; signer::address_of(delegator);<br/>    let delegation_pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);<br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/>    let delegator_vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(<br/>        delegation_pool,<br/>        governance_records,<br/>        delegator_address<br/>    );<br/>    let pending_voter: address &#61; delegator_vote_delegation.pending_voter;<br/><br/>    // No need to update if the voter doesn&apos;t really change.<br/>    if (pending_voter !&#61; new_voter) &#123;<br/>        delegator_vote_delegation.pending_voter &#61; new_voter;<br/>        let active_shares &#61; get_delegator_active_shares(delegation_pool, delegator_address);<br/>        // &lt;active shares&gt; of &lt;pending voter of shareholder&gt; &#45;&#61; &lt;active_shares&gt;<br/>        // &lt;active shares&gt; of &lt;new voter of shareholder&gt; &#43;&#61; &lt;active_shares&gt;<br/>        let pending_delegated_votes &#61; update_and_borrow_mut_delegated_votes(<br/>            delegation_pool,<br/>            governance_records,<br/>            pending_voter<br/>        );<br/>        pending_delegated_votes.active_shares_next_lockup &#61;<br/>            pending_delegated_votes.active_shares_next_lockup &#45; active_shares;<br/><br/>        let new_delegated_votes &#61; update_and_borrow_mut_delegated_votes(<br/>            delegation_pool,<br/>            governance_records,<br/>            new_voter<br/>        );<br/>        new_delegated_votes.active_shares_next_lockup &#61;<br/>            new_delegated_votes.active_shares_next_lockup &#43; active_shares;<br/>    &#125;;<br/><br/>    if (features::module_event_migration_enabled()) &#123;<br/>        event::emit(DelegateVotingPower &#123;<br/>            pool_address,<br/>            delegator: delegator_address,<br/>            voter: new_voter,<br/>        &#125;)<br/>    &#125;;<br/><br/>    event::emit_event(&amp;mut governance_records.delegate_voting_power_events, DelegateVotingPowerEvent &#123;<br/>        pool_address,<br/>        delegator: delegator_address,<br/>        voter: new_voter,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_enable_delegators_allowlisting"></a>

## Function `enable_delegators_allowlisting`

Enable delegators allowlisting as the pool owner.


<pre><code>public entry fun enable_delegators_allowlisting(owner: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun enable_delegators_allowlisting(<br/>    owner: &amp;signer,<br/>) acquires DelegationPoolOwnership, DelegationPool &#123;<br/>    assert!(<br/>        features::delegation_pool_allowlisting_enabled(),<br/>        error::invalid_state(EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED)<br/>    );<br/><br/>    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));<br/>    if (allowlisting_enabled(pool_address)) &#123; return &#125;;<br/><br/>    let pool_signer &#61; retrieve_stake_pool_owner(borrow_global&lt;DelegationPool&gt;(pool_address));<br/>    move_to(&amp;pool_signer, DelegationPoolAllowlisting &#123; allowlist: smart_table::new&lt;address, bool&gt;() &#125;);<br/><br/>    event::emit(EnableDelegatorsAllowlisting &#123; pool_address &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_disable_delegators_allowlisting"></a>

## Function `disable_delegators_allowlisting`

Disable delegators allowlisting as the pool owner. The existing allowlist will be emptied.


<pre><code>public entry fun disable_delegators_allowlisting(owner: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun disable_delegators_allowlisting(<br/>    owner: &amp;signer,<br/>) acquires DelegationPoolOwnership, DelegationPoolAllowlisting &#123;<br/>    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));<br/>    assert_allowlisting_enabled(pool_address);<br/><br/>    let DelegationPoolAllowlisting &#123; allowlist &#125; &#61; move_from&lt;DelegationPoolAllowlisting&gt;(pool_address);<br/>    // if the allowlist becomes too large, the owner can always remove some delegators<br/>    smart_table::destroy(allowlist);<br/><br/>    event::emit(DisableDelegatorsAllowlisting &#123; pool_address &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_delegator"></a>

## Function `allowlist_delegator`

Allowlist a delegator as the pool owner.


<pre><code>public entry fun allowlist_delegator(owner: &amp;signer, delegator_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun allowlist_delegator(<br/>    owner: &amp;signer,<br/>    delegator_address: address,<br/>) acquires DelegationPoolOwnership, DelegationPoolAllowlisting &#123;<br/>    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));<br/>    assert_allowlisting_enabled(pool_address);<br/><br/>    if (delegator_allowlisted(pool_address, delegator_address)) &#123; return &#125;;<br/><br/>    smart_table::add(borrow_mut_delegators_allowlist(pool_address), delegator_address, true);<br/><br/>    event::emit(AllowlistDelegator &#123; pool_address, delegator_address &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_remove_delegator_from_allowlist"></a>

## Function `remove_delegator_from_allowlist`

Remove a delegator from the allowlist as the pool owner, but do not unlock their stake.


<pre><code>public entry fun remove_delegator_from_allowlist(owner: &amp;signer, delegator_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun remove_delegator_from_allowlist(<br/>    owner: &amp;signer,<br/>    delegator_address: address,<br/>) acquires DelegationPoolOwnership, DelegationPoolAllowlisting &#123;<br/>    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));<br/>    assert_allowlisting_enabled(pool_address);<br/><br/>    if (!delegator_allowlisted(pool_address, delegator_address)) &#123; return &#125;;<br/><br/>    smart_table::remove(borrow_mut_delegators_allowlist(pool_address), delegator_address);<br/><br/>    event::emit(RemoveDelegatorFromAllowlist &#123; pool_address, delegator_address &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_evict_delegator"></a>

## Function `evict_delegator`

Evict a delegator that is not allowlisted by unlocking their entire stake.


<pre><code>public entry fun evict_delegator(owner: &amp;signer, delegator_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun evict_delegator(<br/>    owner: &amp;signer,<br/>    delegator_address: address,<br/>) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage, DelegationPoolAllowlisting &#123;<br/>    let pool_address &#61; get_owned_pool_address(signer::address_of(owner));<br/>    assert_allowlisting_enabled(pool_address);<br/>    assert!(<br/>        !delegator_allowlisted(pool_address, delegator_address),<br/>        error::invalid_state(ECANNOT_EVICT_ALLOWLISTED_DELEGATOR)<br/>    );<br/><br/>    // synchronize pool in order to query latest balance of delegator<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    let pool &#61; borrow_global&lt;DelegationPool&gt;(pool_address);<br/>    if (get_delegator_active_shares(pool, delegator_address) &#61;&#61; 0) &#123; return &#125;;<br/><br/>    unlock_internal(delegator_address, pool_address, pool_u64::balance(&amp;pool.active_shares, delegator_address));<br/><br/>    event::emit(EvictDelegator &#123; pool_address, delegator_address &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins to the delegation pool <code>pool_address</code>.


<pre><code>public entry fun add_stake(delegator: &amp;signer, pool_address: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_stake(<br/>    delegator: &amp;signer,<br/>    pool_address: address,<br/>    amount: u64<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage, DelegationPoolAllowlisting &#123;<br/>    // short&#45;circuit if amount to add is 0 so no event is emitted<br/>    if (amount &#61;&#61; 0) &#123; return &#125;;<br/><br/>    let delegator_address &#61; signer::address_of(delegator);<br/>    assert_delegator_allowlisted(pool_address, delegator_address);<br/><br/>    // synchronize delegation and stake pools before any user operation<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    // fee to be charged for adding `amount` stake on this delegation pool at this epoch<br/>    let add_stake_fee &#61; get_add_stake_fee(pool_address, amount);<br/><br/>    let pool &#61; borrow_global_mut&lt;DelegationPool&gt;(pool_address);<br/><br/>    // stake the entire amount to the stake pool<br/>    aptos_account::transfer(delegator, pool_address, amount);<br/>    stake::add_stake(&amp;retrieve_stake_pool_owner(pool), amount);<br/><br/>    // but buy shares for delegator just for the remaining amount after fee<br/>    buy_in_active_shares(pool, delegator_address, amount &#45; add_stake_fee);<br/>    assert_min_active_balance(pool, delegator_address);<br/><br/>    // grant temporary ownership over `add_stake` fees to a separate shareholder in order to:<br/>    // &#45; not mistake them for rewards to pay the operator from<br/>    // &#45; distribute them together with the `active` rewards when this epoch ends<br/>    // in order to appreciate all shares on the active pool atomically<br/>    buy_in_active_shares(pool, NULL_SHAREHOLDER, add_stake_fee);<br/><br/>    if (features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            AddStake &#123;<br/>                pool_address,<br/>                delegator_address,<br/>                amount_added: amount,<br/>                add_stake_fee,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/><br/>    event::emit_event(<br/>        &amp;mut pool.add_stake_events,<br/>        AddStakeEvent &#123;<br/>            pool_address,<br/>            delegator_address,<br/>            amount_added: amount,<br/>            add_stake_fee,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_unlock"></a>

## Function `unlock`

Unlock <code>amount</code> from the active &#43; pending_active stake of <code>delegator</code> or<br/> at most how much active stake there is on the stake pool.


<pre><code>public entry fun unlock(delegator: &amp;signer, pool_address: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock(<br/>    delegator: &amp;signer,<br/>    pool_address: address,<br/>    amount: u64<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    // short&#45;circuit if amount to unlock is 0 so no event is emitted<br/>    if (amount &#61;&#61; 0) &#123; return &#125;;<br/><br/>    // synchronize delegation and stake pools before any user operation<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    let delegator_address &#61; signer::address_of(delegator);<br/>    unlock_internal(delegator_address, pool_address, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_unlock_internal"></a>

## Function `unlock_internal`



<pre><code>fun unlock_internal(delegator_address: address, pool_address: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun unlock_internal(<br/>    delegator_address: address,<br/>    pool_address: address,<br/>    amount: u64<br/>) acquires DelegationPool, GovernanceRecords &#123;<br/>    assert!(delegator_address !&#61; NULL_SHAREHOLDER, error::invalid_argument(ECANNOT_UNLOCK_NULL_SHAREHOLDER));<br/><br/>    // fail unlock of more stake than `active` on the stake pool<br/>    let (active, _, _, _) &#61; stake::get_stake(pool_address);<br/>    assert!(amount &lt;&#61; active, error::invalid_argument(ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK));<br/><br/>    let pool &#61; borrow_global_mut&lt;DelegationPool&gt;(pool_address);<br/>    amount &#61; coins_to_transfer_to_ensure_min_stake(<br/>        &amp;pool.active_shares,<br/>        pending_inactive_shares_pool(pool),<br/>        delegator_address,<br/>        amount,<br/>    );<br/>    amount &#61; redeem_active_shares(pool, delegator_address, amount);<br/><br/>    stake::unlock(&amp;retrieve_stake_pool_owner(pool), amount);<br/><br/>    buy_in_pending_inactive_shares(pool, delegator_address, amount);<br/>    assert_min_pending_inactive_balance(pool, delegator_address);<br/><br/>    if (features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            UnlockStake &#123;<br/>                pool_address,<br/>                delegator_address,<br/>                amount_unlocked: amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/><br/>    event::emit_event(<br/>        &amp;mut pool.unlock_stake_events,<br/>        UnlockStakeEvent &#123;<br/>            pool_address,<br/>            delegator_address,<br/>            amount_unlocked: amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code>public entry fun reactivate_stake(delegator: &amp;signer, pool_address: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reactivate_stake(<br/>    delegator: &amp;signer,<br/>    pool_address: address,<br/>    amount: u64<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage, DelegationPoolAllowlisting &#123;<br/>    // short&#45;circuit if amount to reactivate is 0 so no event is emitted<br/>    if (amount &#61;&#61; 0) &#123; return &#125;;<br/><br/>    let delegator_address &#61; signer::address_of(delegator);<br/>    assert_delegator_allowlisted(pool_address, delegator_address);<br/><br/>    // synchronize delegation and stake pools before any user operation<br/>    synchronize_delegation_pool(pool_address);<br/><br/>    let pool &#61; borrow_global_mut&lt;DelegationPool&gt;(pool_address);<br/>    amount &#61; coins_to_transfer_to_ensure_min_stake(<br/>        pending_inactive_shares_pool(pool),<br/>        &amp;pool.active_shares,<br/>        delegator_address,<br/>        amount,<br/>    );<br/>    let observed_lockup_cycle &#61; pool.observed_lockup_cycle;<br/>    amount &#61; redeem_inactive_shares(pool, delegator_address, amount, observed_lockup_cycle);<br/><br/>    stake::reactivate_stake(&amp;retrieve_stake_pool_owner(pool), amount);<br/><br/>    buy_in_active_shares(pool, delegator_address, amount);<br/>    assert_min_active_balance(pool, delegator_address);<br/><br/>    if (features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            ReactivateStake &#123;<br/>                pool_address,<br/>                delegator_address,<br/>                amount_reactivated: amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/><br/>    event::emit_event(<br/>        &amp;mut pool.reactivate_stake_events,<br/>        ReactivateStakeEvent &#123;<br/>            pool_address,<br/>            delegator_address,<br/>            amount_reactivated: amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of owned inactive stake from the delegation pool at <code>pool_address</code>.


<pre><code>public entry fun withdraw(delegator: &amp;signer, pool_address: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun withdraw(<br/>    delegator: &amp;signer,<br/>    pool_address: address,<br/>    amount: u64<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert!(amount &gt; 0, error::invalid_argument(EWITHDRAW_ZERO_STAKE));<br/>    // synchronize delegation and stake pools before any user operation<br/>    synchronize_delegation_pool(pool_address);<br/>    withdraw_internal(borrow_global_mut&lt;DelegationPool&gt;(pool_address), signer::address_of(delegator), amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_withdraw_internal"></a>

## Function `withdraw_internal`



<pre><code>fun withdraw_internal(pool: &amp;mut delegation_pool::DelegationPool, delegator_address: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun withdraw_internal(<br/>    pool: &amp;mut DelegationPool,<br/>    delegator_address: address,<br/>    amount: u64<br/>) acquires GovernanceRecords &#123;<br/>    // TODO: recycle storage when a delegator fully exits the delegation pool.<br/>    // short&#45;circuit if amount to withdraw is 0 so no event is emitted<br/>    if (amount &#61;&#61; 0) &#123; return &#125;;<br/><br/>    let pool_address &#61; get_pool_address(pool);<br/>    let (withdrawal_exists, withdrawal_olc) &#61; pending_withdrawal_exists(pool, delegator_address);<br/>    // exit if no withdrawal or (it is pending and cannot withdraw pending_inactive stake from stake pool)<br/>    if (!(<br/>        withdrawal_exists &amp;&amp;<br/>            (withdrawal_olc.index &lt; pool.observed_lockup_cycle.index &#124;&#124; can_withdraw_pending_inactive(pool_address))<br/>    )) &#123; return &#125;;<br/><br/>    if (withdrawal_olc.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;<br/>        amount &#61; coins_to_redeem_to_ensure_min_stake(<br/>            pending_inactive_shares_pool(pool),<br/>            delegator_address,<br/>            amount,<br/>        )<br/>    &#125;;<br/>    amount &#61; redeem_inactive_shares(pool, delegator_address, amount, withdrawal_olc);<br/><br/>    let stake_pool_owner &#61; &amp;retrieve_stake_pool_owner(pool);<br/>    // stake pool will inactivate entire pending_inactive stake at `stake::withdraw` to make it withdrawable<br/>    // however, bypassing the inactivation of excess stake (inactivated but not withdrawn) ensures<br/>    // the OLC is not advanced indefinitely on `unlock`&#45;`withdraw` paired calls<br/>    if (can_withdraw_pending_inactive(pool_address)) &#123;<br/>        // get excess stake before being entirely inactivated<br/>        let (_, _, _, pending_inactive) &#61; stake::get_stake(pool_address);<br/>        if (withdrawal_olc.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;<br/>            // `amount` less excess if withdrawing pending_inactive stake<br/>            pending_inactive &#61; pending_inactive &#45; amount<br/>        &#125;;<br/>        // escape excess stake from inactivation<br/>        stake::reactivate_stake(stake_pool_owner, pending_inactive);<br/>        stake::withdraw(stake_pool_owner, amount);<br/>        // restore excess stake to the pending_inactive state<br/>        stake::unlock(stake_pool_owner, pending_inactive);<br/>    &#125; else &#123;<br/>        // no excess stake if `stake::withdraw` does not inactivate at all<br/>        stake::withdraw(stake_pool_owner, amount);<br/>    &#125;;<br/>    aptos_account::transfer(stake_pool_owner, delegator_address, amount);<br/><br/>    // commit withdrawal of possibly inactive stake to the `total_coins_inactive`<br/>    // known by the delegation pool in order to not mistake it for slashing at next synchronization<br/>    let (_, inactive, _, _) &#61; stake::get_stake(pool_address);<br/>    pool.total_coins_inactive &#61; inactive;<br/><br/>    if (features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            WithdrawStake &#123;<br/>                pool_address,<br/>                delegator_address,<br/>                amount_withdrawn: amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/><br/>    event::emit_event(<br/>        &amp;mut pool.withdraw_stake_events,<br/>        WithdrawStakeEvent &#123;<br/>            pool_address,<br/>            delegator_address,<br/>            amount_withdrawn: amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_pending_withdrawal_exists"></a>

## Function `pending_withdrawal_exists`

Return the unique observed lockup cycle where delegator <code>delegator_address</code> may have<br/> unlocking (or already unlocked) stake to be withdrawn from delegation pool <code>pool</code>.<br/> A bool is returned to signal if a pending withdrawal exists at all.


<pre><code>fun pending_withdrawal_exists(pool: &amp;delegation_pool::DelegationPool, delegator_address: address): (bool, delegation_pool::ObservedLockupCycle)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun pending_withdrawal_exists(pool: &amp;DelegationPool, delegator_address: address): (bool, ObservedLockupCycle) &#123;<br/>    if (table::contains(&amp;pool.pending_withdrawals, delegator_address)) &#123;<br/>        (true, &#42;table::borrow(&amp;pool.pending_withdrawals, delegator_address))<br/>    &#125; else &#123;<br/>        (false, olc_with_index(0))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_pending_inactive_shares_pool_mut"></a>

## Function `pending_inactive_shares_pool_mut`

Return a mutable reference to the shares pool of <code>pending_inactive</code> stake on the<br/> delegation pool, always the last item in <code>inactive_shares</code>.


<pre><code>fun pending_inactive_shares_pool_mut(pool: &amp;mut delegation_pool::DelegationPool): &amp;mut pool_u64_unbound::Pool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun pending_inactive_shares_pool_mut(pool: &amp;mut DelegationPool): &amp;mut pool_u64::Pool &#123;<br/>    let observed_lockup_cycle &#61; pool.observed_lockup_cycle;<br/>    table::borrow_mut(&amp;mut pool.inactive_shares, observed_lockup_cycle)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_pending_inactive_shares_pool"></a>

## Function `pending_inactive_shares_pool`



<pre><code>fun pending_inactive_shares_pool(pool: &amp;delegation_pool::DelegationPool): &amp;pool_u64_unbound::Pool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun pending_inactive_shares_pool(pool: &amp;DelegationPool): &amp;pool_u64::Pool &#123;<br/>    table::borrow(&amp;pool.inactive_shares, pool.observed_lockup_cycle)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_execute_pending_withdrawal"></a>

## Function `execute_pending_withdrawal`

Execute the pending withdrawal of <code>delegator_address</code> on delegation pool <code>pool</code><br/> if existing and already inactive to allow the creation of a new one.<br/> <code>pending_inactive</code> stake would be left untouched even if withdrawable and should<br/> be explicitly withdrawn by delegator


<pre><code>fun execute_pending_withdrawal(pool: &amp;mut delegation_pool::DelegationPool, delegator_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun execute_pending_withdrawal(pool: &amp;mut DelegationPool, delegator_address: address) acquires GovernanceRecords &#123;<br/>    let (withdrawal_exists, withdrawal_olc) &#61; pending_withdrawal_exists(pool, delegator_address);<br/>    if (withdrawal_exists &amp;&amp; withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) &#123;<br/>        withdraw_internal(pool, delegator_address, MAX_U64);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_buy_in_active_shares"></a>

## Function `buy_in_active_shares`

Buy shares into the active pool on behalf of delegator <code>shareholder</code> who<br/> deposited <code>coins_amount</code>. This function doesn&apos;t make any coin transfer.


<pre><code>fun buy_in_active_shares(pool: &amp;mut delegation_pool::DelegationPool, shareholder: address, coins_amount: u64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun buy_in_active_shares(<br/>    pool: &amp;mut DelegationPool,<br/>    shareholder: address,<br/>    coins_amount: u64,<br/>): u128 acquires GovernanceRecords &#123;<br/>    let new_shares &#61; pool_u64::amount_to_shares(&amp;pool.active_shares, coins_amount);<br/>    // No need to buy 0 shares.<br/>    if (new_shares &#61;&#61; 0) &#123; return 0 &#125;;<br/><br/>    // Always update governance records before any change to the shares pool.<br/>    let pool_address &#61; get_pool_address(pool);<br/>    if (partial_governance_voting_enabled(pool_address)) &#123;<br/>        update_governance_records_for_buy_in_active_shares(pool, pool_address, new_shares, shareholder);<br/>    &#125;;<br/><br/>    pool_u64::buy_in(&amp;mut pool.active_shares, shareholder, coins_amount);<br/>    new_shares<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_buy_in_pending_inactive_shares"></a>

## Function `buy_in_pending_inactive_shares`

Buy shares into the pending_inactive pool on behalf of delegator <code>shareholder</code> who<br/> redeemed <code>coins_amount</code> from the active pool to schedule it for unlocking.<br/> If delegator&apos;s pending withdrawal exists and has been inactivated, execute it firstly<br/> to ensure there is always only one withdrawal request.


<pre><code>fun buy_in_pending_inactive_shares(pool: &amp;mut delegation_pool::DelegationPool, shareholder: address, coins_amount: u64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun buy_in_pending_inactive_shares(<br/>    pool: &amp;mut DelegationPool,<br/>    shareholder: address,<br/>    coins_amount: u64,<br/>): u128 acquires GovernanceRecords &#123;<br/>    let new_shares &#61; pool_u64::amount_to_shares(pending_inactive_shares_pool(pool), coins_amount);<br/>    // never create a new pending withdrawal unless delegator owns some pending_inactive shares<br/>    if (new_shares &#61;&#61; 0) &#123; return 0 &#125;;<br/><br/>    // Always update governance records before any change to the shares pool.<br/>    let pool_address &#61; get_pool_address(pool);<br/>    if (partial_governance_voting_enabled(pool_address)) &#123;<br/>        update_governance_records_for_buy_in_pending_inactive_shares(pool, pool_address, new_shares, shareholder);<br/>    &#125;;<br/><br/>    // cannot buy inactive shares, only pending_inactive at current lockup cycle<br/>    pool_u64::buy_in(pending_inactive_shares_pool_mut(pool), shareholder, coins_amount);<br/><br/>    // execute the pending withdrawal if exists and is inactive before creating a new one<br/>    execute_pending_withdrawal(pool, shareholder);<br/><br/>    // save observed lockup cycle for the new pending withdrawal<br/>    let observed_lockup_cycle &#61; pool.observed_lockup_cycle;<br/>    assert!(&#42;table::borrow_mut_with_default(<br/>        &amp;mut pool.pending_withdrawals,<br/>        shareholder,<br/>        observed_lockup_cycle<br/>    ) &#61;&#61; observed_lockup_cycle,<br/>        error::invalid_state(EPENDING_WITHDRAWAL_EXISTS)<br/>    );<br/><br/>    new_shares<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_amount_to_shares_to_redeem"></a>

## Function `amount_to_shares_to_redeem`

Convert <code>coins_amount</code> of coins to be redeemed from shares pool <code>shares_pool</code><br/> to the exact number of shares to redeem in order to achieve this.


<pre><code>fun amount_to_shares_to_redeem(shares_pool: &amp;pool_u64_unbound::Pool, shareholder: address, coins_amount: u64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun amount_to_shares_to_redeem(<br/>    shares_pool: &amp;pool_u64::Pool,<br/>    shareholder: address,<br/>    coins_amount: u64,<br/>): u128 &#123;<br/>    if (coins_amount &gt;&#61; pool_u64::balance(shares_pool, shareholder)) &#123;<br/>        // cap result at total shares of shareholder to pass `EINSUFFICIENT_SHARES` on subsequent redeem<br/>        pool_u64::shares(shares_pool, shareholder)<br/>    &#125; else &#123;<br/>        pool_u64::amount_to_shares(shares_pool, coins_amount)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_redeem_active_shares"></a>

## Function `redeem_active_shares`

Redeem shares from the active pool on behalf of delegator <code>shareholder</code> who<br/> wants to unlock <code>coins_amount</code> of its active stake.<br/> Extracted coins will be used to buy shares into the pending_inactive pool and<br/> be available for withdrawal when current OLC ends.


<pre><code>fun redeem_active_shares(pool: &amp;mut delegation_pool::DelegationPool, shareholder: address, coins_amount: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun redeem_active_shares(<br/>    pool: &amp;mut DelegationPool,<br/>    shareholder: address,<br/>    coins_amount: u64,<br/>): u64 acquires GovernanceRecords &#123;<br/>    let shares_to_redeem &#61; amount_to_shares_to_redeem(&amp;pool.active_shares, shareholder, coins_amount);<br/>    // silently exit if not a shareholder otherwise redeem would fail with `ESHAREHOLDER_NOT_FOUND`<br/>    if (shares_to_redeem &#61;&#61; 0) return 0;<br/><br/>    // Always update governance records before any change to the shares pool.<br/>    let pool_address &#61; get_pool_address(pool);<br/>    if (partial_governance_voting_enabled(pool_address)) &#123;<br/>        update_governanace_records_for_redeem_active_shares(pool, pool_address, shares_to_redeem, shareholder);<br/>    &#125;;<br/><br/>    pool_u64::redeem_shares(&amp;mut pool.active_shares, shareholder, shares_to_redeem)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_redeem_inactive_shares"></a>

## Function `redeem_inactive_shares`

Redeem shares from the inactive pool at <code>lockup_cycle</code> &lt; current OLC on behalf of<br/> delegator <code>shareholder</code> who wants to withdraw <code>coins_amount</code> of its unlocked stake.<br/> Redeem shares from the pending_inactive pool at <code>lockup_cycle</code> &#61;&#61; current OLC on behalf of<br/> delegator <code>shareholder</code> who wants to reactivate <code>coins_amount</code> of its unlocking stake.<br/> For latter case, extracted coins will be used to buy shares into the active pool and<br/> escape inactivation when current lockup ends.


<pre><code>fun redeem_inactive_shares(pool: &amp;mut delegation_pool::DelegationPool, shareholder: address, coins_amount: u64, lockup_cycle: delegation_pool::ObservedLockupCycle): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun redeem_inactive_shares(<br/>    pool: &amp;mut DelegationPool,<br/>    shareholder: address,<br/>    coins_amount: u64,<br/>    lockup_cycle: ObservedLockupCycle,<br/>): u64 acquires GovernanceRecords &#123;<br/>    let shares_to_redeem &#61; amount_to_shares_to_redeem(<br/>        table::borrow(&amp;pool.inactive_shares, lockup_cycle),<br/>        shareholder,<br/>        coins_amount);<br/>    // silently exit if not a shareholder otherwise redeem would fail with `ESHAREHOLDER_NOT_FOUND`<br/>    if (shares_to_redeem &#61;&#61; 0) return 0;<br/><br/>    // Always update governance records before any change to the shares pool.<br/>    let pool_address &#61; get_pool_address(pool);<br/>    // Only redeem shares from the pending_inactive pool at `lockup_cycle` &#61;&#61; current OLC.<br/>    if (partial_governance_voting_enabled(pool_address) &amp;&amp; lockup_cycle.index &#61;&#61; pool.observed_lockup_cycle.index) &#123;<br/>        update_governanace_records_for_redeem_pending_inactive_shares(<br/>            pool,<br/>            pool_address,<br/>            shares_to_redeem,<br/>            shareholder<br/>        );<br/>    &#125;;<br/><br/>    let inactive_shares &#61; table::borrow_mut(&amp;mut pool.inactive_shares, lockup_cycle);<br/>    // 1. reaching here means delegator owns inactive/pending_inactive shares at OLC `lockup_cycle`<br/>    let redeemed_coins &#61; pool_u64::redeem_shares(inactive_shares, shareholder, shares_to_redeem);<br/><br/>    // if entirely reactivated pending_inactive stake or withdrawn inactive one,<br/>    // re&#45;enable unlocking for delegator by deleting this pending withdrawal<br/>    if (pool_u64::shares(inactive_shares, shareholder) &#61;&#61; 0) &#123;<br/>        // 2. a delegator owns inactive/pending_inactive shares only at the OLC of its pending withdrawal<br/>        // 1 &amp; 2: the pending withdrawal itself has been emptied of shares and can be safely deleted<br/>        table::remove(&amp;mut pool.pending_withdrawals, shareholder);<br/>    &#125;;<br/>    // destroy inactive shares pool of past OLC if all its stake has been withdrawn<br/>    if (lockup_cycle.index &lt; pool.observed_lockup_cycle.index &amp;&amp; total_coins(inactive_shares) &#61;&#61; 0) &#123;<br/>        pool_u64::destroy_empty(table::remove(&amp;mut pool.inactive_shares, lockup_cycle));<br/>    &#125;;<br/><br/>    redeemed_coins<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_calculate_stake_pool_drift"></a>

## Function `calculate_stake_pool_drift`

Calculate stake deviations between the delegation and stake pools in order to<br/> capture the rewards earned in the meantime, resulted operator commission and<br/> whether the lockup expired on the stake pool.


<pre><code>fun calculate_stake_pool_drift(pool: &amp;delegation_pool::DelegationPool): (bool, u64, u64, u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_stake_pool_drift(pool: &amp;DelegationPool): (bool, u64, u64, u64, u64) &#123;<br/>    let (active, inactive, pending_active, pending_inactive) &#61; stake::get_stake(get_pool_address(pool));<br/>    assert!(<br/>        inactive &gt;&#61; pool.total_coins_inactive,<br/>        error::invalid_state(ESLASHED_INACTIVE_STAKE_ON_PAST_OLC)<br/>    );<br/>    // determine whether a new lockup cycle has been ended on the stake pool and<br/>    // inactivated SOME `pending_inactive` stake which should stop earning rewards now,<br/>    // thus requiring separation of the `pending_inactive` stake on current observed lockup<br/>    // and the future one on the newly started lockup<br/>    let lockup_cycle_ended &#61; inactive &gt; pool.total_coins_inactive;<br/><br/>    // actual coins on stake pool belonging to the active shares pool<br/>    active &#61; active &#43; pending_active;<br/>    // actual coins on stake pool belonging to the shares pool hosting `pending_inactive` stake<br/>    // at current observed lockup cycle, either pending: `pending_inactive` or already inactivated:<br/>    if (lockup_cycle_ended) &#123;<br/>        // `inactive` on stake pool &#61; any previous `inactive` stake &#43;<br/>        // any previous `pending_inactive` stake and its rewards (both inactivated)<br/>        pending_inactive &#61; inactive &#45; pool.total_coins_inactive<br/>    &#125;;<br/><br/>    // on stake&#45;management operations, total coins on the internal shares pools and individual<br/>    // stakes on the stake pool are updated simultaneously, thus the only stakes becoming<br/>    // unsynced are rewards and slashes routed exclusively to/out the stake pool<br/><br/>    // operator `active` rewards not persisted yet to the active shares pool<br/>    let pool_active &#61; total_coins(&amp;pool.active_shares);<br/>    let commission_active &#61; if (active &gt; pool_active) &#123;<br/>        math64::mul_div(active &#45; pool_active, pool.operator_commission_percentage, MAX_FEE)<br/>    &#125; else &#123;<br/>        // handle any slashing applied to `active` stake<br/>        0<br/>    &#125;;<br/>    // operator `pending_inactive` rewards not persisted yet to the pending_inactive shares pool<br/>    let pool_pending_inactive &#61; total_coins(pending_inactive_shares_pool(pool));<br/>    let commission_pending_inactive &#61; if (pending_inactive &gt; pool_pending_inactive) &#123;<br/>        math64::mul_div(<br/>            pending_inactive &#45; pool_pending_inactive,<br/>            pool.operator_commission_percentage,<br/>            MAX_FEE<br/>        )<br/>    &#125; else &#123;<br/>        // handle any slashing applied to `pending_inactive` stake<br/>        0<br/>    &#125;;<br/><br/>    (lockup_cycle_ended, active, pending_inactive, commission_active, commission_pending_inactive)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_synchronize_delegation_pool"></a>

## Function `synchronize_delegation_pool`

Synchronize delegation and stake pools: distribute yet&#45;undetected rewards to the corresponding internal<br/> shares pools, assign commission to operator and eventually prepare delegation pool for a new lockup cycle.


<pre><code>public entry fun synchronize_delegation_pool(pool_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun synchronize_delegation_pool(<br/>    pool_address: address<br/>) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage &#123;<br/>    assert_delegation_pool_exists(pool_address);<br/>    let pool &#61; borrow_global_mut&lt;DelegationPool&gt;(pool_address);<br/>    let (<br/>        lockup_cycle_ended,<br/>        active,<br/>        pending_inactive,<br/>        commission_active,<br/>        commission_pending_inactive<br/>    ) &#61; calculate_stake_pool_drift(pool);<br/><br/>    // zero `pending_active` stake indicates that either there are no `add_stake` fees or<br/>    // previous epoch has ended and should release the shares owning the existing fees<br/>    let (_, _, pending_active, _) &#61; stake::get_stake(pool_address);<br/>    if (pending_active &#61;&#61; 0) &#123;<br/>        // renounce ownership over the `add_stake` fees by redeeming all shares of<br/>        // the special shareholder, implicitly their equivalent coins, out of the active shares pool<br/>        redeem_active_shares(pool, NULL_SHAREHOLDER, MAX_U64);<br/>    &#125;;<br/><br/>    // distribute rewards remaining after commission, to delegators (to already existing shares)<br/>    // before buying shares for the operator for its entire commission fee<br/>    // otherwise, operator&apos;s new shares would additionally appreciate from rewards it does not own<br/><br/>    // update total coins accumulated by `active` &#43; `pending_active` shares<br/>    // redeemed `add_stake` fees are restored and distributed to the rest of the pool as rewards<br/>    pool_u64::update_total_coins(&amp;mut pool.active_shares, active &#45; commission_active);<br/>    // update total coins accumulated by `pending_inactive` shares at current observed lockup cycle<br/>    pool_u64::update_total_coins(<br/>        pending_inactive_shares_pool_mut(pool),<br/>        pending_inactive &#45; commission_pending_inactive<br/>    );<br/><br/>    // reward operator its commission out of uncommitted active rewards (`add_stake` fees already excluded)<br/>    buy_in_active_shares(pool, beneficiary_for_operator(stake::get_operator(pool_address)), commission_active);<br/>    // reward operator its commission out of uncommitted pending_inactive rewards<br/>    buy_in_pending_inactive_shares(<br/>        pool,<br/>        beneficiary_for_operator(stake::get_operator(pool_address)),<br/>        commission_pending_inactive<br/>    );<br/><br/>    event::emit_event(<br/>        &amp;mut pool.distribute_commission_events,<br/>        DistributeCommissionEvent &#123;<br/>            pool_address,<br/>            operator: stake::get_operator(pool_address),<br/>            commission_active,<br/>            commission_pending_inactive,<br/>        &#125;,<br/>    );<br/><br/>    if (features::operator_beneficiary_change_enabled()) &#123;<br/>        emit(DistributeCommission &#123;<br/>            pool_address,<br/>            operator: stake::get_operator(pool_address),<br/>            beneficiary: beneficiary_for_operator(stake::get_operator(pool_address)),<br/>            commission_active,<br/>            commission_pending_inactive,<br/>        &#125;)<br/>    &#125;;<br/><br/>    // advance lockup cycle on delegation pool if already ended on stake pool (AND stake explicitly inactivated)<br/>    if (lockup_cycle_ended) &#123;<br/>        // capture inactive coins over all ended lockup cycles (including this ending one)<br/>        let (_, inactive, _, _) &#61; stake::get_stake(pool_address);<br/>        pool.total_coins_inactive &#61; inactive;<br/><br/>        // advance lockup cycle on the delegation pool<br/>        pool.observed_lockup_cycle.index &#61; pool.observed_lockup_cycle.index &#43; 1;<br/>        // start new lockup cycle with a fresh shares pool for `pending_inactive` stake<br/>        table::add(<br/>            &amp;mut pool.inactive_shares,<br/>            pool.observed_lockup_cycle,<br/>            pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR)<br/>        );<br/>    &#125;;<br/><br/>    if (is_next_commission_percentage_effective(pool_address)) &#123;<br/>        pool.operator_commission_percentage &#61; borrow_global&lt;NextCommissionPercentage&gt;(<br/>            pool_address<br/>        ).commission_percentage_next_lockup_cycle;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_assert_and_update_proposal_used_voting_power"></a>

## Function `assert_and_update_proposal_used_voting_power`



<pre><code>fun assert_and_update_proposal_used_voting_power(governance_records: &amp;mut delegation_pool::GovernanceRecords, pool_address: address, proposal_id: u64, voting_power: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_and_update_proposal_used_voting_power(<br/>    governance_records: &amp;mut GovernanceRecords, pool_address: address, proposal_id: u64, voting_power: u64<br/>) &#123;<br/>    let stake_pool_remaining_voting_power &#61; aptos_governance::get_remaining_voting_power(pool_address, proposal_id);<br/>    let stake_pool_used_voting_power &#61; aptos_governance::get_voting_power(<br/>        pool_address<br/>    ) &#45; stake_pool_remaining_voting_power;<br/>    let proposal_used_voting_power &#61; smart_table::borrow_mut_with_default(<br/>        &amp;mut governance_records.votes_per_proposal,<br/>        proposal_id,<br/>        0<br/>    );<br/>    // A edge case: Before enabling partial governance voting on a delegation pool, the delegation pool has<br/>    // a voter which can vote with all voting power of this delegation pool. If the voter votes on a proposal after<br/>    // partial governance voting flag is enabled, the delegation pool doesn&apos;t have enough voting power on this<br/>    // proposal for all the delegators. To be fair, no one can vote on this proposal through this delegation pool.<br/>    // To detect this case, check if the stake pool had used voting power not through delegation_pool module.<br/>    assert!(<br/>        stake_pool_used_voting_power &#61;&#61; &#42;proposal_used_voting_power,<br/>        error::invalid_argument(EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING)<br/>    );<br/>    &#42;proposal_used_voting_power &#61; &#42;proposal_used_voting_power &#43; voting_power;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_update_governance_records_for_buy_in_active_shares"></a>

## Function `update_governance_records_for_buy_in_active_shares`



<pre><code>fun update_governance_records_for_buy_in_active_shares(pool: &amp;delegation_pool::DelegationPool, pool_address: address, new_shares: u128, shareholder: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_governance_records_for_buy_in_active_shares(<br/>    pool: &amp;DelegationPool, pool_address: address, new_shares: u128, shareholder: address<br/>) acquires GovernanceRecords &#123;<br/>    // &lt;active shares&gt; of &lt;shareholder&gt; &#43;&#61; &lt;new_shares&gt; &#45;&#45;&#45;&#45;&gt;<br/>    // &lt;active shares&gt; of &lt;current voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;<br/>    // &lt;active shares&gt; of &lt;next voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;<br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/>    let vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(pool, governance_records, shareholder);<br/>    let current_voter &#61; vote_delegation.voter;<br/>    let pending_voter &#61; vote_delegation.pending_voter;<br/>    let current_delegated_votes &#61;<br/>        update_and_borrow_mut_delegated_votes(pool, governance_records, current_voter);<br/>    current_delegated_votes.active_shares &#61; current_delegated_votes.active_shares &#43; new_shares;<br/>    if (pending_voter &#61;&#61; current_voter) &#123;<br/>        current_delegated_votes.active_shares_next_lockup &#61;<br/>            current_delegated_votes.active_shares_next_lockup &#43; new_shares;<br/>    &#125; else &#123;<br/>        let pending_delegated_votes &#61;<br/>            update_and_borrow_mut_delegated_votes(pool, governance_records, pending_voter);<br/>        pending_delegated_votes.active_shares_next_lockup &#61;<br/>            pending_delegated_votes.active_shares_next_lockup &#43; new_shares;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_update_governance_records_for_buy_in_pending_inactive_shares"></a>

## Function `update_governance_records_for_buy_in_pending_inactive_shares`



<pre><code>fun update_governance_records_for_buy_in_pending_inactive_shares(pool: &amp;delegation_pool::DelegationPool, pool_address: address, new_shares: u128, shareholder: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_governance_records_for_buy_in_pending_inactive_shares(<br/>    pool: &amp;DelegationPool, pool_address: address, new_shares: u128, shareholder: address<br/>) acquires GovernanceRecords &#123;<br/>    // &lt;pending inactive shares&gt; of &lt;shareholder&gt; &#43;&#61; &lt;new_shares&gt;   &#45;&#45;&#45;&#45;&gt;<br/>    // &lt;pending inactive shares&gt; of &lt;current voter of shareholder&gt; &#43;&#61; &lt;new_shares&gt;<br/>    // no impact on &lt;pending inactive shares&gt; of &lt;next voter of shareholder&gt;<br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/>    let current_voter &#61; calculate_and_update_delegator_voter_internal(pool, governance_records, shareholder);<br/>    let current_delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, current_voter);<br/>    current_delegated_votes.pending_inactive_shares &#61; current_delegated_votes.pending_inactive_shares &#43; new_shares;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_update_governanace_records_for_redeem_active_shares"></a>

## Function `update_governanace_records_for_redeem_active_shares`



<pre><code>fun update_governanace_records_for_redeem_active_shares(pool: &amp;delegation_pool::DelegationPool, pool_address: address, shares_to_redeem: u128, shareholder: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_governanace_records_for_redeem_active_shares(<br/>    pool: &amp;DelegationPool, pool_address: address, shares_to_redeem: u128, shareholder: address<br/>) acquires GovernanceRecords &#123;<br/>    // &lt;active shares&gt; of &lt;shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt; &#45;&#45;&#45;&#45;&gt;<br/>    // &lt;active shares&gt; of &lt;current voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;<br/>    // &lt;active shares&gt; of &lt;next voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;<br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/>    let vote_delegation &#61; update_and_borrow_mut_delegator_vote_delegation(<br/>        pool,<br/>        governance_records,<br/>        shareholder<br/>    );<br/>    let current_voter &#61; vote_delegation.voter;<br/>    let pending_voter &#61; vote_delegation.pending_voter;<br/>    let current_delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, current_voter);<br/>    current_delegated_votes.active_shares &#61; current_delegated_votes.active_shares &#45; shares_to_redeem;<br/>    if (current_voter &#61;&#61; pending_voter) &#123;<br/>        current_delegated_votes.active_shares_next_lockup &#61;<br/>            current_delegated_votes.active_shares_next_lockup &#45; shares_to_redeem;<br/>    &#125; else &#123;<br/>        let pending_delegated_votes &#61;<br/>            update_and_borrow_mut_delegated_votes(pool, governance_records, pending_voter);<br/>        pending_delegated_votes.active_shares_next_lockup &#61;<br/>            pending_delegated_votes.active_shares_next_lockup &#45; shares_to_redeem;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_update_governanace_records_for_redeem_pending_inactive_shares"></a>

## Function `update_governanace_records_for_redeem_pending_inactive_shares`



<pre><code>fun update_governanace_records_for_redeem_pending_inactive_shares(pool: &amp;delegation_pool::DelegationPool, pool_address: address, shares_to_redeem: u128, shareholder: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_governanace_records_for_redeem_pending_inactive_shares(<br/>    pool: &amp;DelegationPool, pool_address: address, shares_to_redeem: u128, shareholder: address<br/>) acquires GovernanceRecords &#123;<br/>    // &lt;pending inactive shares&gt; of &lt;shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;  &#45;&#45;&#45;&#45;&gt;<br/>    // &lt;pending inactive shares&gt; of &lt;current voter of shareholder&gt; &#45;&#61; &lt;shares_to_redeem&gt;<br/>    // no impact on &lt;pending inactive shares&gt; of &lt;next voter of shareholder&gt;<br/>    let governance_records &#61; borrow_global_mut&lt;GovernanceRecords&gt;(pool_address);<br/>    let current_voter &#61; calculate_and_update_delegator_voter_internal(pool, governance_records, shareholder);<br/>    let current_delegated_votes &#61; update_and_borrow_mut_delegated_votes(pool, governance_records, current_voter);<br/>    current_delegated_votes.pending_inactive_shares &#61; current_delegated_votes.pending_inactive_shares &#45; shares_to_redeem;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_delegation_pool_multiply_then_divide"></a>

## Function `multiply_then_divide`

Deprecated, prefer math64::mul_div


<pre><code>&#35;[deprecated]<br/>public fun multiply_then_divide(x: u64, y: u64, z: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multiply_then_divide(x: u64, y: u64, z: u64): u64 &#123;<br/>    math64::mul_div(x, y, z)<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;Every DelegationPool has only one corresponding StakePool stored at the same address.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;Upon calling the initialize_delegation_pool function, a resource account is created from the &quot;owner&quot; signer to host the delegation pool resource and own the underlying stake pool.&lt;/td&gt;<br/>&lt;td&gt;Audited that the address of StakePool equals address of DelegationPool and the data invariant on the DelegationPool.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The signer capability within the delegation pool has an address equal to the address of the delegation pool.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The initialize_delegation_pool function moves the DelegationPool resource to the address associated with stake_pool_signer, which also possesses the signer capability.&lt;/td&gt;<br/>&lt;td&gt;Audited that the address of signer cap equals address of DelegationPool.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;A delegator holds shares exclusively in one inactive shares pool, which could either be an already inactive pool or the pending_inactive pool.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The get_stake function returns the inactive stake owned by a delegator and checks which state the shares are in via the get_pending_withdrawal function.&lt;/td&gt;<br/>&lt;td&gt;Audited that either inactive or pending_inactive stake after invoking the get_stake function is zero and both are never non&#45;zero.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The specific pool in which the delegator possesses inactive shares becomes designated as the pending withdrawal pool for that delegator.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The get_pending_withdrawal function checks if any pending withdrawal exists for a delegate address and if there is neither inactive nor pending_inactive stake, the pending_withdrawal_exists returns false.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;The existence of a pending withdrawal implies that it is associated with a pool where the delegator possesses inactive shares.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;In the get_pending_withdrawal function, if withdrawal_exists is true, the function returns true and a non&#45;zero amount&lt;/td&gt;<br/>&lt;td&gt;get_pending_withdrawal has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;An inactive shares pool should have coins allocated to it; otherwise, it should become deleted.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The redeem_inactive_shares function has a check that destroys the inactive shares pool, given that it is empty.&lt;/td&gt;<br/>&lt;td&gt;shares pools have been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;The index of the pending withdrawal will not exceed the current OLC on DelegationPool.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The get_pending_withdrawal function has a check which ensures that withdrawal_olc.index &lt; pool.observed_lockup_cycle.index.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;8&lt;/td&gt;<br/>&lt;td&gt;Slashing is not possible for inactive stakes.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The number of inactive staked coins must be greater than or equal to the total_coins_inactive of the pool.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;9&lt;/td&gt;<br/>&lt;td&gt;The delegator&apos;s active or pending inactive stake will always meet or exceed the minimum allowed value.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The add_stake, unlock and reactivate_stake functions ensure the active_shares or pending_inactive_shares balance for the delegator is greater than or equal to the MIN_COINS_ON_SHARES_POOL value.&lt;/td&gt;<br/>&lt;td&gt;Audited the comparison of active_shares or inactive_shares balance for the delegator with the MIN_COINS_ON_SHARES_POOL value.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;10&lt;/td&gt;<br/>&lt;td&gt;The delegation pool exists at a given address.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;Functions that operate on the DelegationPool abort if there is no DelegationPool struct under the given pool_address.&lt;/td&gt;<br/>&lt;td&gt;Audited that there is no DelegationPool structure assigned to the pool_address given as a parameter.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;11&lt;/td&gt;<br/>&lt;td&gt;The initialization of the delegation pool is contingent upon enabling the delegation pools feature.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The initialize_delegation_pool function should proceed if the DELEGATION_POOLS feature is enabled.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify&#61;false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
