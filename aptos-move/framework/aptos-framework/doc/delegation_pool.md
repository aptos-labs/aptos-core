
<a name="0x1_delegation_pool"></a>

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
-  [Struct `AddStakeEvent`](#0x1_delegation_pool_AddStakeEvent)
-  [Struct `ReactivateStakeEvent`](#0x1_delegation_pool_ReactivateStakeEvent)
-  [Struct `UnlockStakeEvent`](#0x1_delegation_pool_UnlockStakeEvent)
-  [Struct `WithdrawStakeEvent`](#0x1_delegation_pool_WithdrawStakeEvent)
-  [Struct `DistributeCommissionEvent`](#0x1_delegation_pool_DistributeCommissionEvent)
-  [Constants](#@Constants_0)
-  [Function `owner_cap_exists`](#0x1_delegation_pool_owner_cap_exists)
-  [Function `get_owned_pool_address`](#0x1_delegation_pool_get_owned_pool_address)
-  [Function `delegation_pool_exists`](#0x1_delegation_pool_delegation_pool_exists)
-  [Function `observed_lockup_cycle`](#0x1_delegation_pool_observed_lockup_cycle)
-  [Function `operator_commission_percentage`](#0x1_delegation_pool_operator_commission_percentage)
-  [Function `shareholders_count_active_pool`](#0x1_delegation_pool_shareholders_count_active_pool)
-  [Function `get_delegation_pool_stake`](#0x1_delegation_pool_get_delegation_pool_stake)
-  [Function `get_pending_withdrawal`](#0x1_delegation_pool_get_pending_withdrawal)
-  [Function `get_stake`](#0x1_delegation_pool_get_stake)
-  [Function `get_add_stake_fee`](#0x1_delegation_pool_get_add_stake_fee)
-  [Function `can_withdraw_pending_inactive`](#0x1_delegation_pool_can_withdraw_pending_inactive)
-  [Function `initialize_delegation_pool`](#0x1_delegation_pool_initialize_delegation_pool)
-  [Function `assert_owner_cap_exists`](#0x1_delegation_pool_assert_owner_cap_exists)
-  [Function `assert_delegation_pool_exists`](#0x1_delegation_pool_assert_delegation_pool_exists)
-  [Function `assert_min_active_balance`](#0x1_delegation_pool_assert_min_active_balance)
-  [Function `assert_min_pending_inactive_balance`](#0x1_delegation_pool_assert_min_pending_inactive_balance)
-  [Function `coins_to_redeem_to_ensure_min_stake`](#0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake)
-  [Function `coins_to_transfer_to_ensure_min_stake`](#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake)
-  [Function `retrieve_stake_pool_owner`](#0x1_delegation_pool_retrieve_stake_pool_owner)
-  [Function `get_pool_address`](#0x1_delegation_pool_get_pool_address)
-  [Function `olc_with_index`](#0x1_delegation_pool_olc_with_index)
-  [Function `set_operator`](#0x1_delegation_pool_set_operator)
-  [Function `set_delegated_voter`](#0x1_delegation_pool_set_delegated_voter)
-  [Function `add_stake`](#0x1_delegation_pool_add_stake)
-  [Function `unlock`](#0x1_delegation_pool_unlock)
-  [Function `reactivate_stake`](#0x1_delegation_pool_reactivate_stake)
-  [Function `withdraw`](#0x1_delegation_pool_withdraw)
-  [Function `withdraw_internal`](#0x1_delegation_pool_withdraw_internal)
-  [Function `pending_withdrawal_exists`](#0x1_delegation_pool_pending_withdrawal_exists)
-  [Function `pending_inactive_shares_pool_mut`](#0x1_delegation_pool_pending_inactive_shares_pool_mut)
-  [Function `pending_inactive_shares_pool`](#0x1_delegation_pool_pending_inactive_shares_pool)
-  [Function `execute_pending_withdrawal`](#0x1_delegation_pool_execute_pending_withdrawal)
-  [Function `buy_in_pending_inactive_shares`](#0x1_delegation_pool_buy_in_pending_inactive_shares)
-  [Function `amount_to_shares_to_redeem`](#0x1_delegation_pool_amount_to_shares_to_redeem)
-  [Function `redeem_active_shares`](#0x1_delegation_pool_redeem_active_shares)
-  [Function `redeem_inactive_shares`](#0x1_delegation_pool_redeem_inactive_shares)
-  [Function `calculate_stake_pool_drift`](#0x1_delegation_pool_calculate_stake_pool_drift)
-  [Function `synchronize_delegation_pool`](#0x1_delegation_pool_synchronize_delegation_pool)
-  [Function `multiply_then_divide`](#0x1_delegation_pool_multiply_then_divide)
-  [Function `to_u128`](#0x1_delegation_pool_to_u128)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/pool_u64_unbound.md#0x1_pool_u64_unbound">0x1::pool_u64_unbound</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_delegation_pool_DelegationPoolOwnership"></a>

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

<a name="0x1_delegation_pool_ObservedLockupCycle"></a>

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

<a name="0x1_delegation_pool_DelegationPool"></a>

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

<a name="0x1_delegation_pool_AddStakeEvent"></a>

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

<a name="0x1_delegation_pool_ReactivateStakeEvent"></a>

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

<a name="0x1_delegation_pool_UnlockStakeEvent"></a>

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

<a name="0x1_delegation_pool_WithdrawStakeEvent"></a>

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

<a name="0x1_delegation_pool_DistributeCommissionEvent"></a>

## Struct `DistributeCommissionEvent`



<pre><code><b>struct</b> <a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">DistributeCommissionEvent</a> <b>has</b> drop, store
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_delegation_pool_MAX_U64"></a>



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a name="0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS"></a>

Account is already owning a delegation pool.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>: u64 = 2;
</code></pre>



<a name="0x1_delegation_pool_EOWNER_CAP_NOT_FOUND"></a>

Delegation pool owner capability does not exist at the provided account.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_NOT_FOUND">EOWNER_CAP_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a name="0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE"></a>



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_VALIDATOR_STATUS_INACTIVE">VALIDATOR_STATUS_INACTIVE</a>: u64 = 4;
</code></pre>



<a name="0x1_delegation_pool_EDELEGATION_POOLS_DISABLED"></a>

Creating delegation pools is not enabled yet.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOLS_DISABLED">EDELEGATION_POOLS_DISABLED</a>: u64 = 10;
</code></pre>



<a name="0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST"></a>

Delegation pool does not exist at the provided pool address.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOL_DOES_NOT_EXIST">EDELEGATION_POOL_DOES_NOT_EXIST</a>: u64 = 3;
</code></pre>



<a name="0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW"></a>

Delegator's active balance cannot be less than <code><a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a></code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_ACTIVE_BALANCE_TOO_LOW">EDELEGATOR_ACTIVE_BALANCE_TOO_LOW</a>: u64 = 8;
</code></pre>



<a name="0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW"></a>

Delegator's pending_inactive balance cannot be less than <code><a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a></code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW">EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW</a>: u64 = 9;
</code></pre>



<a name="0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE"></a>

Commission percentage has to be between 0 and <code><a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a></code> - 100%.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>: u64 = 5;
</code></pre>



<a name="0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK"></a>

There is not enough <code>active</code> stake on the stake pool to <code>unlock</code>.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK">ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK</a>: u64 = 6;
</code></pre>



<a name="0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS"></a>

There is a pending withdrawal to be executed before <code>unlock</code>ing any new stake.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EPENDING_WITHDRAWAL_EXISTS">EPENDING_WITHDRAWAL_EXISTS</a>: u64 = 4;
</code></pre>



<a name="0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC"></a>

Slashing (if implemented) should not be applied to already <code>inactive</code> stake.
Not only it invalidates the accounting of past observed lockup cycles (OLC),
but is also unfair to delegators whose stake has been inactive before validator started misbehaving.
Additionally, the inactive stake does not count on the voting power of validator.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_ESLASHED_INACTIVE_STAKE_ON_PAST_OLC">ESLASHED_INACTIVE_STAKE_ON_PAST_OLC</a>: u64 = 7;
</code></pre>



<a name="0x1_delegation_pool_EWITHDRAW_ZERO_STAKE"></a>

Cannot request to withdraw zero stake.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_EWITHDRAW_ZERO_STAKE">EWITHDRAW_ZERO_STAKE</a>: u64 = 11;
</code></pre>



<a name="0x1_delegation_pool_MAX_FEE"></a>

Maximum operator percentage fee(of double digit precision): 22.85% is represented as 2285


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>: u64 = 10000;
</code></pre>



<a name="0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL"></a>

Minimum coins to exist on a shares pool at all times.
Enforced per delegator for both active and pending_inactive pools.
This constraint ensures the share price cannot overly increase and lead to
substantial loses when buying shares (can lose at most 1 share which may
be worth a lot if current share price is high).
This constraint is not enforced on inactive pools as they only allow redeems
(can lose at most 1 coin regardless of current share price).


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MIN_COINS_ON_SHARES_POOL">MIN_COINS_ON_SHARES_POOL</a>: u64 = 1000000000;
</code></pre>



<a name="0x1_delegation_pool_MODULE_SALT"></a>



<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_MODULE_SALT">MODULE_SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 100, 101, 108, 101, 103, 97, 116, 105, 111, 110, 95, 112, 111, 111, 108];
</code></pre>



<a name="0x1_delegation_pool_NULL_SHAREHOLDER"></a>

Special shareholder temporarily owning the <code>add_stake</code> fees charged during this epoch.
On each <code>add_stake</code> operation any resulted fee is used to buy active shares for this shareholder.
First synchronization after this epoch ends will distribute accumulated fees to the rest of the pool as refunds.


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>: <b>address</b> = 0x0;
</code></pre>



<a name="0x1_delegation_pool_SHARES_SCALING_FACTOR"></a>

Scaling factor of shares pools used within the delegation pool


<pre><code><b>const</b> <a href="delegation_pool.md#0x1_delegation_pool_SHARES_SCALING_FACTOR">SHARES_SCALING_FACTOR</a>: u64 = 10000000000000000;
</code></pre>



<a name="0x1_delegation_pool_owner_cap_exists"></a>

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

<a name="0x1_delegation_pool_get_owned_pool_address"></a>

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

<a name="0x1_delegation_pool_delegation_pool_exists"></a>

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

<a name="0x1_delegation_pool_observed_lockup_cycle"></a>

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

<a name="0x1_delegation_pool_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the operator commission percentage set on the delegation pool <code>pool_address</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a>(pool_address: <b>address</b>): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
    <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address).operator_commission_percentage
}
</code></pre>



</details>

<a name="0x1_delegation_pool_shareholders_count_active_pool"></a>

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

<a name="0x1_delegation_pool_get_delegation_pool_stake"></a>

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

<a name="0x1_delegation_pool_get_pending_withdrawal"></a>

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

<a name="0x1_delegation_pool_get_stake"></a>

## Function `get_stake`

Return total stake owned by <code>delegator_address</code> within delegation pool <code>pool_address</code>
in each of its individual states: (<code>active</code>,<code>inactive</code>,<code>pending_inactive</code>)


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_stake">get_stake</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): (u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_stake">get_stake</a>(pool_address: <b>address</b>, delegator_address: <b>address</b>): (u64, u64, u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
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
    <b>if</b> (delegator_address == <a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address)) {
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

<a name="0x1_delegation_pool_get_add_stake_fee"></a>

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


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_get_add_stake_fee">get_add_stake_fee</a>(pool_address: <b>address</b>, amount: u64): u64 <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <b>if</b> (<a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(pool_address)) {
        <b>let</b> (rewards_rate, rewards_rate_denominator) = <a href="staking_config.md#0x1_staking_config_get_reward_rate">staking_config::get_reward_rate</a>(&<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());
        <b>if</b> (rewards_rate_denominator &gt; 0) {
            <a href="delegation_pool.md#0x1_delegation_pool_assert_delegation_pool_exists">assert_delegation_pool_exists</a>(pool_address);
            <b>let</b> pool = <b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);

            rewards_rate = rewards_rate * (<a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a> - pool.operator_commission_percentage);
            rewards_rate_denominator = rewards_rate_denominator * <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>;
            ((((amount <b>as</b> u128) * (rewards_rate <b>as</b> u128)) / ((rewards_rate <b>as</b> u128) + (rewards_rate_denominator <b>as</b> u128))) <b>as</b> u64)
        } <b>else</b> { 0 }
    } <b>else</b> { 0 }
}
</code></pre>



</details>

<a name="0x1_delegation_pool_can_withdraw_pending_inactive"></a>

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

<a name="0x1_delegation_pool_initialize_delegation_pool"></a>

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
) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_delegation_pools_enabled">features::delegation_pools_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool.md#0x1_delegation_pool_EDELEGATION_POOLS_DISABLED">EDELEGATION_POOLS_DISABLED</a>));
    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <b>assert</b>!(!<a href="delegation_pool.md#0x1_delegation_pool_owner_cap_exists">owner_cap_exists</a>(owner_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="delegation_pool.md#0x1_delegation_pool_EOWNER_CAP_ALREADY_EXISTS">EOWNER_CAP_ALREADY_EXISTS</a>));
    <b>assert</b>!(<a href="delegation_pool.md#0x1_delegation_pool_operator_commission_percentage">operator_commission_percentage</a> &lt;= <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>));

    // generate a seed <b>to</b> be used <b>to</b> create the resource <a href="account.md#0x1_account">account</a> hosting the delegation pool
    <b>let</b> seed = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    // <b>include</b> <b>module</b> salt (before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> subseeds) <b>to</b> avoid conflicts <b>with</b> other modules creating resource accounts
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, <a href="delegation_pool.md#0x1_delegation_pool_MODULE_SALT">MODULE_SALT</a>);
    // <b>include</b> an additional salt in case the same resource <a href="account.md#0x1_account">account</a> <b>has</b> already been created
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, delegation_pool_creation_seed);

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
}
</code></pre>



</details>

<a name="0x1_delegation_pool_assert_owner_cap_exists"></a>

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

<a name="0x1_delegation_pool_assert_delegation_pool_exists"></a>

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

<a name="0x1_delegation_pool_assert_min_active_balance"></a>

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

<a name="0x1_delegation_pool_assert_min_pending_inactive_balance"></a>

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

<a name="0x1_delegation_pool_coins_to_redeem_to_ensure_min_stake"></a>

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

<a name="0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake"></a>

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

<a name="0x1_delegation_pool_retrieve_stake_pool_owner"></a>

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

<a name="0x1_delegation_pool_get_pool_address"></a>

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

<a name="0x1_delegation_pool_olc_with_index"></a>

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

<a name="0x1_delegation_pool_set_operator"></a>

## Function `set_operator`

Allows an owner to change the operator of the underlying stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_operator">set_operator</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_operator">set_operator</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_operator: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    // ensure the <b>old</b> operator is paid its uncommitted commission rewards
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);
    <a href="stake.md#0x1_stake_set_operator">stake::set_operator</a>(&<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address)), new_operator);
}
</code></pre>



</details>

<a name="0x1_delegation_pool_set_delegated_voter"></a>

## Function `set_delegated_voter`

Allows an owner to change the delegated voter of the underlying stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_delegated_voter">set_delegated_voter</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_set_delegated_voter">set_delegated_voter</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_voter: <b>address</b>
) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPoolOwnership">DelegationPoolOwnership</a>, <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <b>let</b> pool_address = <a href="delegation_pool.md#0x1_delegation_pool_get_owned_pool_address">get_owned_pool_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);
    <a href="stake.md#0x1_stake_set_delegated_voter">stake::set_delegated_voter</a>(&<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(<b>borrow_global</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address)), new_voter);
}
</code></pre>



</details>

<a name="0x1_delegation_pool_add_stake"></a>

## Function `add_stake`

Add <code>amount</code> of coins to the delegation pool <code>pool_address</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_add_stake">add_stake</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_add_stake">add_stake</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    // short-circuit <b>if</b> amount <b>to</b> add is 0 so no <a href="event.md#0x1_event">event</a> is emitted
    <b>if</b> (amount == 0) { <b>return</b> };
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    // fee <b>to</b> be charged for adding `amount` <a href="stake.md#0x1_stake">stake</a> on this delegation pool at this epoch
    <b>let</b> add_stake_fee = <a href="delegation_pool.md#0x1_delegation_pool_get_add_stake_fee">get_add_stake_fee</a>(pool_address, amount);

    <b>let</b> pool = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> delegator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);

    // <a href="stake.md#0x1_stake">stake</a> the entire amount <b>to</b> the <a href="stake.md#0x1_stake">stake</a> pool
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(delegator, pool_address, amount);
    <a href="stake.md#0x1_stake_add_stake">stake::add_stake</a>(&<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool), amount);

    // but buy shares for delegator just for the remaining amount after fee
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(&<b>mut</b> pool.active_shares, delegator_address, amount - add_stake_fee);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool, delegator_address);

    // grant temporary ownership over `add_stake` fees <b>to</b> a separate shareholder in order <b>to</b>:
    // - not mistake them for rewards <b>to</b> pay the operator from
    // - distribute them together <b>with</b> the `active` rewards when this epoch ends
    // in order <b>to</b> appreciate all shares on the active pool atomically
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(&<b>mut</b> pool.active_shares, <a href="delegation_pool.md#0x1_delegation_pool_NULL_SHAREHOLDER">NULL_SHAREHOLDER</a>, add_stake_fee);

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> pool.add_stake_events,
        <a href="delegation_pool.md#0x1_delegation_pool_AddStakeEvent">AddStakeEvent</a> {
            pool_address,
            delegator_address,
            amount_added: amount,
            add_stake_fee,
        },
    );
}
</code></pre>



</details>

<a name="0x1_delegation_pool_unlock"></a>

## Function `unlock`

Unlock <code>amount</code> from the active + pending_active stake of <code>delegator</code> or
at most how much active stake there is on the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock">unlock</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_unlock">unlock</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    // short-circuit <b>if</b> amount <b>to</b> unlock is 0 so no <a href="event.md#0x1_event">event</a> is emitted
    <b>if</b> (amount == 0) { <b>return</b> };

    // fail unlock of more <a href="stake.md#0x1_stake">stake</a> than `active` on the <a href="stake.md#0x1_stake">stake</a> pool
    <b>let</b> (active, _, _, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
    <b>assert</b>!(amount &lt;= active, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK">ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK</a>));

    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> pool = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> delegator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);

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

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> pool.unlock_stake_events,
        <a href="delegation_pool.md#0x1_delegation_pool_UnlockStakeEvent">UnlockStakeEvent</a> {
            pool_address,
            delegator_address,
            amount_unlocked: amount,
        },
    );
}
</code></pre>



</details>

<a name="0x1_delegation_pool_reactivate_stake"></a>

## Function `reactivate_stake`

Move <code>amount</code> of coins from pending_inactive to active.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_reactivate_stake">reactivate_stake</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_reactivate_stake">reactivate_stake</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    // short-circuit <b>if</b> amount <b>to</b> reactivate is 0 so no <a href="event.md#0x1_event">event</a> is emitted
    <b>if</b> (amount == 0) { <b>return</b> };
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);

    <b>let</b> pool = <b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address);
    <b>let</b> delegator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator);

    amount = <a href="delegation_pool.md#0x1_delegation_pool_coins_to_transfer_to_ensure_min_stake">coins_to_transfer_to_ensure_min_stake</a>(
        <a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool),
        &pool.active_shares,
        delegator_address,
        amount,
    );
    <b>let</b> observed_lockup_cycle = pool.observed_lockup_cycle;
    amount = <a href="delegation_pool.md#0x1_delegation_pool_redeem_inactive_shares">redeem_inactive_shares</a>(pool, delegator_address, amount, observed_lockup_cycle);

    <a href="stake.md#0x1_stake_reactivate_stake">stake::reactivate_stake</a>(&<a href="delegation_pool.md#0x1_delegation_pool_retrieve_stake_pool_owner">retrieve_stake_pool_owner</a>(pool), amount);

    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(&<b>mut</b> pool.active_shares, delegator_address, amount);
    <a href="delegation_pool.md#0x1_delegation_pool_assert_min_active_balance">assert_min_active_balance</a>(pool, delegator_address);

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> pool.reactivate_stake_events,
        <a href="delegation_pool.md#0x1_delegation_pool_ReactivateStakeEvent">ReactivateStakeEvent</a> {
            pool_address,
            delegator_address,
            amount_reactivated: amount,
        },
    );
}
</code></pre>



</details>

<a name="0x1_delegation_pool_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of owned inactive stake from the delegation pool at <code>pool_address</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw">withdraw</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw">withdraw</a>(delegator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pool_address: <b>address</b>, amount: u64) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="delegation_pool.md#0x1_delegation_pool_EWITHDRAW_ZERO_STAKE">EWITHDRAW_ZERO_STAKE</a>));
    // synchronize delegation and <a href="stake.md#0x1_stake">stake</a> pools before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> user operation
    <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address);
    <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(<b>borrow_global_mut</b>&lt;<a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>&gt;(pool_address), <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegator), amount);
}
</code></pre>



</details>

<a name="0x1_delegation_pool_withdraw_internal"></a>

## Function `withdraw_internal`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>, amount: u64) {
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
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(stake_pool_owner, delegator_address, amount);

    // commit withdrawal of possibly inactive <a href="stake.md#0x1_stake">stake</a> <b>to</b> the `total_coins_inactive`
    // known by the delegation pool in order <b>to</b> not mistake it for slashing at next synchronization
    <b>let</b> (_, inactive, _, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
    pool.total_coins_inactive = inactive;

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> pool.withdraw_stake_events,
        <a href="delegation_pool.md#0x1_delegation_pool_WithdrawStakeEvent">WithdrawStakeEvent</a> {
            pool_address,
            delegator_address,
            amount_withdrawn: amount,
        },
    );
}
</code></pre>



</details>

<a name="0x1_delegation_pool_pending_withdrawal_exists"></a>

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

<a name="0x1_delegation_pool_pending_inactive_shares_pool_mut"></a>

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

<a name="0x1_delegation_pool_pending_inactive_shares_pool"></a>

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

<a name="0x1_delegation_pool_execute_pending_withdrawal"></a>

## Function `execute_pending_withdrawal`

Execute the pending withdrawal of <code>delegator_address</code> on delegation pool <code>pool</code>
if existing and already inactive to allow the creation of a new one.
<code>pending_inactive</code> stake would be left untouched even if withdrawable and should
be explicitly withdrawn by delegator


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_execute_pending_withdrawal">execute_pending_withdrawal</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">delegation_pool::DelegationPool</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_execute_pending_withdrawal">execute_pending_withdrawal</a>(pool: &<b>mut</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a>, delegator_address: <b>address</b>) {
    <b>let</b> (withdrawal_exists, withdrawal_olc) = <a href="delegation_pool.md#0x1_delegation_pool_pending_withdrawal_exists">pending_withdrawal_exists</a>(pool, delegator_address);
    <b>if</b> (withdrawal_exists && withdrawal_olc.index &lt; pool.observed_lockup_cycle.index) {
        <a href="delegation_pool.md#0x1_delegation_pool_withdraw_internal">withdraw_internal</a>(pool, delegator_address, <a href="delegation_pool.md#0x1_delegation_pool_MAX_U64">MAX_U64</a>);
    }
}
</code></pre>



</details>

<a name="0x1_delegation_pool_buy_in_pending_inactive_shares"></a>

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
): u128 {
    // cannot buy inactive shares, only pending_inactive at current lockup cycle
    <b>let</b> new_shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool_mut">pending_inactive_shares_pool_mut</a>(pool), shareholder, coins_amount);
    // never create a new pending withdrawal unless delegator owns some pending_inactive shares
    <b>if</b> (new_shares == 0) { <b>return</b> 0 };

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

<a name="0x1_delegation_pool_amount_to_shares_to_redeem"></a>

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

<a name="0x1_delegation_pool_redeem_active_shares"></a>

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
): u64 {
    <b>let</b> shares_to_redeem = <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(&pool.active_shares, shareholder, coins_amount);
    // silently exit <b>if</b> not a shareholder otherwise redeem would fail <b>with</b> `ESHAREHOLDER_NOT_FOUND`
    <b>if</b> (shares_to_redeem == 0) <b>return</b> 0;
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_redeem_shares">pool_u64::redeem_shares</a>(&<b>mut</b> pool.active_shares, shareholder, shares_to_redeem)
}
</code></pre>



</details>

<a name="0x1_delegation_pool_redeem_inactive_shares"></a>

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
): u64 {
    <b>let</b> inactive_shares = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> pool.inactive_shares, lockup_cycle);
    <b>let</b> shares_to_redeem = <a href="delegation_pool.md#0x1_delegation_pool_amount_to_shares_to_redeem">amount_to_shares_to_redeem</a>(inactive_shares, shareholder, coins_amount);
    // silently exit <b>if</b> not a shareholder otherwise redeem would fail <b>with</b> `ESHAREHOLDER_NOT_FOUND`
    <b>if</b> (shares_to_redeem == 0) <b>return</b> 0;
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

<a name="0x1_delegation_pool_calculate_stake_pool_drift"></a>

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
    <b>let</b> commission_active = total_coins(&pool.active_shares);
    commission_active = <b>if</b> (active &gt; commission_active) {
        <a href="delegation_pool.md#0x1_delegation_pool_multiply_then_divide">multiply_then_divide</a>(active - commission_active, pool.operator_commission_percentage, <a href="delegation_pool.md#0x1_delegation_pool_MAX_FEE">MAX_FEE</a>)
    } <b>else</b> {
        // handle <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> slashing applied <b>to</b> `active` <a href="stake.md#0x1_stake">stake</a>
        0
    };
    // operator `pending_inactive` rewards not persisted yet <b>to</b> the pending_inactive shares pool
    <b>let</b> commission_pending_inactive = total_coins(<a href="delegation_pool.md#0x1_delegation_pool_pending_inactive_shares_pool">pending_inactive_shares_pool</a>(pool));
    commission_pending_inactive = <b>if</b> (pending_inactive &gt; commission_pending_inactive) {
        <a href="delegation_pool.md#0x1_delegation_pool_multiply_then_divide">multiply_then_divide</a>(
            pending_inactive - commission_pending_inactive,
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

<a name="0x1_delegation_pool_synchronize_delegation_pool"></a>

## Function `synchronize_delegation_pool`

Synchronize delegation and stake pools: distribute yet-undetected rewards to the corresponding internal
shares pools, assign commission to operator and eventually prepare delegation pool for a new lockup cycle.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_synchronize_delegation_pool">synchronize_delegation_pool</a>(pool_address: <b>address</b>) <b>acquires</b> <a href="delegation_pool.md#0x1_delegation_pool_DelegationPool">DelegationPool</a> {
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
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(&<b>mut</b> pool.active_shares, <a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address), commission_active);
    // reward operator its commission out of uncommitted pending_inactive rewards
    <a href="delegation_pool.md#0x1_delegation_pool_buy_in_pending_inactive_shares">buy_in_pending_inactive_shares</a>(pool, <a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address), commission_pending_inactive);

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> pool.distribute_commission_events,
        <a href="delegation_pool.md#0x1_delegation_pool_DistributeCommissionEvent">DistributeCommissionEvent</a> {
            pool_address,
            operator: <a href="stake.md#0x1_stake_get_operator">stake::get_operator</a>(pool_address),
            commission_active,
            commission_pending_inactive,
        },
    );

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
    }
}
</code></pre>



</details>

<a name="0x1_delegation_pool_multiply_then_divide"></a>

## Function `multiply_then_divide`



<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_multiply_then_divide">multiply_then_divide</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_multiply_then_divide">multiply_then_divide</a>(x: u64, y: u64, z: u64): u64 {
    <b>let</b> result = (<a href="delegation_pool.md#0x1_delegation_pool_to_u128">to_u128</a>(x) * <a href="delegation_pool.md#0x1_delegation_pool_to_u128">to_u128</a>(y)) / <a href="delegation_pool.md#0x1_delegation_pool_to_u128">to_u128</a>(z);
    (result <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x1_delegation_pool_to_u128"></a>

## Function `to_u128`



<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_to_u128">to_u128</a>(num: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool.md#0x1_delegation_pool_to_u128">to_u128</a>(num: u64): u128 {
    (num <b>as</b> u128)
}
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
