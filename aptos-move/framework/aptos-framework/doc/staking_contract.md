
<a id="0x1_staking_contract"></a>

# Module `0x1::staking_contract`

Allow stakers and operators to enter a staking contract with reward sharing.
The main accounting logic in a staking contract consists of 2 parts:
1. Tracks how much commission needs to be paid out to the operator. This is tracked with an increasing principal
amount that's updated every time the operator requests commission, the staker withdraws funds, or the staker
switches operators.
2. Distributions of funds to operators (commissions) and stakers (stake withdrawals) use the shares model provided
by the pool_u64 to track shares that increase in price as the stake pool accumulates rewards.

Example flow:
1. A staker creates a staking contract with an operator by calling create_staking_contract() with 100 coins of
initial stake and commission = 10%. This means the operator will receive 10% of any accumulated rewards. A new stake
pool will be created and hosted in a separate account that's controlled by the staking contract.
2. The operator sets up a validator node and, once ready, joins the validator set by calling stake::join_validator_set
3. After some time, the stake pool gains rewards and now has 150 coins.
4. Operator can now call request_commission. 10% of (150 - 100) = 5 coins will be unlocked from the stake pool. The
staker's principal is now updated from 100 to 145 (150 coins - 5 coins of commission). The pending distribution pool
has 5 coins total and the operator owns all 5 shares of it.
5. Some more time has passed. The pool now has 50 more coins in rewards and a total balance of 195. The operator
calls request_commission again. Since the previous 5 coins have now become withdrawable, it'll be deposited into the
operator's account first. Their new commission will be 10% of (195 coins - 145 principal) = 5 coins. Principal is
updated to be 190 (195 - 5). Pending distribution pool has 5 coins and operator owns all 5 shares.
6. Staker calls unlock_stake to unlock 50 coins of stake, which gets added to the pending distribution pool. Based
on shares math, staker will be owning 50 shares and operator still owns 5 shares of the 55-coin pending distribution
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


<pre><code>use 0x1::account;
use 0x1::aptos_account;
use 0x1::aptos_coin;
use 0x1::bcs;
use 0x1::coin;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::pool_u64;
use 0x1::signer;
use 0x1::simple_map;
use 0x1::stake;
use 0x1::staking_config;
use 0x1::vector;
</code></pre>



<a id="0x1_staking_contract_StakingGroupContainer"></a>

## Struct `StakingGroupContainer`



<pre><code>&#35;[resource_group(&#35;[scope &#61; module_])]
struct StakingGroupContainer
</code></pre>



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



<pre><code>struct StakingContract has store
</code></pre>



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



<pre><code>struct Store has key
</code></pre>



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

<a id="0x1_staking_contract_UpdateCommissionEvent"></a>

## Struct `UpdateCommissionEvent`



<pre><code>struct UpdateCommissionEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct UpdateCommission has drop, store
</code></pre>



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



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::staking_contract::StakingGroupContainer])]
struct StakingGroupUpdateCommissionEvent has key
</code></pre>



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



<pre><code>&#35;[event]
struct CreateStakingContract has drop, store
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



<pre><code>&#35;[event]
struct UpdateVoter has drop, store
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



<pre><code>&#35;[event]
struct ResetLockup has drop, store
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
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddStake"></a>

## Struct `AddStake`



<pre><code>&#35;[event]
struct AddStake has drop, store
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



<pre><code>&#35;[event]
struct RequestCommission has drop, store
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



<pre><code>&#35;[event]
struct UnlockStake has drop, store
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



<pre><code>&#35;[event]
struct SwitchOperator has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct AddDistribution has drop, store
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



<pre><code>&#35;[event]
struct Distribute has drop, store
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

<a id="0x1_staking_contract_CreateStakingContractEvent"></a>

## Struct `CreateStakingContractEvent`



<pre><code>struct CreateStakingContractEvent has drop, store
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



<pre><code>struct UpdateVoterEvent has drop, store
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



<pre><code>struct ResetLockupEvent has drop, store
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
<code>pool_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_contract_AddStakeEvent"></a>

## Struct `AddStakeEvent`



<pre><code>struct AddStakeEvent has drop, store
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



<pre><code>struct RequestCommissionEvent has drop, store
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



<pre><code>struct UnlockStakeEvent has drop, store
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



<pre><code>struct SwitchOperatorEvent has drop, store
</code></pre>



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



<pre><code>struct AddDistributionEvent has drop, store
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



<pre><code>struct DistributeEvent has drop, store
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


<pre><code>const EINVALID_COMMISSION_PERCENTAGE: u64 &#61; 2;
</code></pre>



<a id="0x1_staking_contract_EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED"></a>

Chaning beneficiaries for operators is not supported.


<pre><code>const EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED: u64 &#61; 9;
</code></pre>



<a id="0x1_staking_contract_ECANT_MERGE_STAKING_CONTRACTS"></a>

Staking contracts can't be merged.


<pre><code>const ECANT_MERGE_STAKING_CONTRACTS: u64 &#61; 5;
</code></pre>



<a id="0x1_staking_contract_EINSUFFICIENT_ACTIVE_STAKE_TO_WITHDRAW"></a>

Not enough active stake to withdraw. Some stake might still pending and will be active in the next epoch.


<pre><code>const EINSUFFICIENT_ACTIVE_STAKE_TO_WITHDRAW: u64 &#61; 7;
</code></pre>



<a id="0x1_staking_contract_EINSUFFICIENT_STAKE_AMOUNT"></a>

Store amount must be at least the min stake required for a stake pool to join the validator set.


<pre><code>const EINSUFFICIENT_STAKE_AMOUNT: u64 &#61; 1;
</code></pre>



<a id="0x1_staking_contract_ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY"></a>

Caller must be either the staker, operator, or beneficiary.


<pre><code>const ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY: u64 &#61; 8;
</code></pre>



<a id="0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR"></a>

No staking contract between the staker and operator found.


<pre><code>const ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR: u64 &#61; 4;
</code></pre>



<a id="0x1_staking_contract_ENO_STAKING_CONTRACT_FOUND_FOR_STAKER"></a>

Staker has no staking contracts.


<pre><code>const ENO_STAKING_CONTRACT_FOUND_FOR_STAKER: u64 &#61; 3;
</code></pre>



<a id="0x1_staking_contract_ESTAKING_CONTRACT_ALREADY_EXISTS"></a>

The staking contract already exists and cannot be re-created.


<pre><code>const ESTAKING_CONTRACT_ALREADY_EXISTS: u64 &#61; 6;
</code></pre>



<a id="0x1_staking_contract_MAXIMUM_PENDING_DISTRIBUTIONS"></a>

Maximum number of distributions a stake pool can support.


<pre><code>const MAXIMUM_PENDING_DISTRIBUTIONS: u64 &#61; 20;
</code></pre>



<a id="0x1_staking_contract_SALT"></a>



<pre><code>const SALT: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 115, 116, 97, 107, 105, 110, 103, 95, 99, 111, 110, 116, 114, 97, 99, 116];
</code></pre>



<a id="0x1_staking_contract_stake_pool_address"></a>

## Function `stake_pool_address`

Return the address of the underlying stake pool for the staking contract between the provided staker and
operator.

This errors out the staking contract with the provided staker and operator doesn't exist.


