
<a id="0x1_staking_contract"></a>

# Module `0x1::staking_contract`

Allow stakers and operators to enter a staking contract with reward sharing.<br/> The main accounting logic in a staking contract consists of 2 parts:<br/> 1. Tracks how much commission needs to be paid out to the operator. This is tracked with an increasing principal<br/> amount that&apos;s updated every time the operator requests commission, the staker withdraws funds, or the staker<br/> switches operators.<br/> 2. Distributions of funds to operators (commissions) and stakers (stake withdrawals) use the shares model provided<br/> by the pool_u64 to track shares that increase in price as the stake pool accumulates rewards.<br/><br/> Example flow:<br/> 1. A staker creates a staking contract with an operator by calling create_staking_contract() with 100 coins of<br/> initial stake and commission &#61; 10%. This means the operator will receive 10% of any accumulated rewards. A new stake<br/> pool will be created and hosted in a separate account that&apos;s controlled by the staking contract.<br/> 2. The operator sets up a validator node and, once ready, joins the validator set by calling stake::join_validator_set<br/> 3. After some time, the stake pool gains rewards and now has 150 coins.<br/> 4. Operator can now call request_commission. 10% of (150 &#45; 100) &#61; 5 coins will be unlocked from the stake pool. The<br/> staker&apos;s principal is now updated from 100 to 145 (150 coins &#45; 5 coins of commission). The pending distribution pool<br/> has 5 coins total and the operator owns all 5 shares of it.<br/> 5. Some more time has passed. The pool now has 50 more coins in rewards and a total balance of 195. The operator<br/> calls request_commission again. Since the previous 5 coins have now become withdrawable, it&apos;ll be deposited into the<br/> operator&apos;s account first. Their new commission will be 10% of (195 coins &#45; 145 principal) &#61; 5 coins. Principal is<br/> updated to be 190 (195 &#45; 5). Pending distribution pool has 5 coins and operator owns all 5 shares.<br/> 6. Staker calls unlock_stake to unlock 50 coins of stake, which gets added to the pending distribution pool. Based<br/> on shares math, staker will be owning 50 shares and operator still owns 5 shares of the 55&#45;coin pending distribution<br/> pool.<br/> 7. Some time passes and the 55 coins become fully withdrawable from the stake pool. Due to accumulated rewards, the<br/> 55 coins become 70 coins. Calling distribute() distributes 6 coins to the operator and 64 coins to the validator.


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


<pre><code>use 0x1::account;<br/>use 0x1::aptos_account;<br/>use 0x1::aptos_coin;<br/>use 0x1::bcs;<br/>use 0x1::coin;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::pool_u64;<br/>use 0x1::signer;<br/>use 0x1::simple_map;<br/>use 0x1::stake;<br/>use 0x1::staking_config;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_staking_contract_StakingGroupContainer"></a>

## Struct `StakingGroupContainer`



<pre><code>&#35;[resource_group(&#35;[scope &#61; module_])]<br/>struct StakingGroupContainer<br/></code></pre>



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



<pre><code>struct StakingContract has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>principal: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>owner_cap: stake::OwnerCapability</code>
</dt>
<dd>

</dd>
<dt>
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>distribution_pool: pool_u64::Pool</code>
</dt>
<dd>

</dd>
<dt>
<code>signer_cap: account::SignerCapability</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_Store"></a>

## Resource `Store`



<pre><code>struct Store has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>staking_contracts: simple_map::SimpleMap&lt;address, staking_contract::StakingContract&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_staking_contract_events: event::EventHandle&lt;staking_contract::CreateStakingContractEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_voter_events: event::EventHandle&lt;staking_contract::UpdateVoterEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reset_lockup_events: event::EventHandle&lt;staking_contract::ResetLockupEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_stake_events: event::EventHandle&lt;staking_contract::AddStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>request_commission_events: event::EventHandle&lt;staking_contract::RequestCommissionEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_stake_events: event::EventHandle&lt;staking_contract::UnlockStakeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>switch_operator_events: event::EventHandle&lt;staking_contract::SwitchOperatorEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_distribution_events: event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>distribute_events: event::EventHandle&lt;staking_contract::DistributeEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_BeneficiaryForOperator"></a>

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

<a id="0x1_staking_contract_UpdateCommissionEvent"></a>

## Struct `UpdateCommissionEvent`



<pre><code>struct UpdateCommissionEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>staker: address</code>
</dt>
<dd>

</dd>
<dt>
<code>operator: address</code>
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



<pre><code>&#35;[event]<br/>struct UpdateCommission has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>staker: address</code>
</dt>
<dd>

</dd>
<dt>
<code>operator: address</code>
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



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::staking_contract::StakingGroupContainer])]<br/>struct StakingGroupUpdateCommissionEvent has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>update_commission_events: event::EventHandle&lt;staking_contract::UpdateCommissionEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_CreateStakingContract"></a>

## Struct `CreateStakingContract`



<pre><code>&#35;[event]<br/>struct CreateStakingContract has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
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



<pre><code>&#35;[event]<br/>struct UpdateVoter has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_voter: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_ResetLockup"></a>

## Struct `ResetLockup`



<pre><code>&#35;[event]<br/>struct ResetLockup has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddStake"></a>

## Struct `AddStake`



<pre><code>&#35;[event]<br/>struct AddStake has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
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

<a id="0x1_staking_contract_RequestCommission"></a>

## Struct `RequestCommission`



<pre><code>&#35;[event]<br/>struct RequestCommission has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
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



<pre><code>&#35;[event]<br/>struct UnlockStake has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
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
<dt>
<code>commission_paid: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_SwitchOperator"></a>

## Struct `SwitchOperator`



<pre><code>&#35;[event]<br/>struct SwitchOperator has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
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
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddDistribution"></a>

## Struct `AddDistribution`



<pre><code>&#35;[event]<br/>struct AddDistribution has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
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

<a id="0x1_staking_contract_Distribute"></a>

## Struct `Distribute`



<pre><code>&#35;[event]<br/>struct Distribute has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: address</code>
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

<a id="0x1_staking_contract_CreateStakingContractEvent"></a>

## Struct `CreateStakingContractEvent`



<pre><code>struct CreateStakingContractEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
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



<pre><code>struct UpdateVoterEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_voter: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_ResetLockupEvent"></a>

## Struct `ResetLockupEvent`



<pre><code>struct ResetLockupEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddStakeEvent"></a>

## Struct `AddStakeEvent`



<pre><code>struct AddStakeEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
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

<a id="0x1_staking_contract_RequestCommissionEvent"></a>

## Struct `RequestCommissionEvent`



<pre><code>struct RequestCommissionEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
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



<pre><code>struct UnlockStakeEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
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
<dt>
<code>commission_paid: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_SwitchOperatorEvent"></a>

## Struct `SwitchOperatorEvent`



<pre><code>struct SwitchOperatorEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
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
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddDistributionEvent"></a>

## Struct `AddDistributionEvent`



<pre><code>struct AddDistributionEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
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

<a id="0x1_staking_contract_DistributeEvent"></a>

## Struct `DistributeEvent`



<pre><code>struct DistributeEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: address</code>
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


<pre><code>const EINVALID_COMMISSION_PERCENTAGE: u64 &#61; 2;<br/></code></pre>



<a id="0x1_staking_contract_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED"></a>

Chaning beneficiaries for operators is not supported.


<pre><code>const EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED: u64 &#61; 9;<br/></code></pre>



<a id="0x1_staking_contract_ECANT_MERGE_STAKING_CONTRACTS"></a>

Staking contracts can&apos;t be merged.


<pre><code>const ECANT_MERGE_STAKING_CONTRACTS: u64 &#61; 5;<br/></code></pre>



<a id="0x1_staking_contract_EINSUFFICIENT_ACTIVE_STAKE_TO_WITHDRAW"></a>

Not enough active stake to withdraw. Some stake might still pending and will be active in the next epoch.


<pre><code>const EINSUFFICIENT_ACTIVE_STAKE_TO_WITHDRAW: u64 &#61; 7;<br/></code></pre>



<a id="0x1_staking_contract_EINSUFFICIENT_STAKE_AMOUNT"></a>

Store amount must be at least the min stake required for a stake pool to join the validator set.


<pre><code>const EINSUFFICIENT_STAKE_AMOUNT: u64 &#61; 1;<br/></code></pre>



<a id="0x1_staking_contract_ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY"></a>

Caller must be either the staker, operator, or beneficiary.


<pre><code>const ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY: u64 &#61; 8;<br/></code></pre>



<a id="0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR"></a>

No staking contract between the staker and operator found.


<pre><code>const ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR: u64 &#61; 4;<br/></code></pre>



<a id="0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_STAKER"></a>

Staker has no staking contracts.


<pre><code>const ENO_STAKING_CONTRACT_FOUND_FOR_STAKER: u64 &#61; 3;<br/></code></pre>



<a id="0x1_staking_contract_ESTAKING_CONTRACT_ALREADY_EXISTS"></a>

The staking contract already exists and cannot be re&#45;created.


<pre><code>const ESTAKING_CONTRACT_ALREADY_EXISTS: u64 &#61; 6;<br/></code></pre>



