
<a id="0x1_staking_contract"></a>

# Module `0x1::staking_contract`

Allow stakers and operators to enter a staking contract with reward sharing.
The main accounting logic in a staking contract consists of 2 parts:
1. Tracks how much commission needs to be paid out to the operator. This is tracked with an increasing principal
amount that&apos;s updated every time the operator requests commission, the staker withdraws funds, or the staker
switches operators.
2. Distributions of funds to operators (commissions) and stakers (stake withdrawals) use the shares model provided
by the pool_u64 to track shares that increase in price as the stake pool accumulates rewards.

Example flow:
1. A staker creates a staking contract with an operator by calling create_staking_contract() with 100 coins of
initial stake and commission &#61; 10%. This means the operator will receive 10% of any accumulated rewards. A new stake
pool will be created and hosted in a separate account that&apos;s controlled by the staking contract.
2. The operator sets up a validator node and, once ready, joins the validator set by calling stake::join_validator_set
3. After some time, the stake pool gains rewards and now has 150 coins.
4. Operator can now call request_commission. 10% of (150 &#45; 100) &#61; 5 coins will be unlocked from the stake pool. The
staker&apos;s principal is now updated from 100 to 145 (150 coins &#45; 5 coins of commission). The pending distribution pool
has 5 coins total and the operator owns all 5 shares of it.
5. Some more time has passed. The pool now has 50 more coins in rewards and a total balance of 195. The operator
calls request_commission again. Since the previous 5 coins have now become withdrawable, it&apos;ll be deposited into the
operator&apos;s account first. Their new commission will be 10% of (195 coins &#45; 145 principal) &#61; 5 coins. Principal is
updated to be 190 (195 &#45; 5). Pending distribution pool has 5 coins and operator owns all 5 shares.
6. Staker calls unlock_stake to unlock 50 coins of stake, which gets added to the pending distribution pool. Based
on shares math, staker will be owning 50 shares and operator still owns 5 shares of the 55&#45;coin pending distribution
pool.
7. Some time passes and the 55 coins become fully withdrawable from the stake pool. Due to accumulated rewards, the
55 coins become 70 coins. Calling distribute() distributes 6 coins to the operator and 64 coins to the validator.


-  [Struct `StakingGroupContainer`](#0x1_staking_contract_StakingGroupContainer)
-  [Struct `StakingContract`](#0x1_staking_contract_StakingContract)
-  [Resource `Store`](#0x1_staking_contract_Store)
-  [Resource `BeneficiaryForOperator`](#0x1_staking_contract_BeneficiaryForOperator)
-  [Struct `UpdateCommissionEvent`](#0x1_staking_contract_UpdateCommissionEvent)
-  [Struct `UpdateCommission`](#0x1_staking_contract_UpdateCommission)
-  [Resource `StakingGroupUpdateCommissionEvent`](#0x1_staking_contract_StakingGroupUpdateCommissionEvent)
-  [Struct `CreateStakingContract`](#0x1_staking_contract_CreateStakingContract)
-  [Struct `UpdateVoter`](#0x1_staking_contract_UpdateVoter)
-  [Struct `ResetLockup`](#0x1_staking_contract_ResetLockup)
-  [Struct `AddStake`](#0x1_staking_contract_AddStake)
-  [Struct `RequestCommission`](#0x1_staking_contract_RequestCommission)
-  [Struct `UnlockStake`](#0x1_staking_contract_UnlockStake)
-  [Struct `SwitchOperator`](#0x1_staking_contract_SwitchOperator)
-  [Struct `AddDistribution`](#0x1_staking_contract_AddDistribution)
-  [Struct `Distribute`](#0x1_staking_contract_Distribute)
-  [Struct `SetBeneficiaryForOperator`](#0x1_staking_contract_SetBeneficiaryForOperator)
-  [Struct `CreateStakingContractEvent`](#0x1_staking_contract_CreateStakingContractEvent)
-  [Struct `UpdateVoterEvent`](#0x1_staking_contract_UpdateVoterEvent)
-  [Struct `ResetLockupEvent`](#0x1_staking_contract_ResetLockupEvent)
-  [Struct `AddStakeEvent`](#0x1_staking_contract_AddStakeEvent)
-  [Struct `RequestCommissionEvent`](#0x1_staking_contract_RequestCommissionEvent)
-  [Struct `UnlockStakeEvent`](#0x1_staking_contract_UnlockStakeEvent)
-  [Struct `SwitchOperatorEvent`](#0x1_staking_contract_SwitchOperatorEvent)
-  [Struct `AddDistributionEvent`](#0x1_staking_contract_AddDistributionEvent)
-  [Struct `DistributeEvent`](#0x1_staking_contract_DistributeEvent)
-  [Constants](#@Constants_0)
-  [Function `stake_pool_address`](#0x1_staking_contract_stake_pool_address)
-  [Function `last_recorded_principal`](#0x1_staking_contract_last_recorded_principal)
-  [Function `commission_percentage`](#0x1_staking_contract_commission_percentage)
-  [Function `staking_contract_amounts`](#0x1_staking_contract_staking_contract_amounts)
-  [Function `pending_distribution_counts`](#0x1_staking_contract_pending_distribution_counts)
-  [Function `staking_contract_exists`](#0x1_staking_contract_staking_contract_exists)
-  [Function `beneficiary_for_operator`](#0x1_staking_contract_beneficiary_for_operator)
-  [Function `get_expected_stake_pool_address`](#0x1_staking_contract_get_expected_stake_pool_address)
-  [Function `create_staking_contract`](#0x1_staking_contract_create_staking_contract)
-  [Function `create_staking_contract_with_coins`](#0x1_staking_contract_create_staking_contract_with_coins)
-  [Function `add_stake`](#0x1_staking_contract_add_stake)
-  [Function `update_voter`](#0x1_staking_contract_update_voter)
-  [Function `reset_lockup`](#0x1_staking_contract_reset_lockup)
-  [Function `update_commision`](#0x1_staking_contract_update_commision)
-  [Function `request_commission`](#0x1_staking_contract_request_commission)
-  [Function `request_commission_internal`](#0x1_staking_contract_request_commission_internal)
-  [Function `unlock_stake`](#0x1_staking_contract_unlock_stake)
-  [Function `unlock_rewards`](#0x1_staking_contract_unlock_rewards)
-  [Function `switch_operator_with_same_commission`](#0x1_staking_contract_switch_operator_with_same_commission)
-  [Function `switch_operator`](#0x1_staking_contract_switch_operator)
-  [Function `set_beneficiary_for_operator`](#0x1_staking_contract_set_beneficiary_for_operator)
-  [Function `distribute`](#0x1_staking_contract_distribute)
-  [Function `distribute_internal`](#0x1_staking_contract_distribute_internal)
-  [Function `assert_staking_contract_exists`](#0x1_staking_contract_assert_staking_contract_exists)
-  [Function `add_distribution`](#0x1_staking_contract_add_distribution)
-  [Function `get_staking_contract_amounts_internal`](#0x1_staking_contract_get_staking_contract_amounts_internal)
-  [Function `create_stake_pool`](#0x1_staking_contract_create_stake_pool)
-  [Function `update_distribution_pool`](#0x1_staking_contract_update_distribution_pool)
-  [Function `create_resource_account_seed`](#0x1_staking_contract_create_resource_account_seed)
-  [Function `new_staking_contracts_holder`](#0x1_staking_contract_new_staking_contracts_holder)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `stake_pool_address`](#@Specification_1_stake_pool_address)
    -  [Function `last_recorded_principal`](#@Specification_1_last_recorded_principal)
    -  [Function `commission_percentage`](#@Specification_1_commission_percentage)
    -  [Function `staking_contract_amounts`](#@Specification_1_staking_contract_amounts)
    -  [Function `pending_distribution_counts`](#@Specification_1_pending_distribution_counts)
    -  [Function `staking_contract_exists`](#@Specification_1_staking_contract_exists)
    -  [Function `beneficiary_for_operator`](#@Specification_1_beneficiary_for_operator)
    -  [Function `create_staking_contract`](#@Specification_1_create_staking_contract)
    -  [Function `create_staking_contract_with_coins`](#@Specification_1_create_staking_contract_with_coins)
    -  [Function `add_stake`](#@Specification_1_add_stake)
    -  [Function `update_voter`](#@Specification_1_update_voter)
    -  [Function `reset_lockup`](#@Specification_1_reset_lockup)
    -  [Function `update_commision`](#@Specification_1_update_commision)
    -  [Function `request_commission`](#@Specification_1_request_commission)
    -  [Function `request_commission_internal`](#@Specification_1_request_commission_internal)
    -  [Function `unlock_stake`](#@Specification_1_unlock_stake)
    -  [Function `unlock_rewards`](#@Specification_1_unlock_rewards)
    -  [Function `switch_operator_with_same_commission`](#@Specification_1_switch_operator_with_same_commission)
    -  [Function `switch_operator`](#@Specification_1_switch_operator)
    -  [Function `set_beneficiary_for_operator`](#@Specification_1_set_beneficiary_for_operator)
    -  [Function `distribute`](#@Specification_1_distribute)
    -  [Function `distribute_internal`](#@Specification_1_distribute_internal)
    -  [Function `assert_staking_contract_exists`](#@Specification_1_assert_staking_contract_exists)
    -  [Function `add_distribution`](#@Specification_1_add_distribution)
    -  [Function `get_staking_contract_amounts_internal`](#@Specification_1_get_staking_contract_amounts_internal)
    -  [Function `create_stake_pool`](#@Specification_1_create_stake_pool)
    -  [Function `update_distribution_pool`](#@Specification_1_update_distribution_pool)
    -  [Function `new_staking_contracts_holder`](#@Specification_1_new_staking_contracts_holder)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64">0x1::pool_u64</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_staking_contract_StakingGroupContainer"></a>

## Struct `StakingGroupContainer`



<pre><code>&#35;[resource_group(&#35;[scope &#61; module_])]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_StakingGroupContainer">StakingGroupContainer</a><br /></code></pre>



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

<a id="0x1_staking_contract_StakingContract"></a>

## Struct `StakingContract`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">StakingContract</a> <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>principal: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>owner_cap: <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a></code>
</dt>
<dd>

</dd>
<dt>
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>distribution_pool: <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a></code>
</dt>
<dd>

</dd>
<dt>
<code>signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_Store"></a>

## Resource `Store`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>staking_contracts: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_staking_contract_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_CreateStakingContractEvent">staking_contract::CreateStakingContractEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_voter_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_UpdateVoterEvent">staking_contract::UpdateVoterEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reset_lockup_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_ResetLockupEvent">staking_contract::ResetLockupEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_AddStakeEvent">staking_contract::AddStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>request_commission_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_RequestCommissionEvent">staking_contract::RequestCommissionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_stake_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_UnlockStakeEvent">staking_contract::UnlockStakeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>switch_operator_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_SwitchOperatorEvent">staking_contract::SwitchOperatorEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_distribution_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">staking_contract::AddDistributionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>distribute_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_DistributeEvent">staking_contract::DistributeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_BeneficiaryForOperator"></a>

## Resource `BeneficiaryForOperator`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> <b>has</b> key<br /></code></pre>



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

<a id="0x1_staking_contract_UpdateCommissionEvent"></a>

## Struct `UpdateCommissionEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_UpdateCommissionEvent">UpdateCommissionEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>staker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_commission_percentage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_commission_percentage: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_UpdateCommission"></a>

## Struct `UpdateCommission`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_UpdateCommission">UpdateCommission</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>staker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_commission_percentage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_commission_percentage: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_StakingGroupUpdateCommissionEvent"></a>

## Resource `StakingGroupUpdateCommissionEvent`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="staking_contract.md#0x1_staking_contract_StakingGroupContainer">0x1::staking_contract::StakingGroupContainer</a>])]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_StakingGroupUpdateCommissionEvent">StakingGroupUpdateCommissionEvent</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>update_commission_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_UpdateCommissionEvent">staking_contract::UpdateCommissionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_CreateStakingContract"></a>

## Struct `CreateStakingContract`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_CreateStakingContract">CreateStakingContract</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>principal: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_UpdateVoter"></a>

## Struct `UpdateVoter`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_UpdateVoter">UpdateVoter</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_voter: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_ResetLockup"></a>

## Struct `ResetLockup`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_ResetLockup">ResetLockup</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddStake"></a>

## Struct `AddStake`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_AddStake">AddStake</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
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

<a id="0x1_staking_contract_RequestCommission"></a>

## Struct `RequestCommission`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_RequestCommission">RequestCommission</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>accumulated_rewards: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>commission_amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_UnlockStake"></a>

## Struct `UnlockStake`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_UnlockStake">UnlockStake</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
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
<dt>
<code>commission_paid: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_SwitchOperator"></a>

## Struct `SwitchOperator`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_SwitchOperator">SwitchOperator</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
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
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddDistribution"></a>

## Struct `AddDistribution`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_AddDistribution">AddDistribution</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
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

<a id="0x1_staking_contract_Distribute"></a>

## Struct `Distribute`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_Distribute">Distribute</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: <b>address</b></code>
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

<a id="0x1_staking_contract_SetBeneficiaryForOperator"></a>

## Struct `SetBeneficiaryForOperator`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_SetBeneficiaryForOperator">SetBeneficiaryForOperator</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_staking_contract_CreateStakingContractEvent"></a>

## Struct `CreateStakingContractEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_CreateStakingContractEvent">CreateStakingContractEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>principal: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_UpdateVoterEvent"></a>

## Struct `UpdateVoterEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_UpdateVoterEvent">UpdateVoterEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_voter: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_ResetLockupEvent"></a>

## Struct `ResetLockupEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_ResetLockupEvent">ResetLockupEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddStakeEvent"></a>

## Struct `AddStakeEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_AddStakeEvent">AddStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
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

<a id="0x1_staking_contract_RequestCommissionEvent"></a>

## Struct `RequestCommissionEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_RequestCommissionEvent">RequestCommissionEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>accumulated_rewards: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>commission_amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_UnlockStakeEvent"></a>

## Struct `UnlockStakeEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_UnlockStakeEvent">UnlockStakeEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
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
<dt>
<code>commission_paid: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_SwitchOperatorEvent"></a>

## Struct `SwitchOperatorEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_SwitchOperatorEvent">SwitchOperatorEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
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
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddDistributionEvent"></a>

## Struct `AddDistributionEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">AddDistributionEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
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

<a id="0x1_staking_contract_DistributeEvent"></a>

## Struct `DistributeEvent`



<pre><code><b>struct</b> <a href="staking_contract.md#0x1_staking_contract_DistributeEvent">DistributeEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: <b>address</b></code>
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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_staking_contract_EINVALID_COMMISSION_PERCENTAGE"></a>

Commission percentage has to be between 0 and 100.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_staking_contract_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED"></a>

Chaning beneficiaries for operators is not supported.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED">EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_staking_contract_ECANT_MERGE_STAKING_CONTRACTS"></a>

Staking contracts can&apos;t be merged.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_ECANT_MERGE_STAKING_CONTRACTS">ECANT_MERGE_STAKING_CONTRACTS</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_staking_contract_EINSUFFICIENT_ACTIVE_STAKE_TO_WITHDRAW"></a>

Not enough active stake to withdraw. Some stake might still pending and will be active in the next epoch.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_EINSUFFICIENT_ACTIVE_STAKE_TO_WITHDRAW">EINSUFFICIENT_ACTIVE_STAKE_TO_WITHDRAW</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_staking_contract_EINSUFFICIENT_STAKE_AMOUNT"></a>

Store amount must be at least the min stake required for a stake pool to join the validator set.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_EINSUFFICIENT_STAKE_AMOUNT">EINSUFFICIENT_STAKE_AMOUNT</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_staking_contract_ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY"></a>

Caller must be either the staker, operator, or beneficiary.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY">ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR"></a>

No staking contract between the staker and operator found.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR">ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_STAKER"></a>

Staker has no staking contracts.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_STAKER">ENO_STAKING_CONTRACT_FOUND_FOR_STAKER</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_staking_contract_ESTAKING_CONTRACT_ALREADY_EXISTS"></a>

The staking contract already exists and cannot be re&#45;created.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_ESTAKING_CONTRACT_ALREADY_EXISTS">ESTAKING_CONTRACT_ALREADY_EXISTS</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_staking_contract_MAXIMUM_PENDING_DISTRIBUTIONS"></a>

Maximum number of distributions a stake pool can support.


<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_MAXIMUM_PENDING_DISTRIBUTIONS">MAXIMUM_PENDING_DISTRIBUTIONS</a>: u64 &#61; 20;<br /></code></pre>



<a id="0x1_staking_contract_SALT"></a>



<pre><code><b>const</b> <a href="staking_contract.md#0x1_staking_contract_SALT">SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 115, 116, 97, 107, 105, 110, 103, 95, 99, 111, 110, 116, 114, 97, 99, 116];<br /></code></pre>



<a id="0x1_staking_contract_stake_pool_address"></a>

## Function `stake_pool_address`

Return the address of the underlying stake pool for the staking contract between the provided staker and
operator.

This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_stake_pool_address">stake_pool_address</a>(staker: <b>address</b>, operator: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_stake_pool_address">stake_pool_address</a>(staker: <b>address</b>, operator: <b>address</b>): <b>address</b> <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker, operator);<br />    <b>let</b> staking_contracts &#61; &amp;<b>borrow_global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(staking_contracts, &amp;operator).pool_address<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_last_recorded_principal"></a>

## Function `last_recorded_principal`

Return the last recorded principal (the amount that 100% belongs to the staker with commission already paid for)
for staking contract between the provided staker and operator.

This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_last_recorded_principal">last_recorded_principal</a>(staker: <b>address</b>, operator: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_last_recorded_principal">last_recorded_principal</a>(staker: <b>address</b>, operator: <b>address</b>): u64 <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker, operator);<br />    <b>let</b> staking_contracts &#61; &amp;<b>borrow_global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(staking_contracts, &amp;operator).principal<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_commission_percentage"></a>

## Function `commission_percentage`

Return percentage of accumulated rewards that will be paid to the operator as commission for staking contract
between the provided staker and operator.

This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_commission_percentage">commission_percentage</a>(staker: <b>address</b>, operator: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_commission_percentage">commission_percentage</a>(staker: <b>address</b>, operator: <b>address</b>): u64 <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker, operator);<br />    <b>let</b> staking_contracts &#61; &amp;<b>borrow_global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(staking_contracts, &amp;operator).commission_percentage<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_staking_contract_amounts"></a>

## Function `staking_contract_amounts`

Return a tuple of three numbers:
1. The total active stake in the underlying stake pool
2. The total accumulated rewards that haven&apos;t had commission paid out
3. The commission amount owned from those accumulated rewards.

This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_staking_contract_amounts">staking_contract_amounts</a>(staker: <b>address</b>, operator: <b>address</b>): (u64, u64, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_staking_contract_amounts">staking_contract_amounts</a>(staker: <b>address</b>, operator: <b>address</b>): (u64, u64, u64) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker, operator);<br />    <b>let</b> staking_contracts &#61; &amp;<b>borrow_global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br />    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(staking_contracts, &amp;operator);<br />    <a href="staking_contract.md#0x1_staking_contract_get_staking_contract_amounts_internal">get_staking_contract_amounts_internal</a>(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_pending_distribution_counts"></a>

## Function `pending_distribution_counts`

Return the number of pending distributions (e.g. commission, withdrawals from stakers).

This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_pending_distribution_counts">pending_distribution_counts</a>(staker: <b>address</b>, operator: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_pending_distribution_counts">pending_distribution_counts</a>(staker: <b>address</b>, operator: <b>address</b>): u64 <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker, operator);<br />    <b>let</b> staking_contracts &#61; &amp;<b>borrow_global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders_count">pool_u64::shareholders_count</a>(&amp;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(staking_contracts, &amp;operator).distribution_pool)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_staking_contract_exists"></a>

## Function `staking_contract_exists`

Return true if the staking contract between the provided staker and operator exists.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_staking_contract_exists">staking_contract_exists</a>(staker: <b>address</b>, operator: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_staking_contract_exists">staking_contract_exists</a>(staker: <b>address</b>, operator: <b>address</b>): bool <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker)) &#123;<br />        <b>return</b> <b>false</b><br />    &#125;;<br /><br />    <b>let</b> store &#61; <b>borrow_global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker);<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;store.staking_contracts, &amp;operator)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_beneficiary_for_operator"></a>

## Function `beneficiary_for_operator`

Return the beneficiary address of the operator.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_beneficiary_for_operator">beneficiary_for_operator</a>(operator: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_beneficiary_for_operator">beneficiary_for_operator</a>(operator: <b>address</b>): <b>address</b> <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator)) &#123;<br />        <b>return</b> <b>borrow_global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator).beneficiary_for_operator<br />    &#125; <b>else</b> &#123;<br />        operator<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_get_expected_stake_pool_address"></a>

## Function `get_expected_stake_pool_address`

Return the address of the stake pool to be created with the provided staker, operator and seed.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_get_expected_stake_pool_address">get_expected_stake_pool_address</a>(staker: <b>address</b>, operator: <b>address</b>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_get_expected_stake_pool_address">get_expected_stake_pool_address</a>(<br />    staker: <b>address</b>,<br />    operator: <b>address</b>,<br />    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />): <b>address</b> &#123;<br />    <b>let</b> seed &#61; <a href="staking_contract.md#0x1_staking_contract_create_resource_account_seed">create_resource_account_seed</a>(staker, operator, contract_creation_seed);<br />    <a href="account.md#0x1_account_create_resource_address">account::create_resource_address</a>(&amp;staker, seed)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_create_staking_contract"></a>

## Function `create_staking_contract`

Staker can call this function to create a simple staking contract with a specified operator.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_staking_contract">create_staking_contract</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, voter: <b>address</b>, amount: u64, commission_percentage: u64, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_staking_contract">create_staking_contract</a>(<br />    staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    operator: <b>address</b>,<br />    voter: <b>address</b>,<br />    amount: u64,<br />    commission_percentage: u64,<br />    // Optional seed used when creating the staking contract <a href="account.md#0x1_account">account</a>.<br />    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <b>let</b> staked_coins &#61; <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;AptosCoin&gt;(staker, amount);<br />    <a href="staking_contract.md#0x1_staking_contract_create_staking_contract_with_coins">create_staking_contract_with_coins</a>(<br />        staker, operator, voter, staked_coins, commission_percentage, contract_creation_seed);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_create_staking_contract_with_coins"></a>

## Function `create_staking_contract_with_coins`

Staker can call this function to create a simple staking contract with a specified operator.


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_staking_contract_with_coins">create_staking_contract_with_coins</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, voter: <b>address</b>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;, commission_percentage: u64, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_staking_contract_with_coins">create_staking_contract_with_coins</a>(<br />    staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    operator: <b>address</b>,<br />    voter: <b>address</b>,<br />    coins: Coin&lt;AptosCoin&gt;,<br />    commission_percentage: u64,<br />    // Optional seed used when creating the staking contract <a href="account.md#0x1_account">account</a>.<br />    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />): <b>address</b> <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <b>assert</b>!(<br />        commission_percentage &gt;&#61; 0 &amp;&amp; <a href="staking_contract.md#0x1_staking_contract_commission_percentage">commission_percentage</a> &lt;&#61; 100,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_contract.md#0x1_staking_contract_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>),<br />    );<br />    // The amount should be at least the min_stake_required, so the <a href="stake.md#0x1_stake">stake</a> pool will be eligible <b>to</b> join the<br />    // validator set.<br />    <b>let</b> (min_stake_required, _) &#61; <a href="staking_config.md#0x1_staking_config_get_required_stake">staking_config::get_required_stake</a>(&amp;<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());<br />    <b>let</b> principal &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;coins);<br />    <b>assert</b>!(principal &gt;&#61; min_stake_required, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_contract.md#0x1_staking_contract_EINSUFFICIENT_STAKE_AMOUNT">EINSUFFICIENT_STAKE_AMOUNT</a>));<br /><br />    // Initialize <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> resource <b>if</b> this is the first time the staker <b>has</b> delegated <b>to</b> anyone.<br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address)) &#123;<br />        <b>move_to</b>(staker, <a href="staking_contract.md#0x1_staking_contract_new_staking_contracts_holder">new_staking_contracts_holder</a>(staker));<br />    &#125;;<br /><br />    // Cannot create the staking contract <b>if</b> it already <b>exists</b>.<br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br />    <b>let</b> staking_contracts &#61; &amp;<b>mut</b> store.staking_contracts;<br />    <b>assert</b>!(<br />        !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(staking_contracts, &amp;operator),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="staking_contract.md#0x1_staking_contract_ESTAKING_CONTRACT_ALREADY_EXISTS">ESTAKING_CONTRACT_ALREADY_EXISTS</a>)<br />    );<br /><br />    // Initialize the <a href="stake.md#0x1_stake">stake</a> pool in a new resource <a href="account.md#0x1_account">account</a>. This allows the same staker <b>to</b> contract <b>with</b> multiple<br />    // different operators.<br />    <b>let</b> (stake_pool_signer, stake_pool_signer_cap, owner_cap) &#61;<br />        <a href="staking_contract.md#0x1_staking_contract_create_stake_pool">create_stake_pool</a>(staker, operator, voter, contract_creation_seed);<br /><br />    // Add the <a href="stake.md#0x1_stake">stake</a> <b>to</b> the <a href="stake.md#0x1_stake">stake</a> pool.<br />    <a href="stake.md#0x1_stake_add_stake_with_cap">stake::add_stake_with_cap</a>(&amp;owner_cap, coins);<br /><br />    // Create the contract record.<br />    <b>let</b> pool_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;stake_pool_signer);<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(staking_contracts, operator, <a href="staking_contract.md#0x1_staking_contract_StakingContract">StakingContract</a> &#123;<br />        principal,<br />        pool_address,<br />        owner_cap,<br />        commission_percentage,<br />        // Make sure we don&apos;t have too many pending recipients in the distribution pool.<br />        // Otherwise, a griefing attack is possible <b>where</b> the staker can keep switching operators and create too<br />        // many pending distributions. This can lead <b>to</b> out&#45;of&#45;gas failure whenever <a href="staking_contract.md#0x1_staking_contract_distribute">distribute</a>() is called.<br />        distribution_pool: <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create">pool_u64::create</a>(<a href="staking_contract.md#0x1_staking_contract_MAXIMUM_PENDING_DISTRIBUTIONS">MAXIMUM_PENDING_DISTRIBUTIONS</a>),<br />        signer_cap: stake_pool_signer_cap,<br />    &#125;);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<a href="staking_contract.md#0x1_staking_contract_CreateStakingContract">CreateStakingContract</a> &#123; operator, voter, pool_address, principal, commission_percentage &#125;);<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> store.create_staking_contract_events,<br />        <a href="staking_contract.md#0x1_staking_contract_CreateStakingContractEvent">CreateStakingContractEvent</a> &#123; operator, voter, pool_address, principal, commission_percentage &#125;,<br />    );<br />    pool_address<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_add_stake"></a>

## Function `add_stake`

Add more stake to an existing staking contract.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_add_stake">add_stake</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_add_stake">add_stake</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, amount: u64) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker_address, operator);<br /><br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br />    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> store.staking_contracts, &amp;operator);<br /><br />    // Add the <a href="stake.md#0x1_stake">stake</a> <b>to</b> the <a href="stake.md#0x1_stake">stake</a> pool.<br />    <b>let</b> staked_coins &#61; <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;AptosCoin&gt;(staker, amount);<br />    <a href="stake.md#0x1_stake_add_stake_with_cap">stake::add_stake_with_cap</a>(&amp;<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap, staked_coins);<br /><br />    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal &#43; amount;<br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<a href="staking_contract.md#0x1_staking_contract_AddStake">AddStake</a> &#123; operator, pool_address, amount &#125;);<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> store.add_stake_events,<br />        <a href="staking_contract.md#0x1_staking_contract_AddStakeEvent">AddStakeEvent</a> &#123; operator, pool_address, amount &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_update_voter"></a>

## Function `update_voter`

Convenient function to allow the staker to update the voter address in a staking contract they made.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_voter">update_voter</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_voter">update_voter</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker_address, operator);<br /><br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br />    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> store.staking_contracts, &amp;operator);<br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br />    <b>let</b> old_voter &#61; <a href="stake.md#0x1_stake_get_delegated_voter">stake::get_delegated_voter</a>(pool_address);<br />    <a href="stake.md#0x1_stake_set_delegated_voter_with_cap">stake::set_delegated_voter_with_cap</a>(&amp;<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap, new_voter);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<a href="staking_contract.md#0x1_staking_contract_UpdateVoter">UpdateVoter</a> &#123; operator, pool_address, old_voter, new_voter &#125;);<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> store.update_voter_events,<br />        <a href="staking_contract.md#0x1_staking_contract_UpdateVoterEvent">UpdateVoterEvent</a> &#123; operator, pool_address, old_voter, new_voter &#125;,<br />    );<br /><br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_reset_lockup"></a>

## Function `reset_lockup`

Convenient function to allow the staker to reset their stake pool&apos;s lockup period to start now.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_reset_lockup">reset_lockup</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_reset_lockup">reset_lockup</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker_address, operator);<br /><br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br />    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> store.staking_contracts, &amp;operator);<br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br />    <a href="stake.md#0x1_stake_increase_lockup_with_cap">stake::increase_lockup_with_cap</a>(&amp;<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<a href="staking_contract.md#0x1_staking_contract_ResetLockup">ResetLockup</a> &#123; operator, pool_address &#125;);<br />    &#125;;<br />    emit_event(&amp;<b>mut</b> store.reset_lockup_events, <a href="staking_contract.md#0x1_staking_contract_ResetLockupEvent">ResetLockupEvent</a> &#123; operator, pool_address &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_update_commision"></a>

## Function `update_commision`

Convenience function to allow a staker to update the commission percentage paid to the operator.
TODO: fix the typo in function name. commision &#45;&gt; commission


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_commision">update_commision</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_commission_percentage: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_commision">update_commision</a>(<br />    staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    operator: <b>address</b>,<br />    new_commission_percentage: u64<br />) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a>, <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a>, <a href="staking_contract.md#0x1_staking_contract_StakingGroupUpdateCommissionEvent">StakingGroupUpdateCommissionEvent</a> &#123;<br />    <b>assert</b>!(<br />        new_commission_percentage &gt;&#61; 0 &amp;&amp; new_commission_percentage &lt;&#61; 100,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_contract.md#0x1_staking_contract_EINVALID_COMMISSION_PERCENTAGE">EINVALID_COMMISSION_PERCENTAGE</a>),<br />    );<br /><br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="staking_contract.md#0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_STAKER">ENO_STAKING_CONTRACT_FOUND_FOR_STAKER</a>));<br /><br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br />    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> store.staking_contracts, &amp;operator);<br />    <a href="staking_contract.md#0x1_staking_contract_distribute_internal">distribute_internal</a>(staker_address, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, &amp;<b>mut</b> store.distribute_events);<br />    <a href="staking_contract.md#0x1_staking_contract_request_commission_internal">request_commission_internal</a>(<br />        operator,<br />        <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>,<br />        &amp;<b>mut</b> store.add_distribution_events,<br />        &amp;<b>mut</b> store.request_commission_events,<br />    );<br />    <b>let</b> old_commission_percentage &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage;<br />    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &#61; new_commission_percentage;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_StakingGroupUpdateCommissionEvent">StakingGroupUpdateCommissionEvent</a>&gt;(staker_address)) &#123;<br />        <b>move_to</b>(<br />            staker,<br />            <a href="staking_contract.md#0x1_staking_contract_StakingGroupUpdateCommissionEvent">StakingGroupUpdateCommissionEvent</a> &#123;<br />                update_commission_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_UpdateCommissionEvent">UpdateCommissionEvent</a>&gt;(<br />                    staker<br />                )<br />            &#125;<br />        )<br />    &#125;;<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="staking_contract.md#0x1_staking_contract_UpdateCommission">UpdateCommission</a> &#123; staker: staker_address, operator, old_commission_percentage, new_commission_percentage &#125;<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_StakingGroupUpdateCommissionEvent">StakingGroupUpdateCommissionEvent</a>&gt;(staker_address).update_commission_events,<br />        <a href="staking_contract.md#0x1_staking_contract_UpdateCommissionEvent">UpdateCommissionEvent</a> &#123; staker: staker_address, operator, old_commission_percentage, new_commission_percentage &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_request_commission"></a>

## Function `request_commission`

Unlock commission amount from the stake pool. Operator needs to wait for the amount to become withdrawable
at the end of the stake pool&apos;s lockup period before they can actually can withdraw_commission.

Only staker, operator or beneficiary can call this.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_request_commission">request_commission</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, staker: <b>address</b>, operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_request_commission">request_commission</a>(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    staker: <b>address</b>,<br />    operator: <b>address</b><br />) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a>, <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<br />        account_addr &#61;&#61; staker &#124;&#124; account_addr &#61;&#61; operator &#124;&#124; account_addr &#61;&#61; <a href="staking_contract.md#0x1_staking_contract_beneficiary_for_operator">beneficiary_for_operator</a>(operator),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="staking_contract.md#0x1_staking_contract_ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY">ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY</a>)<br />    );<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker, operator);<br /><br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker);<br />    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> store.staking_contracts, &amp;operator);<br />    // Short&#45;circuit <b>if</b> zero commission.<br />    <b>if</b> (<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &#61;&#61; 0) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    // Force distribution of <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> already inactive <a href="stake.md#0x1_stake">stake</a>.<br />    <a href="staking_contract.md#0x1_staking_contract_distribute_internal">distribute_internal</a>(staker, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, &amp;<b>mut</b> store.distribute_events);<br /><br />    <a href="staking_contract.md#0x1_staking_contract_request_commission_internal">request_commission_internal</a>(<br />        operator,<br />        <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>,<br />        &amp;<b>mut</b> store.add_distribution_events,<br />        &amp;<b>mut</b> store.request_commission_events,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_request_commission_internal"></a>

## Function `request_commission_internal`



<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_request_commission_internal">request_commission_internal</a>(operator: <b>address</b>, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>, add_distribution_events: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">staking_contract::AddDistributionEvent</a>&gt;, request_commission_events: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_RequestCommissionEvent">staking_contract::RequestCommissionEvent</a>&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_request_commission_internal">request_commission_internal</a>(<br />    operator: <b>address</b>,<br />    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">StakingContract</a>,<br />    add_distribution_events: &amp;<b>mut</b> EventHandle&lt;<a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">AddDistributionEvent</a>&gt;,<br />    request_commission_events: &amp;<b>mut</b> EventHandle&lt;<a href="staking_contract.md#0x1_staking_contract_RequestCommissionEvent">RequestCommissionEvent</a>&gt;,<br />): u64 &#123;<br />    // Unlock just the commission portion from the <a href="stake.md#0x1_stake">stake</a> pool.<br />    <b>let</b> (total_active_stake, accumulated_rewards, commission_amount) &#61;<br />        <a href="staking_contract.md#0x1_staking_contract_get_staking_contract_amounts_internal">get_staking_contract_amounts_internal</a>(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>);<br />    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal &#61; total_active_stake &#45; commission_amount;<br /><br />    // Short&#45;circuit <b>if</b> there&apos;s no commission <b>to</b> pay.<br />    <b>if</b> (commission_amount &#61;&#61; 0) &#123;<br />        <b>return</b> 0<br />    &#125;;<br /><br />    // Add a distribution for the operator.<br />    <a href="staking_contract.md#0x1_staking_contract_add_distribution">add_distribution</a>(operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, operator, commission_amount, add_distribution_events);<br /><br />    // Request <b>to</b> unlock the commission from the <a href="stake.md#0x1_stake">stake</a> pool.<br />    // This won&apos;t become fully unlocked until the <a href="stake.md#0x1_stake">stake</a> pool&apos;s lockup expires.<br />    <a href="stake.md#0x1_stake_unlock_with_cap">stake::unlock_with_cap</a>(commission_amount, &amp;<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap);<br /><br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<a href="staking_contract.md#0x1_staking_contract_RequestCommission">RequestCommission</a> &#123; operator, pool_address, accumulated_rewards, commission_amount &#125;);<br />    &#125;;<br />    emit_event(<br />        request_commission_events,<br />        <a href="staking_contract.md#0x1_staking_contract_RequestCommissionEvent">RequestCommissionEvent</a> &#123; operator, pool_address, accumulated_rewards, commission_amount &#125;,<br />    );<br /><br />    commission_amount<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_unlock_stake"></a>

## Function `unlock_stake`

Staker can call this to request withdrawal of part or all of their staking_contract.
This also triggers paying commission to the operator for accounting simplicity.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_unlock_stake">unlock_stake</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_unlock_stake">unlock_stake</a>(<br />    staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    operator: <b>address</b>,<br />    amount: u64<br />) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a>, <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    // Short&#45;circuit <b>if</b> amount is 0.<br />    <b>if</b> (amount &#61;&#61; 0) <b>return</b>;<br /><br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker_address, operator);<br /><br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br />    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> store.staking_contracts, &amp;operator);<br /><br />    // Force distribution of <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> already inactive <a href="stake.md#0x1_stake">stake</a>.<br />    <a href="staking_contract.md#0x1_staking_contract_distribute_internal">distribute_internal</a>(staker_address, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, &amp;<b>mut</b> store.distribute_events);<br /><br />    // For simplicity, we request commission <b>to</b> be paid out first. This avoids having <b>to</b> ensure <b>to</b> staker doesn&apos;t<br />    // withdraw into the commission portion.<br />    <b>let</b> commission_paid &#61; <a href="staking_contract.md#0x1_staking_contract_request_commission_internal">request_commission_internal</a>(<br />        operator,<br />        <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>,<br />        &amp;<b>mut</b> store.add_distribution_events,<br />        &amp;<b>mut</b> store.request_commission_events,<br />    );<br /><br />    // If there&apos;s less active <a href="stake.md#0x1_stake">stake</a> remaining than the amount requested (potentially due <b>to</b> commission),<br />    // only withdraw up <b>to</b> the active amount.<br />    <b>let</b> (active, _, _, _) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address);<br />    <b>if</b> (active &lt; amount) &#123;<br />        amount &#61; active;<br />    &#125;;<br />    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal &#45; amount;<br /><br />    // Record a distribution for the staker.<br />    <a href="staking_contract.md#0x1_staking_contract_add_distribution">add_distribution</a>(operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, staker_address, amount, &amp;<b>mut</b> store.add_distribution_events);<br /><br />    // Request <b>to</b> unlock the distribution amount from the <a href="stake.md#0x1_stake">stake</a> pool.<br />    // This won&apos;t become fully unlocked until the <a href="stake.md#0x1_stake">stake</a> pool&apos;s lockup expires.<br />    <a href="stake.md#0x1_stake_unlock_with_cap">stake::unlock_with_cap</a>(amount, &amp;<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap);<br /><br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<a href="staking_contract.md#0x1_staking_contract_UnlockStake">UnlockStake</a> &#123; pool_address, operator, amount, commission_paid &#125;);<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> store.unlock_stake_events,<br />        <a href="staking_contract.md#0x1_staking_contract_UnlockStakeEvent">UnlockStakeEvent</a> &#123; pool_address, operator, amount, commission_paid &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_unlock_rewards"></a>

## Function `unlock_rewards`

Unlock all accumulated rewards since the last recorded principals.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_unlock_rewards">unlock_rewards</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_unlock_rewards">unlock_rewards</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a>, <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker_address, operator);<br /><br />    // Calculate how much rewards belongs <b>to</b> the staker after commission is paid.<br />    <b>let</b> (_, accumulated_rewards, unpaid_commission) &#61; <a href="staking_contract.md#0x1_staking_contract_staking_contract_amounts">staking_contract_amounts</a>(staker_address, operator);<br />    <b>let</b> staker_rewards &#61; accumulated_rewards &#45; unpaid_commission;<br />    <a href="staking_contract.md#0x1_staking_contract_unlock_stake">unlock_stake</a>(staker, operator, staker_rewards);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_switch_operator_with_same_commission"></a>

## Function `switch_operator_with_same_commission`

Allows staker to switch operator without going through the lenghthy process to unstake, without resetting commission.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_switch_operator_with_same_commission">switch_operator_with_same_commission</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_switch_operator_with_same_commission">switch_operator_with_same_commission</a>(<br />    staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    old_operator: <b>address</b>,<br />    new_operator: <b>address</b>,<br />) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a>, <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker_address, old_operator);<br /><br />    <b>let</b> commission_percentage &#61; <a href="staking_contract.md#0x1_staking_contract_commission_percentage">commission_percentage</a>(staker_address, old_operator);<br />    <a href="staking_contract.md#0x1_staking_contract_switch_operator">switch_operator</a>(staker, old_operator, new_operator, commission_percentage);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_switch_operator"></a>

## Function `switch_operator`

Allows staker to switch operator without going through the lenghthy process to unstake.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_switch_operator">switch_operator</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>, new_commission_percentage: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_switch_operator">switch_operator</a>(<br />    staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    old_operator: <b>address</b>,<br />    new_operator: <b>address</b>,<br />    new_commission_percentage: u64,<br />) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a>, <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker_address, old_operator);<br /><br />    // Merging two existing staking contracts is too complex <b>as</b> we&apos;d need <b>to</b> merge two separate <a href="stake.md#0x1_stake">stake</a> pools.<br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br />    <b>let</b> staking_contracts &#61; &amp;<b>mut</b> store.staking_contracts;<br />    <b>assert</b>!(<br />        !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(staking_contracts, &amp;new_operator),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="staking_contract.md#0x1_staking_contract_ECANT_MERGE_STAKING_CONTRACTS">ECANT_MERGE_STAKING_CONTRACTS</a>),<br />    );<br /><br />    <b>let</b> (_, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>) &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(staking_contracts, &amp;old_operator);<br />    // Force distribution of <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> already inactive <a href="stake.md#0x1_stake">stake</a>.<br />    <a href="staking_contract.md#0x1_staking_contract_distribute_internal">distribute_internal</a>(staker_address, old_operator, &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, &amp;<b>mut</b> store.distribute_events);<br /><br />    // For simplicity, we request commission <b>to</b> be paid out first. This avoids having <b>to</b> ensure <b>to</b> staker doesn&apos;t<br />    // withdraw into the commission portion.<br />    <a href="staking_contract.md#0x1_staking_contract_request_commission_internal">request_commission_internal</a>(<br />        old_operator,<br />        &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>,<br />        &amp;<b>mut</b> store.add_distribution_events,<br />        &amp;<b>mut</b> store.request_commission_events,<br />    );<br /><br />    // Update the staking contract&apos;s commission rate and <a href="stake.md#0x1_stake">stake</a> pool&apos;s operator.<br />    <a href="stake.md#0x1_stake_set_operator_with_cap">stake::set_operator_with_cap</a>(&amp;<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap, new_operator);<br />    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &#61; new_commission_percentage;<br /><br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(staking_contracts, new_operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<a href="staking_contract.md#0x1_staking_contract_SwitchOperator">SwitchOperator</a> &#123; pool_address, old_operator, new_operator &#125;);<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> store.switch_operator_events,<br />        <a href="staking_contract.md#0x1_staking_contract_SwitchOperatorEvent">SwitchOperatorEvent</a> &#123; pool_address, old_operator, new_operator &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Allows an operator to change its beneficiary. Any existing unpaid commission rewards will be paid to the new
beneficiary. To ensures payment to the current beneficiary, one should first call <code>distribute</code> before switching
the beneficiary. An operator can set one beneficiary for staking contract pools, not a separate one for each pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_beneficiary: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(<br />    operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    new_beneficiary: <b>address</b><br />) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operator_beneficiary_change_enabled">features::operator_beneficiary_change_enabled</a>(), std::error::invalid_state(<br />        <a href="staking_contract.md#0x1_staking_contract_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED">EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED</a><br />    ));<br />    // The beneficiay <b>address</b> of an operator is stored under the operator&apos;s <b>address</b>.<br />    // So, the operator does not need <b>to</b> be validated <b>with</b> respect <b>to</b> a staking pool.<br />    <b>let</b> operator_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(operator);<br />    <b>let</b> old_beneficiary &#61; <a href="staking_contract.md#0x1_staking_contract_beneficiary_for_operator">beneficiary_for_operator</a>(operator_addr);<br />    <b>if</b> (<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator_addr)) &#123;<br />        <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a>&gt;(operator_addr).beneficiary_for_operator &#61; new_beneficiary;<br />    &#125; <b>else</b> &#123;<br />        <b>move_to</b>(operator, <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123; beneficiary_for_operator: new_beneficiary &#125;);<br />    &#125;;<br /><br />    emit(<a href="staking_contract.md#0x1_staking_contract_SetBeneficiaryForOperator">SetBeneficiaryForOperator</a> &#123;<br />        operator: operator_addr,<br />        old_beneficiary,<br />        new_beneficiary,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_distribute"></a>

## Function `distribute`

Allow anyone to distribute already unlocked funds. This does not affect reward compounding and therefore does
not need to be restricted to just the staker or operator.


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_distribute">distribute</a>(staker: <b>address</b>, operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_distribute">distribute</a>(staker: <b>address</b>, operator: <b>address</b>) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a>, <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker, operator);<br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker);<br />    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> store.staking_contracts, &amp;operator);<br />    <a href="staking_contract.md#0x1_staking_contract_distribute_internal">distribute_internal</a>(staker, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, &amp;<b>mut</b> store.distribute_events);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_distribute_internal"></a>

## Function `distribute_internal`

Distribute all unlocked (inactive) funds according to distribution shares.


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_distribute_internal">distribute_internal</a>(staker: <b>address</b>, operator: <b>address</b>, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>, distribute_events: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_DistributeEvent">staking_contract::DistributeEvent</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_distribute_internal">distribute_internal</a>(<br />    staker: <b>address</b>,<br />    operator: <b>address</b>,<br />    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">StakingContract</a>,<br />    distribute_events: &amp;<b>mut</b> EventHandle&lt;<a href="staking_contract.md#0x1_staking_contract_DistributeEvent">DistributeEvent</a>&gt;,<br />) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_BeneficiaryForOperator">BeneficiaryForOperator</a> &#123;<br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br />    <b>let</b> (_, inactive, _, pending_inactive) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);<br />    <b>let</b> total_potential_withdrawable &#61; inactive &#43; pending_inactive;<br />    <b>let</b> coins &#61; <a href="stake.md#0x1_stake_withdraw_with_cap">stake::withdraw_with_cap</a>(&amp;<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap, total_potential_withdrawable);<br />    <b>let</b> distribution_amount &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;coins);<br />    <b>if</b> (distribution_amount &#61;&#61; 0) &#123;<br />        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);<br />        <b>return</b><br />    &#125;;<br /><br />    <b>let</b> distribution_pool &#61; &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.distribution_pool;<br />    <a href="staking_contract.md#0x1_staking_contract_update_distribution_pool">update_distribution_pool</a>(<br />        distribution_pool, distribution_amount, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage);<br /><br />    // Buy all recipients out of the distribution pool.<br />    <b>while</b> (<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders_count">pool_u64::shareholders_count</a>(distribution_pool) &gt; 0) &#123;<br />        <b>let</b> recipients &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders">pool_u64::shareholders</a>(distribution_pool);<br />        <b>let</b> recipient &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;<b>mut</b> recipients, 0);<br />        <b>let</b> current_shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(distribution_pool, recipient);<br />        <b>let</b> amount_to_distribute &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_redeem_shares">pool_u64::redeem_shares</a>(distribution_pool, recipient, current_shares);<br />        // If the recipient is the operator, send the commission <b>to</b> the beneficiary instead.<br />        <b>if</b> (recipient &#61;&#61; operator) &#123;<br />            recipient &#61; <a href="staking_contract.md#0x1_staking_contract_beneficiary_for_operator">beneficiary_for_operator</a>(operator);<br />        &#125;;<br />        <a href="aptos_account.md#0x1_aptos_account_deposit_coins">aptos_account::deposit_coins</a>(recipient, <a href="coin.md#0x1_coin_extract">coin::extract</a>(&amp;<b>mut</b> coins, amount_to_distribute));<br /><br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            emit(<a href="staking_contract.md#0x1_staking_contract_Distribute">Distribute</a> &#123; operator, pool_address, recipient, amount: amount_to_distribute &#125;);<br />        &#125;;<br />        emit_event(<br />            distribute_events,<br />            <a href="staking_contract.md#0x1_staking_contract_DistributeEvent">DistributeEvent</a> &#123; operator, pool_address, recipient, amount: amount_to_distribute &#125;<br />        );<br />    &#125;;<br /><br />    // In case there&apos;s <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> dust left, send them all <b>to</b> the staker.<br />    <b>if</b> (<a href="coin.md#0x1_coin_value">coin::value</a>(&amp;coins) &gt; 0) &#123;<br />        <a href="aptos_account.md#0x1_aptos_account_deposit_coins">aptos_account::deposit_coins</a>(staker, coins);<br />        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_update_total_coins">pool_u64::update_total_coins</a>(distribution_pool, 0);<br />    &#125; <b>else</b> &#123;<br />        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_assert_staking_contract_exists"></a>

## Function `assert_staking_contract_exists`

Assert that a staking_contract exists for the staker/operator pair.


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker: <b>address</b>, operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker: <b>address</b>, operator: <b>address</b>) <b>acquires</b> <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="staking_contract.md#0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_STAKER">ENO_STAKING_CONTRACT_FOUND_FOR_STAKER</a>));<br />    <b>let</b> staking_contracts &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(staking_contracts, &amp;operator),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="staking_contract.md#0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR">ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR</a>),<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_add_distribution"></a>

## Function `add_distribution`

Add a new distribution for <code>recipient</code> and <code>amount</code> to the staking contract&apos;s distributions list.


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_add_distribution">add_distribution</a>(operator: <b>address</b>, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>, recipient: <b>address</b>, coins_amount: u64, add_distribution_events: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">staking_contract::AddDistributionEvent</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_add_distribution">add_distribution</a>(<br />    operator: <b>address</b>,<br />    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">StakingContract</a>,<br />    recipient: <b>address</b>,<br />    coins_amount: u64,<br />    add_distribution_events: &amp;<b>mut</b> EventHandle&lt;<a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">AddDistributionEvent</a>&gt;<br />) &#123;<br />    <b>let</b> distribution_pool &#61; &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.distribution_pool;<br />    <b>let</b> (_, _, _, total_distribution_amount) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address);<br />    <a href="staking_contract.md#0x1_staking_contract_update_distribution_pool">update_distribution_pool</a>(<br />        distribution_pool, total_distribution_amount, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage);<br /><br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(distribution_pool, recipient, coins_amount);<br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<a href="staking_contract.md#0x1_staking_contract_AddDistribution">AddDistribution</a> &#123; operator, pool_address, amount: coins_amount &#125;);<br />    &#125;;<br />    emit_event(<br />        add_distribution_events,<br />        <a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">AddDistributionEvent</a> &#123; operator, pool_address, amount: coins_amount &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_get_staking_contract_amounts_internal"></a>

## Function `get_staking_contract_amounts_internal`

Calculate accumulated rewards and commissions since last update.


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_get_staking_contract_amounts_internal">get_staking_contract_amounts_internal</a>(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>): (u64, u64, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_get_staking_contract_amounts_internal">get_staking_contract_amounts_internal</a>(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<a href="staking_contract.md#0x1_staking_contract_StakingContract">StakingContract</a>): (u64, u64, u64) &#123;<br />    // Pending_inactive is not included in the calculation because pending_inactive can only come from:<br />    // 1. Outgoing commissions. This means commission <b>has</b> already been extracted.<br />    // 2. Stake withdrawals from stakers. This also means commission <b>has</b> already been extracted <b>as</b><br />    // request_commission_internal is called in unlock_stake<br />    <b>let</b> (active, _, pending_active, _) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address);<br />    <b>let</b> total_active_stake &#61; active &#43; pending_active;<br />    <b>let</b> accumulated_rewards &#61; total_active_stake &#45; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br />    <b>let</b> commission_amount &#61; accumulated_rewards &#42; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;<br /><br />    (total_active_stake, accumulated_rewards, commission_amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_create_stake_pool"></a>

## Function `create_stake_pool`



<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_stake_pool">create_stake_pool</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, voter: <b>address</b>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_stake_pool">create_stake_pool</a>(<br />    staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    operator: <b>address</b>,<br />    voter: <b>address</b>,<br />    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, SignerCapability, OwnerCapability) &#123;<br />    // Generate a seed that will be used <b>to</b> create the resource <a href="account.md#0x1_account">account</a> that hosts the staking contract.<br />    <b>let</b> seed &#61; <a href="staking_contract.md#0x1_staking_contract_create_resource_account_seed">create_resource_account_seed</a>(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker), operator, contract_creation_seed);<br /><br />    <b>let</b> (stake_pool_signer, stake_pool_signer_cap) &#61; <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(staker, seed);<br />    <a href="stake.md#0x1_stake_initialize_stake_owner">stake::initialize_stake_owner</a>(&amp;stake_pool_signer, 0, operator, voter);<br /><br />    // Extract owner_cap from the StakePool, so we have control over it in the staking_contracts flow.<br />    // This is stored <b>as</b> part of the <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>. Thus, the staker would not have direct control over it without<br />    // going through well&#45;defined functions in this <b>module</b>.<br />    <b>let</b> owner_cap &#61; <a href="stake.md#0x1_stake_extract_owner_cap">stake::extract_owner_cap</a>(&amp;stake_pool_signer);<br /><br />    (stake_pool_signer, stake_pool_signer_cap, owner_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_update_distribution_pool"></a>

## Function `update_distribution_pool`



<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_distribution_pool">update_distribution_pool</a>(distribution_pool: &amp;<b>mut</b> <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, updated_total_coins: u64, operator: <b>address</b>, commission_percentage: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_distribution_pool">update_distribution_pool</a>(<br />    distribution_pool: &amp;<b>mut</b> Pool,<br />    updated_total_coins: u64,<br />    operator: <b>address</b>,<br />    commission_percentage: u64,<br />) &#123;<br />    // Short&#45;circuit and do nothing <b>if</b> the pool&apos;s total value <b>has</b> not changed.<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_total_coins">pool_u64::total_coins</a>(distribution_pool) &#61;&#61; updated_total_coins) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    // Charge all stakeholders (<b>except</b> for the operator themselves) commission on <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> rewards earnt relatively <b>to</b> the<br />    // previous value of the distribution pool.<br />    <b>let</b> shareholders &#61; &amp;<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders">pool_u64::shareholders</a>(distribution_pool);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(shareholders, &#124;shareholder&#124; &#123;<br />        <b>let</b> shareholder: <b>address</b> &#61; &#42;shareholder;<br />        <b>if</b> (shareholder !&#61; operator) &#123;<br />            <b>let</b> shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(distribution_pool, shareholder);<br />            <b>let</b> previous_worth &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_balance">pool_u64::balance</a>(distribution_pool, shareholder);<br />            <b>let</b> current_worth &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">pool_u64::shares_to_amount_with_total_coins</a>(<br />                distribution_pool, shares, updated_total_coins);<br />            <b>let</b> unpaid_commission &#61; (current_worth &#45; previous_worth) &#42; commission_percentage / 100;<br />            // Transfer shares from current shareholder <b>to</b> the operator <b>as</b> payment.<br />            // The value of the shares should <b>use</b> the updated pool&apos;s total value.<br />            <b>let</b> shares_to_transfer &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_amount_to_shares_with_total_coins">pool_u64::amount_to_shares_with_total_coins</a>(<br />                distribution_pool, unpaid_commission, updated_total_coins);<br />            <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_transfer_shares">pool_u64::transfer_shares</a>(distribution_pool, shareholder, operator, shares_to_transfer);<br />        &#125;;<br />    &#125;);<br /><br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_update_total_coins">pool_u64::update_total_coins</a>(distribution_pool, updated_total_coins);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_create_resource_account_seed"></a>

## Function `create_resource_account_seed`

Create the seed to derive the resource account address.


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_resource_account_seed">create_resource_account_seed</a>(staker: <b>address</b>, operator: <b>address</b>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_resource_account_seed">create_resource_account_seed</a>(<br />    staker: <b>address</b>,<br />    operator: <b>address</b>,<br />    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <b>let</b> seed &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;staker);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;operator));<br />    // Include a salt <b>to</b> avoid conflicts <b>with</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> other modules out there that might also generate<br />    // deterministic resource accounts for the same staker &#43; operator addresses.<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, <a href="staking_contract.md#0x1_staking_contract_SALT">SALT</a>);<br />    // Add an extra salt given by the staker in case an <a href="account.md#0x1_account">account</a> <b>with</b> the same <b>address</b> <b>has</b> already been created.<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, contract_creation_seed);<br />    seed<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_contract_new_staking_contracts_holder"></a>

## Function `new_staking_contracts_holder`

Create a new staking_contracts resource.


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_new_staking_contracts_holder">new_staking_contracts_holder</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_new_staking_contracts_holder">new_staking_contracts_holder</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />    <a href="staking_contract.md#0x1_staking_contract_Store">Store</a> &#123;<br />        staking_contracts: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, <a href="staking_contract.md#0x1_staking_contract_StakingContract">StakingContract</a>&gt;(),<br />        // Events.<br />        create_staking_contract_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_CreateStakingContractEvent">CreateStakingContractEvent</a>&gt;(staker),<br />        update_voter_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_UpdateVoterEvent">UpdateVoterEvent</a>&gt;(staker),<br />        reset_lockup_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_ResetLockupEvent">ResetLockupEvent</a>&gt;(staker),<br />        add_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_AddStakeEvent">AddStakeEvent</a>&gt;(staker),<br />        request_commission_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_RequestCommissionEvent">RequestCommissionEvent</a>&gt;(staker),<br />        unlock_stake_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_UnlockStakeEvent">UnlockStakeEvent</a>&gt;(staker),<br />        switch_operator_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_SwitchOperatorEvent">SwitchOperatorEvent</a>&gt;(staker),<br />        add_distribution_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">AddDistributionEvent</a>&gt;(staker),<br />        distribute_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_DistributeEvent">DistributeEvent</a>&gt;(staker),<br />    &#125;<br />&#125;<br /></code></pre>



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
<td>The Store structure for the staker exists after the staking contract is created.</td>
<td>Medium</td>
<td>The create_staking_contract_with_coins function ensures that the staker account has a Store structure assigned.</td>
<td>Formally verified via <a href="#high-level-req-1">CreateStakingContractWithCoinsAbortsifAndEnsures</a>.</td>
</tr>

<tr>
<td>2</td>
<td>A staking contract is created and stored in a mapping within the Store resource.</td>
<td>High</td>
<td>The create_staking_contract_with_coins function adds the newly created StakingContract to the staking_contracts map with the operator as a key of the Store resource, effectively storing the staking contract.</td>
<td>Formally verified via <a href="#high-level-req-2">CreateStakingContractWithCoinsAbortsifAndEnsures</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Adding stake to the stake pool increases the principal value of the pool, reflecting the additional stake amount.</td>
<td>High</td>
<td>The add_stake function transfers the specified amount of staked coins from the staker&apos;s account to the stake pool associated with the staking contract. It increases the principal value of the staking contract by the added stake amount.</td>
<td>Formally verified via <a href="#high-level-req-3">add_stake</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The staker may update the voter of a staking contract, enabling them to modify the assigned voter address and ensure it accurately reflects their desired choice.</td>
<td>High</td>
<td>The update_voter function ensures that the voter address in a staking contract may be updated by the staker, resulting in the modification of the delegated voter address in the associated stake pool to reflect the new address provided.</td>
<td>Formally verified via <a href="#high-level-req-4">update_voter</a>.</td>
</tr>

<tr>
<td>5</td>
<td>Only the owner of the stake pool has the permission to reset the lockup period of the pool.</td>
<td>Critical</td>
<td>The reset_lockup function ensures that only the staker who owns the stake pool has the authority to reset the lockup period of the pool.</td>
<td>Formally verified via <a href="#high-level-req-5">reset_lockup</a>.</td>
</tr>

<tr>
<td>6</td>
<td>Unlocked funds are correctly distributed to recipients based on their distribution shares, taking into account the associated commission percentage.</td>
<td>High</td>
<td>The distribution process, implemented in the distribute_internal function, accurately allocates unlocked funds to their intended recipients based on their distribution shares. It guarantees that each recipient receives the correct amount of funds, considering the commission percentage associated with the staking contract.</td>
<td>Audited that the correct amount of unlocked funds is distributed according to distribution shares.</td>
</tr>

<tr>
<td>7</td>
<td>The stake pool ensures that the commission is correctly requested and paid out from the old operator&apos;s stake pool before allowing the switch to the new operator.</td>
<td>High</td>
<td>The switch_operator function initiates the commission payout from the stake pool associated with the old operator, ensuring a smooth transition. Paying out the commission before the switch guarantees that the staker receives the appropriate commission amount and maintains the integrity of the staking process.</td>
<td>Audited that the commission is paid to the old operator.</td>
</tr>

<tr>
<td>8</td>
<td>Stakers can withdraw their funds from the staking contract, ensuring the unlocked amount becomes available for withdrawal after the lockup period.</td>
<td>High</td>
<td>The unlock_stake function ensures that the requested amount is properly unlocked from the stake pool, considering the lockup period and that the funds become available for withdrawal when the lockup expires.</td>
<td>Audited that funds are unlocked properly.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_stake_pool_address"></a>

### Function `stake_pool_address`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_stake_pool_address">stake_pool_address</a>(staker: <b>address</b>, operator: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a>;<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br /><b>ensures</b> result &#61;&#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator).pool_address;<br /></code></pre>



<a id="@Specification_1_last_recorded_principal"></a>

### Function `last_recorded_principal`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_last_recorded_principal">last_recorded_principal</a>(staker: <b>address</b>, operator: <b>address</b>): u64<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a>;<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br /><b>ensures</b> result &#61;&#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator).principal;<br /></code></pre>



<a id="@Specification_1_commission_percentage"></a>

### Function `commission_percentage`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_commission_percentage">commission_percentage</a>(staker: <b>address</b>, operator: <b>address</b>): u64<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a>;<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br /><b>ensures</b> result &#61;&#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator).commission_percentage;<br /></code></pre>



<a id="@Specification_1_staking_contract_amounts"></a>

### Function `staking_contract_amounts`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_staking_contract_amounts">staking_contract_amounts</a>(staker: <b>address</b>, operator: <b>address</b>): (u64, u64, u64)<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>requires</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &gt;&#61; 0 &amp;&amp; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.<a href="staking_contract.md#0x1_staking_contract_commission_percentage">commission_percentage</a> &lt;&#61; 100;<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a>;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_GetStakingContractAmountsAbortsIf">GetStakingContractAmountsAbortsIf</a> &#123; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#125;;<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);<br /><b>let</b> pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);<br /><b>let</b> total_active_stake &#61; active &#43; pending_active;<br /><b>let</b> accumulated_rewards &#61; total_active_stake &#45; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>ensures</b> result_1 &#61;&#61; total_active_stake;<br /><b>ensures</b> result_2 &#61;&#61; accumulated_rewards;<br /></code></pre>



<a id="@Specification_1_pending_distribution_counts"></a>

### Function `pending_distribution_counts`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_pending_distribution_counts">pending_distribution_counts</a>(staker: <b>address</b>, operator: <b>address</b>): u64<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a>;<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);<br /><b>let</b> shareholders_count &#61; len(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.distribution_pool.shareholders);<br /><b>ensures</b> result &#61;&#61; shareholders_count;<br /></code></pre>



<a id="@Specification_1_staking_contract_exists"></a>

### Function `staking_contract_exists`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_staking_contract_exists">staking_contract_exists</a>(staker: <b>address</b>, operator: <b>address</b>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="staking_contract.md#0x1_staking_contract_spec_staking_contract_exists">spec_staking_contract_exists</a>(staker, operator);<br /></code></pre>




<a id="0x1_staking_contract_spec_staking_contract_exists"></a>


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_spec_staking_contract_exists">spec_staking_contract_exists</a>(staker: <b>address</b>, operator: <b>address</b>): bool &#123;<br />   <b>if</b> (!<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker)) &#123;<br />       <b>false</b><br />   &#125; <b>else</b> &#123;<br />       <b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker);<br />       <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(store.staking_contracts, operator)<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_beneficiary_for_operator"></a>

### Function `beneficiary_for_operator`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_beneficiary_for_operator">beneficiary_for_operator</a>(operator: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_create_staking_contract"></a>

### Function `create_staking_contract`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_staking_contract">create_staking_contract</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, voter: <b>address</b>, amount: u64, commission_percentage: u64, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>


Account is not frozen and sufficient to withdraw.


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_PreconditionsInCreateContract">PreconditionsInCreateContract</a>;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;AptosCoin&gt; &#123; <a href="account.md#0x1_account">account</a>: staker &#125;;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_CreateStakingContractWithCoinsAbortsIfAndEnsures">CreateStakingContractWithCoinsAbortsIfAndEnsures</a>;<br /></code></pre>



<a id="@Specification_1_create_staking_contract_with_coins"></a>

### Function `create_staking_contract_with_coins`


<pre><code><b>public</b> <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_staking_contract_with_coins">create_staking_contract_with_coins</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, voter: <b>address</b>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;, commission_percentage: u64, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b><br /></code></pre>


The amount should be at least the min_stake_required, so the stake pool will be eligible to join the validator set.
Initialize Store resource if this is the first time the staker has delegated to anyone.
Cannot create the staking contract if it already exists.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_PreconditionsInCreateContract">PreconditionsInCreateContract</a>;<br /><b>let</b> amount &#61; coins.value;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_CreateStakingContractWithCoinsAbortsIfAndEnsures">CreateStakingContractWithCoinsAbortsIfAndEnsures</a> &#123; amount &#125;;<br /></code></pre>



<a id="@Specification_1_add_stake"></a>

### Function `add_stake`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_add_stake">add_stake</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, amount: u64)<br /></code></pre>


Account is not frozen and sufficient to withdraw.
Staking_contract exists the stacker/operator pair.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 600;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;<br /><b>aborts_if</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">reconfiguration_state::spec_is_in_progress</a>();<br /><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123; staker: staker_address &#125;;<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;AptosCoin&gt; &#123; <a href="account.md#0x1_account">account</a>: staker &#125;;<br /><b>let</b> balance &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(staker_address).<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>let</b> <b>post</b> post_coin &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(staker_address).<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>ensures</b> post_coin &#61;&#61; balance &#45; amount;<br /><b>let</b> owner_cap &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap;<br /><b>include</b> <a href="stake.md#0x1_stake_AddStakeWithCapAbortsIfAndEnsures">stake::AddStakeWithCapAbortsIfAndEnsures</a> &#123; owner_cap &#125;;<br /><b>let</b> <b>post</b> post_store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br /><b>let</b> <b>post</b> post_staking_contract &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_store.staking_contracts, operator);<br /><b>aborts_if</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal &#43; amount &gt; MAX_U64;<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> post_staking_contract.principal &#61;&#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal &#43; amount;<br /></code></pre>



<a id="@Specification_1_update_voter"></a>

### Function `update_voter`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_voter">update_voter</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_UpdateVoterSchema">UpdateVoterSchema</a> &#123; staker: staker_address &#125;;<br /><b>let</b> <b>post</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br /><b>let</b> <b>post</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);<br /><b>let</b> <b>post</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address;<br /><b>let</b> <b>post</b> new_delegated_voter &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address).delegated_voter;<br /><b>ensures</b> new_delegated_voter &#61;&#61; new_voter;<br /></code></pre>



<a id="@Specification_1_reset_lockup"></a>

### Function `reset_lockup`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_reset_lockup">reset_lockup</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>)<br /></code></pre>


Staking_contract exists the stacker/operator pair.
Only active validator can update locked_until_secs.


<pre><code><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br />// This enforces <a id="high-level-req-5" href="#high-level-req">high&#45;level requirement 5</a>:
<b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123; staker: staker_address &#125;;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_IncreaseLockupWithCapAbortsIf">IncreaseLockupWithCapAbortsIf</a> &#123; staker: staker_address &#125;;<br /></code></pre>



<a id="@Specification_1_update_commision"></a>

### Function `update_commision`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_commision">update_commision</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_commission_percentage: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>aborts_if</b> new_commission_percentage &gt; 100;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123; staker: staker_address &#125;;<br /></code></pre>



<a id="@Specification_1_request_commission"></a>

### Function `request_commission`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_request_commission">request_commission</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, staker: <b>address</b>, operator: <b>address</b>)<br /></code></pre>


Only staker or operator can call this.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123; staker &#125;;<br /><b>aborts_if</b> account_addr !&#61; staker &amp;&amp; account_addr !&#61; operator;<br /></code></pre>



<a id="@Specification_1_request_commission_internal"></a>

### Function `request_commission_internal`


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_request_commission_internal">request_commission_internal</a>(operator: <b>address</b>, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>, add_distribution_events: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">staking_contract::AddDistributionEvent</a>&gt;, request_commission_events: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_RequestCommissionEvent">staking_contract::RequestCommissionEvent</a>&gt;): u64<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_GetStakingContractAmountsAbortsIf">GetStakingContractAmountsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_unlock_stake"></a>

### Function `unlock_stake`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_unlock_stake">unlock_stake</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>requires</b> amount &gt; 0;<br /><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123; staker: staker_address &#125;;<br /></code></pre>



<a id="@Specification_1_unlock_rewards"></a>

### Function `unlock_rewards`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_unlock_rewards">unlock_rewards</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>)<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
<b>requires</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &gt;&#61; 0 &amp;&amp; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.<a href="staking_contract.md#0x1_staking_contract_commission_percentage">commission_percentage</a> &lt;&#61; 100;<br /><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address).staking_contracts;<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123; staker: staker_address &#125;;<br /></code></pre>



<a id="@Specification_1_switch_operator_with_same_commission"></a>

### Function `switch_operator_with_same_commission`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_switch_operator_with_same_commission">switch_operator_with_same_commission</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123; staker: staker_address, operator: old_operator &#125;;<br /></code></pre>



<a id="@Specification_1_switch_operator"></a>

### Function `switch_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_switch_operator">switch_operator</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>, new_commission_percentage: u64)<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123; staker: staker_address, operator: old_operator &#125;;<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br /><b>let</b> staking_contracts &#61; store.staking_contracts;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(staking_contracts, new_operator);<br /></code></pre>



<a id="@Specification_1_set_beneficiary_for_operator"></a>

### Function `set_beneficiary_for_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_beneficiary: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_distribute"></a>

### Function `distribute`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_contract.md#0x1_staking_contract_distribute">distribute</a>(staker: <b>address</b>, operator: <b>address</b>)<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_distribute_internal"></a>

### Function `distribute_internal`


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_distribute_internal">distribute_internal</a>(staker: <b>address</b>, operator: <b>address</b>, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>, distribute_events: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_DistributeEvent">staking_contract::DistributeEvent</a>&gt;)<br /></code></pre>


The StakePool exists under the pool_address of StakingContract.
The value of inactive and pending_inactive in the stake_pool is up to MAX_U64.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>let</b> stake_pool &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &gt; MAX_U64;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address);<br /></code></pre>



<a id="@Specification_1_assert_staking_contract_exists"></a>

### Function `assert_staking_contract_exists`


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_assert_staking_contract_exists">assert_staking_contract_exists</a>(staker: <b>address</b>, operator: <b>address</b>)<br /></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_add_distribution"></a>

### Function `add_distribution`


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_add_distribution">add_distribution</a>(operator: <b>address</b>, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<b>mut</b> <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>, recipient: <b>address</b>, coins_amount: u64, add_distribution_events: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="staking_contract.md#0x1_staking_contract_AddDistributionEvent">staking_contract::AddDistributionEvent</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_get_staking_contract_amounts_internal"></a>

### Function `get_staking_contract_amounts_internal`


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_get_staking_contract_amounts_internal">get_staking_contract_amounts_internal</a>(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: &amp;<a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>): (u64, u64, u64)<br /></code></pre>


The StakePool exists under the pool_address of StakingContract.


<pre><code><b>include</b> <a href="staking_contract.md#0x1_staking_contract_GetStakingContractAmountsAbortsIf">GetStakingContractAmountsAbortsIf</a>;<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);<br /><b>let</b> pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);<br /><b>let</b> total_active_stake &#61; active &#43; pending_active;<br /><b>let</b> accumulated_rewards &#61; total_active_stake &#45; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>let</b> commission_amount &#61; accumulated_rewards &#42; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;<br /><b>ensures</b> result_1 &#61;&#61; total_active_stake;<br /><b>ensures</b> result_2 &#61;&#61; accumulated_rewards;<br /><b>ensures</b> result_3 &#61;&#61; commission_amount;<br /></code></pre>



<a id="@Specification_1_create_stake_pool"></a>

### Function `create_stake_pool`


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_create_stake_pool">create_stake_pool</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, voter: <b>address</b>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, <a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;<br /><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>let</b> seed_0 &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(staker_address);<br /><b>let</b> seed_1 &#61; concat(concat(concat(seed_0, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(operator)), <a href="staking_contract.md#0x1_staking_contract_SALT">SALT</a>), contract_creation_seed);<br /><b>let</b> resource_addr &#61; <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(staker_address, seed_1);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_CreateStakePoolAbortsIf">CreateStakePoolAbortsIf</a> &#123; resource_addr &#125;;<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);<br /><b>let</b> <b>post</b> post_account &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);<br /><b>ensures</b> post_account.authentication_key &#61;&#61; <a href="account.md#0x1_account_ZERO_AUTH_KEY">account::ZERO_AUTH_KEY</a>;<br /><b>ensures</b> post_account.signer_capability_offer.for &#61;&#61; std::option::spec_some(resource_addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(resource_addr);<br /><b>let</b> <b>post</b> post_owner_cap &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>&gt;(resource_addr);<br /><b>let</b> <b>post</b> post_pool_address &#61; post_owner_cap.pool_address;<br /><b>let</b> <b>post</b> post_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(post_pool_address);<br /><b>let</b> <b>post</b> post_operator &#61; post_stake_pool.operator_address;<br /><b>let</b> <b>post</b> post_delegated_voter &#61; post_stake_pool.delegated_voter;<br /><b>ensures</b> resource_addr !&#61; operator &#61;&#61;&gt; post_operator &#61;&#61; operator;<br /><b>ensures</b> resource_addr !&#61; voter &#61;&#61;&gt; post_delegated_voter &#61;&#61; voter;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result_1) &#61;&#61; resource_addr;<br /><b>ensures</b> result_2 &#61;&#61; SignerCapability &#123; <a href="account.md#0x1_account">account</a>: resource_addr &#125;;<br /><b>ensures</b> result_3 &#61;&#61; OwnerCapability &#123; pool_address: resource_addr &#125;;<br /></code></pre>



<a id="@Specification_1_update_distribution_pool"></a>

### Function `update_distribution_pool`


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_update_distribution_pool">update_distribution_pool</a>(distribution_pool: &amp;<b>mut</b> <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, updated_total_coins: u64, operator: <b>address</b>, commission_percentage: u64)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /></code></pre>



<a id="@Specification_1_new_staking_contracts_holder"></a>

### Function `new_staking_contracts_holder`


<pre><code><b>fun</b> <a href="staking_contract.md#0x1_staking_contract_new_staking_contracts_holder">new_staking_contracts_holder</a>(staker: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a><br /></code></pre>


The Account exists under the staker.
The guid_creation_num of the ccount resource is up to MAX_U64.


<pre><code><b>include</b> <a href="staking_contract.md#0x1_staking_contract_NewStakingContractsHolderAbortsIf">NewStakingContractsHolderAbortsIf</a>;<br /></code></pre>




<a id="0x1_staking_contract_NewStakingContractsHolderAbortsIf"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_NewStakingContractsHolderAbortsIf">NewStakingContractsHolderAbortsIf</a> &#123;<br />staker: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br />&#125;<br /></code></pre>


The Store exists under the staker.
a staking_contract exists for the staker/operator pair.


<a id="0x1_staking_contract_ContractExistsAbortsIf"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a> &#123;<br />staker: <b>address</b>;<br />operator: <b>address</b>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker);<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker).staking_contracts;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(staking_contracts, operator);<br />&#125;<br /></code></pre>




<a id="0x1_staking_contract_UpdateVoterSchema"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_UpdateVoterSchema">UpdateVoterSchema</a> &#123;<br />staker: <b>address</b>;<br />operator: <b>address</b>;<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker);<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address);<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">ContractExistsAbortsIf</a>;<br />&#125;<br /></code></pre>




<a id="0x1_staking_contract_WithdrawAbortsIf"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;CoinType&gt; &#123;<br /><a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />amount: u64;<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> balance &#61; coin_store.<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> coin_store.frozen;<br /><b>aborts_if</b> balance &lt; amount;<br />&#125;<br /></code></pre>




<a id="0x1_staking_contract_GetStakingContractAmountsAbortsIf"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_GetStakingContractAmountsAbortsIf">GetStakingContractAmountsAbortsIf</a> &#123;<br /><a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: <a href="staking_contract.md#0x1_staking_contract_StakingContract">StakingContract</a>;<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);<br /><b>let</b> pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);<br /><b>let</b> total_active_stake &#61; active &#43; pending_active;<br /><b>let</b> accumulated_rewards &#61; total_active_stake &#45; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> active &#43; pending_active &gt; MAX_U64;<br /><b>aborts_if</b> total_active_stake &lt; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>aborts_if</b> accumulated_rewards &#42; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &gt; MAX_U64;<br />&#125;<br /></code></pre>




<a id="0x1_staking_contract_IncreaseLockupWithCapAbortsIf"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_IncreaseLockupWithCapAbortsIf">IncreaseLockupWithCapAbortsIf</a> &#123;<br />staker: <b>address</b>;<br />operator: <b>address</b>;<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker);<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address;<br /><b>aborts_if</b> !<a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(pool_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> config &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> old_locked_until_secs &#61; stake_pool.locked_until_secs;<br /><b>let</b> seconds &#61; <b>global</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(<br />    @aptos_framework<br />).microseconds / <a href="timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">timestamp::MICRO_CONVERSION_FACTOR</a>;<br /><b>let</b> new_locked_until_secs &#61; seconds &#43; config.recurring_lockup_duration_secs;<br /><b>aborts_if</b> seconds &#43; config.recurring_lockup_duration_secs &gt; MAX_U64;<br /><b>aborts_if</b> old_locked_until_secs &gt; new_locked_until_secs &#124;&#124; old_locked_until_secs &#61;&#61; new_locked_until_secs;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker);<br /><b>let</b> <b>post</b> post_staking_contract &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_store.staking_contracts, operator);<br /><b>let</b> <b>post</b> post_stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(post_staking_contract.owner_cap.pool_address);<br /><b>ensures</b> post_stake_pool.locked_until_secs &#61;&#61; new_locked_until_secs;<br />&#125;<br /></code></pre>




<a id="0x1_staking_contract_CreateStakingContractWithCoinsAbortsIfAndEnsures"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_CreateStakingContractWithCoinsAbortsIfAndEnsures">CreateStakingContractWithCoinsAbortsIfAndEnsures</a> &#123;<br />staker: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />operator: <b>address</b>;<br />voter: <b>address</b>;<br />amount: u64;<br />commission_percentage: u64;<br />contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>aborts_if</b> commission_percentage &gt; 100;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> config &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> min_stake_required &#61; config.minimum_stake;<br /><b>aborts_if</b> amount &lt; min_stake_required;<br /><b>let</b> staker_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(staker);<br /><b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(staker_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address) &amp;&amp; !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(staker_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address) &amp;&amp; <a href="account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
    <b>ensures</b> <b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br /><b>let</b> staking_contracts &#61; store.staking_contracts;<br /><b>let</b> owner_cap &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator).owner_cap;<br /><b>let</b> <b>post</b> post_store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">Store</a>&gt;(staker_address);<br /><b>let</b> <b>post</b> post_staking_contracts &#61; post_store.staking_contracts;<br />&#125;<br /></code></pre>




<a id="0x1_staking_contract_PreconditionsInCreateContract"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_PreconditionsInCreateContract">PreconditionsInCreateContract</a> &#123;<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">stake::ValidatorPerformance</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(<br />    @aptos_framework<br />) &#124;&#124; !std::features::spec_periodical_reward_rate_decrease_enabled();<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;aptos_framework::timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_AptosCoinCapabilities">stake::AptosCoinCapabilities</a>&gt;(@aptos_framework);<br />&#125;<br /></code></pre>




<a id="0x1_staking_contract_CreateStakePoolAbortsIf"></a>


<pre><code><b>schema</b> <a href="staking_contract.md#0x1_staking_contract_CreateStakePoolAbortsIf">CreateStakePoolAbortsIf</a> &#123;<br />resource_addr: <b>address</b>;<br />operator: <b>address</b>;<br />voter: <b>address</b>;<br />contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>let</b> acc &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr) &amp;&amp; (len(<br />    acc.signer_capability_offer.for.vec<br />) !&#61; 0 &#124;&#124; acc.sequence_number !&#61; 0);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr) &amp;&amp; len(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(resource_addr)) !&#61; 32;<br /><b>aborts_if</b> len(<a href="account.md#0x1_account_ZERO_AUTH_KEY">account::ZERO_AUTH_KEY</a>) !&#61; 32;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorConfig">stake::ValidatorConfig</a>&gt;(resource_addr);<br /><b>let</b> allowed &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">stake::AllowedValidators</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_AllowedValidators">stake::AllowedValidators</a>&gt;(@aptos_framework) &amp;&amp; !contains(allowed.accounts, resource_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(resource_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>&gt;(resource_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(<br />    resource_addr<br />) &amp;&amp; acc.guid_creation_num &#43; 12 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