<pre><code>&#35;[view]
public fun stake_pool_address(staker: address, operator: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun stake_pool_address(staker: address, operator: address): address acquires Store &#123;
    assert_staking_contract_exists(staker, operator);
    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;
    simple_map::borrow(staking_contracts, &amp;operator).pool_address
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_last_recorded_principal"></a>

## Function `last_recorded_principal`

Return the last recorded principal (the amount that 100% belongs to the staker with commission already paid for)
for staking contract between the provided staker and operator.

This errors out the staking contract with the provided staker and operator doesn't exist.


<pre><code>&#35;[view]
public fun last_recorded_principal(staker: address, operator: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun last_recorded_principal(staker: address, operator: address): u64 acquires Store &#123;
    assert_staking_contract_exists(staker, operator);
    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;
    simple_map::borrow(staking_contracts, &amp;operator).principal
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_commission_percentage"></a>

## Function `commission_percentage`

Return percentage of accumulated rewards that will be paid to the operator as commission for staking contract
between the provided staker and operator.

This errors out the staking contract with the provided staker and operator doesn't exist.


<pre><code>&#35;[view]
public fun commission_percentage(staker: address, operator: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commission_percentage(staker: address, operator: address): u64 acquires Store &#123;
    assert_staking_contract_exists(staker, operator);
    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;
    simple_map::borrow(staking_contracts, &amp;operator).commission_percentage
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_staking_contract_amounts"></a>

## Function `staking_contract_amounts`

Return a tuple of three numbers:
1. The total active stake in the underlying stake pool
2. The total accumulated rewards that haven't had commission paid out
3. The commission amount owned from those accumulated rewards.

This errors out the staking contract with the provided staker and operator doesn't exist.


<pre><code>&#35;[view]
public fun staking_contract_amounts(staker: address, operator: address): (u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun staking_contract_amounts(staker: address, operator: address): (u64, u64, u64) acquires Store &#123;
    assert_staking_contract_exists(staker, operator);
    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;
    let staking_contract &#61; simple_map::borrow(staking_contracts, &amp;operator);
    get_staking_contract_amounts_internal(staking_contract)
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_pending_distribution_counts"></a>

## Function `pending_distribution_counts`

Return the number of pending distributions (e.g. commission, withdrawals from stakers).

This errors out the staking contract with the provided staker and operator doesn't exist.


<pre><code>&#35;[view]
public fun pending_distribution_counts(staker: address, operator: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pending_distribution_counts(staker: address, operator: address): u64 acquires Store &#123;
    assert_staking_contract_exists(staker, operator);
    let staking_contracts &#61; &amp;borrow_global&lt;Store&gt;(staker).staking_contracts;
    pool_u64::shareholders_count(&amp;simple_map::borrow(staking_contracts, &amp;operator).distribution_pool)
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_staking_contract_exists"></a>

## Function `staking_contract_exists`

Return true if the staking contract between the provided staker and operator exists.


<pre><code>&#35;[view]
public fun staking_contract_exists(staker: address, operator: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun staking_contract_exists(staker: address, operator: address): bool acquires Store &#123;
    if (!exists&lt;Store&gt;(staker)) &#123;
        return false
    &#125;;

    let store &#61; borrow_global&lt;Store&gt;(staker);
    simple_map::contains_key(&amp;store.staking_contracts, &amp;operator)
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_beneficiary_for_operator"></a>

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

<a id="0x1_staking_contract_get_expected_stake_pool_address"></a>

## Function `get_expected_stake_pool_address`

Return the address of the stake pool to be created with the provided staker, operator and seed.


<pre><code>&#35;[view]
public fun get_expected_stake_pool_address(staker: address, operator: address, contract_creation_seed: vector&lt;u8&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_expected_stake_pool_address(
    staker: address,
    operator: address,
    contract_creation_seed: vector&lt;u8&gt;,
): address &#123;
    let seed &#61; create_resource_account_seed(staker, operator, contract_creation_seed);
    account::create_resource_address(&amp;staker, seed)
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_create_staking_contract"></a>

## Function `create_staking_contract`

Staker can call this function to create a simple staking contract with a specified operator.


<pre><code>public entry fun create_staking_contract(staker: &amp;signer, operator: address, voter: address, amount: u64, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_staking_contract(
    staker: &amp;signer,
    operator: address,
    voter: address,
    amount: u64,
    commission_percentage: u64,
    // Optional seed used when creating the staking contract account.
    contract_creation_seed: vector&lt;u8&gt;,
) acquires Store &#123;
    let staked_coins &#61; coin::withdraw&lt;AptosCoin&gt;(staker, amount);
    create_staking_contract_with_coins(
        staker, operator, voter, staked_coins, commission_percentage, contract_creation_seed);
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_create_staking_contract_with_coins"></a>

## Function `create_staking_contract_with_coins`

Staker can call this function to create a simple staking contract with a specified operator.


<pre><code>public fun create_staking_contract_with_coins(staker: &amp;signer, operator: address, voter: address, coins: coin::Coin&lt;aptos_coin::AptosCoin&gt;, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_staking_contract_with_coins(
    staker: &amp;signer,
    operator: address,
    voter: address,
    coins: Coin&lt;AptosCoin&gt;,
    commission_percentage: u64,
    // Optional seed used when creating the staking contract account.
    contract_creation_seed: vector&lt;u8&gt;,
): address acquires Store &#123;
    assert!(
        commission_percentage &gt;&#61; 0 &amp;&amp; commission_percentage &lt;&#61; 100,
        error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE),
    );
    // The amount should be at least the min_stake_required, so the stake pool will be eligible to join the
    // validator set.
    let (min_stake_required, _) &#61; staking_config::get_required_stake(&amp;staking_config::get());
    let principal &#61; coin::value(&amp;coins);
    assert!(principal &gt;&#61; min_stake_required, error::invalid_argument(EINSUFFICIENT_STAKE_AMOUNT));

    // Initialize Store resource if this is the first time the staker has delegated to anyone.
    let staker_address &#61; signer::address_of(staker);
    if (!exists&lt;Store&gt;(staker_address)) &#123;
        move_to(staker, new_staking_contracts_holder(staker));
    &#125;;

    // Cannot create the staking contract if it already exists.
    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);
    let staking_contracts &#61; &amp;mut store.staking_contracts;
    assert!(
        !simple_map::contains_key(staking_contracts, &amp;operator),
        error::already_exists(ESTAKING_CONTRACT_ALREADY_EXISTS)
    );

    // Initialize the stake pool in a new resource account. This allows the same staker to contract with multiple
    // different operators.
    let (stake_pool_signer, stake_pool_signer_cap, owner_cap) &#61;
        create_stake_pool(staker, operator, voter, contract_creation_seed);

    // Add the stake to the stake pool.
    stake::add_stake_with_cap(&amp;owner_cap, coins);

    // Create the contract record.
    let pool_address &#61; signer::address_of(&amp;stake_pool_signer);
    simple_map::add(staking_contracts, operator, StakingContract &#123;
        principal,
        pool_address,
        owner_cap,
        commission_percentage,
        // Make sure we don&apos;t have too many pending recipients in the distribution pool.
        // Otherwise, a griefing attack is possible where the staker can keep switching operators and create too
        // many pending distributions. This can lead to out&#45;of&#45;gas failure whenever distribute() is called.
        distribution_pool: pool_u64::create(MAXIMUM_PENDING_DISTRIBUTIONS),
        signer_cap: stake_pool_signer_cap,
    &#125;);

    if (std::features::module_event_migration_enabled()) &#123;
        emit(CreateStakingContract &#123; operator, voter, pool_address, principal, commission_percentage &#125;);
    &#125;;
    emit_event(
        &amp;mut store.create_staking_contract_events,
        CreateStakingContractEvent &#123; operator, voter, pool_address, principal, commission_percentage &#125;,
    );
    pool_address
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_add_stake"></a>

## Function `add_stake`

Add more stake to an existing staking contract.


<pre><code>public entry fun add_stake(staker: &amp;signer, operator: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_stake(staker: &amp;signer, operator: address, amount: u64) acquires Store &#123;
    let staker_address &#61; signer::address_of(staker);
    assert_staking_contract_exists(staker_address, operator);

    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);
    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);

    // Add the stake to the stake pool.
    let staked_coins &#61; coin::withdraw&lt;AptosCoin&gt;(staker, amount);
    stake::add_stake_with_cap(&amp;staking_contract.owner_cap, staked_coins);

    staking_contract.principal &#61; staking_contract.principal &#43; amount;
    let pool_address &#61; staking_contract.pool_address;
    if (std::features::module_event_migration_enabled()) &#123;
        emit(AddStake &#123; operator, pool_address, amount &#125;);
    &#125;;
    emit_event(
        &amp;mut store.add_stake_events,
        AddStakeEvent &#123; operator, pool_address, amount &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_update_voter"></a>

## Function `update_voter`

Convenient function to allow the staker to update the voter address in a staking contract they made.


<pre><code>public entry fun update_voter(staker: &amp;signer, operator: address, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_voter(staker: &amp;signer, operator: address, new_voter: address) acquires Store &#123;
    let staker_address &#61; signer::address_of(staker);
    assert_staking_contract_exists(staker_address, operator);

    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);
    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);
    let pool_address &#61; staking_contract.pool_address;
    let old_voter &#61; stake::get_delegated_voter(pool_address);
    stake::set_delegated_voter_with_cap(&amp;staking_contract.owner_cap, new_voter);

    if (std::features::module_event_migration_enabled()) &#123;
        emit(UpdateVoter &#123; operator, pool_address, old_voter, new_voter &#125;);
    &#125;;
    emit_event(
        &amp;mut store.update_voter_events,
        UpdateVoterEvent &#123; operator, pool_address, old_voter, new_voter &#125;,
    );

&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_reset_lockup"></a>

## Function `reset_lockup`

Convenient function to allow the staker to reset their stake pool's lockup period to start now.


<pre><code>public entry fun reset_lockup(staker: &amp;signer, operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reset_lockup(staker: &amp;signer, operator: address) acquires Store &#123;
    let staker_address &#61; signer::address_of(staker);
    assert_staking_contract_exists(staker_address, operator);

    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);
    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);
    let pool_address &#61; staking_contract.pool_address;
    stake::increase_lockup_with_cap(&amp;staking_contract.owner_cap);

    if (std::features::module_event_migration_enabled()) &#123;
        emit(ResetLockup &#123; operator, pool_address &#125;);
    &#125;;
    emit_event(&amp;mut store.reset_lockup_events, ResetLockupEvent &#123; operator, pool_address &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_update_commision"></a>

## Function `update_commision`

Convenience function to allow a staker to update the commission percentage paid to the operator.
TODO: fix the typo in function name. commision -> commission


<pre><code>public entry fun update_commision(staker: &amp;signer, operator: address, new_commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_commision(
    staker: &amp;signer,
    operator: address,
    new_commission_percentage: u64
) acquires Store, BeneficiaryForOperator, StakingGroupUpdateCommissionEvent &#123;
    assert!(
        new_commission_percentage &gt;&#61; 0 &amp;&amp; new_commission_percentage &lt;&#61; 100,
        error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE),
    );

    let staker_address &#61; signer::address_of(staker);
    assert!(exists&lt;Store&gt;(staker_address), error::not_found(ENO_STAKING_CONTRACT_FOUND_FOR_STAKER));

    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);
    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);
    distribute_internal(staker_address, operator, staking_contract, &amp;mut store.distribute_events);
    request_commission_internal(
        operator,
        staking_contract,
        &amp;mut store.add_distribution_events,
        &amp;mut store.request_commission_events,
    );
    let old_commission_percentage &#61; staking_contract.commission_percentage;
    staking_contract.commission_percentage &#61; new_commission_percentage;
    if (!exists&lt;StakingGroupUpdateCommissionEvent&gt;(staker_address)) &#123;
        move_to(
            staker,
            StakingGroupUpdateCommissionEvent &#123;
                update_commission_events: account::new_event_handle&lt;UpdateCommissionEvent&gt;(
                    staker
                )
            &#125;
        )
    &#125;;
    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            UpdateCommission &#123; staker: staker_address, operator, old_commission_percentage, new_commission_percentage &#125;
        );
    &#125;;
    emit_event(
        &amp;mut borrow_global_mut&lt;StakingGroupUpdateCommissionEvent&gt;(staker_address).update_commission_events,
        UpdateCommissionEvent &#123; staker: staker_address, operator, old_commission_percentage, new_commission_percentage &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_request_commission"></a>

## Function `request_commission`

Unlock commission amount from the stake pool. Operator needs to wait for the amount to become withdrawable
at the end of the stake pool's lockup period before they can actually can withdraw_commission.

Only staker, operator or beneficiary can call this.


<pre><code>public entry fun request_commission(account: &amp;signer, staker: address, operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun request_commission(
    account: &amp;signer,
    staker: address,
    operator: address
) acquires Store, BeneficiaryForOperator &#123;
    let account_addr &#61; signer::address_of(account);
    assert!(
        account_addr &#61;&#61; staker &#124;&#124; account_addr &#61;&#61; operator &#124;&#124; account_addr &#61;&#61; beneficiary_for_operator(operator),
        error::unauthenticated(ENOT_STAKER_OR_OPERATOR_OR_BENEFICIARY)
    );
    assert_staking_contract_exists(staker, operator);

    let store &#61; borrow_global_mut&lt;Store&gt;(staker);
    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);
    // Short&#45;circuit if zero commission.
    if (staking_contract.commission_percentage &#61;&#61; 0) &#123;
        return
    &#125;;

    // Force distribution of any already inactive stake.
    distribute_internal(staker, operator, staking_contract, &amp;mut store.distribute_events);

    request_commission_internal(
        operator,
        staking_contract,
        &amp;mut store.add_distribution_events,
        &amp;mut store.request_commission_events,
    );
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_request_commission_internal"></a>

## Function `request_commission_internal`



<pre><code>fun request_commission_internal(operator: address, staking_contract: &amp;mut staking_contract::StakingContract, add_distribution_events: &amp;mut event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;, request_commission_events: &amp;mut event::EventHandle&lt;staking_contract::RequestCommissionEvent&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun request_commission_internal(
    operator: address,
    staking_contract: &amp;mut StakingContract,
    add_distribution_events: &amp;mut EventHandle&lt;AddDistributionEvent&gt;,
    request_commission_events: &amp;mut EventHandle&lt;RequestCommissionEvent&gt;,
): u64 &#123;
    // Unlock just the commission portion from the stake pool.
    let (total_active_stake, accumulated_rewards, commission_amount) &#61;
        get_staking_contract_amounts_internal(staking_contract);
    staking_contract.principal &#61; total_active_stake &#45; commission_amount;

    // Short&#45;circuit if there&apos;s no commission to pay.
    if (commission_amount &#61;&#61; 0) &#123;
        return 0
    &#125;;

    // Add a distribution for the operator.
    add_distribution(operator, staking_contract, operator, commission_amount, add_distribution_events);

    // Request to unlock the commission from the stake pool.
    // This won&apos;t become fully unlocked until the stake pool&apos;s lockup expires.
    stake::unlock_with_cap(commission_amount, &amp;staking_contract.owner_cap);

    let pool_address &#61; staking_contract.pool_address;
    if (std::features::module_event_migration_enabled()) &#123;
        emit(RequestCommission &#123; operator, pool_address, accumulated_rewards, commission_amount &#125;);
    &#125;;
    emit_event(
        request_commission_events,
        RequestCommissionEvent &#123; operator, pool_address, accumulated_rewards, commission_amount &#125;,
    );

    commission_amount
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_unlock_stake"></a>

## Function `unlock_stake`

Staker can call this to request withdrawal of part or all of their staking_contract.
This also triggers paying commission to the operator for accounting simplicity.


<pre><code>public entry fun unlock_stake(staker: &amp;signer, operator: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock_stake(
    staker: &amp;signer,
    operator: address,
    amount: u64
) acquires Store, BeneficiaryForOperator &#123;
    // Short&#45;circuit if amount is 0.
    if (amount &#61;&#61; 0) return;

    let staker_address &#61; signer::address_of(staker);
    assert_staking_contract_exists(staker_address, operator);

    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);
    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);

    // Force distribution of any already inactive stake.
    distribute_internal(staker_address, operator, staking_contract, &amp;mut store.distribute_events);

    // For simplicity, we request commission to be paid out first. This avoids having to ensure to staker doesn&apos;t
    // withdraw into the commission portion.
    let commission_paid &#61; request_commission_internal(
        operator,
        staking_contract,
        &amp;mut store.add_distribution_events,
        &amp;mut store.request_commission_events,
    );

    // If there&apos;s less active stake remaining than the amount requested (potentially due to commission),
    // only withdraw up to the active amount.
    let (active, _, _, _) &#61; stake::get_stake(staking_contract.pool_address);
    if (active &lt; amount) &#123;
        amount &#61; active;
    &#125;;
    staking_contract.principal &#61; staking_contract.principal &#45; amount;

    // Record a distribution for the staker.
    add_distribution(operator, staking_contract, staker_address, amount, &amp;mut store.add_distribution_events);

    // Request to unlock the distribution amount from the stake pool.
    // This won&apos;t become fully unlocked until the stake pool&apos;s lockup expires.
    stake::unlock_with_cap(amount, &amp;staking_contract.owner_cap);

    let pool_address &#61; staking_contract.pool_address;
    if (std::features::module_event_migration_enabled()) &#123;
        emit(UnlockStake &#123; pool_address, operator, amount, commission_paid &#125;);
    &#125;;
    emit_event(
        &amp;mut store.unlock_stake_events,
        UnlockStakeEvent &#123; pool_address, operator, amount, commission_paid &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_unlock_rewards"></a>

## Function `unlock_rewards`

Unlock all accumulated rewards since the last recorded principals.


<pre><code>public entry fun unlock_rewards(staker: &amp;signer, operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock_rewards(staker: &amp;signer, operator: address) acquires Store, BeneficiaryForOperator &#123;
    let staker_address &#61; signer::address_of(staker);
    assert_staking_contract_exists(staker_address, operator);

    // Calculate how much rewards belongs to the staker after commission is paid.
    let (_, accumulated_rewards, unpaid_commission) &#61; staking_contract_amounts(staker_address, operator);
    let staker_rewards &#61; accumulated_rewards &#45; unpaid_commission;
    unlock_stake(staker, operator, staker_rewards);
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_switch_operator_with_same_commission"></a>

## Function `switch_operator_with_same_commission`

Allows staker to switch operator without going through the lenghthy process to unstake, without resetting commission.


<pre><code>public entry fun switch_operator_with_same_commission(staker: &amp;signer, old_operator: address, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun switch_operator_with_same_commission(
    staker: &amp;signer,
    old_operator: address,
    new_operator: address,
) acquires Store, BeneficiaryForOperator &#123;
    let staker_address &#61; signer::address_of(staker);
    assert_staking_contract_exists(staker_address, old_operator);

    let commission_percentage &#61; commission_percentage(staker_address, old_operator);
    switch_operator(staker, old_operator, new_operator, commission_percentage);
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_switch_operator"></a>

## Function `switch_operator`

Allows staker to switch operator without going through the lenghthy process to unstake.


<pre><code>public entry fun switch_operator(staker: &amp;signer, old_operator: address, new_operator: address, new_commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun switch_operator(
    staker: &amp;signer,
    old_operator: address,
    new_operator: address,
    new_commission_percentage: u64,
) acquires Store, BeneficiaryForOperator &#123;
    let staker_address &#61; signer::address_of(staker);
    assert_staking_contract_exists(staker_address, old_operator);

    // Merging two existing staking contracts is too complex as we&apos;d need to merge two separate stake pools.
    let store &#61; borrow_global_mut&lt;Store&gt;(staker_address);
    let staking_contracts &#61; &amp;mut store.staking_contracts;
    assert!(
        !simple_map::contains_key(staking_contracts, &amp;new_operator),
        error::invalid_state(ECANT_MERGE_STAKING_CONTRACTS),
    );

    let (_, staking_contract) &#61; simple_map::remove(staking_contracts, &amp;old_operator);
    // Force distribution of any already inactive stake.
    distribute_internal(staker_address, old_operator, &amp;mut staking_contract, &amp;mut store.distribute_events);

    // For simplicity, we request commission to be paid out first. This avoids having to ensure to staker doesn&apos;t
    // withdraw into the commission portion.
    request_commission_internal(
        old_operator,
        &amp;mut staking_contract,
        &amp;mut store.add_distribution_events,
        &amp;mut store.request_commission_events,
    );

    // Update the staking contract&apos;s commission rate and stake pool&apos;s operator.
    stake::set_operator_with_cap(&amp;staking_contract.owner_cap, new_operator);
    staking_contract.commission_percentage &#61; new_commission_percentage;

    let pool_address &#61; staking_contract.pool_address;
    simple_map::add(staking_contracts, new_operator, staking_contract);
    if (std::features::module_event_migration_enabled()) &#123;
        emit(SwitchOperator &#123; pool_address, old_operator, new_operator &#125;);
    &#125;;
    emit_event(
        &amp;mut store.switch_operator_events,
        SwitchOperatorEvent &#123; pool_address, old_operator, new_operator &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Allows an operator to change its beneficiary. Any existing unpaid commission rewards will be paid to the new
beneficiary. To ensures payment to the current beneficiary, one should first call <code>distribute</code> before switching
the beneficiary. An operator can set one beneficiary for staking contract pools, not a separate one for each pool.


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

<a id="0x1_staking_contract_distribute"></a>

## Function `distribute`

Allow anyone to distribute already unlocked funds. This does not affect reward compounding and therefore does
not need to be restricted to just the staker or operator.


<pre><code>public entry fun distribute(staker: address, operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun distribute(staker: address, operator: address) acquires Store, BeneficiaryForOperator &#123;
    assert_staking_contract_exists(staker, operator);
    let store &#61; borrow_global_mut&lt;Store&gt;(staker);
    let staking_contract &#61; simple_map::borrow_mut(&amp;mut store.staking_contracts, &amp;operator);
    distribute_internal(staker, operator, staking_contract, &amp;mut store.distribute_events);
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_distribute_internal"></a>

## Function `distribute_internal`

Distribute all unlocked (inactive) funds according to distribution shares.


<pre><code>fun distribute_internal(staker: address, operator: address, staking_contract: &amp;mut staking_contract::StakingContract, distribute_events: &amp;mut event::EventHandle&lt;staking_contract::DistributeEvent&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun distribute_internal(
    staker: address,
    operator: address,
    staking_contract: &amp;mut StakingContract,
    distribute_events: &amp;mut EventHandle&lt;DistributeEvent&gt;,
) acquires BeneficiaryForOperator &#123;
    let pool_address &#61; staking_contract.pool_address;
    let (_, inactive, _, pending_inactive) &#61; stake::get_stake(pool_address);
    let total_potential_withdrawable &#61; inactive &#43; pending_inactive;
    let coins &#61; stake::withdraw_with_cap(&amp;staking_contract.owner_cap, total_potential_withdrawable);
    let distribution_amount &#61; coin::value(&amp;coins);
    if (distribution_amount &#61;&#61; 0) &#123;
        coin::destroy_zero(coins);
        return
    &#125;;

    let distribution_pool &#61; &amp;mut staking_contract.distribution_pool;
    update_distribution_pool(
        distribution_pool, distribution_amount, operator, staking_contract.commission_percentage);

    // Buy all recipients out of the distribution pool.
    while (pool_u64::shareholders_count(distribution_pool) &gt; 0) &#123;
        let recipients &#61; pool_u64::shareholders(distribution_pool);
        let recipient &#61; &#42;vector::borrow(&amp;mut recipients, 0);
        let current_shares &#61; pool_u64::shares(distribution_pool, recipient);
        let amount_to_distribute &#61; pool_u64::redeem_shares(distribution_pool, recipient, current_shares);
        // If the recipient is the operator, send the commission to the beneficiary instead.
        if (recipient &#61;&#61; operator) &#123;
            recipient &#61; beneficiary_for_operator(operator);
        &#125;;
        aptos_account::deposit_coins(recipient, coin::extract(&amp;mut coins, amount_to_distribute));

        if (std::features::module_event_migration_enabled()) &#123;
            emit(Distribute &#123; operator, pool_address, recipient, amount: amount_to_distribute &#125;);
        &#125;;
        emit_event(
            distribute_events,
            DistributeEvent &#123; operator, pool_address, recipient, amount: amount_to_distribute &#125;
        );
    &#125;;

    // In case there&apos;s any dust left, send them all to the staker.
    if (coin::value(&amp;coins) &gt; 0) &#123;
        aptos_account::deposit_coins(staker, coins);
        pool_u64::update_total_coins(distribution_pool, 0);
    &#125; else &#123;
        coin::destroy_zero(coins);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_assert_staking_contract_exists"></a>

## Function `assert_staking_contract_exists`

Assert that a staking_contract exists for the staker/operator pair.


<pre><code>fun assert_staking_contract_exists(staker: address, operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_staking_contract_exists(staker: address, operator: address) acquires Store &#123;
    assert!(exists&lt;Store&gt;(staker), error::not_found(ENO_STAKING_CONTRACT_FOUND_FOR_STAKER));
    let staking_contracts &#61; &amp;mut borrow_global_mut&lt;Store&gt;(staker).staking_contracts;
    assert!(
        simple_map::contains_key(staking_contracts, &amp;operator),
        error::not_found(ENO_STAKING_CONTRACT_FOUND_FOR_OPERATOR),
    );
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_add_distribution"></a>

## Function `add_distribution`

Add a new distribution for <code>recipient</code> and <code>amount</code> to the staking contract's distributions list.


<pre><code>fun add_distribution(operator: address, staking_contract: &amp;mut staking_contract::StakingContract, recipient: address, coins_amount: u64, add_distribution_events: &amp;mut event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun add_distribution(
    operator: address,
    staking_contract: &amp;mut StakingContract,
    recipient: address,
    coins_amount: u64,
    add_distribution_events: &amp;mut EventHandle&lt;AddDistributionEvent&gt;
) &#123;
    let distribution_pool &#61; &amp;mut staking_contract.distribution_pool;
    let (_, _, _, total_distribution_amount) &#61; stake::get_stake(staking_contract.pool_address);
    update_distribution_pool(
        distribution_pool, total_distribution_amount, operator, staking_contract.commission_percentage);

    pool_u64::buy_in(distribution_pool, recipient, coins_amount);
    let pool_address &#61; staking_contract.pool_address;
    if (std::features::module_event_migration_enabled()) &#123;
        emit(AddDistribution &#123; operator, pool_address, amount: coins_amount &#125;);
    &#125;;
    emit_event(
        add_distribution_events,
        AddDistributionEvent &#123; operator, pool_address, amount: coins_amount &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_get_staking_contract_amounts_internal"></a>

## Function `get_staking_contract_amounts_internal`

Calculate accumulated rewards and commissions since last update.


<pre><code>fun get_staking_contract_amounts_internal(staking_contract: &amp;staking_contract::StakingContract): (u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_staking_contract_amounts_internal(staking_contract: &amp;StakingContract): (u64, u64, u64) &#123;
    // Pending_inactive is not included in the calculation because pending_inactive can only come from:
    // 1. Outgoing commissions. This means commission has already been extracted.
    // 2. Stake withdrawals from stakers. This also means commission has already been extracted as
    // request_commission_internal is called in unlock_stake
    let (active, _, pending_active, _) &#61; stake::get_stake(staking_contract.pool_address);
    let total_active_stake &#61; active &#43; pending_active;
    let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;
    let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;

    (total_active_stake, accumulated_rewards, commission_amount)
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_create_stake_pool"></a>

## Function `create_stake_pool`



<pre><code>fun create_stake_pool(staker: &amp;signer, operator: address, voter: address, contract_creation_seed: vector&lt;u8&gt;): (signer, account::SignerCapability, stake::OwnerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_stake_pool(
    staker: &amp;signer,
    operator: address,
    voter: address,
    contract_creation_seed: vector&lt;u8&gt;,
): (signer, SignerCapability, OwnerCapability) &#123;
    // Generate a seed that will be used to create the resource account that hosts the staking contract.
    let seed &#61; create_resource_account_seed(
        signer::address_of(staker), operator, contract_creation_seed);

    let (stake_pool_signer, stake_pool_signer_cap) &#61; account::create_resource_account(staker, seed);
    stake::initialize_stake_owner(&amp;stake_pool_signer, 0, operator, voter);

    // Extract owner_cap from the StakePool, so we have control over it in the staking_contracts flow.
    // This is stored as part of the staking_contract. Thus, the staker would not have direct control over it without
    // going through well&#45;defined functions in this module.
    let owner_cap &#61; stake::extract_owner_cap(&amp;stake_pool_signer);

    (stake_pool_signer, stake_pool_signer_cap, owner_cap)
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_update_distribution_pool"></a>

## Function `update_distribution_pool`



<pre><code>fun update_distribution_pool(distribution_pool: &amp;mut pool_u64::Pool, updated_total_coins: u64, operator: address, commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_distribution_pool(
    distribution_pool: &amp;mut Pool,
    updated_total_coins: u64,
    operator: address,
    commission_percentage: u64,
) &#123;
    // Short&#45;circuit and do nothing if the pool&apos;s total value has not changed.
    if (pool_u64::total_coins(distribution_pool) &#61;&#61; updated_total_coins) &#123;
        return
    &#125;;

    // Charge all stakeholders (except for the operator themselves) commission on any rewards earnt relatively to the
    // previous value of the distribution pool.
    let shareholders &#61; &amp;pool_u64::shareholders(distribution_pool);
    vector::for_each_ref(shareholders, &#124;shareholder&#124; &#123;
        let shareholder: address &#61; &#42;shareholder;
        if (shareholder !&#61; operator) &#123;
            let shares &#61; pool_u64::shares(distribution_pool, shareholder);
            let previous_worth &#61; pool_u64::balance(distribution_pool, shareholder);
            let current_worth &#61; pool_u64::shares_to_amount_with_total_coins(
                distribution_pool, shares, updated_total_coins);
            let unpaid_commission &#61; (current_worth &#45; previous_worth) &#42; commission_percentage / 100;
            // Transfer shares from current shareholder to the operator as payment.
            // The value of the shares should use the updated pool&apos;s total value.
            let shares_to_transfer &#61; pool_u64::amount_to_shares_with_total_coins(
                distribution_pool, unpaid_commission, updated_total_coins);
            pool_u64::transfer_shares(distribution_pool, shareholder, operator, shares_to_transfer);
        &#125;;
    &#125;);

    pool_u64::update_total_coins(distribution_pool, updated_total_coins);
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_create_resource_account_seed"></a>

## Function `create_resource_account_seed`

Create the seed to derive the resource account address.


<pre><code>fun create_resource_account_seed(staker: address, operator: address, contract_creation_seed: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_resource_account_seed(
    staker: address,
    operator: address,
    contract_creation_seed: vector&lt;u8&gt;,
): vector&lt;u8&gt; &#123;
    let seed &#61; bcs::to_bytes(&amp;staker);
    vector::append(&amp;mut seed, bcs::to_bytes(&amp;operator));
    // Include a salt to avoid conflicts with any other modules out there that might also generate
    // deterministic resource accounts for the same staker &#43; operator addresses.
    vector::append(&amp;mut seed, SALT);
    // Add an extra salt given by the staker in case an account with the same address has already been created.
    vector::append(&amp;mut seed, contract_creation_seed);
    seed
&#125;
</code></pre>



</details>

<a id="0x1_staking_contract_new_staking_contracts_holder"></a>

## Function `new_staking_contracts_holder`

Create a new staking_contracts resource.


<pre><code>fun new_staking_contracts_holder(staker: &amp;signer): staking_contract::Store
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun new_staking_contracts_holder(staker: &amp;signer): Store &#123;
    Store &#123;
        staking_contracts: simple_map::create&lt;address, StakingContract&gt;(),
        // Events.
        create_staking_contract_events: account::new_event_handle&lt;CreateStakingContractEvent&gt;(staker),
        update_voter_events: account::new_event_handle&lt;UpdateVoterEvent&gt;(staker),
        reset_lockup_events: account::new_event_handle&lt;ResetLockupEvent&gt;(staker),
        add_stake_events: account::new_event_handle&lt;AddStakeEvent&gt;(staker),
        request_commission_events: account::new_event_handle&lt;RequestCommissionEvent&gt;(staker),
        unlock_stake_events: account::new_event_handle&lt;UnlockStakeEvent&gt;(staker),
        switch_operator_events: account::new_event_handle&lt;SwitchOperatorEvent&gt;(staker),
        add_distribution_events: account::new_event_handle&lt;AddDistributionEvent&gt;(staker),
        distribute_events: account::new_event_handle&lt;DistributeEvent&gt;(staker),
    &#125;
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
<td>The add_stake function transfers the specified amount of staked coins from the staker's account to the stake pool associated with the staking contract. It increases the principal value of the staking contract by the added stake amount.</td>
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
<td>The stake pool ensures that the commission is correctly requested and paid out from the old operator's stake pool before allowing the switch to the new operator.</td>
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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_stake_pool_address"></a>

### Function `stake_pool_address`


<pre><code>&#35;[view]
public fun stake_pool_address(staker: address, operator: address): address
</code></pre>




<pre><code>include ContractExistsAbortsIf;
let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;
ensures result &#61;&#61; simple_map::spec_get(staking_contracts, operator).pool_address;
</code></pre>



<a id="@Specification_1_last_recorded_principal"></a>

### Function `last_recorded_principal`


<pre><code>&#35;[view]
public fun last_recorded_principal(staker: address, operator: address): u64
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>include ContractExistsAbortsIf;
let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;
ensures result &#61;&#61; simple_map::spec_get(staking_contracts, operator).principal;
</code></pre>



<a id="@Specification_1_commission_percentage"></a>

### Function `commission_percentage`


<pre><code>&#35;[view]
public fun commission_percentage(staker: address, operator: address): u64
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>include ContractExistsAbortsIf;
let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;
ensures result &#61;&#61; simple_map::spec_get(staking_contracts, operator).commission_percentage;
</code></pre>



<a id="@Specification_1_staking_contract_amounts"></a>

### Function `staking_contract_amounts`


<pre><code>&#35;[view]
public fun staking_contract_amounts(staker: address, operator: address): (u64, u64, u64)
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify_duration_estimate &#61; 120;
requires staking_contract.commission_percentage &gt;&#61; 0 &amp;&amp; staking_contract.commission_percentage &lt;&#61; 100;
let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;
let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);
include ContractExistsAbortsIf;
include GetStakingContractAmountsAbortsIf &#123; staking_contract &#125;;
let pool_address &#61; staking_contract.pool_address;
let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);
let active &#61; coin::value(stake_pool.active);
let pending_active &#61; coin::value(stake_pool.pending_active);
let total_active_stake &#61; active &#43; pending_active;
let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;
ensures result_1 &#61;&#61; total_active_stake;
ensures result_2 &#61;&#61; accumulated_rewards;
</code></pre>



<a id="@Specification_1_pending_distribution_counts"></a>

### Function `pending_distribution_counts`


<pre><code>&#35;[view]
public fun pending_distribution_counts(staker: address, operator: address): u64
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>include ContractExistsAbortsIf;
let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;
let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);
let shareholders_count &#61; len(staking_contract.distribution_pool.shareholders);
ensures result &#61;&#61; shareholders_count;
</code></pre>



<a id="@Specification_1_staking_contract_exists"></a>

### Function `staking_contract_exists`


<pre><code>&#35;[view]
public fun staking_contract_exists(staker: address, operator: address): bool
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_staking_contract_exists(staker, operator);
</code></pre>




<a id="0x1_staking_contract_spec_staking_contract_exists"></a>


<pre><code>fun spec_staking_contract_exists(staker: address, operator: address): bool &#123;
   if (!exists&lt;Store&gt;(staker)) &#123;
       false
   &#125; else &#123;
       let store &#61; global&lt;Store&gt;(staker);
       simple_map::spec_contains_key(store.staking_contracts, operator)
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_beneficiary_for_operator"></a>

### Function `beneficiary_for_operator`


<pre><code>&#35;[view]
public fun beneficiary_for_operator(operator: address): address
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_create_staking_contract"></a>

### Function `create_staking_contract`


<pre><code>public entry fun create_staking_contract(staker: &amp;signer, operator: address, voter: address, amount: u64, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;)
</code></pre>


Account is not frozen and sufficient to withdraw.


<pre><code>pragma aborts_if_is_partial;
pragma verify_duration_estimate &#61; 120;
include PreconditionsInCreateContract;
include WithdrawAbortsIf&lt;AptosCoin&gt; &#123; account: staker &#125;;
include CreateStakingContractWithCoinsAbortsIfAndEnsures;
</code></pre>



<a id="@Specification_1_create_staking_contract_with_coins"></a>

### Function `create_staking_contract_with_coins`


<pre><code>public fun create_staking_contract_with_coins(staker: &amp;signer, operator: address, voter: address, coins: coin::Coin&lt;aptos_coin::AptosCoin&gt;, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;): address
</code></pre>


The amount should be at least the min_stake_required, so the stake pool will be eligible to join the validator set.
Initialize Store resource if this is the first time the staker has delegated to anyone.
Cannot create the staking contract if it already exists.


<pre><code>pragma verify_duration_estimate &#61; 120;
pragma aborts_if_is_partial;
include PreconditionsInCreateContract;
let amount &#61; coins.value;
include CreateStakingContractWithCoinsAbortsIfAndEnsures &#123; amount &#125;;
</code></pre>



<a id="@Specification_1_add_stake"></a>

### Function `add_stake`


<pre><code>public entry fun add_stake(staker: &amp;signer, operator: address, amount: u64)
</code></pre>


Account is not frozen and sufficient to withdraw.
Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify_duration_estimate &#61; 600;
include stake::ResourceRequirement;
aborts_if reconfiguration_state::spec_is_in_progress();
let staker_address &#61; signer::address_of(staker);
include ContractExistsAbortsIf &#123; staker: staker_address &#125;;
let store &#61; global&lt;Store&gt;(staker_address);
let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);
include WithdrawAbortsIf&lt;AptosCoin&gt; &#123; account: staker &#125;;
let balance &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(staker_address).coin.value;
let post post_coin &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(staker_address).coin.value;
ensures post_coin &#61;&#61; balance &#45; amount;
let owner_cap &#61; staking_contract.owner_cap;
include stake::AddStakeWithCapAbortsIfAndEnsures &#123; owner_cap &#125;;
let post post_store &#61; global&lt;Store&gt;(staker_address);
let post post_staking_contract &#61; simple_map::spec_get(post_store.staking_contracts, operator);
aborts_if staking_contract.principal &#43; amount &gt; MAX_U64;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures post_staking_contract.principal &#61;&#61; staking_contract.principal &#43; amount;
</code></pre>



<a id="@Specification_1_update_voter"></a>

### Function `update_voter`


<pre><code>public entry fun update_voter(staker: &amp;signer, operator: address, new_voter: address)
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>let staker_address &#61; signer::address_of(staker);
include UpdateVoterSchema &#123; staker: staker_address &#125;;
let post store &#61; global&lt;Store&gt;(staker_address);
let post staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);
let post pool_address &#61; staking_contract.owner_cap.pool_address;
let post new_delegated_voter &#61; global&lt;stake::StakePool&gt;(pool_address).delegated_voter;
ensures new_delegated_voter &#61;&#61; new_voter;
</code></pre>



<a id="@Specification_1_reset_lockup"></a>

### Function `reset_lockup`


<pre><code>public entry fun reset_lockup(staker: &amp;signer, operator: address)
</code></pre>


Staking_contract exists the stacker/operator pair.
Only active validator can update locked_until_secs.


<pre><code>let staker_address &#61; signer::address_of(staker);
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
include ContractExistsAbortsIf &#123; staker: staker_address &#125;;
include IncreaseLockupWithCapAbortsIf &#123; staker: staker_address &#125;;
</code></pre>



<a id="@Specification_1_update_commision"></a>

### Function `update_commision`


<pre><code>public entry fun update_commision(staker: &amp;signer, operator: address, new_commission_percentage: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let staker_address &#61; signer::address_of(staker);
aborts_if new_commission_percentage &gt; 100;
include ContractExistsAbortsIf &#123; staker: staker_address &#125;;
</code></pre>



<a id="@Specification_1_request_commission"></a>

### Function `request_commission`


<pre><code>public entry fun request_commission(account: &amp;signer, staker: address, operator: address)
</code></pre>


Only staker or operator can call this.


<pre><code>pragma verify &#61; false;
let account_addr &#61; signer::address_of(account);
include ContractExistsAbortsIf &#123; staker &#125;;
aborts_if account_addr !&#61; staker &amp;&amp; account_addr !&#61; operator;
</code></pre>



<a id="@Specification_1_request_commission_internal"></a>

### Function `request_commission_internal`


<pre><code>fun request_commission_internal(operator: address, staking_contract: &amp;mut staking_contract::StakingContract, add_distribution_events: &amp;mut event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;, request_commission_events: &amp;mut event::EventHandle&lt;staking_contract::RequestCommissionEvent&gt;): u64
</code></pre>




<pre><code>pragma verify &#61; false;
include GetStakingContractAmountsAbortsIf;
</code></pre>



<a id="@Specification_1_unlock_stake"></a>

### Function `unlock_stake`


<pre><code>public entry fun unlock_stake(staker: &amp;signer, operator: address, amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
requires amount &gt; 0;
let staker_address &#61; signer::address_of(staker);
include ContractExistsAbortsIf &#123; staker: staker_address &#125;;
</code></pre>



<a id="@Specification_1_unlock_rewards"></a>

### Function `unlock_rewards`


<pre><code>public entry fun unlock_rewards(staker: &amp;signer, operator: address)
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify &#61; false;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
requires staking_contract.commission_percentage &gt;&#61; 0 &amp;&amp; staking_contract.commission_percentage &lt;&#61; 100;
let staker_address &#61; signer::address_of(staker);
let staking_contracts &#61; global&lt;Store&gt;(staker_address).staking_contracts;
let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);
include ContractExistsAbortsIf &#123; staker: staker_address &#125;;
</code></pre>



<a id="@Specification_1_switch_operator_with_same_commission"></a>

### Function `switch_operator_with_same_commission`


<pre><code>public entry fun switch_operator_with_same_commission(staker: &amp;signer, old_operator: address, new_operator: address)
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify_duration_estimate &#61; 120;
pragma aborts_if_is_partial;
let staker_address &#61; signer::address_of(staker);
include ContractExistsAbortsIf &#123; staker: staker_address, operator: old_operator &#125;;
</code></pre>



<a id="@Specification_1_switch_operator"></a>

### Function `switch_operator`


<pre><code>public entry fun switch_operator(staker: &amp;signer, old_operator: address, new_operator: address, new_commission_percentage: u64)
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify &#61; false;
let staker_address &#61; signer::address_of(staker);
include ContractExistsAbortsIf &#123; staker: staker_address, operator: old_operator &#125;;
let store &#61; global&lt;Store&gt;(staker_address);
let staking_contracts &#61; store.staking_contracts;
aborts_if simple_map::spec_contains_key(staking_contracts, new_operator);
</code></pre>



<a id="@Specification_1_set_beneficiary_for_operator"></a>

### Function `set_beneficiary_for_operator`


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_distribute"></a>

### Function `distribute`


<pre><code>public entry fun distribute(staker: address, operator: address)
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>pragma verify_duration_estimate &#61; 120;
pragma aborts_if_is_partial;
include ContractExistsAbortsIf;
</code></pre>



<a id="@Specification_1_distribute_internal"></a>

### Function `distribute_internal`


<pre><code>fun distribute_internal(staker: address, operator: address, staking_contract: &amp;mut staking_contract::StakingContract, distribute_events: &amp;mut event::EventHandle&lt;staking_contract::DistributeEvent&gt;)
</code></pre>


The StakePool exists under the pool_address of StakingContract.
The value of inactive and pending_inactive in the stake_pool is up to MAX_U64.


<pre><code>pragma verify_duration_estimate &#61; 120;
pragma aborts_if_is_partial;
let pool_address &#61; staking_contract.pool_address;
let stake_pool &#61; borrow_global&lt;stake::StakePool&gt;(pool_address);
aborts_if !exists&lt;stake::StakePool&gt;(pool_address);
aborts_if stake_pool.inactive.value &#43; stake_pool.pending_inactive.value &gt; MAX_U64;
aborts_if !exists&lt;stake::StakePool&gt;(staking_contract.owner_cap.pool_address);
</code></pre>



<a id="@Specification_1_assert_staking_contract_exists"></a>

### Function `assert_staking_contract_exists`


<pre><code>fun assert_staking_contract_exists(staker: address, operator: address)
</code></pre>


Staking_contract exists the stacker/operator pair.


<pre><code>include ContractExistsAbortsIf;
</code></pre>



<a id="@Specification_1_add_distribution"></a>

### Function `add_distribution`


<pre><code>fun add_distribution(operator: address, staking_contract: &amp;mut staking_contract::StakingContract, recipient: address, coins_amount: u64, add_distribution_events: &amp;mut event::EventHandle&lt;staking_contract::AddDistributionEvent&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_get_staking_contract_amounts_internal"></a>

### Function `get_staking_contract_amounts_internal`


<pre><code>fun get_staking_contract_amounts_internal(staking_contract: &amp;staking_contract::StakingContract): (u64, u64, u64)
</code></pre>


The StakePool exists under the pool_address of StakingContract.


<pre><code>include GetStakingContractAmountsAbortsIf;
let pool_address &#61; staking_contract.pool_address;
let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);
let active &#61; coin::value(stake_pool.active);
let pending_active &#61; coin::value(stake_pool.pending_active);
let total_active_stake &#61; active &#43; pending_active;
let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;
let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;
ensures result_1 &#61;&#61; total_active_stake;
ensures result_2 &#61;&#61; accumulated_rewards;
ensures result_3 &#61;&#61; commission_amount;
</code></pre>



<a id="@Specification_1_create_stake_pool"></a>

### Function `create_stake_pool`


<pre><code>fun create_stake_pool(staker: &amp;signer, operator: address, voter: address, contract_creation_seed: vector&lt;u8&gt;): (signer, account::SignerCapability, stake::OwnerCapability)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
include stake::ResourceRequirement;
let staker_address &#61; signer::address_of(staker);
let seed_0 &#61; bcs::to_bytes(staker_address);
let seed_1 &#61; concat(concat(concat(seed_0, bcs::to_bytes(operator)), SALT), contract_creation_seed);
let resource_addr &#61; account::spec_create_resource_address(staker_address, seed_1);
include CreateStakePoolAbortsIf &#123; resource_addr &#125;;
ensures exists&lt;account::Account&gt;(resource_addr);
let post post_account &#61; global&lt;account::Account&gt;(resource_addr);
ensures post_account.authentication_key &#61;&#61; account::ZERO_AUTH_KEY;
ensures post_account.signer_capability_offer.for &#61;&#61; std::option::spec_some(resource_addr);
ensures exists&lt;stake::StakePool&gt;(resource_addr);
let post post_owner_cap &#61; global&lt;stake::OwnerCapability&gt;(resource_addr);
let post post_pool_address &#61; post_owner_cap.pool_address;
let post post_stake_pool &#61; global&lt;stake::StakePool&gt;(post_pool_address);
let post post_operator &#61; post_stake_pool.operator_address;
let post post_delegated_voter &#61; post_stake_pool.delegated_voter;
ensures resource_addr !&#61; operator &#61;&#61;&gt; post_operator &#61;&#61; operator;
ensures resource_addr !&#61; voter &#61;&#61;&gt; post_delegated_voter &#61;&#61; voter;
ensures signer::address_of(result_1) &#61;&#61; resource_addr;
ensures result_2 &#61;&#61; SignerCapability &#123; account: resource_addr &#125;;
ensures result_3 &#61;&#61; OwnerCapability &#123; pool_address: resource_addr &#125;;
</code></pre>



<a id="@Specification_1_update_distribution_pool"></a>

### Function `update_distribution_pool`


<pre><code>fun update_distribution_pool(distribution_pool: &amp;mut pool_u64::Pool, updated_total_coins: u64, operator: address, commission_percentage: u64)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
</code></pre>



<a id="@Specification_1_new_staking_contracts_holder"></a>

### Function `new_staking_contracts_holder`


<pre><code>fun new_staking_contracts_holder(staker: &amp;signer): staking_contract::Store
</code></pre>


The Account exists under the staker.
The guid_creation_num of the ccount resource is up to MAX_U64.


<pre><code>include NewStakingContractsHolderAbortsIf;
</code></pre>




<a id="0x1_staking_contract_NewStakingContractsHolderAbortsIf"></a>


<pre><code>schema NewStakingContractsHolderAbortsIf &#123;
    staker: signer;
    let addr &#61; signer::address_of(staker);
    let account &#61; global&lt;account::Account&gt;(addr);
    aborts_if !exists&lt;account::Account&gt;(addr);
    aborts_if account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
    aborts_if account.guid_creation_num &#43; 9 &gt; MAX_U64;
&#125;
</code></pre>


The Store exists under the staker.
a staking_contract exists for the staker/operator pair.


<a id="0x1_staking_contract_ContractExistsAbortsIf"></a>


<pre><code>schema ContractExistsAbortsIf &#123;
    staker: address;
    operator: address;
    aborts_if !exists&lt;Store&gt;(staker);
    let staking_contracts &#61; global&lt;Store&gt;(staker).staking_contracts;
    aborts_if !simple_map::spec_contains_key(staking_contracts, operator);
&#125;
</code></pre>




<a id="0x1_staking_contract_UpdateVoterSchema"></a>


<pre><code>schema UpdateVoterSchema &#123;
    staker: address;
    operator: address;
    let store &#61; global&lt;Store&gt;(staker);
    let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);
    let pool_address &#61; staking_contract.pool_address;
    aborts_if !exists&lt;stake::StakePool&gt;(pool_address);
    aborts_if !exists&lt;stake::StakePool&gt;(staking_contract.owner_cap.pool_address);
    include ContractExistsAbortsIf;
&#125;
</code></pre>




<a id="0x1_staking_contract_WithdrawAbortsIf"></a>


<pre><code>schema WithdrawAbortsIf&lt;CoinType&gt; &#123;
    account: signer;
    amount: u64;
    let account_addr &#61; signer::address_of(account);
    let coin_store &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr);
    let balance &#61; coin_store.coin.value;
    aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr);
    aborts_if coin_store.frozen;
    aborts_if balance &lt; amount;
&#125;
</code></pre>




<a id="0x1_staking_contract_GetStakingContractAmountsAbortsIf"></a>


<pre><code>schema GetStakingContractAmountsAbortsIf &#123;
    staking_contract: StakingContract;
    let pool_address &#61; staking_contract.pool_address;
    let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);
    let active &#61; coin::value(stake_pool.active);
    let pending_active &#61; coin::value(stake_pool.pending_active);
    let total_active_stake &#61; active &#43; pending_active;
    let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;
    aborts_if !exists&lt;stake::StakePool&gt;(pool_address);
    aborts_if active &#43; pending_active &gt; MAX_U64;
    aborts_if total_active_stake &lt; staking_contract.principal;
    aborts_if accumulated_rewards &#42; staking_contract.commission_percentage &gt; MAX_U64;
&#125;
</code></pre>




<a id="0x1_staking_contract_IncreaseLockupWithCapAbortsIf"></a>


<pre><code>schema IncreaseLockupWithCapAbortsIf &#123;
    staker: address;
    operator: address;
    let store &#61; global&lt;Store&gt;(staker);
    let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);
    let pool_address &#61; staking_contract.owner_cap.pool_address;
    aborts_if !stake::stake_pool_exists(pool_address);
    aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);
    let config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
    let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);
    let old_locked_until_secs &#61; stake_pool.locked_until_secs;
    let seconds &#61; global&lt;timestamp::CurrentTimeMicroseconds&gt;(
        @aptos_framework
    ).microseconds / timestamp::MICRO_CONVERSION_FACTOR;
    let new_locked_until_secs &#61; seconds &#43; config.recurring_lockup_duration_secs;
    aborts_if seconds &#43; config.recurring_lockup_duration_secs &gt; MAX_U64;
    aborts_if old_locked_until_secs &gt; new_locked_until_secs &#124;&#124; old_locked_until_secs &#61;&#61; new_locked_until_secs;
    aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
    let post post_store &#61; global&lt;Store&gt;(staker);
    let post post_staking_contract &#61; simple_map::spec_get(post_store.staking_contracts, operator);
    let post post_stake_pool &#61; global&lt;stake::StakePool&gt;(post_staking_contract.owner_cap.pool_address);
    ensures post_stake_pool.locked_until_secs &#61;&#61; new_locked_until_secs;
&#125;
</code></pre>




<a id="0x1_staking_contract_CreateStakingContractWithCoinsAbortsIfAndEnsures"></a>


<pre><code>schema CreateStakingContractWithCoinsAbortsIfAndEnsures &#123;
    staker: signer;
    operator: address;
    voter: address;
    amount: u64;
    commission_percentage: u64;
    contract_creation_seed: vector&lt;u8&gt;;
    aborts_if commission_percentage &gt; 100;
    aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);
    let config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
    let min_stake_required &#61; config.minimum_stake;
    aborts_if amount &lt; min_stake_required;
    let staker_address &#61; signer::address_of(staker);
    let account &#61; global&lt;account::Account&gt;(staker_address);
    aborts_if !exists&lt;Store&gt;(staker_address) &amp;&amp; !exists&lt;account::Account&gt;(staker_address);
    aborts_if !exists&lt;Store&gt;(staker_address) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
    // This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
    ensures exists&lt;Store&gt;(staker_address);
    let store &#61; global&lt;Store&gt;(staker_address);
    let staking_contracts &#61; store.staking_contracts;
    let owner_cap &#61; simple_map::spec_get(store.staking_contracts, operator).owner_cap;
    let post post_store &#61; global&lt;Store&gt;(staker_address);
    let post post_staking_contracts &#61; post_store.staking_contracts;
&#125;
</code></pre>




<a id="0x1_staking_contract_PreconditionsInCreateContract"></a>


<pre><code>schema PreconditionsInCreateContract &#123;
    requires exists&lt;stake::ValidatorPerformance&gt;(@aptos_framework);
    requires exists&lt;stake::ValidatorSet&gt;(@aptos_framework);
    requires exists&lt;staking_config::StakingRewardsConfig&gt;(
        @aptos_framework
    ) &#124;&#124; !std::features::spec_periodical_reward_rate_decrease_enabled();
    requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
    requires exists&lt;aptos_framework::timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
    requires exists&lt;stake::AptosCoinCapabilities&gt;(@aptos_framework);
&#125;
</code></pre>




<a id="0x1_staking_contract_CreateStakePoolAbortsIf"></a>


<pre><code>schema CreateStakePoolAbortsIf &#123;
    resource_addr: address;
    operator: address;
    voter: address;
    contract_creation_seed: vector&lt;u8&gt;;
    let acc &#61; global&lt;account::Account&gt;(resource_addr);
    aborts_if exists&lt;account::Account&gt;(resource_addr) &amp;&amp; (len(
        acc.signer_capability_offer.for.vec
    ) !&#61; 0 &#124;&#124; acc.sequence_number !&#61; 0);
    aborts_if !exists&lt;account::Account&gt;(resource_addr) &amp;&amp; len(bcs::to_bytes(resource_addr)) !&#61; 32;
    aborts_if len(account::ZERO_AUTH_KEY) !&#61; 32;
    aborts_if exists&lt;stake::ValidatorConfig&gt;(resource_addr);
    let allowed &#61; global&lt;stake::AllowedValidators&gt;(@aptos_framework);
    aborts_if exists&lt;stake::AllowedValidators&gt;(@aptos_framework) &amp;&amp; !contains(allowed.accounts, resource_addr);
    aborts_if exists&lt;stake::StakePool&gt;(resource_addr);
    aborts_if exists&lt;stake::OwnerCapability&gt;(resource_addr);
    aborts_if exists&lt;account::Account&gt;(
        resource_addr
    ) &amp;&amp; acc.guid_creation_num &#43; 12 &gt;&#61; account::MAX_GUID_CREATION_NUM;
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