<a id="0x1_staking_contract_MAXIMUM_PENDING_DISTRIBUTIONS"></a>

Maximum number of distributions a stake pool can support.


<pre><code>const MAXIMUM_PENDING_DISTRIBUTIONS: u64 &#61; 20;<br/></code></pre>



<a id="0x1_staking_contract_SALT"></a>



<pre><code>const SALT: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 115, 116, 97, 107, 105, 110, 103, 95, 99, 111, 110, 116, 114, 97, 99, 116];<br/></code></pre>



<a id="0x1_staking_contract_stake_pool_address"></a>

## Function `stake_pool_address`

Return the address of the underlying stake pool for the staking contract between the provided staker and<br/> operator.<br/><br/> This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun stake_pool_address(staker: address, operator: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun stake_pool_address(staker: address, operator: address): address acquires Store &#123;<br/>    assert_staking_contract_exists(staker, operator);<br/>    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;<br/>    simple_map::borrow(staking_contracts, &amp;operator).pool_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_last_recorded_principal"></a>

## Function `last_recorded_principal`

Return the last recorded principal (the amount that 100% belongs to the staker with commission already paid for)<br/> for staking contract between the provided staker and operator.<br/><br/> This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun last_recorded_principal(staker: address, operator: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun last_recorded_principal(staker: address, operator: address): u64 acquires Store &#123;<br/>    assert_staking_contract_exists(staker, operator);<br/>    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;<br/>    simple_map::borrow(staking_contracts, &amp;operator).principal<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_commission_percentage"></a>

## Function `commission_percentage`

Return percentage of accumulated rewards that will be paid to the operator as commission for staking contract<br/> between the provided staker and operator.<br/><br/> This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun commission_percentage(staker: address, operator: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commission_percentage(staker: address, operator: address): u64 acquires Store &#123;<br/>    assert_staking_contract_exists(staker, operator);<br/>    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;<br/>    simple_map::borrow(staking_contracts, &amp;operator).commission_percentage<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_staking_contract_amounts"></a>

## Function `staking_contract_amounts`

Return a tuple of three numbers:<br/> 1. The total active stake in the underlying stake pool<br/> 2. The total accumulated rewards that haven&apos;t had commission paid out<br/> 3. The commission amount owned from those accumulated rewards.<br/><br/> This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun staking_contract_amounts(staker: address, operator: address): (u64, u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun staking_contract_amounts(staker: address, operator: address): (u64, u64, u64) acquires Store &#123;<br/>    assert_staking_contract_exists(staker, operator);<br/>    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;<br/>    let staking_contract &#61; simple_map::borrow(staking_contracts, &amp;operator);<br/>    get_staking_contract_amounts_internal(staking_contract)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_pending_distribution_counts"></a>

## Function `pending_distribution_counts`

Return the number of pending distributions (e.g. commission, withdrawals from stakers).<br/><br/> This errors out the staking contract with the provided staker and operator doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun pending_distribution_counts(staker: address, operator: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pending_distribution_counts(staker: address, operator: address): u64 acquires Store &#123;<br/>    assert_staking_contract_exists(staker, operator);<br/>    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;<br/>    pool_u64::shareholders_count(&amp;simple_map::borrow(staking_contracts, &amp;operator).distribution_pool)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_staking_contract_exists"></a>

## Function `staking_contract_exists`

Return true if the staking contract between the provided staker and operator exists.


<pre><code>&#35;[view]<br/>public fun staking_contract_exists(staker: address, operator: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun staking_contract_exists(staker: address, operator: address): bool acquires Store &#123;<br/>    if (!exists&lt;Store&gt;(staker)) &#123;<br/>        return false<br/>    &#125;;<br/><br/>    let store &#61; borrow_global&lt;Store&gt;(staker);<br/>    simple_map::contains_key(&amp;store.staking_contracts, &amp;operator)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_beneficiary_for_operator"></a>

## Function `beneficiary_for_operator`

Return the beneficiary address of the operator.


<pre><code>&#35;[view]<br/>public fun beneficiary_for_operator(operator: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun beneficiary_for_operator(operator: address): address acquires BeneficiaryForOperator &#123;<br/>    if (exists&lt;BeneficiaryForOperator&gt;(operator)) &#123;<br/>        return borrow_global&lt;BeneficiaryForOperator&gt;(operator).beneficiary_for_operator<br/>    &#125; else &#123;<br/>        operator<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_get_expected_stake_pool_address"></a>

## Function `get_expected_stake_pool_address`

Return the address of the stake pool to be created with the provided staker, operator and seed.


<pre><code>&#35;[view]<br/>public fun get_expected_stake_pool_address(staker: address, operator: address, contract_creation_seed: vector&lt;u8&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_expected_stake_pool_address(<br/>    staker: address,<br/>    operator: address,<br/>    contract_creation_seed: vector&lt;u8&gt;,<br/>): address &#123;<br/>    let seed &#61; create_resource_account_seed(staker, operator, contract_creation_seed);<br/>    account::create_resource_address(&amp;staker, seed)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_create_staking_contract"></a>

## Function `create_staking_contract`

Staker can call this function to create a simple staking contract with a specified operator.


<pre><code>public entry fun create_staking_contract(staker: &amp;signer, operator: address, voter: address, amount: u64, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_staking_contract(<br/>    staker: &amp;signer,<br/>    operator: address,<br/>    voter: address,<br/>    amount: u64,<br/>    commission_percentage: u64,<br/>    // Optional seed used when creating the staking contract account.<br/>    contract_creation_seed: vector&lt;u8&gt;,<br/>) acquires Store &#123;<br/>    let staked_coins &#61; coin::withdraw&lt;AptosCoin&gt;(staker, amount);<br/>    create_staking_contract_with_coins(<br/>        staker, operator, voter, staked_coins, commission_percentage, contract_creation_seed);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_create_staking_contract_with_coins"></a>

## Function `create_staking_contract_with_coins`

Staker can call this function to create a simple staking contract with a specified operator.


<pre><code>public fun create_staking_contract_with_coins(staker: &amp;signer, operator: address, voter: address, coins: coin::Coin&lt;aptos_coin::AptosCoin&gt;, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_staking_contract_with_coins(<br/>    staker: &amp;signer,<br/>    operator: address,<br/>    voter: address,<br/>    coins: Coin&lt;AptosCoin&gt;,<br/>    commission_percentage: u64,<br/>    // Optional seed used when creating the staking contract account.<br/>    contract_creation_seed: vector&lt;u8&gt;,<br/>): address acquires Store &#123;<br/>    assert!(<br/>        commission_percentage &gt;&#61; 0 &amp;&amp; commission_percentage &lt;&#61; 100,<br/>        error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE),<br/>    );<br/>    // The amount should be at least the min_stake_required, so the stake pool will be eligible to join the<br/>    // validator set.<br/>    let (min_stake_required, _) &#61; staking_config::get_required_stake(&amp;staking_config::get());<br/>    let principal &#61; coin::value(&amp;coins);<br/>    assert!(principal &gt;&#61; min_stake_required, error::invalid_argument(EINSUFFICIENT_STAKE_AMOUNT));<br/><br/>    // Initialize Store resource if this is the first time the staker has delegated to anyone.<br/>    let staker_address &#61; signer::address_of(staker);<br/>    if (!exists&lt;Store&gt;(staker_address)) &#123;<br/>        move_to(staker, new_staking_contracts_holder(staker));<br/>    &#125;;<br/><br/>    // Cannot create the staking contract if it already exists.<br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);<br/>    let staking_contracts &#61; &amp;mut store.staking_contracts;<br/>    assert!(<br/>        !simple_map::contains_key(staking_contracts, &amp;operator),<br/>        error::already_exists(ESTAKING_CONTRACT_ALREADY_EXISTS)<br/>    );<br/><br/>    // Initialize the stake pool in a new resource account. This allows the same staker to contract with multiple<br/>    // different operators.<br/>    let (stake_pool_signer, stake_pool_signer_cap, owner_cap) &#61;<br/>        create_stake_pool(staker, operator, voter, contract_creation_seed);<br/><br/>    // Add the stake to the stake pool.<br/>    stake::add_stake_with_cap(&amp;owner_cap, coins);<br/><br/>    // Create the contract record.<br/>    let pool_address &#61; signer::address_of(&amp;stake_pool_signer);<br/>    simple_map::add(staking_contracts, operator, StakingContract &#123;<br/>        principal,<br/>        pool_address,<br/>        owner_cap,<br/>        commission_percentage,<br/>        // Make sure we don&apos;t have too many pending recipients in the distribution pool.<br/>        // Otherwise, a griefing attack is possible where the staker can keep switching operators and create too<br/>        // many pending distributions. This can lead to out&#45;of&#45;gas failure whenever distribute() is called.<br/>        distribution_pool: pool_u64::create(MAXIMUM_PENDING_DISTRIBUTIONS),<br/>        signer_cap: stake_pool_signer_cap,<br/>    &#125;);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(CreateStakingContract &#123; operator, voter, pool_address, principal, commission_percentage &#125;);<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut store.create_staking_contract_events,<br/>        CreateStakingContractEvent &#123; operator, voter, pool_address, principal, commission_percentage &#125;,<br/>    );<br/>    pool_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_add_stake"></a>

## Function `add_stake`

Add more stake to an existing staking contract.


<pre><code>public entry fun add_stake(staker: &amp;signer, operator: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_stake(staker: &amp;signer, operator: address, amount: u64) acquires Store &#123;<br/>    let staker_address &#61; signer::address_of(staker);<br/>    assert_staking_contract_exists(staker_address, operator);<br/><br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);<br/>    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);<br/><br/>    // Add the stake to the stake pool.<br/>    let staked_coins &#61; coin::withdraw&lt;AptosCoin&gt;(staker, amount);<br/>    stake::add_stake_with_cap(&amp;staking_contract.owner_cap, staked_coins);<br/><br/>    staking_contract.principal &#61; staking_contract.principal &#43; amount;<br/>    let pool_address &#61; staking_contract.pool_address;<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(AddStake &#123; operator, pool_address, amount &#125;);<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut store.add_stake_events,<br/>        AddStakeEvent &#123; operator, pool_address, amount &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_update_voter"></a>

## Function `update_voter`

Convenient function to allow the staker to update the voter address in a staking contract they made.


<pre><code>public entry fun update_voter(staker: &amp;signer, operator: address, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_voter(staker: &amp;signer, operator: address, new_voter: address) acquires Store &#123;<br/>    let staker_address &#61; signer::address_of(staker);<br/>    assert_staking_contract_exists(staker_address, operator);<br/><br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);<br/>    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);<br/>    let pool_address &#61; staking_contract.pool_address;<br/>    let old_voter &#61; stake::get_delegated_voter(pool_address);<br/>    stake::set_delegated_voter_with_cap(&amp;staking_contract.owner_cap, new_voter);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(UpdateVoter &#123; operator, pool_address, old_voter, new_voter &#125;);<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut store.update_voter_events,<br/>        UpdateVoterEvent &#123; operator, pool_address, old_voter, new_voter &#125;,<br/>    );<br/><br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_reset_lockup"></a>

## Function `reset_lockup`

Convenient function to allow the staker to reset their stake pool&apos;s lockup period to start now.


<pre><code>public entry fun reset_lockup(staker: &amp;signer, operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reset_lockup(staker: &amp;signer, operator: address) acquires Store &#123;<br/>    let staker_address &#61; signer::address_of(staker);<br/>    assert_staking_contract_exists(staker_address, operator);<br/><br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);<br/>    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);<br/>    let pool_address &#61; staking_contract.pool_address;<br/>    stake::increase_lockup_with_cap(&amp;staking_contract.owner_cap);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(ResetLockup &#123; operator, pool_address &#125;);<br/>    &#125;;<br/>    emit_event(&amp;mut store.reset_lockup_events, ResetLockupEvent &#123; operator, pool_address &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_update_commision"></a>

## Function `update_commision`

Convenience function to allow a staker to update the commission percentage paid to the operator.<br/> TODO: fix the typo in function name. commision &#45;&gt; commission


<pre><code>public entry fun update_commision(staker: &amp;signer, operator: address, new_commission_percentage: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_commision(<br/>    staker: &amp;signer,<br/>    operator: address,<br/>    new_commission_percentage: u64<br/>) acquires Store, BeneficiaryForOperator, StakingGroupUpdateCommissionEvent &#123;<br/>    assert!(<br/>        new_commission_percentage &gt;&#61; 0 &amp;&amp; new_commission_percentage &lt;&#61; 100,<br/>        error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE),<br/>    );<br/><br/>    let staker_address &#61; signer::address_of(staker);<br/>    assert!(exists&lt;Store&gt;(staker_address), error::not_found(ENO_STAKING_CONTRACT_FOUND_FOR_STAKER));<br/><br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);<br/>    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);<br/>    distribute_internal(staker_address, operator, staking_contract, &amp;mut store.distribute_events);<br/>    request_commission_internal(<br/>        operator,<br/>        staking_contract,<br/>        &amp;mut store.add_distribution_events,<br/>        &amp;mut store.request_commission_events,<br/>    );<br/>    let old_commission_percentage &#61; staking_contract.commission_percentage;<br/>    staking_contract.commission_percentage &#61; new_commission_percentage;<br/>    if (!exists&lt;StakingGroupUpdateCommissionEvent&gt;(staker_address)) &#123;<br/>        move_to(<br/>            staker,<br/>            StakingGroupUpdateCommissionEvent &#123;<br/>                update_commission_events: account::new_event_handle&lt;UpdateCommissionEvent&gt;(<br/>                    staker<br/>                )<br/>            &#125;<br/>        )<br/>    &#125;;<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            UpdateCommission &#123; staker: staker_address, operator, old_commission_percentage, new_commission_percentage &#125;<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut borrow_global_mut&lt;StakingGroupUpdateCommissionEvent&gt;(staker_address).update_commission_events,<br/>        UpdateCommissionEvent &#123; staker: staker_address, operator, old_commission_percentage, new_commission_percentage &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_request_commission"></a>

## Function `request_commission`

Unlock commission amount from the stake pool. Operator needs to wait for the amount to become withdrawable<br/> at the end of the stake pool&apos;s lockup period before they can actually can withdraw_commission.<br/><br/> Only staker, operator or beneficiary can call this.


<pre><code>public entry fun request_commission(account: &amp;signer, staker: address, operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun request_commission(<br/>    account: &amp;signer,<br/>    staker: address,<br/>    operator: address<br/>) acquires Store, BeneficiaryForOperator &#123;<br/>    let account_addr &#61; signer::address_of(account);<br/>    assert!(<br/>        account_addr &#61;&#61; staker &#124;&#124; account_addr &#61;&#61; operator &#124;&#124; account_addr &#61;&#61; beneficiary_for_operator(operator),<br/>        error::unauthenticated(ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY)<br/>    );<br/>    assert_staking_contract_exists(staker, operator);<br/><br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker);<br/>    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);<br/>    // Short&#45;circuit if zero commission.<br/>    if (staking_contract.commission_percentage &#61;&#61; 0) &#123;<br/>        return<br/>    &#125;;<br/><br/>    // Force distribution of any already inactive stake.<br/>    distribute_internal(staker, operator, staking_contract, &amp;mut store.distribute_events);<br/><br/>    request_commission_internal(<br/>        operator,<br/>        staking_contract,<br/>        &amp;mut store.add_distribution_events,<br/>        &amp;mut store.request_commission_events,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_request_commission_internal"></a>

## Function `request_commission_internal`



<pre><code>fun request_commission_internal(operator: address, staking_contract: &amp;mut staking_contract::StakingContract, add_distribution_events: &amp;mut event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;, request_commission_events: &amp;mut event::EventHandle&lt;staking_contract::RequestCommissionEvent&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun request_commission_internal(<br/>    operator: address,<br/>    staking_contract: &amp;mut StakingContract,<br/>    add_distribution_events: &amp;mut EventHandle&lt;AddDistributionEvent&gt;,<br/>    request_commission_events: &amp;mut EventHandle&lt;RequestCommissionEvent&gt;,<br/>): u64 &#123;<br/>    // Unlock just the commission portion from the stake pool.<br/>    let (total_active_stake, accumulated_rewards, commission_amount) &#61;<br/>        get_staking_contract_amounts_internal(staking_contract);<br/>    staking_contract.principal &#61; total_active_stake &#45; commission_amount;<br/><br/>    // Short&#45;circuit if there&apos;s no commission to pay.<br/>    if (commission_amount &#61;&#61; 0) &#123;<br/>        return 0<br/>    &#125;;<br/><br/>    // Add a distribution for the operator.<br/>    add_distribution(operator, staking_contract, operator, commission_amount, add_distribution_events);<br/><br/>    // Request to unlock the commission from the stake pool.<br/>    // This won&apos;t become fully unlocked until the stake pool&apos;s lockup expires.<br/>    stake::unlock_with_cap(commission_amount, &amp;staking_contract.owner_cap);<br/><br/>    let pool_address &#61; staking_contract.pool_address;<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(RequestCommission &#123; operator, pool_address, accumulated_rewards, commission_amount &#125;);<br/>    &#125;;<br/>    emit_event(<br/>        request_commission_events,<br/>        RequestCommissionEvent &#123; operator, pool_address, accumulated_rewards, commission_amount &#125;,<br/>    );<br/><br/>    commission_amount<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_unlock_stake"></a>

## Function `unlock_stake`

Staker can call this to request withdrawal of part or all of their staking_contract.<br/> This also triggers paying commission to the operator for accounting simplicity.


<pre><code>public entry fun unlock_stake(staker: &amp;signer, operator: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock_stake(<br/>    staker: &amp;signer,<br/>    operator: address,<br/>    amount: u64<br/>) acquires Store, BeneficiaryForOperator &#123;<br/>    // Short&#45;circuit if amount is 0.<br/>    if (amount &#61;&#61; 0) return;<br/><br/>    let staker_address &#61; signer::address_of(staker);<br/>    assert_staking_contract_exists(staker_address, operator);<br/><br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);<br/>    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);<br/><br/>    // Force distribution of any already inactive stake.<br/>    distribute_internal(staker_address, operator, staking_contract, &amp;mut store.distribute_events);<br/><br/>    // For simplicity, we request commission to be paid out first. This avoids having to ensure to staker doesn&apos;t<br/>    // withdraw into the commission portion.<br/>    let commission_paid &#61; request_commission_internal(<br/>        operator,<br/>        staking_contract,<br/>        &amp;mut store.add_distribution_events,<br/>        &amp;mut store.request_commission_events,<br/>    );<br/><br/>    // If there&apos;s less active stake remaining than the amount requested (potentially due to commission),<br/>    // only withdraw up to the active amount.<br/>    let (active, _, _, _) &#61; stake::get_stake(staking_contract.pool_address);<br/>    if (active &lt; amount) &#123;<br/>        amount &#61; active;<br/>    &#125;;<br/>    staking_contract.principal &#61; staking_contract.principal &#45; amount;<br/><br/>    // Record a distribution for the staker.<br/>    add_distribution(operator, staking_contract, staker_address, amount, &amp;mut store.add_distribution_events);<br/><br/>    // Request to unlock the distribution amount from the stake pool.<br/>    // This won&apos;t become fully unlocked until the stake pool&apos;s lockup expires.<br/>    stake::unlock_with_cap(amount, &amp;staking_contract.owner_cap);<br/><br/>    let pool_address &#61; staking_contract.pool_address;<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(UnlockStake &#123; pool_address, operator, amount, commission_paid &#125;);<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut store.unlock_stake_events,<br/>        UnlockStakeEvent &#123; pool_address, operator, amount, commission_paid &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_unlock_rewards"></a>

## Function `unlock_rewards`

Unlock all accumulated rewards since the last recorded principals.


<pre><code>public entry fun unlock_rewards(staker: &amp;signer, operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock_rewards(staker: &amp;signer, operator: address) acquires Store, BeneficiaryForOperator &#123;<br/>    let staker_address &#61; signer::address_of(staker);<br/>    assert_staking_contract_exists(staker_address, operator);<br/><br/>    // Calculate how much rewards belongs to the staker after commission is paid.<br/>    let (_, accumulated_rewards, unpaid_commission) &#61; staking_contract_amounts(staker_address, operator);<br/>    let staker_rewards &#61; accumulated_rewards &#45; unpaid_commission;<br/>    unlock_stake(staker, operator, staker_rewards);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_switch_operator_with_same_commission"></a>

## Function `switch_operator_with_same_commission`

Allows staker to switch operator without going through the lenghthy process to unstake, without resetting commission.


<pre><code>public entry fun switch_operator_with_same_commission(staker: &amp;signer, old_operator: address, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun switch_operator_with_same_commission(<br/>    staker: &amp;signer,<br/>    old_operator: address,<br/>    new_operator: address,<br/>) acquires Store, BeneficiaryForOperator &#123;<br/>    let staker_address &#61; signer::address_of(staker);<br/>    assert_staking_contract_exists(staker_address, old_operator);<br/><br/>    let commission_percentage &#61; commission_percentage(staker_address, old_operator);<br/>    switch_operator(staker, old_operator, new_operator, commission_percentage);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_switch_operator"></a>

## Function `switch_operator`

Allows staker to switch operator without going through the lenghthy process to unstake.


<pre><code>public entry fun switch_operator(staker: &amp;signer, old_operator: address, new_operator: address, new_commission_percentage: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun switch_operator(<br/>    staker: &amp;signer,<br/>    old_operator: address,<br/>    new_operator: address,<br/>    new_commission_percentage: u64,<br/>) acquires Store, BeneficiaryForOperator &#123;<br/>    let staker_address &#61; signer::address_of(staker);<br/>    assert_staking_contract_exists(staker_address, old_operator);<br/><br/>    // Merging two existing staking contracts is too complex as we&apos;d need to merge two separate stake pools.<br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);<br/>    let staking_contracts &#61; &amp;mut store.staking_contracts;<br/>    assert!(<br/>        !simple_map::contains_key(staking_contracts, &amp;new_operator),<br/>        error::invalid_state(ECANT_MERGE_STAKING_CONTRACTS),<br/>    );<br/><br/>    let (_, staking_contract) &#61; simple_map::remove(staking_contracts, &amp;old_operator);<br/>    // Force distribution of any already inactive stake.<br/>    distribute_internal(staker_address, old_operator, &amp;mut staking_contract, &amp;mut store.distribute_events);<br/><br/>    // For simplicity, we request commission to be paid out first. This avoids having to ensure to staker doesn&apos;t<br/>    // withdraw into the commission portion.<br/>    request_commission_internal(<br/>        old_operator,<br/>        &amp;mut staking_contract,<br/>        &amp;mut store.add_distribution_events,<br/>        &amp;mut store.request_commission_events,<br/>    );<br/><br/>    // Update the staking contract&apos;s commission rate and stake pool&apos;s operator.<br/>    stake::set_operator_with_cap(&amp;staking_contract.owner_cap, new_operator);<br/>    staking_contract.commission_percentage &#61; new_commission_percentage;<br/><br/>    let pool_address &#61; staking_contract.pool_address;<br/>    simple_map::add(staking_contracts, new_operator, staking_contract);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(SwitchOperator &#123; pool_address, old_operator, new_operator &#125;);<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut store.switch_operator_events,<br/>        SwitchOperatorEvent &#123; pool_address, old_operator, new_operator &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Allows an operator to change its beneficiary. Any existing unpaid commission rewards will be paid to the new<br/> beneficiary. To ensures payment to the current beneficiary, one should first call <code>distribute</code> before switching<br/> the beneficiary. An operator can set one beneficiary for staking contract pools, not a separate one for each pool.


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary_for_operator(<br/>    operator: &amp;signer,<br/>    new_beneficiary: address<br/>) acquires BeneficiaryForOperator &#123;<br/>    assert!(features::operator_beneficiary_change_enabled(), std::error::invalid_state(<br/>        EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED<br/>    ));<br/>    // The beneficiay address of an operator is stored under the operator&apos;s address.<br/>    // So, the operator does not need to be validated with respect to a staking pool.<br/>    let operator_addr &#61; signer::address_of(operator);<br/>    let old_beneficiary &#61; beneficiary_for_operator(operator_addr);<br/>    if (exists&lt;BeneficiaryForOperator&gt;(operator_addr)) &#123;<br/>        borrow_global_mut&lt;BeneficiaryForOperator&gt;(operator_addr).beneficiary_for_operator &#61; new_beneficiary;<br/>    &#125; else &#123;<br/>        move_to(operator, BeneficiaryForOperator &#123; beneficiary_for_operator: new_beneficiary &#125;);<br/>    &#125;;<br/><br/>    emit(SetBeneficiaryForOperator &#123;<br/>        operator: operator_addr,<br/>        old_beneficiary,<br/>        new_beneficiary,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_distribute"></a>

## Function `distribute`

Allow anyone to distribute already unlocked funds. This does not affect reward compounding and therefore does<br/> not need to be restricted to just the staker or operator.


<pre><code>public entry fun distribute(staker: address, operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun distribute(staker: address, operator: address) acquires Store, BeneficiaryForOperator &#123;<br/>    assert_staking_contract_exists(staker, operator);<br/>    let store &#61; borrow_global_mut&lt;Store&gt;(staker);<br/>    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);<br/>    distribute_internal(staker, operator, staking_contract, &amp;mut store.distribute_events);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_distribute_internal"></a>

## Function `distribute_internal`

Distribute all unlocked (inactive) funds according to distribution shares.


<pre><code>fun distribute_internal(staker: address, operator: address, staking_contract: &amp;mut staking_contract::StakingContract, distribute_events: &amp;mut event::EventHandle&lt;staking_contract::DistributeEvent&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun distribute_internal(<br/>    staker: address,<br/>    operator: address,<br/>    staking_contract: &amp;mut StakingContract,<br/>    distribute_events: &amp;mut EventHandle&lt;DistributeEvent&gt;,<br/>) acquires BeneficiaryForOperator &#123;<br/>    let pool_address &#61; staking_contract.pool_address;<br/>    let (_, inactive, _, pending_inactive) &#61; stake::get_stake(pool_address);<br/>    let total_potential_withdrawable &#61; inactive &#43; pending_inactive;<br/>    let coins &#61; stake::withdraw_with_cap(&amp;staking_contract.owner_cap, total_potential_withdrawable);<br/>    let distribution_amount &#61; coin::value(&amp;coins);<br/>    if (distribution_amount &#61;&#61; 0) &#123;<br/>        coin::destroy_zero(coins);<br/>        return<br/>    &#125;;<br/><br/>    let distribution_pool &#61; &amp;mut staking_contract.distribution_pool;<br/>    update_distribution_pool(<br/>        distribution_pool, distribution_amount, operator, staking_contract.commission_percentage);<br/><br/>    // Buy all recipients out of the distribution pool.<br/>    while (pool_u64::shareholders_count(distribution_pool) &gt; 0) &#123;<br/>        let recipients &#61; pool_u64::shareholders(distribution_pool);<br/>        let recipient &#61; &#42;vector::borrow(&amp;mut recipients, 0);<br/>        let current_shares &#61; pool_u64::shares(distribution_pool, recipient);<br/>        let amount_to_distribute &#61; pool_u64::redeem_shares(distribution_pool, recipient, current_shares);<br/>        // If the recipient is the operator, send the commission to the beneficiary instead.<br/>        if (recipient &#61;&#61; operator) &#123;<br/>            recipient &#61; beneficiary_for_operator(operator);<br/>        &#125;;<br/>        aptos_account::deposit_coins(recipient, coin::extract(&amp;mut coins, amount_to_distribute));<br/><br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            emit(Distribute &#123; operator, pool_address, recipient, amount: amount_to_distribute &#125;);<br/>        &#125;;<br/>        emit_event(<br/>            distribute_events,<br/>            DistributeEvent &#123; operator, pool_address, recipient, amount: amount_to_distribute &#125;<br/>        );<br/>    &#125;;<br/><br/>    // In case there&apos;s any dust left, send them all to the staker.<br/>    if (coin::value(&amp;coins) &gt; 0) &#123;<br/>        aptos_account::deposit_coins(staker, coins);<br/>        pool_u64::update_total_coins(distribution_pool, 0);<br/>    &#125; else &#123;<br/>        coin::destroy_zero(coins);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_assert_staking_contract_exists"></a>

## Function `assert_staking_contract_exists`

Assert that a staking_contract exists for the staker/operator pair.


<pre><code>fun assert_staking_contract_exists(staker: address, operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_staking_contract_exists(staker: address, operator: address) acquires Store &#123;<br/>    assert!(exists&lt;Store&gt;(staker), error::not_found(ENO_STAKING_CONTRACT_FOUND_FOR_STAKER));<br/>    let staking_contracts &#61; &amp;mut borrow_global_mut&lt;Store&gt;(staker).staking_contracts;<br/>    assert!(<br/>        simple_map::contains_key(staking_contracts, &amp;operator),<br/>        error::not_found(ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_add_distribution"></a>

## Function `add_distribution`

Add a new distribution for <code>recipient</code> and <code>amount</code> to the staking contract&apos;s distributions list.


<pre><code>fun add_distribution(operator: address, staking_contract: &amp;mut staking_contract::StakingContract, recipient: address, coins_amount: u64, add_distribution_events: &amp;mut event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun add_distribution(<br/>    operator: address,<br/>    staking_contract: &amp;mut StakingContract,<br/>    recipient: address,<br/>    coins_amount: u64,<br/>    add_distribution_events: &amp;mut EventHandle&lt;AddDistributionEvent&gt;<br/>) &#123;<br/>    let distribution_pool &#61; &amp;mut staking_contract.distribution_pool;<br/>    let (_, _, _, total_distribution_amount) &#61; stake::get_stake(staking_contract.pool_address);<br/>    update_distribution_pool(<br/>        distribution_pool, total_distribution_amount, operator, staking_contract.commission_percentage);<br/><br/>    pool_u64::buy_in(distribution_pool, recipient, coins_amount);<br/>    let pool_address &#61; staking_contract.pool_address;<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(AddDistribution &#123; operator, pool_address, amount: coins_amount &#125;);<br/>    &#125;;<br/>    emit_event(<br/>        add_distribution_events,<br/>        AddDistributionEvent &#123; operator, pool_address, amount: coins_amount &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_get_staking_contract_amounts_internal"></a>

## Function `get_staking_contract_amounts_internal`

Calculate accumulated rewards and commissions since last update.


<pre><code>fun get_staking_contract_amounts_internal(staking_contract: &amp;staking_contract::StakingContract): (u64, u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_staking_contract_amounts_internal(staking_contract: &amp;StakingContract): (u64, u64, u64) &#123;<br/>    // Pending_inactive is not included in the calculation because pending_inactive can only come from:<br/>    // 1. Outgoing commissions. This means commission has already been extracted.<br/>    // 2. Stake withdrawals from stakers. This also means commission has already been extracted as<br/>    // request_commission_internal is called in unlock_stake<br/>    let (active, _, pending_active, _) &#61; stake::get_stake(staking_contract.pool_address);<br/>    let total_active_stake &#61; active &#43; pending_active;<br/>    let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;<br/>    let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;<br/><br/>    (total_active_stake, accumulated_rewards, commission_amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_create_stake_pool"></a>

## Function `create_stake_pool`



<pre><code>fun create_stake_pool(staker: &amp;signer, operator: address, voter: address, contract_creation_seed: vector&lt;u8&gt;): (signer, account::SignerCapability, stake::OwnerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_stake_pool(<br/>    staker: &amp;signer,<br/>    operator: address,<br/>    voter: address,<br/>    contract_creation_seed: vector&lt;u8&gt;,<br/>): (signer, SignerCapability, OwnerCapability) &#123;<br/>    // Generate a seed that will be used to create the resource account that hosts the staking contract.<br/>    let seed &#61; create_resource_account_seed(<br/>        signer::address_of(staker), operator, contract_creation_seed);<br/><br/>    let (stake_pool_signer, stake_pool_signer_cap) &#61; account::create_resource_account(staker, seed);<br/>    stake::initialize_stake_owner(&amp;stake_pool_signer, 0, operator, voter);<br/><br/>    // Extract owner_cap from the StakePool, so we have control over it in the staking_contracts flow.<br/>    // This is stored as part of the staking_contract. Thus, the staker would not have direct control over it without<br/>    // going through well&#45;defined functions in this module.<br/>    let owner_cap &#61; stake::extract_owner_cap(&amp;stake_pool_signer);<br/><br/>    (stake_pool_signer, stake_pool_signer_cap, owner_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_update_distribution_pool"></a>

## Function `update_distribution_pool`



<pre><code>fun update_distribution_pool(distribution_pool: &amp;mut pool_u64::Pool, updated_total_coins: u64, operator: address, commission_percentage: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_distribution_pool(<br/>    distribution_pool: &amp;mut Pool,<br/>    updated_total_coins: u64,<br/>    operator: address,<br/>    commission_percentage: u64,<br/>) &#123;<br/>    // Short&#45;circuit and do nothing if the pool&apos;s total value has not changed.<br/>    if (pool_u64::total_coins(distribution_pool) &#61;&#61; updated_total_coins) &#123;<br/>        return<br/>    &#125;;<br/><br/>    // Charge all stakeholders (except for the operator themselves) commission on any rewards earnt relatively to the<br/>    // previous value of the distribution pool.<br/>    let shareholders &#61; &amp;pool_u64::shareholders(distribution_pool);<br/>    vector::for_each_ref(shareholders, &#124;shareholder&#124; &#123;<br/>        let shareholder: address &#61; &#42;shareholder;<br/>        if (shareholder !&#61; operator) &#123;<br/>            let shares &#61; pool_u64::shares(distribution_pool, shareholder);<br/>            let previous_worth &#61; pool_u64::balance(distribution_pool, shareholder);<br/>            let current_worth &#61; pool_u64::shares_to_amount_with_total_coins(<br/>                distribution_pool, shares, updated_total_coins);<br/>            let unpaid_commission &#61; (current_worth &#45; previous_worth) &#42; commission_percentage / 100;<br/>            // Transfer shares from current shareholder to the operator as payment.<br/>            // The value of the shares should use the updated pool&apos;s total value.<br/>            let shares_to_transfer &#61; pool_u64::amount_to_shares_with_total_coins(<br/>                distribution_pool, unpaid_commission, updated_total_coins);<br/>            pool_u64::transfer_shares(distribution_pool, shareholder, operator, shares_to_transfer);<br/>        &#125;;<br/>    &#125;);<br/><br/>    pool_u64::update_total_coins(distribution_pool, updated_total_coins);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_create_resource_account_seed"></a>

## Function `create_resource_account_seed`

Create the seed to derive the resource account address.


<pre><code>fun create_resource_account_seed(staker: address, operator: address, contract_creation_seed: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_resource_account_seed(<br/>    staker: address,<br/>    operator: address,<br/>    contract_creation_seed: vector&lt;u8&gt;,<br/>): vector&lt;u8&gt; &#123;<br/>    let seed &#61; bcs::to_bytes(&amp;staker);<br/>    vector::append(&amp;mut seed, bcs::to_bytes(&amp;operator));<br/>    // Include a salt to avoid conflicts with any other modules out there that might also generate<br/>    // deterministic resource accounts for the same staker &#43; operator addresses.<br/>    vector::append(&amp;mut seed, SALT);<br/>    // Add an extra salt given by the staker in case an account with the same address has already been created.<br/>    vector::append(&amp;mut seed, contract_creation_seed);<br/>    seed<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_contract_new_staking_contracts_holder"></a>

## Function `new_staking_contracts_holder`

Create a new staking_contracts resource.


<pre><code>fun new_staking_contracts_holder(staker: &amp;signer): staking_contract::Store<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun new_staking_contracts_holder(staker: &amp;signer): Store &#123;<br/>    Store &#123;<br/>        staking_contracts: simple_map::create&lt;address, StakingContract&gt;(),<br/>        // Events.<br/>        create_staking_contract_events: account::new_event_handle&lt;CreateStakingContractEvent&gt;(staker),<br/>        update_voter_events: account::new_event_handle&lt;UpdateVoterEvent&gt;(staker),<br/>        reset_lockup_events: account::new_event_handle&lt;ResetLockupEvent&gt;(staker),<br/>        add_stake_events: account::new_event_handle&lt;AddStakeEvent&gt;(staker),<br/>        request_commission_events: account::new_event_handle&lt;RequestCommissionEvent&gt;(staker),<br/>        unlock_stake_events: account::new_event_handle&lt;UnlockStakeEvent&gt;(staker),<br/>        switch_operator_events: account::new_event_handle&lt;SwitchOperatorEvent&gt;(staker),<br/>        add_distribution_events: account::new_event_handle&lt;AddDistributionEvent&gt;(staker),<br/>        distribute_events: account::new_event_handle&lt;DistributeEvent&gt;(staker),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The Store structure for the staker exists after the staking contract is created.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The create_staking_contract_with_coins function ensures that the staker account has a Store structure assigned.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;CreateStakingContractWithCoinsAbortsifAndEnsures&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;A staking contract is created and stored in a mapping within the Store resource.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The create_staking_contract_with_coins function adds the newly created StakingContract to the staking_contracts map with the operator as a key of the Store resource, effectively storing the staking contract.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;CreateStakingContractWithCoinsAbortsifAndEnsures&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;Adding stake to the stake pool increases the principal value of the pool, reflecting the additional stake amount.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The add_stake function transfers the specified amount of staked coins from the staker&apos;s account to the stake pool associated with the staking contract. It increases the principal value of the staking contract by the added stake amount.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;add_stake&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The staker may update the voter of a staking contract, enabling them to modify the assigned voter address and ensure it accurately reflects their desired choice.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The update_voter function ensures that the voter address in a staking contract may be updated by the staker, resulting in the modification of the delegated voter address in the associated stake pool to reflect the new address provided.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;update_voter&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;Only the owner of the stake pool has the permission to reset the lockup period of the pool.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The reset_lockup function ensures that only the staker who owns the stake pool has the authority to reset the lockup period of the pool.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5&quot;&gt;reset_lockup&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;Unlocked funds are correctly distributed to recipients based on their distribution shares, taking into account the associated commission percentage.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The distribution process, implemented in the distribute_internal function, accurately allocates unlocked funds to their intended recipients based on their distribution shares. It guarantees that each recipient receives the correct amount of funds, considering the commission percentage associated with the staking contract.&lt;/td&gt;<br/>&lt;td&gt;Audited that the correct amount of unlocked funds is distributed according to distribution shares.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;The stake pool ensures that the commission is correctly requested and paid out from the old operator&apos;s stake pool before allowing the switch to the new operator.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The switch_operator function initiates the commission payout from the stake pool associated with the old operator, ensuring a smooth transition. Paying out the commission before the switch guarantees that the staker receives the appropriate commission amount and maintains the integrity of the staking process.&lt;/td&gt;<br/>&lt;td&gt;Audited that the commission is paid to the old operator.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;8&lt;/td&gt;<br/>&lt;td&gt;Stakers can withdraw their funds from the staking contract, ensuring the unlocked amount becomes available for withdrawal after the lockup period.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The unlock_stake function ensures that the requested amount is properly unlocked from the stake pool, considering the lockup period and that the funds become available for withdrawal when the lockup expires.&lt;/td&gt;<br/>&lt;td&gt;Audited that funds are unlocked properly.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_stake_pool_address"></a>

### Function `stake_pool_address`


<pre><code>&#35;[view]<br/>public fun stake_pool_address(staker: address, operator: address): address<br/></code></pre>




<pre><code>include ContractExistsAbortsIf;<br/>let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;<br/>ensures result &#61;&#61; simple_map::spec_get(staking_contracts, operator).pool_address;<br/></code></pre>



<a id="@Specification_1_last_recorded_principal"></a>

### Function `last_recorded_principal`


<pre><code>&#35;[view]<br/>public fun last_recorded_principal(staker: address, operator: address): u64<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>include ContractExistsAbortsIf;<br/>let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;<br/>ensures result &#61;&#61; simple_map::spec_get(staking_contracts, operator).principal;<br/></code></pre>



<a id="@Specification_1_commission_percentage"></a>

### Function `commission_percentage`


<pre><code>&#35;[view]<br/>public fun commission_percentage(staker: address, operator: address): u64<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>include ContractExistsAbortsIf;<br/>let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;<br/>ensures result &#61;&#61; simple_map::spec_get(staking_contracts, operator).commission_percentage;<br/></code></pre>



<a id="@Specification_1_staking_contract_amounts"></a>

### Function `staking_contract_amounts`


<pre><code>&#35;[view]<br/>public fun staking_contract_amounts(staker: address, operator: address): (u64, u64, u64)<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>requires staking_contract.commission_percentage &gt;&#61; 0 &amp;&amp; staking_contract.commission_percentage &lt;&#61; 100;<br/>let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;<br/>let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);<br/>include ContractExistsAbortsIf;<br/>include GetStakingContractAmountsAbortsIf &#123; staking_contract &#125;;<br/>let pool_address &#61; staking_contract.pool_address;<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);<br/>let active &#61; coin::value(stake_pool.active);<br/>let pending_active &#61; coin::value(stake_pool.pending_active);<br/>let total_active_stake &#61; active &#43; pending_active;<br/>let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;<br/>ensures result_1 &#61;&#61; total_active_stake;<br/>ensures result_2 &#61;&#61; accumulated_rewards;<br/></code></pre>



<a id="@Specification_1_pending_distribution_counts"></a>

### Function `pending_distribution_counts`


<pre><code>&#35;[view]<br/>public fun pending_distribution_counts(staker: address, operator: address): u64<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>include ContractExistsAbortsIf;<br/>let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;<br/>let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);<br/>let shareholders_count &#61; len(staking_contract.distribution_pool.shareholders);<br/>ensures result &#61;&#61; shareholders_count;<br/></code></pre>



<a id="@Specification_1_staking_contract_exists"></a>

### Function `staking_contract_exists`


<pre><code>&#35;[view]<br/>public fun staking_contract_exists(staker: address, operator: address): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; spec_staking_contract_exists(staker, operator);<br/></code></pre>




<a id="0x1_staking_contract_spec_staking_contract_exists"></a>


<pre><code>fun spec_staking_contract_exists(staker: address, operator: address): bool &#123;<br/>   if (!exists&lt;Store&gt;(staker)) &#123;<br/>       false<br/>   &#125; else &#123;<br/>       let store &#61; global&lt;Store&gt;(staker);<br/>       simple_map::spec_contains_key(store.staking_contracts, operator)<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_beneficiary_for_operator"></a>

### Function `beneficiary_for_operator`


<pre><code>&#35;[view]<br/>public fun beneficiary_for_operator(operator: address): address<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_create_staking_contract"></a>

### Function `create_staking_contract`


<pre><code>public entry fun create_staking_contract(staker: &amp;signer, operator: address, voter: address, amount: u64, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;)<br/></code></pre>


Account is not frozen and sufficient to withdraw.


<pre><code>pragma aborts_if_is_partial;<br/>pragma verify_duration_estimate &#61; 120;<br/>include PreconditionsInCreateContract;<br/>include WithdrawAbortsIf&lt;AptosCoin&gt; &#123; account: staker &#125;;<br/>include CreateStakingContractWithCoinsAbortsIfAndEnsures;<br/></code></pre>



<a id="@Specification_1_create_staking_contract_with_coins"></a>

### Function `create_staking_contract_with_coins`


<pre><code>public fun create_staking_contract_with_coins(staker: &amp;signer, operator: address, voter: address, coins: coin::Coin&lt;aptos_coin::AptosCoin&gt;, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;): address<br/></code></pre>


The amount should be at least the min_stake_required, so the stake pool will be eligible to join the validator set.<br/> Initialize Store resource if this is the first time the staker has delegated to anyone.<br/> Cannot create the staking contract if it already exists.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>pragma aborts_if_is_partial;<br/>include PreconditionsInCreateContract;<br/>let amount &#61; coins.value;<br/>include CreateStakingContractWithCoinsAbortsIfAndEnsures &#123; amount &#125;;<br/></code></pre>



<a id="@Specification_1_add_stake"></a>

### Function `add_stake`


<pre><code>public entry fun add_stake(staker: &amp;signer, operator: address, amount: u64)<br/></code></pre>


Account is not frozen and sufficient to withdraw.<br/> Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify_duration_estimate &#61; 600;<br/>include stake::ResourceRequirement;<br/>aborts_if reconfiguration_state::spec_is_in_progress();<br/>let staker_address &#61; signer::address_of(staker);<br/>include ContractExistsAbortsIf &#123; staker: staker_address &#125;;<br/>let store &#61; global&lt;Store&gt;(staker_address);<br/>let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);<br/>include WithdrawAbortsIf&lt;AptosCoin&gt; &#123; account: staker &#125;;<br/>let balance &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(staker_address).coin.value;<br/>let post post_coin &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(staker_address).coin.value;<br/>ensures post_coin &#61;&#61; balance &#45; amount;<br/>let owner_cap &#61; staking_contract.owner_cap;<br/>include stake::AddStakeWithCapAbortsIfAndEnsures &#123; owner_cap &#125;;<br/>let post post_store &#61; global&lt;Store&gt;(staker_address);<br/>let post post_staking_contract &#61; simple_map::spec_get(post_store.staking_contracts, operator);<br/>aborts_if staking_contract.principal &#43; amount &gt; MAX_U64;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
ensures post_staking_contract.principal &#61;&#61; staking_contract.principal &#43; amount;<br/></code></pre>



<a id="@Specification_1_update_voter"></a>

### Function `update_voter`


<pre><code>public entry fun update_voter(staker: &amp;signer, operator: address, new_voter: address)<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>let staker_address &#61; signer::address_of(staker);<br/>include UpdateVoterSchema &#123; staker: staker_address &#125;;<br/>let post store &#61; global&lt;Store&gt;(staker_address);<br/>let post staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);<br/>let post pool_address &#61; staking_contract.owner_cap.pool_address;<br/>let post new_delegated_voter &#61; global&lt;stake::StakePool&gt;(pool_address).delegated_voter;<br/>ensures new_delegated_voter &#61;&#61; new_voter;<br/></code></pre>



<a id="@Specification_1_reset_lockup"></a>

### Function `reset_lockup`


<pre><code>public entry fun reset_lockup(staker: &amp;signer, operator: address)<br/></code></pre>


Staking_contract exists the stacker/operator pair.<br/> Only active validator can update locked_until_secs.


<pre><code>let staker_address &#61; signer::address_of(staker);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
include ContractExistsAbortsIf &#123; staker: staker_address &#125;;<br/>include IncreaseLockupWithCapAbortsIf &#123; staker: staker_address &#125;;<br/></code></pre>



<a id="@Specification_1_update_commision"></a>

### Function `update_commision`


<pre><code>public entry fun update_commision(staker: &amp;signer, operator: address, new_commission_percentage: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let staker_address &#61; signer::address_of(staker);<br/>aborts_if new_commission_percentage &gt; 100;<br/>include ContractExistsAbortsIf &#123; staker: staker_address &#125;;<br/></code></pre>



<a id="@Specification_1_request_commission"></a>

### Function `request_commission`


<pre><code>public entry fun request_commission(account: &amp;signer, staker: address, operator: address)<br/></code></pre>


Only staker or operator can call this.


<pre><code>pragma verify &#61; false;<br/>let account_addr &#61; signer::address_of(account);<br/>include ContractExistsAbortsIf &#123; staker &#125;;<br/>aborts_if account_addr !&#61; staker &amp;&amp; account_addr !&#61; operator;<br/></code></pre>



<a id="@Specification_1_request_commission_internal"></a>

### Function `request_commission_internal`


<pre><code>fun request_commission_internal(operator: address, staking_contract: &amp;mut staking_contract::StakingContract, add_distribution_events: &amp;mut event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;, request_commission_events: &amp;mut event::EventHandle&lt;staking_contract::RequestCommissionEvent&gt;): u64<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include GetStakingContractAmountsAbortsIf;<br/></code></pre>



<a id="@Specification_1_unlock_stake"></a>

### Function `unlock_stake`


<pre><code>public entry fun unlock_stake(staker: &amp;signer, operator: address, amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>requires amount &gt; 0;<br/>let staker_address &#61; signer::address_of(staker);<br/>include ContractExistsAbortsIf &#123; staker: staker_address &#125;;<br/></code></pre>



<a id="@Specification_1_unlock_rewards"></a>

### Function `unlock_rewards`


<pre><code>public entry fun unlock_rewards(staker: &amp;signer, operator: address)<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify &#61; false;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt;:
requires staking_contract.commission_percentage &gt;&#61; 0 &amp;&amp; staking_contract.commission_percentage &lt;&#61; 100;<br/>let staker_address &#61; signer::address_of(staker);<br/>let staking_contracts &#61; global&lt;Store&gt;(staker_address).staking_contracts;<br/>let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);<br/>include ContractExistsAbortsIf &#123; staker: staker_address &#125;;<br/></code></pre>



<a id="@Specification_1_switch_operator_with_same_commission"></a>

### Function `switch_operator_with_same_commission`


<pre><code>public entry fun switch_operator_with_same_commission(staker: &amp;signer, old_operator: address, new_operator: address)<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>pragma aborts_if_is_partial;<br/>let staker_address &#61; signer::address_of(staker);<br/>include ContractExistsAbortsIf &#123; staker: staker_address, operator: old_operator &#125;;<br/></code></pre>



<a id="@Specification_1_switch_operator"></a>

### Function `switch_operator`


<pre><code>public entry fun switch_operator(staker: &amp;signer, old_operator: address, new_operator: address, new_commission_percentage: u64)<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify &#61; false;<br/>let staker_address &#61; signer::address_of(staker);<br/>include ContractExistsAbortsIf &#123; staker: staker_address, operator: old_operator &#125;;<br/>let store &#61; global&lt;Store&gt;(staker_address);<br/>let staking_contracts &#61; store.staking_contracts;<br/>aborts_if simple_map::spec_contains_key(staking_contracts, new_operator);<br/></code></pre>



<a id="@Specification_1_set_beneficiary_for_operator"></a>

### Function `set_beneficiary_for_operator`


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_distribute"></a>

### Function `distribute`


<pre><code>public entry fun distribute(staker: address, operator: address)<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>pragma aborts_if_is_partial;<br/>include ContractExistsAbortsIf;<br/></code></pre>



<a id="@Specification_1_distribute_internal"></a>

### Function `distribute_internal`


<pre><code>fun distribute_internal(staker: address, operator: address, staking_contract: &amp;mut staking_contract::StakingContract, distribute_events: &amp;mut event::EventHandle&lt;staking_contract::DistributeEvent&gt;)<br/></code></pre>


The StakePool exists under the pool_address of StakingContract.<br/> The value of inactive and pending_inactive in the stake_pool is up to MAX_U64.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>pragma aborts_if_is_partial;<br/>let pool_address &#61; staking_contract.pool_address;<br/>let stake_pool &#61; borrow_global&lt;stake::StakePool&gt;(pool_address);<br/>aborts_if !exists&lt;stake::StakePool&gt;(pool_address);<br/>aborts_if stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &gt; MAX_U64;<br/>aborts_if !exists&lt;stake::StakePool&gt;(staking_contract.owner_cap.pool_address);<br/></code></pre>



<a id="@Specification_1_assert_staking_contract_exists"></a>

### Function `assert_staking_contract_exists`


<pre><code>fun assert_staking_contract_exists(staker: address, operator: address)<br/></code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>include ContractExistsAbortsIf;<br/></code></pre>



<a id="@Specification_1_add_distribution"></a>

### Function `add_distribution`


<pre><code>fun add_distribution(operator: address, staking_contract: &amp;mut staking_contract::StakingContract, recipient: address, coins_amount: u64, add_distribution_events: &amp;mut event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_get_staking_contract_amounts_internal"></a>

### Function `get_staking_contract_amounts_internal`


<pre><code>fun get_staking_contract_amounts_internal(staking_contract: &amp;staking_contract::StakingContract): (u64, u64, u64)<br/></code></pre>


The StakePool exists under the pool_address of StakingContract.


<pre><code>include GetStakingContractAmountsAbortsIf;<br/>let pool_address &#61; staking_contract.pool_address;<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);<br/>let active &#61; coin::value(stake_pool.active);<br/>let pending_active &#61; coin::value(stake_pool.pending_active);<br/>let total_active_stake &#61; active &#43; pending_active;<br/>let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;<br/>let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;<br/>ensures result_1 &#61;&#61; total_active_stake;<br/>ensures result_2 &#61;&#61; accumulated_rewards;<br/>ensures result_3 &#61;&#61; commission_amount;<br/></code></pre>



<a id="@Specification_1_create_stake_pool"></a>

### Function `create_stake_pool`


<pre><code>fun create_stake_pool(staker: &amp;signer, operator: address, voter: address, contract_creation_seed: vector&lt;u8&gt;): (signer, account::SignerCapability, stake::OwnerCapability)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>include stake::ResourceRequirement;<br/>let staker_address &#61; signer::address_of(staker);<br/>let seed_0 &#61; bcs::to_bytes(staker_address);<br/>let seed_1 &#61; concat(concat(concat(seed_0, bcs::to_bytes(operator)), SALT), contract_creation_seed);<br/>let resource_addr &#61; account::spec_create_resource_address(staker_address, seed_1);<br/>include CreateStakePoolAbortsIf &#123; resource_addr &#125;;<br/>ensures exists&lt;account::Account&gt;(resource_addr);<br/>let post post_account &#61; global&lt;account::Account&gt;(resource_addr);<br/>ensures post_account.authentication_key &#61;&#61; account::ZERO_AUTH_KEY;<br/>ensures post_account.signer_capability_offer.for &#61;&#61; std::option::spec_some(resource_addr);<br/>ensures exists&lt;stake::StakePool&gt;(resource_addr);<br/>let post post_owner_cap &#61; global&lt;stake::OwnerCapability&gt;(resource_addr);<br/>let post post_pool_address &#61; post_owner_cap.pool_address;<br/>let post post_stake_pool &#61; global&lt;stake::StakePool&gt;(post_pool_address);<br/>let post post_operator &#61; post_stake_pool.operator_address;<br/>let post post_delegated_voter &#61; post_stake_pool.delegated_voter;<br/>ensures resource_addr !&#61; operator &#61;&#61;&gt; post_operator &#61;&#61; operator;<br/>ensures resource_addr !&#61; voter &#61;&#61;&gt; post_delegated_voter &#61;&#61; voter;<br/>ensures signer::address_of(result_1) &#61;&#61; resource_addr;<br/>ensures result_2 &#61;&#61; SignerCapability &#123; account: resource_addr &#125;;<br/>ensures result_3 &#61;&#61; OwnerCapability &#123; pool_address: resource_addr &#125;;<br/></code></pre>



<a id="@Specification_1_update_distribution_pool"></a>

### Function `update_distribution_pool`


<pre><code>fun update_distribution_pool(distribution_pool: &amp;mut pool_u64::Pool, updated_total_coins: u64, operator: address, commission_percentage: u64)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/></code></pre>



<a id="@Specification_1_new_staking_contracts_holder"></a>

### Function `new_staking_contracts_holder`


<pre><code>fun new_staking_contracts_holder(staker: &amp;signer): staking_contract::Store<br/></code></pre>


The Account exists under the staker.<br/> The guid_creation_num of the ccount resource is up to MAX_U64.


<pre><code>include NewStakingContractsHolderAbortsIf;<br/></code></pre>




<a id="0x1_staking_contract_NewStakingContractsHolderAbortsIf"></a>


<pre><code>schema NewStakingContractsHolderAbortsIf &#123;<br/>staker: signer;<br/>let addr &#61; signer::address_of(staker);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;account::Account&gt;(addr);<br/>aborts_if account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/>&#125;<br/></code></pre>


The Store exists under the staker.<br/> a staking_contract exists for the staker/operator pair.


<a id="0x1_staking_contract_ContractExistsAbortsIf"></a>


<pre><code>schema ContractExistsAbortsIf &#123;<br/>staker: address;<br/>operator: address;<br/>aborts_if !exists&lt;Store&gt;(staker);<br/>let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;<br/>aborts_if !simple_map::spec_contains_key(staking_contracts, operator);<br/>&#125;<br/></code></pre>




<a id="0x1_staking_contract_UpdateVoterSchema"></a>


<pre><code>schema UpdateVoterSchema &#123;<br/>staker: address;<br/>operator: address;<br/>let store &#61; global&lt;Store&gt;(staker);<br/>let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);<br/>let pool_address &#61; staking_contract.pool_address;<br/>aborts_if !exists&lt;stake::StakePool&gt;(pool_address);<br/>aborts_if !exists&lt;stake::StakePool&gt;(staking_contract.owner_cap.pool_address);<br/>include ContractExistsAbortsIf;<br/>&#125;<br/></code></pre>




<a id="0x1_staking_contract_WithdrawAbortsIf"></a>


<pre><code>schema WithdrawAbortsIf&lt;CoinType&gt; &#123;<br/>account: signer;<br/>amount: u64;<br/>let account_addr &#61; signer::address_of(account);<br/>let coin_store &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>let balance &#61; coin_store.coin.value;<br/>aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if coin_store.frozen;<br/>aborts_if balance &lt; amount;<br/>&#125;<br/></code></pre>




<a id="0x1_staking_contract_GetStakingContractAmountsAbortsIf"></a>


<pre><code>schema GetStakingContractAmountsAbortsIf &#123;<br/>staking_contract: StakingContract;<br/>let pool_address &#61; staking_contract.pool_address;<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);<br/>let active &#61; coin::value(stake_pool.active);<br/>let pending_active &#61; coin::value(stake_pool.pending_active);<br/>let total_active_stake &#61; active &#43; pending_active;<br/>let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;<br/>aborts_if !exists&lt;stake::StakePool&gt;(pool_address);<br/>aborts_if active &#43; pending_active &gt; MAX_U64;<br/>aborts_if total_active_stake &lt; staking_contract.principal;<br/>aborts_if accumulated_rewards &#42; staking_contract.commission_percentage &gt; MAX_U64;<br/>&#125;<br/></code></pre>




<a id="0x1_staking_contract_IncreaseLockupWithCapAbortsIf"></a>


<pre><code>schema IncreaseLockupWithCapAbortsIf &#123;<br/>staker: address;<br/>operator: address;<br/>let store &#61; global&lt;Store&gt;(staker);<br/>let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);<br/>let pool_address &#61; staking_contract.owner_cap.pool_address;<br/>aborts_if !stake::stake_pool_exists(pool_address);<br/>aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>let config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);<br/>let old_locked_until_secs &#61; stake_pool.locked_until_secs;<br/>let seconds &#61; global&lt;timestamp::CurrentTimeMicroseconds&gt;(<br/>    @aptos_framework<br/>).microseconds / timestamp::MICRO_CONVERSION_FACTOR;<br/>let new_locked_until_secs &#61; seconds &#43; config.recurring_lockup_duration_secs;<br/>aborts_if seconds &#43; config.recurring_lockup_duration_secs &gt; MAX_U64;<br/>aborts_if old_locked_until_secs &gt; new_locked_until_secs &#124;&#124; old_locked_until_secs &#61;&#61; new_locked_until_secs;<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>let post post_store &#61; global&lt;Store&gt;(staker);<br/>let post post_staking_contract &#61; simple_map::spec_get(post_store.staking_contracts, operator);<br/>let post post_stake_pool &#61; global&lt;stake::StakePool&gt;(post_staking_contract.owner_cap.pool_address);<br/>ensures post_stake_pool.locked_until_secs &#61;&#61; new_locked_until_secs;<br/>&#125;<br/></code></pre>




<a id="0x1_staking_contract_CreateStakingContractWithCoinsAbortsIfAndEnsures"></a>


<pre><code>schema CreateStakingContractWithCoinsAbortsIfAndEnsures &#123;<br/>staker: signer;<br/>operator: address;<br/>voter: address;<br/>amount: u64;<br/>commission_percentage: u64;<br/>contract_creation_seed: vector&lt;u8&gt;;<br/>aborts_if commission_percentage &gt; 100;<br/>aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>let config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/>let min_stake_required &#61; config.minimum_stake;<br/>aborts_if amount &lt; min_stake_required;<br/>let staker_address &#61; signer::address_of(staker);<br/>let account &#61; global&lt;account::Account&gt;(staker_address);<br/>aborts_if !exists&lt;Store&gt;(staker_address) &amp;&amp; !exists&lt;account::Account&gt;(staker_address);<br/>aborts_if !exists&lt;Store&gt;(staker_address) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
    ensures exists&lt;Store&gt;(staker_address);<br/>let store &#61; global&lt;Store&gt;(staker_address);<br/>let staking_contracts &#61; store.staking_contracts;<br/>let owner_cap &#61; simple_map::spec_get(store.staking_contracts, operator).owner_cap;<br/>let post post_store &#61; global&lt;Store&gt;(staker_address);<br/>let post post_staking_contracts &#61; post_store.staking_contracts;<br/>&#125;<br/></code></pre>




<a id="0x1_staking_contract_PreconditionsInCreateContract"></a>


<pre><code>schema PreconditionsInCreateContract &#123;<br/>requires exists&lt;stake::ValidatorPerformance&gt;(@aptos_framework);<br/>requires exists&lt;stake::ValidatorSet&gt;(@aptos_framework);<br/>requires exists&lt;staking_config::StakingRewardsConfig&gt;(<br/>    @aptos_framework<br/>) &#124;&#124; !std::features::spec_periodical_reward_rate_decrease_enabled();<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>requires exists&lt;aptos_framework::timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>requires exists&lt;stake::AptosCoinCapabilities&gt;(@aptos_framework);<br/>&#125;<br/></code></pre>




<a id="0x1_staking_contract_CreateStakePoolAbortsIf"></a>


<pre><code>schema CreateStakePoolAbortsIf &#123;<br/>resource_addr: address;<br/>operator: address;<br/>voter: address;<br/>contract_creation_seed: vector&lt;u8&gt;;<br/>let acc &#61; global&lt;account::Account&gt;(resource_addr);<br/>aborts_if exists&lt;account::Account&gt;(resource_addr) &amp;&amp; (len(<br/>    acc.signer_capability_offer.for.vec<br/>) !&#61; 0 &#124;&#124; acc.sequence_number !&#61; 0);<br/>aborts_if !exists&lt;account::Account&gt;(resource_addr) &amp;&amp; len(bcs::to_bytes(resource_addr)) !&#61; 32;<br/>aborts_if len(account::ZERO_AUTH_KEY) !&#61; 32;<br/>aborts_if exists&lt;stake::ValidatorConfig&gt;(resource_addr);<br/>let allowed &#61; global&lt;stake::AllowedValidators&gt;(@aptos_framework);<br/>aborts_if exists&lt;stake::AllowedValidators&gt;(@aptos_framework) &amp;&amp; !contains(allowed.accounts, resource_addr);<br/>aborts_if exists&lt;stake::StakePool&gt;(resource_addr);<br/>aborts_if exists&lt;stake::OwnerCapability&gt;(resource_addr);<br/>aborts_if exists&lt;account::Account&gt;(<br/>    resource_addr<br/>) &amp;&amp; acc.guid_creation_num &#43; 12 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
