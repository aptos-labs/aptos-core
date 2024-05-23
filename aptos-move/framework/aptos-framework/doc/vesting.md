
<a id="0x1_vesting"></a>

# Module `0x1::vesting`

<br/> Simple vesting contract that allows specifying how much APT coins should be vesting in each fixed&#45;size period. The<br/> vesting contract also comes with staking and allows shareholders to withdraw rewards anytime.<br/><br/> Vesting schedule is represented as a vector of distributions. For example, a vesting schedule of<br/> [3/48, 3/48, 1/48] means that after the vesting starts:<br/> 1. The first and second periods will vest 3/48 of the total original grant.<br/> 2. The third period will vest 1/48.<br/> 3. All subsequent periods will also vest 1/48 (last distribution in the schedule) until the original grant runs out.<br/><br/> Shareholder flow:<br/> 1. Admin calls create_vesting_contract with a schedule of [3/48, 3/48, 1/48] with a vesting cliff of 1 year and<br/> vesting period of 1 month.<br/> 2. After a month, a shareholder calls unlock_rewards to request rewards. They can also call vest() which would also<br/> unlocks rewards but since the 1 year cliff has not passed (vesting has not started), vest() would not release any of<br/> the original grant.<br/> 3. After the unlocked rewards become fully withdrawable (as it&apos;s subject to staking lockup), shareholders can call<br/> distribute() to send all withdrawable funds to all shareholders based on the original grant&apos;s shares structure.<br/> 4. After 1 year and 1 month, the vesting schedule now starts. Shareholders call vest() to unlock vested coins. vest()<br/> checks the schedule and unlocks 3/48 of the original grant in addition to any accumulated rewards since last<br/> unlock_rewards(). Once the unlocked coins become withdrawable, shareholders can call distribute().<br/> 5. Assuming the shareholders forgot to call vest() for 2 months, when they call vest() again, they will unlock vested<br/> tokens for the next period since last vest. This would be for the first month they missed. They can call vest() a<br/> second time to unlock for the second month they missed.<br/><br/> Admin flow:<br/> 1. After creating the vesting contract, admin cannot change the vesting schedule.<br/> 2. Admin can call update_voter, update_operator, or reset_lockup at any time to update the underlying staking<br/> contract.<br/> 3. Admin can also call update_beneficiary for any shareholder. This would send all distributions (rewards, vested<br/> coins) of that shareholder to the beneficiary account. By defalt, if a beneficiary is not set, the distributions are<br/> send directly to the shareholder account.<br/> 4. Admin can call terminate_vesting_contract to terminate the vesting. This would first finish any distribution but<br/> will prevent any further rewards or vesting distributions from being created. Once the locked up stake becomes<br/> withdrawable, admin can call admin_withdraw to withdraw all funds to the vesting contract&apos;s withdrawal address.


-  [Struct `VestingSchedule`](#0x1_vesting_VestingSchedule)
-  [Struct `StakingInfo`](#0x1_vesting_StakingInfo)
-  [Resource `VestingContract`](#0x1_vesting_VestingContract)
-  [Resource `VestingAccountManagement`](#0x1_vesting_VestingAccountManagement)
-  [Resource `AdminStore`](#0x1_vesting_AdminStore)
-  [Struct `CreateVestingContract`](#0x1_vesting_CreateVestingContract)
-  [Struct `UpdateOperator`](#0x1_vesting_UpdateOperator)
-  [Struct `UpdateVoter`](#0x1_vesting_UpdateVoter)
-  [Struct `ResetLockup`](#0x1_vesting_ResetLockup)
-  [Struct `SetBeneficiary`](#0x1_vesting_SetBeneficiary)
-  [Struct `UnlockRewards`](#0x1_vesting_UnlockRewards)
-  [Struct `Vest`](#0x1_vesting_Vest)
-  [Struct `Distribute`](#0x1_vesting_Distribute)
-  [Struct `Terminate`](#0x1_vesting_Terminate)
-  [Struct `AdminWithdraw`](#0x1_vesting_AdminWithdraw)
-  [Struct `CreateVestingContractEvent`](#0x1_vesting_CreateVestingContractEvent)
-  [Struct `UpdateOperatorEvent`](#0x1_vesting_UpdateOperatorEvent)
-  [Struct `UpdateVoterEvent`](#0x1_vesting_UpdateVoterEvent)
-  [Struct `ResetLockupEvent`](#0x1_vesting_ResetLockupEvent)
-  [Struct `SetBeneficiaryEvent`](#0x1_vesting_SetBeneficiaryEvent)
-  [Struct `UnlockRewardsEvent`](#0x1_vesting_UnlockRewardsEvent)
-  [Struct `VestEvent`](#0x1_vesting_VestEvent)
-  [Struct `DistributeEvent`](#0x1_vesting_DistributeEvent)
-  [Struct `TerminateEvent`](#0x1_vesting_TerminateEvent)
-  [Struct `AdminWithdrawEvent`](#0x1_vesting_AdminWithdrawEvent)
-  [Constants](#@Constants_0)
-  [Function `stake_pool_address`](#0x1_vesting_stake_pool_address)
-  [Function `vesting_start_secs`](#0x1_vesting_vesting_start_secs)
-  [Function `period_duration_secs`](#0x1_vesting_period_duration_secs)
-  [Function `remaining_grant`](#0x1_vesting_remaining_grant)
-  [Function `beneficiary`](#0x1_vesting_beneficiary)
-  [Function `operator_commission_percentage`](#0x1_vesting_operator_commission_percentage)
-  [Function `vesting_contracts`](#0x1_vesting_vesting_contracts)
-  [Function `operator`](#0x1_vesting_operator)
-  [Function `voter`](#0x1_vesting_voter)
-  [Function `vesting_schedule`](#0x1_vesting_vesting_schedule)
-  [Function `total_accumulated_rewards`](#0x1_vesting_total_accumulated_rewards)
-  [Function `accumulated_rewards`](#0x1_vesting_accumulated_rewards)
-  [Function `shareholders`](#0x1_vesting_shareholders)
-  [Function `shareholder`](#0x1_vesting_shareholder)
-  [Function `create_vesting_schedule`](#0x1_vesting_create_vesting_schedule)
-  [Function `create_vesting_contract`](#0x1_vesting_create_vesting_contract)
-  [Function `unlock_rewards`](#0x1_vesting_unlock_rewards)
-  [Function `unlock_rewards_many`](#0x1_vesting_unlock_rewards_many)
-  [Function `vest`](#0x1_vesting_vest)
-  [Function `vest_many`](#0x1_vesting_vest_many)
-  [Function `distribute`](#0x1_vesting_distribute)
-  [Function `distribute_many`](#0x1_vesting_distribute_many)
-  [Function `terminate_vesting_contract`](#0x1_vesting_terminate_vesting_contract)
-  [Function `admin_withdraw`](#0x1_vesting_admin_withdraw)
-  [Function `update_operator`](#0x1_vesting_update_operator)
-  [Function `update_operator_with_same_commission`](#0x1_vesting_update_operator_with_same_commission)
-  [Function `update_commission_percentage`](#0x1_vesting_update_commission_percentage)
-  [Function `update_voter`](#0x1_vesting_update_voter)
-  [Function `reset_lockup`](#0x1_vesting_reset_lockup)
-  [Function `set_beneficiary`](#0x1_vesting_set_beneficiary)
-  [Function `reset_beneficiary`](#0x1_vesting_reset_beneficiary)
-  [Function `set_management_role`](#0x1_vesting_set_management_role)
-  [Function `set_beneficiary_resetter`](#0x1_vesting_set_beneficiary_resetter)
-  [Function `set_beneficiary_for_operator`](#0x1_vesting_set_beneficiary_for_operator)
-  [Function `get_role_holder`](#0x1_vesting_get_role_holder)
-  [Function `get_vesting_account_signer`](#0x1_vesting_get_vesting_account_signer)
-  [Function `get_vesting_account_signer_internal`](#0x1_vesting_get_vesting_account_signer_internal)
-  [Function `create_vesting_contract_account`](#0x1_vesting_create_vesting_contract_account)
-  [Function `verify_admin`](#0x1_vesting_verify_admin)
-  [Function `assert_vesting_contract_exists`](#0x1_vesting_assert_vesting_contract_exists)
-  [Function `assert_active_vesting_contract`](#0x1_vesting_assert_active_vesting_contract)
-  [Function `unlock_stake`](#0x1_vesting_unlock_stake)
-  [Function `withdraw_stake`](#0x1_vesting_withdraw_stake)
-  [Function `get_beneficiary`](#0x1_vesting_get_beneficiary)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `stake_pool_address`](#@Specification_1_stake_pool_address)
    -  [Function `vesting_start_secs`](#@Specification_1_vesting_start_secs)
    -  [Function `period_duration_secs`](#@Specification_1_period_duration_secs)
    -  [Function `remaining_grant`](#@Specification_1_remaining_grant)
    -  [Function `beneficiary`](#@Specification_1_beneficiary)
    -  [Function `operator_commission_percentage`](#@Specification_1_operator_commission_percentage)
    -  [Function `vesting_contracts`](#@Specification_1_vesting_contracts)
    -  [Function `operator`](#@Specification_1_operator)
    -  [Function `voter`](#@Specification_1_voter)
    -  [Function `vesting_schedule`](#@Specification_1_vesting_schedule)
    -  [Function `total_accumulated_rewards`](#@Specification_1_total_accumulated_rewards)
    -  [Function `accumulated_rewards`](#@Specification_1_accumulated_rewards)
    -  [Function `shareholders`](#@Specification_1_shareholders)
    -  [Function `shareholder`](#@Specification_1_shareholder)
    -  [Function `create_vesting_schedule`](#@Specification_1_create_vesting_schedule)
    -  [Function `create_vesting_contract`](#@Specification_1_create_vesting_contract)
    -  [Function `unlock_rewards`](#@Specification_1_unlock_rewards)
    -  [Function `unlock_rewards_many`](#@Specification_1_unlock_rewards_many)
    -  [Function `vest`](#@Specification_1_vest)
    -  [Function `vest_many`](#@Specification_1_vest_many)
    -  [Function `distribute`](#@Specification_1_distribute)
    -  [Function `distribute_many`](#@Specification_1_distribute_many)
    -  [Function `terminate_vesting_contract`](#@Specification_1_terminate_vesting_contract)
    -  [Function `admin_withdraw`](#@Specification_1_admin_withdraw)
    -  [Function `update_operator`](#@Specification_1_update_operator)
    -  [Function `update_operator_with_same_commission`](#@Specification_1_update_operator_with_same_commission)
    -  [Function `update_commission_percentage`](#@Specification_1_update_commission_percentage)
    -  [Function `update_voter`](#@Specification_1_update_voter)
    -  [Function `reset_lockup`](#@Specification_1_reset_lockup)
    -  [Function `set_beneficiary`](#@Specification_1_set_beneficiary)
    -  [Function `reset_beneficiary`](#@Specification_1_reset_beneficiary)
    -  [Function `set_management_role`](#@Specification_1_set_management_role)
    -  [Function `set_beneficiary_resetter`](#@Specification_1_set_beneficiary_resetter)
    -  [Function `set_beneficiary_for_operator`](#@Specification_1_set_beneficiary_for_operator)
    -  [Function `get_role_holder`](#@Specification_1_get_role_holder)
    -  [Function `get_vesting_account_signer`](#@Specification_1_get_vesting_account_signer)
    -  [Function `get_vesting_account_signer_internal`](#@Specification_1_get_vesting_account_signer_internal)
    -  [Function `create_vesting_contract_account`](#@Specification_1_create_vesting_contract_account)
    -  [Function `verify_admin`](#@Specification_1_verify_admin)
    -  [Function `assert_vesting_contract_exists`](#@Specification_1_assert_vesting_contract_exists)
    -  [Function `assert_active_vesting_contract`](#@Specification_1_assert_active_vesting_contract)
    -  [Function `unlock_stake`](#@Specification_1_unlock_stake)
    -  [Function `withdraw_stake`](#@Specification_1_withdraw_stake)
    -  [Function `get_beneficiary`](#@Specification_1_get_beneficiary)


<pre><code>use 0x1::account;<br/>use 0x1::aptos_account;<br/>use 0x1::aptos_coin;<br/>use 0x1::bcs;<br/>use 0x1::coin;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::fixed_point32;<br/>use 0x1::math64;<br/>use 0x1::pool_u64;<br/>use 0x1::signer;<br/>use 0x1::simple_map;<br/>use 0x1::stake;<br/>use 0x1::staking_contract;<br/>use 0x1::string;<br/>use 0x1::system_addresses;<br/>use 0x1::timestamp;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_vesting_VestingSchedule"></a>

## Struct `VestingSchedule`



<pre><code>struct VestingSchedule has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>schedule: vector&lt;fixed_point32::FixedPoint32&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>start_timestamp_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>period_duration: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>last_vested_period: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_StakingInfo"></a>

## Struct `StakingInfo`



<pre><code>struct StakingInfo has store<br/></code></pre>



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
<code>voter: address</code>
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

<a id="0x1_vesting_VestingContract"></a>

## Resource `VestingContract`



<pre><code>struct VestingContract has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>state: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>grant_pool: pool_u64::Pool</code>
</dt>
<dd>

</dd>
<dt>
<code>beneficiaries: simple_map::SimpleMap&lt;address, address&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_schedule: vesting::VestingSchedule</code>
</dt>
<dd>

</dd>
<dt>
<code>withdrawal_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking: vesting::StakingInfo</code>
</dt>
<dd>

</dd>
<dt>
<code>remaining_grant: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>signer_cap: account::SignerCapability</code>
</dt>
<dd>

</dd>
<dt>
<code>update_operator_events: event::EventHandle&lt;vesting::UpdateOperatorEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_voter_events: event::EventHandle&lt;vesting::UpdateVoterEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reset_lockup_events: event::EventHandle&lt;vesting::ResetLockupEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>set_beneficiary_events: event::EventHandle&lt;vesting::SetBeneficiaryEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_rewards_events: event::EventHandle&lt;vesting::UnlockRewardsEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vest_events: event::EventHandle&lt;vesting::VestEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>distribute_events: event::EventHandle&lt;vesting::DistributeEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>terminate_events: event::EventHandle&lt;vesting::TerminateEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>admin_withdraw_events: event::EventHandle&lt;vesting::AdminWithdrawEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_VestingAccountManagement"></a>

## Resource `VestingAccountManagement`



<pre><code>struct VestingAccountManagement has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>roles: simple_map::SimpleMap&lt;string::String, address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_AdminStore"></a>

## Resource `AdminStore`



<pre><code>struct AdminStore has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>vesting_contracts: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>nonce: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>create_events: event::EventHandle&lt;vesting::CreateVestingContractEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_CreateVestingContract"></a>

## Struct `CreateVestingContract`



<pre><code>&#35;[event]<br/>struct CreateVestingContract has drop, store<br/></code></pre>



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
<code>grant_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>withdrawal_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
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

<a id="0x1_vesting_UpdateOperator"></a>

## Struct `UpdateOperator`



<pre><code>&#35;[event]<br/>struct UpdateOperator has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
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
<dt>
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_UpdateVoter"></a>

## Struct `UpdateVoter`



<pre><code>&#35;[event]<br/>struct UpdateVoter has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
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

<a id="0x1_vesting_ResetLockup"></a>

## Struct `ResetLockup`



<pre><code>&#35;[event]<br/>struct ResetLockup has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_lockup_expiration_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_SetBeneficiary"></a>

## Struct `SetBeneficiary`



<pre><code>&#35;[event]<br/>struct SetBeneficiary has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>shareholder: address</code>
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

<a id="0x1_vesting_UnlockRewards"></a>

## Struct `UnlockRewards`



<pre><code>&#35;[event]<br/>struct UnlockRewards has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
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

<a id="0x1_vesting_Vest"></a>

## Struct `Vest`



<pre><code>&#35;[event]<br/>struct Vest has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>period_vested: u64</code>
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

<a id="0x1_vesting_Distribute"></a>

## Struct `Distribute`



<pre><code>&#35;[event]<br/>struct Distribute has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
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

<a id="0x1_vesting_Terminate"></a>

## Struct `Terminate`



<pre><code>&#35;[event]<br/>struct Terminate has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_AdminWithdraw"></a>

## Struct `AdminWithdraw`



<pre><code>&#35;[event]<br/>struct AdminWithdraw has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
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

<a id="0x1_vesting_CreateVestingContractEvent"></a>

## Struct `CreateVestingContractEvent`



<pre><code>struct CreateVestingContractEvent has drop, store<br/></code></pre>



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
<code>grant_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>withdrawal_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
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

<a id="0x1_vesting_UpdateOperatorEvent"></a>

## Struct `UpdateOperatorEvent`



<pre><code>struct UpdateOperatorEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
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
<dt>
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_UpdateVoterEvent"></a>

## Struct `UpdateVoterEvent`



<pre><code>struct UpdateVoterEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
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

<a id="0x1_vesting_ResetLockupEvent"></a>

## Struct `ResetLockupEvent`



<pre><code>struct ResetLockupEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_lockup_expiration_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_SetBeneficiaryEvent"></a>

## Struct `SetBeneficiaryEvent`



<pre><code>struct SetBeneficiaryEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>shareholder: address</code>
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

<a id="0x1_vesting_UnlockRewardsEvent"></a>

## Struct `UnlockRewardsEvent`



<pre><code>struct UnlockRewardsEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
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

<a id="0x1_vesting_VestEvent"></a>

## Struct `VestEvent`



<pre><code>struct VestEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>period_vested: u64</code>
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

<a id="0x1_vesting_DistributeEvent"></a>

## Struct `DistributeEvent`



<pre><code>struct DistributeEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
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

<a id="0x1_vesting_TerminateEvent"></a>

## Struct `TerminateEvent`



<pre><code>struct TerminateEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_AdminWithdrawEvent"></a>

## Struct `AdminWithdrawEvent`



<pre><code>struct AdminWithdrawEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: address</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: address</code>
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


<a id="0x1_vesting_EEMPTY_VESTING_SCHEDULE"></a>

Vesting schedule cannot be empty.


<pre><code>const EEMPTY_VESTING_SCHEDULE: u64 &#61; 2;<br/></code></pre>



<a id="0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS"></a>

Withdrawal address is invalid.


<pre><code>const EINVALID_WITHDRAWAL_ADDRESS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_vesting_ENOT_ADMIN"></a>

The signer is not the admin of the vesting contract.


<pre><code>const ENOT_ADMIN: u64 &#61; 7;<br/></code></pre>



<a id="0x1_vesting_ENO_SHAREHOLDERS"></a>

Shareholders list cannot be empty.


<pre><code>const ENO_SHAREHOLDERS: u64 &#61; 4;<br/></code></pre>



<a id="0x1_vesting_EPENDING_STAKE_FOUND"></a>

Cannot terminate the vesting contract with pending active stake. Need to wait until next epoch.


<pre><code>const EPENDING_STAKE_FOUND: u64 &#61; 11;<br/></code></pre>



<a id="0x1_vesting_EPERMISSION_DENIED"></a>

Account is not admin or does not have the required role to take this action.


<pre><code>const EPERMISSION_DENIED: u64 &#61; 15;<br/></code></pre>



<a id="0x1_vesting_EROLE_NOT_FOUND"></a>

The vesting account has no such management role.


<pre><code>const EROLE_NOT_FOUND: u64 &#61; 14;<br/></code></pre>



<a id="0x1_vesting_ESHARES_LENGTH_MISMATCH"></a>

The length of shareholders and shares lists don&apos;t match.


<pre><code>const ESHARES_LENGTH_MISMATCH: u64 &#61; 5;<br/></code></pre>



<a id="0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION"></a>

Zero items were provided to a &#42;_many function.


<pre><code>const EVEC_EMPTY_FOR_MANY_FUNCTION: u64 &#61; 16;<br/></code></pre>



<a id="0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES"></a>

Vesting account has no other management roles beside admin.


<pre><code>const EVESTING_ACCOUNT_HAS_NO_ROLES: u64 &#61; 13;<br/></code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE"></a>

Vesting contract needs to be in active state.


<pre><code>const EVESTING_CONTRACT_NOT_ACTIVE: u64 &#61; 8;<br/></code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_NOT_FOUND"></a>

No vesting contract found at provided address.


<pre><code>const EVESTING_CONTRACT_NOT_FOUND: u64 &#61; 10;<br/></code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE"></a>

Admin can only withdraw from an inactive (paused or terminated) vesting contract.


<pre><code>const EVESTING_CONTRACT_STILL_ACTIVE: u64 &#61; 9;<br/></code></pre>



<a id="0x1_vesting_EVESTING_START_TOO_SOON"></a>

Vesting cannot start before or at the current block timestamp. Has to be in the future.


<pre><code>const EVESTING_START_TOO_SOON: u64 &#61; 6;<br/></code></pre>



<a id="0x1_vesting_EZERO_GRANT"></a>

Grant amount cannot be 0.


<pre><code>const EZERO_GRANT: u64 &#61; 12;<br/></code></pre>



<a id="0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD"></a>

Vesting period cannot be 0.


<pre><code>const EZERO_VESTING_SCHEDULE_PERIOD: u64 &#61; 3;<br/></code></pre>



<a id="0x1_vesting_MAXIMUM_SHAREHOLDERS"></a>

Maximum number of shareholders a vesting pool can support.


<pre><code>const MAXIMUM_SHAREHOLDERS: u64 &#61; 30;<br/></code></pre>



<a id="0x1_vesting_ROLE_BENEFICIARY_RESETTER"></a>

Roles that can manage certain aspects of the vesting account beyond the main admin.


<pre><code>const ROLE_BENEFICIARY_RESETTER: vector&lt;u8&gt; &#61; [82, 79, 76, 69, 95, 66, 69, 78, 69, 70, 73, 67, 73, 65, 82, 89, 95, 82, 69, 83, 69, 84, 84, 69, 82];<br/></code></pre>



<a id="0x1_vesting_VESTING_POOL_ACTIVE"></a>

Vesting contract states.<br/> Vesting contract is active and distributions can be made.


<pre><code>const VESTING_POOL_ACTIVE: u64 &#61; 1;<br/></code></pre>



<a id="0x1_vesting_VESTING_POOL_SALT"></a>



<pre><code>const VESTING_POOL_SALT: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 118, 101, 115, 116, 105, 110, 103];<br/></code></pre>



<a id="0x1_vesting_VESTING_POOL_TERMINATED"></a>

Vesting contract has been terminated and all funds have been released back to the withdrawal address.


<pre><code>const VESTING_POOL_TERMINATED: u64 &#61; 2;<br/></code></pre>



<a id="0x1_vesting_stake_pool_address"></a>

## Function `stake_pool_address`

Return the address of the underlying stake pool (separate resource account) of the vesting contract.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun stake_pool_address(vesting_contract_address: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun stake_pool_address(vesting_contract_address: address): address acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    borrow_global&lt;VestingContract&gt;(vesting_contract_address).staking.pool_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_vesting_start_secs"></a>

## Function `vesting_start_secs`

Return the vesting start timestamp (in seconds) of the vesting contract.<br/> Vesting will start at this time, and once a full period has passed, the first vest will become unlocked.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun vesting_start_secs(vesting_contract_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vesting_start_secs(vesting_contract_address: address): u64 acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    borrow_global&lt;VestingContract&gt;(vesting_contract_address).vesting_schedule.start_timestamp_secs<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_period_duration_secs"></a>

## Function `period_duration_secs`

Return the duration of one vesting period (in seconds).<br/> Each vest is released after one full period has started, starting from the specified start_timestamp_secs.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun period_duration_secs(vesting_contract_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun period_duration_secs(vesting_contract_address: address): u64 acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    borrow_global&lt;VestingContract&gt;(vesting_contract_address).vesting_schedule.period_duration<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_remaining_grant"></a>

## Function `remaining_grant`

Return the remaining grant, consisting of unvested coins that have not been distributed to shareholders.<br/> Prior to start_timestamp_secs, the remaining grant will always be equal to the original grant.<br/> Once vesting has started, and vested tokens are distributed, the remaining grant will decrease over time,<br/> according to the vesting schedule.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun remaining_grant(vesting_contract_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remaining_grant(vesting_contract_address: address): u64 acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    borrow_global&lt;VestingContract&gt;(vesting_contract_address).remaining_grant<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_beneficiary"></a>

## Function `beneficiary`

Return the beneficiary account of the specified shareholder in a vesting contract.<br/> This is the same as the shareholder address by default and only different if it&apos;s been explicitly set.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun beneficiary(vesting_contract_address: address, shareholder: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun beneficiary(vesting_contract_address: address, shareholder: address): address acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    get_beneficiary(borrow_global&lt;VestingContract&gt;(vesting_contract_address), shareholder)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the percentage of accumulated rewards that is paid to the operator as commission.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun operator_commission_percentage(vesting_contract_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator_commission_percentage(vesting_contract_address: address): u64 acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    borrow_global&lt;VestingContract&gt;(vesting_contract_address).staking.commission_percentage<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_vesting_contracts"></a>

## Function `vesting_contracts`

Return all the vesting contracts a given address is an admin of.


<pre><code>&#35;[view]<br/>public fun vesting_contracts(admin: address): vector&lt;address&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vesting_contracts(admin: address): vector&lt;address&gt; acquires AdminStore &#123;<br/>    if (!exists&lt;AdminStore&gt;(admin)) &#123;<br/>        vector::empty&lt;address&gt;()<br/>    &#125; else &#123;<br/>        borrow_global&lt;AdminStore&gt;(admin).vesting_contracts<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_operator"></a>

## Function `operator`

Return the operator who runs the validator for the vesting contract.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun operator(vesting_contract_address: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator(vesting_contract_address: address): address acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    borrow_global&lt;VestingContract&gt;(vesting_contract_address).staking.operator<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_voter"></a>

## Function `voter`

Return the voter who will be voting on on&#45;chain governance proposals on behalf of the vesting contract&apos;s stake<br/> pool.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun voter(vesting_contract_address: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun voter(vesting_contract_address: address): address acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    borrow_global&lt;VestingContract&gt;(vesting_contract_address).staking.voter<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_vesting_schedule"></a>

## Function `vesting_schedule`

Return the vesting contract&apos;s vesting schedule. The core schedule is represented as a list of u64&#45;based<br/> fractions, where the rightmmost 32 bits can be divided by 2^32 to get the fraction, and anything else is the<br/> whole number.<br/><br/> For example 3/48, or 0.0625, will be represented as 268435456. The fractional portion would be<br/> 268435456 / 2^32 &#61; 0.0625. Since there are fewer than 32 bits, the whole number portion is effectively 0.<br/> So 268435456 &#61; 0.0625.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun vesting_schedule(vesting_contract_address: address): vesting::VestingSchedule<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vesting_schedule(vesting_contract_address: address): VestingSchedule acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(vesting_contract_address);<br/>    borrow_global&lt;VestingContract&gt;(vesting_contract_address).vesting_schedule<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_total_accumulated_rewards"></a>

## Function `total_accumulated_rewards`

Return the total accumulated rewards that have not been distributed to shareholders of the vesting contract.<br/> This excludes any unpaid commission that the operator has not collected.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun total_accumulated_rewards(vesting_contract_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun total_accumulated_rewards(vesting_contract_address: address): u64 acquires VestingContract &#123;<br/>    assert_active_vesting_contract(vesting_contract_address);<br/><br/>    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(vesting_contract_address);<br/>    let (total_active_stake, _, commission_amount) &#61;<br/>        staking_contract::staking_contract_amounts(vesting_contract_address, vesting_contract.staking.operator);<br/>    total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_accumulated_rewards"></a>

## Function `accumulated_rewards`

Return the accumulated rewards that have not been distributed to the provided shareholder. Caller can also pass<br/> the beneficiary address instead of shareholder address.<br/><br/> This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br/>public fun accumulated_rewards(vesting_contract_address: address, shareholder_or_beneficiary: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun accumulated_rewards(<br/>    vesting_contract_address: address, shareholder_or_beneficiary: address): u64 acquires VestingContract &#123;<br/>    assert_active_vesting_contract(vesting_contract_address);<br/><br/>    let total_accumulated_rewards &#61; total_accumulated_rewards(vesting_contract_address);<br/>    let shareholder &#61; shareholder(vesting_contract_address, shareholder_or_beneficiary);<br/>    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(vesting_contract_address);<br/>    let shares &#61; pool_u64::shares(&amp;vesting_contract.grant_pool, shareholder);<br/>    pool_u64::shares_to_amount_with_total_coins(&amp;vesting_contract.grant_pool, shares, total_accumulated_rewards)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_shareholders"></a>

## Function `shareholders`

Return the list of all shareholders in the vesting contract.


<pre><code>&#35;[view]<br/>public fun shareholders(vesting_contract_address: address): vector&lt;address&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shareholders(vesting_contract_address: address): vector&lt;address&gt; acquires VestingContract &#123;<br/>    assert_active_vesting_contract(vesting_contract_address);<br/><br/>    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(vesting_contract_address);<br/>    pool_u64::shareholders(&amp;vesting_contract.grant_pool)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_shareholder"></a>

## Function `shareholder`

Return the shareholder address given the beneficiary address in a given vesting contract. If there are multiple<br/> shareholders with the same beneficiary address, only the first shareholder is returned. If the given beneficiary<br/> address is actually a shareholder address, just return the address back.<br/><br/> This returns 0x0 if no shareholder is found for the given beneficiary / the address is not a shareholder itself.


<pre><code>&#35;[view]<br/>public fun shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shareholder(<br/>    vesting_contract_address: address,<br/>    shareholder_or_beneficiary: address<br/>): address acquires VestingContract &#123;<br/>    assert_active_vesting_contract(vesting_contract_address);<br/><br/>    let shareholders &#61; &amp;shareholders(vesting_contract_address);<br/>    if (vector::contains(shareholders, &amp;shareholder_or_beneficiary)) &#123;<br/>        return shareholder_or_beneficiary<br/>    &#125;;<br/>    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(vesting_contract_address);<br/>    let result &#61; @0x0;<br/>    vector::any(shareholders, &#124;shareholder&#124; &#123;<br/>        if (shareholder_or_beneficiary &#61;&#61; get_beneficiary(vesting_contract, &#42;shareholder)) &#123;<br/>            result &#61; &#42;shareholder;<br/>            true<br/>        &#125; else &#123;<br/>            false<br/>        &#125;<br/>    &#125;);<br/><br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_create_vesting_schedule"></a>

## Function `create_vesting_schedule`

Create a vesting schedule with the given schedule of distributions, a vesting start time and period duration.


<pre><code>public fun create_vesting_schedule(schedule: vector&lt;fixed_point32::FixedPoint32&gt;, start_timestamp_secs: u64, period_duration: u64): vesting::VestingSchedule<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_vesting_schedule(<br/>    schedule: vector&lt;FixedPoint32&gt;,<br/>    start_timestamp_secs: u64,<br/>    period_duration: u64,<br/>): VestingSchedule &#123;<br/>    assert!(vector::length(&amp;schedule) &gt; 0, error::invalid_argument(EEMPTY_VESTING_SCHEDULE));<br/>    assert!(period_duration &gt; 0, error::invalid_argument(EZERO_VESTING_SCHEDULE_PERIOD));<br/>    assert!(<br/>        start_timestamp_secs &gt;&#61; timestamp::now_seconds(),<br/>        error::invalid_argument(EVESTING_START_TOO_SOON),<br/>    );<br/><br/>    VestingSchedule &#123;<br/>        schedule,<br/>        start_timestamp_secs,<br/>        period_duration,<br/>        last_vested_period: 0,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_create_vesting_contract"></a>

## Function `create_vesting_contract`

Create a vesting contract with a given configurations.


<pre><code>public fun create_vesting_contract(admin: &amp;signer, shareholders: &amp;vector&lt;address&gt;, buy_ins: simple_map::SimpleMap&lt;address, coin::Coin&lt;aptos_coin::AptosCoin&gt;&gt;, vesting_schedule: vesting::VestingSchedule, withdrawal_address: address, operator: address, voter: address, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_vesting_contract(<br/>    admin: &amp;signer,<br/>    shareholders: &amp;vector&lt;address&gt;,<br/>    buy_ins: SimpleMap&lt;address, Coin&lt;AptosCoin&gt;&gt;,<br/>    vesting_schedule: VestingSchedule,<br/>    withdrawal_address: address,<br/>    operator: address,<br/>    voter: address,<br/>    commission_percentage: u64,<br/>    // Optional seed used when creating the staking contract account.<br/>    contract_creation_seed: vector&lt;u8&gt;,<br/>): address acquires AdminStore &#123;<br/>    assert!(<br/>        !system_addresses::is_reserved_address(withdrawal_address),<br/>        error::invalid_argument(EINVALID_WITHDRAWAL_ADDRESS),<br/>    );<br/>    assert_account_is_registered_for_apt(withdrawal_address);<br/>    assert!(vector::length(shareholders) &gt; 0, error::invalid_argument(ENO_SHAREHOLDERS));<br/>    assert!(<br/>        simple_map::length(&amp;buy_ins) &#61;&#61; vector::length(shareholders),<br/>        error::invalid_argument(ESHARES_LENGTH_MISMATCH),<br/>    );<br/><br/>    // Create a coins pool to track shareholders and shares of the grant.<br/>    let grant &#61; coin::zero&lt;AptosCoin&gt;();<br/>    let grant_amount &#61; 0;<br/>    let grant_pool &#61; pool_u64::create(MAXIMUM_SHAREHOLDERS);<br/>    vector::for_each_ref(shareholders, &#124;shareholder&#124; &#123;<br/>        let shareholder: address &#61; &#42;shareholder;<br/>        let (_, buy_in) &#61; simple_map::remove(&amp;mut buy_ins, &amp;shareholder);<br/>        let buy_in_amount &#61; coin::value(&amp;buy_in);<br/>        coin::merge(&amp;mut grant, buy_in);<br/>        pool_u64::buy_in(<br/>            &amp;mut grant_pool,<br/>            shareholder,<br/>            buy_in_amount,<br/>        );<br/>        grant_amount &#61; grant_amount &#43; buy_in_amount;<br/>    &#125;);<br/>    assert!(grant_amount &gt; 0, error::invalid_argument(EZERO_GRANT));<br/><br/>    // If this is the first time this admin account has created a vesting contract, initialize the admin store.<br/>    let admin_address &#61; signer::address_of(admin);<br/>    if (!exists&lt;AdminStore&gt;(admin_address)) &#123;<br/>        move_to(admin, AdminStore &#123;<br/>            vesting_contracts: vector::empty&lt;address&gt;(),<br/>            nonce: 0,<br/>            create_events: new_event_handle&lt;CreateVestingContractEvent&gt;(admin),<br/>        &#125;);<br/>    &#125;;<br/><br/>    // Initialize the vesting contract in a new resource account. This allows the same admin to create multiple<br/>    // pools.<br/>    let (contract_signer, contract_signer_cap) &#61; create_vesting_contract_account(admin, contract_creation_seed);<br/>    let pool_address &#61; staking_contract::create_staking_contract_with_coins(<br/>        &amp;contract_signer, operator, voter, grant, commission_percentage, contract_creation_seed);<br/><br/>    // Add the newly created vesting contract&apos;s address to the admin store.<br/>    let contract_address &#61; signer::address_of(&amp;contract_signer);<br/>    let admin_store &#61; borrow_global_mut&lt;AdminStore&gt;(admin_address);<br/>    vector::push_back(&amp;mut admin_store.vesting_contracts, contract_address);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            CreateVestingContract &#123;<br/>                operator,<br/>                voter,<br/>                withdrawal_address,<br/>                grant_amount,<br/>                vesting_contract_address: contract_address,<br/>                staking_pool_address: pool_address,<br/>                commission_percentage,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut admin_store.create_events,<br/>        CreateVestingContractEvent &#123;<br/>            operator,<br/>            voter,<br/>            withdrawal_address,<br/>            grant_amount,<br/>            vesting_contract_address: contract_address,<br/>            staking_pool_address: pool_address,<br/>            commission_percentage,<br/>        &#125;,<br/>    );<br/><br/>    move_to(&amp;contract_signer, VestingContract &#123;<br/>        state: VESTING_POOL_ACTIVE,<br/>        admin: admin_address,<br/>        grant_pool,<br/>        beneficiaries: simple_map::create&lt;address, address&gt;(),<br/>        vesting_schedule,<br/>        withdrawal_address,<br/>        staking: StakingInfo &#123; pool_address, operator, voter, commission_percentage &#125;,<br/>        remaining_grant: grant_amount,<br/>        signer_cap: contract_signer_cap,<br/>        update_operator_events: new_event_handle&lt;UpdateOperatorEvent&gt;(&amp;contract_signer),<br/>        update_voter_events: new_event_handle&lt;UpdateVoterEvent&gt;(&amp;contract_signer),<br/>        reset_lockup_events: new_event_handle&lt;ResetLockupEvent&gt;(&amp;contract_signer),<br/>        set_beneficiary_events: new_event_handle&lt;SetBeneficiaryEvent&gt;(&amp;contract_signer),<br/>        unlock_rewards_events: new_event_handle&lt;UnlockRewardsEvent&gt;(&amp;contract_signer),<br/>        vest_events: new_event_handle&lt;VestEvent&gt;(&amp;contract_signer),<br/>        distribute_events: new_event_handle&lt;DistributeEvent&gt;(&amp;contract_signer),<br/>        terminate_events: new_event_handle&lt;TerminateEvent&gt;(&amp;contract_signer),<br/>        admin_withdraw_events: new_event_handle&lt;AdminWithdrawEvent&gt;(&amp;contract_signer),<br/>    &#125;);<br/><br/>    simple_map::destroy_empty(buy_ins);<br/>    contract_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_unlock_rewards"></a>

## Function `unlock_rewards`

Unlock any accumulated rewards.


<pre><code>public entry fun unlock_rewards(contract_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock_rewards(contract_address: address) acquires VestingContract &#123;<br/>    let accumulated_rewards &#61; total_accumulated_rewards(contract_address);<br/>    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(contract_address);<br/>    unlock_stake(vesting_contract, accumulated_rewards);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_unlock_rewards_many"></a>

## Function `unlock_rewards_many`

Call <code>unlock_rewards</code> for many vesting contracts.


<pre><code>public entry fun unlock_rewards_many(contract_addresses: vector&lt;address&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock_rewards_many(contract_addresses: vector&lt;address&gt;) acquires VestingContract &#123;<br/>    let len &#61; vector::length(&amp;contract_addresses);<br/><br/>    assert!(len !&#61; 0, error::invalid_argument(EVEC_EMPTY_FOR_MANY_FUNCTION));<br/><br/>    vector::for_each_ref(&amp;contract_addresses, &#124;contract_address&#124; &#123;<br/>        let contract_address: address &#61; &#42;contract_address;<br/>        unlock_rewards(contract_address);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_vest"></a>

## Function `vest`

Unlock any vested portion of the grant.


<pre><code>public entry fun vest(contract_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vest(contract_address: address) acquires VestingContract &#123;<br/>    // Unlock all rewards first, if any.<br/>    unlock_rewards(contract_address);<br/><br/>    // Unlock the vested amount. This amount will become withdrawable when the underlying stake pool&apos;s lockup<br/>    // expires.<br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    // Short&#45;circuit if vesting hasn&apos;t started yet.<br/>    if (vesting_contract.vesting_schedule.start_timestamp_secs &gt; timestamp::now_seconds()) &#123;<br/>        return<br/>    &#125;;<br/><br/>    // Check if the next vested period has already passed. If not, short&#45;circuit since there&apos;s nothing to vest.<br/>    let vesting_schedule &#61; &amp;mut vesting_contract.vesting_schedule;<br/>    let last_vested_period &#61; vesting_schedule.last_vested_period;<br/>    let next_period_to_vest &#61; last_vested_period &#43; 1;<br/>    let last_completed_period &#61;<br/>        (timestamp::now_seconds() &#45; vesting_schedule.start_timestamp_secs) / vesting_schedule.period_duration;<br/>    if (last_completed_period &lt; next_period_to_vest) &#123;<br/>        return<br/>    &#125;;<br/><br/>    // Calculate how much has vested, excluding rewards.<br/>    // Index is 0&#45;based while period is 1&#45;based so we need to subtract 1.<br/>    let schedule &#61; &amp;vesting_schedule.schedule;<br/>    let schedule_index &#61; next_period_to_vest &#45; 1;<br/>    let vesting_fraction &#61; if (schedule_index &lt; vector::length(schedule)) &#123;<br/>        &#42;vector::borrow(schedule, schedule_index)<br/>    &#125; else &#123;<br/>        // Last vesting schedule fraction will repeat until the grant runs out.<br/>        &#42;vector::borrow(schedule, vector::length(schedule) &#45; 1)<br/>    &#125;;<br/>    let total_grant &#61; pool_u64::total_coins(&amp;vesting_contract.grant_pool);<br/>    let vested_amount &#61; fixed_point32::multiply_u64(total_grant, vesting_fraction);<br/>    // Cap vested amount by the remaining grant amount so we don&apos;t try to distribute more than what&apos;s remaining.<br/>    vested_amount &#61; min(vested_amount, vesting_contract.remaining_grant);<br/>    vesting_contract.remaining_grant &#61; vesting_contract.remaining_grant &#45; vested_amount;<br/>    vesting_schedule.last_vested_period &#61; next_period_to_vest;<br/>    unlock_stake(vesting_contract, vested_amount);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            Vest &#123;<br/>                admin: vesting_contract.admin,<br/>                vesting_contract_address: contract_address,<br/>                staking_pool_address: vesting_contract.staking.pool_address,<br/>                period_vested: next_period_to_vest,<br/>                amount: vested_amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut vesting_contract.vest_events,<br/>        VestEvent &#123;<br/>            admin: vesting_contract.admin,<br/>            vesting_contract_address: contract_address,<br/>            staking_pool_address: vesting_contract.staking.pool_address,<br/>            period_vested: next_period_to_vest,<br/>            amount: vested_amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_vest_many"></a>

## Function `vest_many`

Call <code>vest</code> for many vesting contracts.


<pre><code>public entry fun vest_many(contract_addresses: vector&lt;address&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vest_many(contract_addresses: vector&lt;address&gt;) acquires VestingContract &#123;<br/>    let len &#61; vector::length(&amp;contract_addresses);<br/><br/>    assert!(len !&#61; 0, error::invalid_argument(EVEC_EMPTY_FOR_MANY_FUNCTION));<br/><br/>    vector::for_each_ref(&amp;contract_addresses, &#124;contract_address&#124; &#123;<br/>        let contract_address &#61; &#42;contract_address;<br/>        vest(contract_address);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_distribute"></a>

## Function `distribute`

Distribute any withdrawable stake from the stake pool.


<pre><code>public entry fun distribute(contract_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun distribute(contract_address: address) acquires VestingContract &#123;<br/>    assert_active_vesting_contract(contract_address);<br/><br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    let coins &#61; withdraw_stake(vesting_contract, contract_address);<br/>    let total_distribution_amount &#61; coin::value(&amp;coins);<br/>    if (total_distribution_amount &#61;&#61; 0) &#123;<br/>        coin::destroy_zero(coins);<br/>        return<br/>    &#125;;<br/><br/>    // Distribute coins to all shareholders in the vesting contract.<br/>    let grant_pool &#61; &amp;vesting_contract.grant_pool;<br/>    let shareholders &#61; &amp;pool_u64::shareholders(grant_pool);<br/>    vector::for_each_ref(shareholders, &#124;shareholder&#124; &#123;<br/>        let shareholder &#61; &#42;shareholder;<br/>        let shares &#61; pool_u64::shares(grant_pool, shareholder);<br/>        let amount &#61; pool_u64::shares_to_amount_with_total_coins(grant_pool, shares, total_distribution_amount);<br/>        let share_of_coins &#61; coin::extract(&amp;mut coins, amount);<br/>        let recipient_address &#61; get_beneficiary(vesting_contract, shareholder);<br/>        aptos_account::deposit_coins(recipient_address, share_of_coins);<br/>    &#125;);<br/><br/>    // Send any remaining &quot;dust&quot; (leftover due to rounding error) to the withdrawal address.<br/>    if (coin::value(&amp;coins) &gt; 0) &#123;<br/>        aptos_account::deposit_coins(vesting_contract.withdrawal_address, coins);<br/>    &#125; else &#123;<br/>        coin::destroy_zero(coins);<br/>    &#125;;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            Distribute &#123;<br/>                admin: vesting_contract.admin,<br/>                vesting_contract_address: contract_address,<br/>                amount: total_distribution_amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut vesting_contract.distribute_events,<br/>        DistributeEvent &#123;<br/>            admin: vesting_contract.admin,<br/>            vesting_contract_address: contract_address,<br/>            amount: total_distribution_amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_distribute_many"></a>

## Function `distribute_many`

Call <code>distribute</code> for many vesting contracts.


<pre><code>public entry fun distribute_many(contract_addresses: vector&lt;address&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun distribute_many(contract_addresses: vector&lt;address&gt;) acquires VestingContract &#123;<br/>    let len &#61; vector::length(&amp;contract_addresses);<br/><br/>    assert!(len !&#61; 0, error::invalid_argument(EVEC_EMPTY_FOR_MANY_FUNCTION));<br/><br/>    vector::for_each_ref(&amp;contract_addresses, &#124;contract_address&#124; &#123;<br/>        let contract_address &#61; &#42;contract_address;<br/>        distribute(contract_address);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_terminate_vesting_contract"></a>

## Function `terminate_vesting_contract`

Terminate the vesting contract and send all funds back to the withdrawal address.


<pre><code>public entry fun terminate_vesting_contract(admin: &amp;signer, contract_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun terminate_vesting_contract(admin: &amp;signer, contract_address: address) acquires VestingContract &#123;<br/>    assert_active_vesting_contract(contract_address);<br/><br/>    // Distribute all withdrawable coins, which should have been from previous rewards withdrawal or vest.<br/>    distribute(contract_address);<br/><br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/>    let (active_stake, _, pending_active_stake, _) &#61; stake::get_stake(vesting_contract.staking.pool_address);<br/>    assert!(pending_active_stake &#61;&#61; 0, error::invalid_state(EPENDING_STAKE_FOUND));<br/><br/>    // Unlock all remaining active stake.<br/>    vesting_contract.state &#61; VESTING_POOL_TERMINATED;<br/>    vesting_contract.remaining_grant &#61; 0;<br/>    unlock_stake(vesting_contract, active_stake);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            Terminate &#123;<br/>                admin: vesting_contract.admin,<br/>                vesting_contract_address: contract_address,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut vesting_contract.terminate_events,<br/>        TerminateEvent &#123;<br/>            admin: vesting_contract.admin,<br/>            vesting_contract_address: contract_address,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_admin_withdraw"></a>

## Function `admin_withdraw`

Withdraw all funds to the preset vesting contract&apos;s withdrawal address. This can only be called if the contract<br/> has already been terminated.


<pre><code>public entry fun admin_withdraw(admin: &amp;signer, contract_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun admin_withdraw(admin: &amp;signer, contract_address: address) acquires VestingContract &#123;<br/>    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(contract_address);<br/>    assert!(<br/>        vesting_contract.state &#61;&#61; VESTING_POOL_TERMINATED,<br/>        error::invalid_state(EVESTING_CONTRACT_STILL_ACTIVE)<br/>    );<br/><br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/>    let coins &#61; withdraw_stake(vesting_contract, contract_address);<br/>    let amount &#61; coin::value(&amp;coins);<br/>    if (amount &#61;&#61; 0) &#123;<br/>        coin::destroy_zero(coins);<br/>        return<br/>    &#125;;<br/>    aptos_account::deposit_coins(vesting_contract.withdrawal_address, coins);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            AdminWithdraw &#123;<br/>                admin: vesting_contract.admin,<br/>                vesting_contract_address: contract_address,<br/>                amount,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut vesting_contract.admin_withdraw_events,<br/>        AdminWithdrawEvent &#123;<br/>            admin: vesting_contract.admin,<br/>            vesting_contract_address: contract_address,<br/>            amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_update_operator"></a>

## Function `update_operator`



<pre><code>public entry fun update_operator(admin: &amp;signer, contract_address: address, new_operator: address, commission_percentage: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_operator(<br/>    admin: &amp;signer,<br/>    contract_address: address,<br/>    new_operator: address,<br/>    commission_percentage: u64,<br/>) acquires VestingContract &#123;<br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/>    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);<br/>    let old_operator &#61; vesting_contract.staking.operator;<br/>    staking_contract::switch_operator(contract_signer, old_operator, new_operator, commission_percentage);<br/>    vesting_contract.staking.operator &#61; new_operator;<br/>    vesting_contract.staking.commission_percentage &#61; commission_percentage;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            UpdateOperator &#123;<br/>                admin: vesting_contract.admin,<br/>                vesting_contract_address: contract_address,<br/>                staking_pool_address: vesting_contract.staking.pool_address,<br/>                old_operator,<br/>                new_operator,<br/>                commission_percentage,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut vesting_contract.update_operator_events,<br/>        UpdateOperatorEvent &#123;<br/>            admin: vesting_contract.admin,<br/>            vesting_contract_address: contract_address,<br/>            staking_pool_address: vesting_contract.staking.pool_address,<br/>            old_operator,<br/>            new_operator,<br/>            commission_percentage,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_update_operator_with_same_commission"></a>

## Function `update_operator_with_same_commission`



<pre><code>public entry fun update_operator_with_same_commission(admin: &amp;signer, contract_address: address, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_operator_with_same_commission(<br/>    admin: &amp;signer,<br/>    contract_address: address,<br/>    new_operator: address,<br/>) acquires VestingContract &#123;<br/>    let commission_percentage &#61; operator_commission_percentage(contract_address);<br/>    update_operator(admin, contract_address, new_operator, commission_percentage);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_update_commission_percentage"></a>

## Function `update_commission_percentage`



<pre><code>public entry fun update_commission_percentage(admin: &amp;signer, contract_address: address, new_commission_percentage: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_commission_percentage(<br/>    admin: &amp;signer,<br/>    contract_address: address,<br/>    new_commission_percentage: u64,<br/>) acquires VestingContract &#123;<br/>    let operator &#61; operator(contract_address);<br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/>    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);<br/>    staking_contract::update_commision(contract_signer, operator, new_commission_percentage);<br/>    vesting_contract.staking.commission_percentage &#61; new_commission_percentage;<br/>    // This function does not emit an event. Instead, `staking_contract::update_commission_percentage`<br/>    // emits the event for this commission percentage update.<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_update_voter"></a>

## Function `update_voter`



<pre><code>public entry fun update_voter(admin: &amp;signer, contract_address: address, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_voter(<br/>    admin: &amp;signer,<br/>    contract_address: address,<br/>    new_voter: address,<br/>) acquires VestingContract &#123;<br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/>    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);<br/>    let old_voter &#61; vesting_contract.staking.voter;<br/>    staking_contract::update_voter(contract_signer, vesting_contract.staking.operator, new_voter);<br/>    vesting_contract.staking.voter &#61; new_voter;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            UpdateVoter &#123;<br/>                admin: vesting_contract.admin,<br/>                vesting_contract_address: contract_address,<br/>                staking_pool_address: vesting_contract.staking.pool_address,<br/>                old_voter,<br/>                new_voter,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut vesting_contract.update_voter_events,<br/>        UpdateVoterEvent &#123;<br/>            admin: vesting_contract.admin,<br/>            vesting_contract_address: contract_address,<br/>            staking_pool_address: vesting_contract.staking.pool_address,<br/>            old_voter,<br/>            new_voter,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_reset_lockup"></a>

## Function `reset_lockup`



<pre><code>public entry fun reset_lockup(admin: &amp;signer, contract_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reset_lockup(<br/>    admin: &amp;signer,<br/>    contract_address: address,<br/>) acquires VestingContract &#123;<br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/>    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);<br/>    staking_contract::reset_lockup(contract_signer, vesting_contract.staking.operator);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            ResetLockup &#123;<br/>                admin: vesting_contract.admin,<br/>                vesting_contract_address: contract_address,<br/>                staking_pool_address: vesting_contract.staking.pool_address,<br/>                new_lockup_expiration_secs: stake::get_lockup_secs(vesting_contract.staking.pool_address),<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut vesting_contract.reset_lockup_events,<br/>        ResetLockupEvent &#123;<br/>            admin: vesting_contract.admin,<br/>            vesting_contract_address: contract_address,<br/>            staking_pool_address: vesting_contract.staking.pool_address,<br/>            new_lockup_expiration_secs: stake::get_lockup_secs(vesting_contract.staking.pool_address),<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_set_beneficiary"></a>

## Function `set_beneficiary`



<pre><code>public entry fun set_beneficiary(admin: &amp;signer, contract_address: address, shareholder: address, new_beneficiary: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary(<br/>    admin: &amp;signer,<br/>    contract_address: address,<br/>    shareholder: address,<br/>    new_beneficiary: address,<br/>) acquires VestingContract &#123;<br/>    // Verify that the beneficiary account is set up to receive APT. This is a requirement so distribute() wouldn&apos;t<br/>    // fail and block all other accounts from receiving APT if one beneficiary is not registered.<br/>    assert_account_is_registered_for_apt(new_beneficiary);<br/><br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/><br/>    let old_beneficiary &#61; get_beneficiary(vesting_contract, shareholder);<br/>    let beneficiaries &#61; &amp;mut vesting_contract.beneficiaries;<br/>    if (simple_map::contains_key(beneficiaries, &amp;shareholder)) &#123;<br/>        let beneficiary &#61; simple_map::borrow_mut(beneficiaries, &amp;shareholder);<br/>        &#42;beneficiary &#61; new_beneficiary;<br/>    &#125; else &#123;<br/>        simple_map::add(beneficiaries, shareholder, new_beneficiary);<br/>    &#125;;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            SetBeneficiary &#123;<br/>                admin: vesting_contract.admin,<br/>                vesting_contract_address: contract_address,<br/>                shareholder,<br/>                old_beneficiary,<br/>                new_beneficiary,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut vesting_contract.set_beneficiary_events,<br/>        SetBeneficiaryEvent &#123;<br/>            admin: vesting_contract.admin,<br/>            vesting_contract_address: contract_address,<br/>            shareholder,<br/>            old_beneficiary,<br/>            new_beneficiary,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_reset_beneficiary"></a>

## Function `reset_beneficiary`

Remove the beneficiary for the given shareholder. All distributions will sent directly to the shareholder<br/> account.


<pre><code>public entry fun reset_beneficiary(account: &amp;signer, contract_address: address, shareholder: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reset_beneficiary(<br/>    account: &amp;signer,<br/>    contract_address: address,<br/>    shareholder: address,<br/>) acquires VestingAccountManagement, VestingContract &#123;<br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    let addr &#61; signer::address_of(account);<br/>    assert!(<br/>        addr &#61;&#61; vesting_contract.admin &#124;&#124;<br/>            addr &#61;&#61; get_role_holder(contract_address, utf8(ROLE_BENEFICIARY_RESETTER)),<br/>        error::permission_denied(EPERMISSION_DENIED),<br/>    );<br/><br/>    let beneficiaries &#61; &amp;mut vesting_contract.beneficiaries;<br/>    if (simple_map::contains_key(beneficiaries, &amp;shareholder)) &#123;<br/>        simple_map::remove(beneficiaries, &amp;shareholder);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_set_management_role"></a>

## Function `set_management_role`



<pre><code>public entry fun set_management_role(admin: &amp;signer, contract_address: address, role: string::String, role_holder: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_management_role(<br/>    admin: &amp;signer,<br/>    contract_address: address,<br/>    role: String,<br/>    role_holder: address,<br/>) acquires VestingAccountManagement, VestingContract &#123;<br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/><br/>    if (!exists&lt;VestingAccountManagement&gt;(contract_address)) &#123;<br/>        let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);<br/>        move_to(contract_signer, VestingAccountManagement &#123;<br/>            roles: simple_map::create&lt;String, address&gt;(),<br/>        &#125;)<br/>    &#125;;<br/>    let roles &#61; &amp;mut borrow_global_mut&lt;VestingAccountManagement&gt;(contract_address).roles;<br/>    if (simple_map::contains_key(roles, &amp;role)) &#123;<br/>        &#42;simple_map::borrow_mut(roles, &amp;role) &#61; role_holder;<br/>    &#125; else &#123;<br/>        simple_map::add(roles, role, role_holder);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_set_beneficiary_resetter"></a>

## Function `set_beneficiary_resetter`



<pre><code>public entry fun set_beneficiary_resetter(admin: &amp;signer, contract_address: address, beneficiary_resetter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary_resetter(<br/>    admin: &amp;signer,<br/>    contract_address: address,<br/>    beneficiary_resetter: address,<br/>) acquires VestingAccountManagement, VestingContract &#123;<br/>    set_management_role(admin, contract_address, utf8(ROLE_BENEFICIARY_RESETTER), beneficiary_resetter);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Set the beneficiary for the operator.


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary_for_operator(<br/>    operator: &amp;signer,<br/>    new_beneficiary: address,<br/>) &#123;<br/>    staking_contract::set_beneficiary_for_operator(operator, new_beneficiary);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_get_role_holder"></a>

## Function `get_role_holder`



<pre><code>public fun get_role_holder(contract_address: address, role: string::String): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_role_holder(contract_address: address, role: String): address acquires VestingAccountManagement &#123;<br/>    assert!(exists&lt;VestingAccountManagement&gt;(contract_address), error::not_found(EVESTING_ACCOUNT_HAS_NO_ROLES));<br/>    let roles &#61; &amp;borrow_global&lt;VestingAccountManagement&gt;(contract_address).roles;<br/>    assert!(simple_map::contains_key(roles, &amp;role), error::not_found(EROLE_NOT_FOUND));<br/>    &#42;simple_map::borrow(roles, &amp;role)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_get_vesting_account_signer"></a>

## Function `get_vesting_account_signer`

For emergency use in case the admin needs emergency control of vesting contract account.<br/> This doesn&apos;t give the admin total power as the admin would still need to follow the rules set by<br/> staking_contract and stake modules.


<pre><code>public fun get_vesting_account_signer(admin: &amp;signer, contract_address: address): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_vesting_account_signer(admin: &amp;signer, contract_address: address): signer acquires VestingContract &#123;<br/>    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);<br/>    verify_admin(admin, vesting_contract);<br/>    get_vesting_account_signer_internal(vesting_contract)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_get_vesting_account_signer_internal"></a>

## Function `get_vesting_account_signer_internal`



<pre><code>fun get_vesting_account_signer_internal(vesting_contract: &amp;vesting::VestingContract): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_vesting_account_signer_internal(vesting_contract: &amp;VestingContract): signer &#123;<br/>    account::create_signer_with_capability(&amp;vesting_contract.signer_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_create_vesting_contract_account"></a>

## Function `create_vesting_contract_account`

Create a salt for generating the resource accounts that will be holding the VestingContract.<br/> This address should be deterministic for the same admin and vesting contract creation nonce.


<pre><code>fun create_vesting_contract_account(admin: &amp;signer, contract_creation_seed: vector&lt;u8&gt;): (signer, account::SignerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_vesting_contract_account(<br/>    admin: &amp;signer,<br/>    contract_creation_seed: vector&lt;u8&gt;,<br/>): (signer, SignerCapability) acquires AdminStore &#123;<br/>    let admin_store &#61; borrow_global_mut&lt;AdminStore&gt;(signer::address_of(admin));<br/>    let seed &#61; bcs::to_bytes(&amp;signer::address_of(admin));<br/>    vector::append(&amp;mut seed, bcs::to_bytes(&amp;admin_store.nonce));<br/>    admin_store.nonce &#61; admin_store.nonce &#43; 1;<br/><br/>    // Include a salt to avoid conflicts with any other modules out there that might also generate<br/>    // deterministic resource accounts for the same admin address &#43; nonce.<br/>    vector::append(&amp;mut seed, VESTING_POOL_SALT);<br/>    vector::append(&amp;mut seed, contract_creation_seed);<br/><br/>    let (account_signer, signer_cap) &#61; account::create_resource_account(admin, seed);<br/>    // Register the vesting contract account to receive APT as it&apos;ll be sent to it when claiming unlocked stake from<br/>    // the underlying staking contract.<br/>    coin::register&lt;AptosCoin&gt;(&amp;account_signer);<br/><br/>    (account_signer, signer_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_verify_admin"></a>

## Function `verify_admin`



<pre><code>fun verify_admin(admin: &amp;signer, vesting_contract: &amp;vesting::VestingContract)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun verify_admin(admin: &amp;signer, vesting_contract: &amp;VestingContract) &#123;<br/>    assert!(signer::address_of(admin) &#61;&#61; vesting_contract.admin, error::unauthenticated(ENOT_ADMIN));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_assert_vesting_contract_exists"></a>

## Function `assert_vesting_contract_exists`



<pre><code>fun assert_vesting_contract_exists(contract_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_vesting_contract_exists(contract_address: address) &#123;<br/>    assert!(exists&lt;VestingContract&gt;(contract_address), error::not_found(EVESTING_CONTRACT_NOT_FOUND));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_assert_active_vesting_contract"></a>

## Function `assert_active_vesting_contract`



<pre><code>fun assert_active_vesting_contract(contract_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_active_vesting_contract(contract_address: address) acquires VestingContract &#123;<br/>    assert_vesting_contract_exists(contract_address);<br/>    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(contract_address);<br/>    assert!(vesting_contract.state &#61;&#61; VESTING_POOL_ACTIVE, error::invalid_state(EVESTING_CONTRACT_NOT_ACTIVE));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_unlock_stake"></a>

## Function `unlock_stake`



<pre><code>fun unlock_stake(vesting_contract: &amp;vesting::VestingContract, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun unlock_stake(vesting_contract: &amp;VestingContract, amount: u64) &#123;<br/>    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);<br/>    staking_contract::unlock_stake(contract_signer, vesting_contract.staking.operator, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_withdraw_stake"></a>

## Function `withdraw_stake`



<pre><code>fun withdraw_stake(vesting_contract: &amp;vesting::VestingContract, contract_address: address): coin::Coin&lt;aptos_coin::AptosCoin&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun withdraw_stake(vesting_contract: &amp;VestingContract, contract_address: address): Coin&lt;AptosCoin&gt; &#123;<br/>    // Claim any withdrawable distribution from the staking contract. The withdrawn coins will be sent directly to<br/>    // the vesting contract&apos;s account.<br/>    staking_contract::distribute(contract_address, vesting_contract.staking.operator);<br/>    let withdrawn_coins &#61; coin::balance&lt;AptosCoin&gt;(contract_address);<br/>    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);<br/>    coin::withdraw&lt;AptosCoin&gt;(contract_signer, withdrawn_coins)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vesting_get_beneficiary"></a>

## Function `get_beneficiary`



<pre><code>fun get_beneficiary(contract: &amp;vesting::VestingContract, shareholder: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_beneficiary(contract: &amp;VestingContract, shareholder: address): address &#123;<br/>    if (simple_map::contains_key(&amp;contract.beneficiaries, &amp;shareholder)) &#123;<br/>        &#42;simple_map::borrow(&amp;contract.beneficiaries, &amp;shareholder)<br/>    &#125; else &#123;<br/>        shareholder<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;In order to retrieve the address of the underlying stake pool, the vesting start timestamp of the vesting contract, the duration of the vesting period, the remaining grant of a vesting contract, the beneficiary account of a shareholder in a vesting contract, the percentage of accumulated rewards that is paid to the operator as commission, the operator who runs the validator, the voter who will be voting on&#45;chain, and the vesting schedule of a vesting contract, the supplied vesting contract should exist.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The vesting_start_secs, period_duration_secs, remaining_grant, beneficiary, operator_commission_percentage, operator, voter, and vesting_schedule functions ensure that the supplied vesting contract address exists by calling the assert_vesting_contract_exists function.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;assert_vesting_contract_exists&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The vesting pool should not exceed a maximum of 30 shareholders.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The maximum number of shareholders a vesting pool can support is stored as a constant in MAXIMUM_SHAREHOLDERS which is passed to the pool_u64::create function.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via a &lt;a href&#61;&quot;&#35;high&#45;level&#45;spec&#45;2&quot;&gt;global invariant&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;Retrieving all the vesting contracts of a given address and retrieving the list of beneficiaries from a vesting contract should never fail.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The function vesting_contracts checks if the supplied admin address contains an AdminStore resource and returns all the vesting contracts as a vector&lt;address&gt;. Otherwise it returns an empty vector. The function get_beneficiary checks for a given vesting contract, a specific shareholder exists, and if so, the beneficiary will be returned, otherwise it will simply return the address of the shareholder.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;spec&#45;3.1&quot;&gt;vesting_contracts&lt;/a&gt; and &lt;a href&#61;&quot;&#35;high&#45;level&#45;spec&#45;3.2&quot;&gt;get_beneficiary&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The shareholders should be able to start vesting only after the vesting cliff and the first vesting period have transpired.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The end of the vesting cliff is stored under VestingContract.vesting_schedule.start_timestamp_secs. The vest function always checks that timestamp::now_seconds is greater or equal to the end of the vesting cliff period.&lt;/td&gt;<br/>&lt;td&gt;Audited the check for the end of vesting cliff: &lt;a href&#61;&quot;https://github.com/aptos&#45;labs/aptos&#45;core/blob/main/aptos&#45;move/framework/aptos&#45;framework/sources/vesting.move&#35;L566&quot;&gt;vest&lt;/a&gt; module.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;In order to retrieve the total accumulated rewards that have not been distributed, the accumulated rewards of a given beneficiary, the list of al shareholders in a vesting contract,the shareholder address given the beneficiary address in a given vesting contract, to terminate a vesting contract and to distribute any withdrawable stake from the stake pool, the supplied vesting contract should exist and be active.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The distribute, terminate_vesting_contract, shareholder, shareholders, accumulated_rewards, and total_accumulated_rewards functions ensure that the supplied vesting contract address exists and is active by calling the assert_active_vesting_contract function.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;spec&#45;5&quot;&gt;ActiveVestingContractAbortsIf&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;A new vesting schedule should not be allowed to start vesting in the past or to supply an empty schedule or for the period duration to be zero.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The create_vesting_schedule function ensures that the length of the schedule vector is greater than 0, that the period duration is greater than 0 and that the start_timestamp_secs is greater or equal to timestamp::now_seconds.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;6&quot;&gt;create_vesting_schedule&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;The shareholders should be able to vest the tokens from previous periods.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;When vesting, the last_completed_period is checked against the next period to vest. This allows to unlock vested tokens for the next period since last vested, in case they didn&apos;t call vest for some periods.&lt;/td&gt;<br/>&lt;td&gt;Audited that vesting doesn&apos;t skip periods, but gradually increments to allow shareholders to retrieve all the vested tokens.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;8&lt;/td&gt;<br/>&lt;td&gt;Actions such as obtaining a list of shareholders, calculating accrued rewards, distributing withdrawable stake, and terminating the vesting contract should be accessible exclusively while the vesting contract remains active.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;Restricting access to inactive vesting contracts is achieved through the assert_active_vesting_contract function.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;spec&#45;8&quot;&gt;ActiveVestingContractAbortsIf&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;9&lt;/td&gt;<br/>&lt;td&gt;The ability to terminate a vesting contract should only be available to the owner.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;Limiting the access of accounts to specific function, is achieved by asserting that the signer matches the admin of the VestingContract.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;9&quot;&gt;verify_admin&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;10&lt;/td&gt;<br/>&lt;td&gt;A new vesting contract should not be allowed to have an empty list of shareholders, have a different amount of shareholders than buy&#45;ins, and provide a withdrawal address which is either reserved or not registered for apt.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The create_vesting_contract function ensures that the withdrawal_address is not a reserved address, that it is registered for apt, that the list of shareholders is non&#45;empty, and that the amount of shareholders matches the amount of buy_ins.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;10&quot;&gt;create_vesting_contract&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;11&lt;/td&gt;<br/>&lt;td&gt;Creating a vesting contract account should require the signer (admin) to own an admin store and should enforce that the seed of the resource account is composed of the admin store&apos;s nonce, the vesting pool salt, and the custom contract creation seed.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The create_vesting_contract_account concatenates to the seed first the admin_store.nonce then the VESTING_POOL_SALT then the contract_creation_seed and then it is passed to the create_resource_account function.&lt;/td&gt;<br/>&lt;td&gt;Enforced via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;11&quot;&gt;create_vesting_contract_account&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;spec&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
invariant forall a: address where exists&lt;VestingContract&gt;(a):<br/>    global&lt;VestingContract&gt;(a).grant_pool.shareholders_limit &lt;&#61; MAXIMUM_SHAREHOLDERS;<br/></code></pre>



<a id="@Specification_1_stake_pool_address"></a>

### Function `stake_pool_address`


<pre><code>&#35;[view]<br/>public fun stake_pool_address(vesting_contract_address: address): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_vesting_start_secs"></a>

### Function `vesting_start_secs`


<pre><code>&#35;[view]<br/>public fun vesting_start_secs(vesting_contract_address: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_period_duration_secs"></a>

### Function `period_duration_secs`


<pre><code>&#35;[view]<br/>public fun period_duration_secs(vesting_contract_address: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_remaining_grant"></a>

### Function `remaining_grant`


<pre><code>&#35;[view]<br/>public fun remaining_grant(vesting_contract_address: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_beneficiary"></a>

### Function `beneficiary`


<pre><code>&#35;[view]<br/>public fun beneficiary(vesting_contract_address: address, shareholder: address): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_operator_commission_percentage"></a>

### Function `operator_commission_percentage`


<pre><code>&#35;[view]<br/>public fun operator_commission_percentage(vesting_contract_address: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_vesting_contracts"></a>

### Function `vesting_contracts`


<pre><code>&#35;[view]<br/>public fun vesting_contracts(admin: address): vector&lt;address&gt;<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;spec&#45;3.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_operator"></a>

### Function `operator`


<pre><code>&#35;[view]<br/>public fun operator(vesting_contract_address: address): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_voter"></a>

### Function `voter`


<pre><code>&#35;[view]<br/>public fun voter(vesting_contract_address: address): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_vesting_schedule"></a>

### Function `vesting_schedule`


<pre><code>&#35;[view]<br/>public fun vesting_schedule(vesting_contract_address: address): vesting::VestingSchedule<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);<br/></code></pre>



<a id="@Specification_1_total_accumulated_rewards"></a>

### Function `total_accumulated_rewards`


<pre><code>&#35;[view]<br/>public fun total_accumulated_rewards(vesting_contract_address: address): u64<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include TotalAccumulatedRewardsAbortsIf;<br/></code></pre>




<a id="0x1_vesting_TotalAccumulatedRewardsAbortsIf"></a>


<pre><code>schema TotalAccumulatedRewardsAbortsIf &#123;<br/>vesting_contract_address: address;<br/>requires staking_contract.commission_percentage &gt;&#61; 0 &amp;&amp; staking_contract.commission_percentage &lt;&#61; 100;<br/>include ActiveVestingContractAbortsIf&lt;VestingContract&gt;&#123;contract_address: vesting_contract_address&#125;;<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(vesting_contract_address);<br/>let staker &#61; vesting_contract_address;<br/>let operator &#61; vesting_contract.staking.operator;<br/>let staking_contracts &#61; global&lt;staking_contract::Store&gt;(staker).staking_contracts;<br/>let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);<br/>aborts_if !exists&lt;staking_contract::Store&gt;(staker);<br/>aborts_if !simple_map::spec_contains_key(staking_contracts, operator);<br/>let pool_address &#61; staking_contract.pool_address;<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);<br/>let active &#61; coin::value(stake_pool.active);<br/>let pending_active &#61; coin::value(stake_pool.pending_active);<br/>let total_active_stake &#61; active &#43; pending_active;<br/>let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;<br/>let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;<br/>aborts_if !exists&lt;stake::StakePool&gt;(pool_address);<br/>aborts_if active &#43; pending_active &gt; MAX_U64;<br/>aborts_if total_active_stake &lt; staking_contract.principal;<br/>aborts_if accumulated_rewards &#42; staking_contract.commission_percentage &gt; MAX_U64;<br/>aborts_if (vesting_contract.remaining_grant &#43; commission_amount) &gt; total_active_stake;<br/>aborts_if total_active_stake &lt; vesting_contract.remaining_grant;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_accumulated_rewards"></a>

### Function `accumulated_rewards`


<pre><code>&#35;[view]<br/>public fun accumulated_rewards(vesting_contract_address: address, shareholder_or_beneficiary: address): u64<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include TotalAccumulatedRewardsAbortsIf;<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(vesting_contract_address);<br/>let operator &#61; vesting_contract.staking.operator;<br/>let staking_contracts &#61; global&lt;staking_contract::Store&gt;(vesting_contract_address).staking_contracts;<br/>let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);<br/>let pool_address &#61; staking_contract.pool_address;<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);<br/>let active &#61; coin::value(stake_pool.active);<br/>let pending_active &#61; coin::value(stake_pool.pending_active);<br/>let total_active_stake &#61; active &#43; pending_active;<br/>let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;<br/>let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;<br/>let total_accumulated_rewards &#61; total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount;<br/>let shareholder &#61; spec_shareholder(vesting_contract_address, shareholder_or_beneficiary);<br/>let pool &#61; vesting_contract.grant_pool;<br/>let shares &#61; pool_u64::spec_shares(pool, shareholder);<br/>aborts_if pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0<br/>    &amp;&amp; (shares &#42; total_accumulated_rewards) / pool.total_shares &gt; MAX_U64;<br/>ensures result &#61;&#61; pool_u64::spec_shares_to_amount_with_total_coins(pool, shares, total_accumulated_rewards);<br/></code></pre>



<a id="@Specification_1_shareholders"></a>

### Function `shareholders`


<pre><code>&#35;[view]<br/>public fun shareholders(vesting_contract_address: address): vector&lt;address&gt;<br/></code></pre>




<pre><code>include ActiveVestingContractAbortsIf&lt;VestingContract&gt;&#123;contract_address: vesting_contract_address&#125;;<br/></code></pre>




<a id="0x1_vesting_spec_shareholder"></a>


<pre><code>fun spec_shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address;<br/></code></pre>



<a id="@Specification_1_shareholder"></a>

### Function `shareholder`


<pre><code>&#35;[view]<br/>public fun shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address<br/></code></pre>




<pre><code>pragma opaque;<br/>include ActiveVestingContractAbortsIf&lt;VestingContract&gt;&#123;contract_address: vesting_contract_address&#125;;<br/>ensures [abstract] result &#61;&#61; spec_shareholder(vesting_contract_address, shareholder_or_beneficiary);<br/></code></pre>



<a id="@Specification_1_create_vesting_schedule"></a>

### Function `create_vesting_schedule`


<pre><code>public fun create_vesting_schedule(schedule: vector&lt;fixed_point32::FixedPoint32&gt;, start_timestamp_secs: u64, period_duration: u64): vesting::VestingSchedule<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;6&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 6&lt;/a&gt;:
aborts_if !(len(schedule) &gt; 0);<br/>aborts_if !(period_duration &gt; 0);<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>aborts_if !(start_timestamp_secs &gt;&#61; timestamp::now_seconds());<br/></code></pre>



<a id="@Specification_1_create_vesting_contract"></a>

### Function `create_vesting_contract`


<pre><code>public fun create_vesting_contract(admin: &amp;signer, shareholders: &amp;vector&lt;address&gt;, buy_ins: simple_map::SimpleMap&lt;address, coin::Coin&lt;aptos_coin::AptosCoin&gt;&gt;, vesting_schedule: vesting::VestingSchedule, withdrawal_address: address, operator: address, voter: address, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;): address<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;10&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 10&lt;/a&gt;:
aborts_if withdrawal_address &#61;&#61; @aptos_framework &#124;&#124; withdrawal_address &#61;&#61; @vm_reserved;<br/>aborts_if !exists&lt;account::Account&gt;(withdrawal_address);<br/>aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(withdrawal_address);<br/>aborts_if len(shareholders) &#61;&#61; 0;<br/>aborts_if simple_map::spec_len(buy_ins) !&#61; len(shareholders);<br/>ensures global&lt;VestingContract&gt;(result).grant_pool.shareholders_limit &#61;&#61; 30;<br/></code></pre>



<a id="@Specification_1_unlock_rewards"></a>

### Function `unlock_rewards`


<pre><code>public entry fun unlock_rewards(contract_address: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include UnlockRewardsAbortsIf;<br/></code></pre>




<a id="0x1_vesting_UnlockRewardsAbortsIf"></a>


<pre><code>schema UnlockRewardsAbortsIf &#123;<br/>contract_address: address;<br/>include TotalAccumulatedRewardsAbortsIf &#123; vesting_contract_address: contract_address &#125;;<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>let operator &#61; vesting_contract.staking.operator;<br/>let staking_contracts &#61; global&lt;staking_contract::Store&gt;(contract_address).staking_contracts;<br/>let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);<br/>let pool_address &#61; staking_contract.pool_address;<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);<br/>let active &#61; coin::value(stake_pool.active);<br/>let pending_active &#61; coin::value(stake_pool.pending_active);<br/>let total_active_stake &#61; active &#43; pending_active;<br/>let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;<br/>let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;<br/>let amount &#61; total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount;<br/>include UnlockStakeAbortsIf &#123; vesting_contract, amount &#125;;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_unlock_rewards_many"></a>

### Function `unlock_rewards_many`


<pre><code>public entry fun unlock_rewards_many(contract_addresses: vector&lt;address&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>aborts_if len(contract_addresses) &#61;&#61; 0;<br/>include PreconditionAbortsIf;<br/></code></pre>



<a id="@Specification_1_vest"></a>

### Function `vest`


<pre><code>public entry fun vest(contract_address: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include UnlockRewardsAbortsIf;<br/></code></pre>



<a id="@Specification_1_vest_many"></a>

### Function `vest_many`


<pre><code>public entry fun vest_many(contract_addresses: vector&lt;address&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>aborts_if len(contract_addresses) &#61;&#61; 0;<br/>include PreconditionAbortsIf;<br/></code></pre>



<a id="@Specification_1_distribute"></a>

### Function `distribute`


<pre><code>public entry fun distribute(contract_address: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include ActiveVestingContractAbortsIf&lt;VestingContract&gt;;<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>include WithdrawStakeAbortsIf &#123; vesting_contract &#125;;<br/></code></pre>



<a id="@Specification_1_distribute_many"></a>

### Function `distribute_many`


<pre><code>public entry fun distribute_many(contract_addresses: vector&lt;address&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>aborts_if len(contract_addresses) &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_terminate_vesting_contract"></a>

### Function `terminate_vesting_contract`


<pre><code>public entry fun terminate_vesting_contract(admin: &amp;signer, contract_address: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include ActiveVestingContractAbortsIf&lt;VestingContract&gt;;<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>include WithdrawStakeAbortsIf &#123; vesting_contract &#125;;<br/></code></pre>



<a id="@Specification_1_admin_withdraw"></a>

### Function `admin_withdraw`


<pre><code>public entry fun admin_withdraw(admin: &amp;signer, contract_address: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>aborts_if vesting_contract.state !&#61; VESTING_POOL_TERMINATED;<br/>include VerifyAdminAbortsIf;<br/>include WithdrawStakeAbortsIf &#123; vesting_contract &#125;;<br/></code></pre>



<a id="@Specification_1_update_operator"></a>

### Function `update_operator`


<pre><code>public entry fun update_operator(admin: &amp;signer, contract_address: address, new_operator: address, commission_percentage: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include VerifyAdminAbortsIf;<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>let acc &#61; vesting_contract.signer_cap.account;<br/>let old_operator &#61; vesting_contract.staking.operator;<br/>include staking_contract::ContractExistsAbortsIf &#123; staker: acc, operator: old_operator &#125;;<br/>let store &#61; global&lt;staking_contract::Store&gt;(acc);<br/>let staking_contracts &#61; store.staking_contracts;<br/>aborts_if simple_map::spec_contains_key(staking_contracts, new_operator);<br/>let staking_contract &#61; simple_map::spec_get(staking_contracts, old_operator);<br/>include DistributeInternalAbortsIf &#123; staker: acc, operator: old_operator, staking_contract, distribute_events: store.distribute_events &#125;;<br/></code></pre>



<a id="@Specification_1_update_operator_with_same_commission"></a>

### Function `update_operator_with_same_commission`


<pre><code>public entry fun update_operator_with_same_commission(admin: &amp;signer, contract_address: address, new_operator: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_update_commission_percentage"></a>

### Function `update_commission_percentage`


<pre><code>public entry fun update_commission_percentage(admin: &amp;signer, contract_address: address, new_commission_percentage: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_update_voter"></a>

### Function `update_voter`


<pre><code>public entry fun update_voter(admin: &amp;signer, contract_address: address, new_voter: address)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>include VerifyAdminAbortsIf;<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>let operator &#61; vesting_contract.staking.operator;<br/>let staker &#61; vesting_contract.signer_cap.account;<br/>include staking_contract::UpdateVoterSchema;<br/></code></pre>



<a id="@Specification_1_reset_lockup"></a>

### Function `reset_lockup`


<pre><code>public entry fun reset_lockup(admin: &amp;signer, contract_address: address)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>aborts_if !exists&lt;VestingContract&gt;(contract_address);<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>aborts_if signer::address_of(admin) !&#61; vesting_contract.admin;<br/>let operator &#61; vesting_contract.staking.operator;<br/>let staker &#61; vesting_contract.signer_cap.account;<br/>include staking_contract::ContractExistsAbortsIf &#123;staker, operator&#125;;<br/>include staking_contract::IncreaseLockupWithCapAbortsIf &#123;staker, operator&#125;;<br/>let store &#61; global&lt;staking_contract::Store&gt;(staker);<br/>let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);<br/>let pool_address &#61; staking_contract.owner_cap.pool_address;<br/>aborts_if !exists&lt;stake::StakePool&gt;(vesting_contract.staking.pool_address);<br/></code></pre>



<a id="@Specification_1_set_beneficiary"></a>

### Function `set_beneficiary`


<pre><code>public entry fun set_beneficiary(admin: &amp;signer, contract_address: address, shareholder: address, new_beneficiary: address)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>pragma aborts_if_is_partial;<br/>aborts_if !account::exists_at(new_beneficiary);<br/>aborts_if !coin::spec_is_account_registered&lt;AptosCoin&gt;(new_beneficiary);<br/>include VerifyAdminAbortsIf;<br/>let post vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>ensures simple_map::spec_contains_key(vesting_contract.beneficiaries,shareholder);<br/></code></pre>



<a id="@Specification_1_reset_beneficiary"></a>

### Function `reset_beneficiary`


<pre><code>public entry fun reset_beneficiary(account: &amp;signer, contract_address: address, shareholder: address)<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(contract_address);<br/>let addr &#61; signer::address_of(account);<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>aborts_if addr !&#61; vesting_contract.admin &amp;&amp; !std::string::spec_internal_check_utf8(ROLE_BENEFICIARY_RESETTER);<br/>aborts_if addr !&#61; vesting_contract.admin &amp;&amp; !exists&lt;VestingAccountManagement&gt;(contract_address);<br/>let roles &#61; global&lt;VestingAccountManagement&gt;(contract_address).roles;<br/>let role &#61; std::string::spec_utf8(ROLE_BENEFICIARY_RESETTER);<br/>aborts_if addr !&#61; vesting_contract.admin &amp;&amp; !simple_map::spec_contains_key(roles, role);<br/>aborts_if addr !&#61; vesting_contract.admin &amp;&amp; addr !&#61; simple_map::spec_get(roles, role);<br/>let post post_vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>ensures !simple_map::spec_contains_key(post_vesting_contract.beneficiaries,shareholder);<br/></code></pre>



<a id="@Specification_1_set_management_role"></a>

### Function `set_management_role`


<pre><code>public entry fun set_management_role(admin: &amp;signer, contract_address: address, role: string::String, role_holder: address)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>include SetManagementRoleAbortsIf;<br/></code></pre>



<a id="@Specification_1_set_beneficiary_resetter"></a>

### Function `set_beneficiary_resetter`


<pre><code>public entry fun set_beneficiary_resetter(admin: &amp;signer, contract_address: address, beneficiary_resetter: address)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>aborts_if !std::string::spec_internal_check_utf8(ROLE_BENEFICIARY_RESETTER);<br/>include SetManagementRoleAbortsIf;<br/></code></pre>



<a id="@Specification_1_set_beneficiary_for_operator"></a>

### Function `set_beneficiary_for_operator`


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_get_role_holder"></a>

### Function `get_role_holder`


<pre><code>public fun get_role_holder(contract_address: address, role: string::String): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;VestingAccountManagement&gt;(contract_address);<br/>let roles &#61; global&lt;VestingAccountManagement&gt;(contract_address).roles;<br/>aborts_if !simple_map::spec_contains_key(roles,role);<br/></code></pre>



<a id="@Specification_1_get_vesting_account_signer"></a>

### Function `get_vesting_account_signer`


<pre><code>public fun get_vesting_account_signer(admin: &amp;signer, contract_address: address): signer<br/></code></pre>




<pre><code>include VerifyAdminAbortsIf;<br/></code></pre>



<a id="@Specification_1_get_vesting_account_signer_internal"></a>

### Function `get_vesting_account_signer_internal`


<pre><code>fun get_vesting_account_signer_internal(vesting_contract: &amp;vesting::VestingContract): signer<br/></code></pre>




<pre><code>aborts_if false;<br/></code></pre>




<a id="0x1_vesting_spec_get_vesting_account_signer"></a>


<pre><code>fun spec_get_vesting_account_signer(vesting_contract: VestingContract): signer;<br/></code></pre>



<a id="@Specification_1_create_vesting_contract_account"></a>

### Function `create_vesting_contract_account`


<pre><code>fun create_vesting_contract_account(admin: &amp;signer, contract_creation_seed: vector&lt;u8&gt;): (signer, account::SignerCapability)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>let admin_addr &#61; signer::address_of(admin);<br/>let admin_store &#61; global&lt;AdminStore&gt;(admin_addr);<br/>let seed &#61; bcs::to_bytes(admin_addr);<br/>let nonce &#61; bcs::to_bytes(admin_store.nonce);<br/>let first &#61; concat(seed, nonce);<br/>let second &#61; concat(first, VESTING_POOL_SALT);<br/>let end &#61; concat(second, contract_creation_seed);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;11&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 11&lt;/a&gt;:
let resource_addr &#61; account::spec_create_resource_address(admin_addr, end);<br/>aborts_if !exists&lt;AdminStore&gt;(admin_addr);<br/>aborts_if len(account::ZERO_AUTH_KEY) !&#61; 32;<br/>aborts_if admin_store.nonce &#43; 1 &gt; MAX_U64;<br/>let ea &#61; account::exists_at(resource_addr);<br/>include if (ea) account::CreateResourceAccountAbortsIf else account::CreateAccountAbortsIf &#123;addr: resource_addr&#125;;<br/>let acc &#61; global&lt;account::Account&gt;(resource_addr);<br/>let post post_acc &#61; global&lt;account::Account&gt;(resource_addr);<br/>aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; !aptos_std::type_info::spec_is_struct&lt;AptosCoin&gt;();<br/>aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; ea &amp;&amp; acc.guid_creation_num &#43; 2 &gt; MAX_U64;<br/>aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; ea &amp;&amp; acc.guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>ensures exists&lt;account::Account&gt;(resource_addr) &amp;&amp; post_acc.authentication_key &#61;&#61; account::ZERO_AUTH_KEY &amp;&amp;<br/>        exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr);<br/>ensures signer::address_of(result_1) &#61;&#61; resource_addr;<br/>ensures result_2.account &#61;&#61; resource_addr;<br/></code></pre>



<a id="@Specification_1_verify_admin"></a>

### Function `verify_admin`


<pre><code>fun verify_admin(admin: &amp;signer, vesting_contract: &amp;vesting::VestingContract)<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;9&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 9&lt;/a&gt;:
aborts_if signer::address_of(admin) !&#61; vesting_contract.admin;<br/></code></pre>



<a id="@Specification_1_assert_vesting_contract_exists"></a>

### Function `assert_vesting_contract_exists`


<pre><code>fun assert_vesting_contract_exists(contract_address: address)<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if !exists&lt;VestingContract&gt;(contract_address);<br/></code></pre>



<a id="@Specification_1_assert_active_vesting_contract"></a>

### Function `assert_active_vesting_contract`


<pre><code>fun assert_active_vesting_contract(contract_address: address)<br/></code></pre>




<pre><code>include ActiveVestingContractAbortsIf&lt;VestingContract&gt;;<br/></code></pre>



<a id="@Specification_1_unlock_stake"></a>

### Function `unlock_stake`


<pre><code>fun unlock_stake(vesting_contract: &amp;vesting::VestingContract, amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include UnlockStakeAbortsIf;<br/></code></pre>




<a id="0x1_vesting_UnlockStakeAbortsIf"></a>


<pre><code>schema UnlockStakeAbortsIf &#123;<br/>vesting_contract: &amp;VestingContract;<br/>amount: u64;<br/>let acc &#61; vesting_contract.signer_cap.account;<br/>let operator &#61; vesting_contract.staking.operator;<br/>include amount !&#61; 0 &#61;&#61;&gt; staking_contract::ContractExistsAbortsIf &#123; staker: acc, operator &#125;;<br/>let store &#61; global&lt;staking_contract::Store&gt;(acc);<br/>let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);<br/>include amount !&#61; 0 &#61;&#61;&gt; DistributeInternalAbortsIf &#123; staker: acc, operator, staking_contract, distribute_events: store.distribute_events &#125;;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_withdraw_stake"></a>

### Function `withdraw_stake`


<pre><code>fun withdraw_stake(vesting_contract: &amp;vesting::VestingContract, contract_address: address): coin::Coin&lt;aptos_coin::AptosCoin&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include WithdrawStakeAbortsIf;<br/></code></pre>




<a id="0x1_vesting_WithdrawStakeAbortsIf"></a>


<pre><code>schema WithdrawStakeAbortsIf &#123;<br/>vesting_contract: &amp;VestingContract;<br/>contract_address: address;<br/>let operator &#61; vesting_contract.staking.operator;<br/>include staking_contract::ContractExistsAbortsIf &#123; staker: contract_address, operator &#125;;<br/>let store &#61; global&lt;staking_contract::Store&gt;(contract_address);<br/>let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);<br/>include DistributeInternalAbortsIf &#123; staker: contract_address, operator, staking_contract, distribute_events: store.distribute_events &#125;;<br/>&#125;<br/></code></pre>




<a id="0x1_vesting_DistributeInternalAbortsIf"></a>


<pre><code>schema DistributeInternalAbortsIf &#123;<br/>staker: address;<br/>operator: address;<br/>staking_contract: staking_contract::StakingContract;<br/>distribute_events: EventHandle&lt;staking_contract::DistributeEvent&gt;;<br/>let pool_address &#61; staking_contract.pool_address;<br/>aborts_if !exists&lt;stake::StakePool&gt;(pool_address);<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);<br/>let inactive &#61; stake_pool.inactive.value;<br/>let pending_inactive &#61; stake_pool.pending_inactive.value;<br/>aborts_if inactive &#43; pending_inactive &gt; MAX_U64;<br/>let total_potential_withdrawable &#61; inactive &#43; pending_inactive;<br/>let pool_address_1 &#61; staking_contract.owner_cap.pool_address;<br/>aborts_if !exists&lt;stake::StakePool&gt;(pool_address_1);<br/>let stake_pool_1 &#61; global&lt;stake::StakePool&gt;(pool_address_1);<br/>aborts_if !exists&lt;stake::ValidatorSet&gt;(@aptos_framework);<br/>let validator_set &#61; global&lt;stake::ValidatorSet&gt;(@aptos_framework);<br/>let inactive_state &#61; !stake::spec_contains(validator_set.pending_active, pool_address_1)<br/>    &amp;&amp; !stake::spec_contains(validator_set.active_validators, pool_address_1)<br/>    &amp;&amp; !stake::spec_contains(validator_set.pending_inactive, pool_address_1);<br/>let inactive_1 &#61; stake_pool_1.inactive.value;<br/>let pending_inactive_1 &#61; stake_pool_1.pending_inactive.value;<br/>let new_inactive_1 &#61; inactive_1 &#43; pending_inactive_1;<br/>aborts_if inactive_state &amp;&amp; timestamp::spec_now_seconds() &gt;&#61; stake_pool_1.locked_until_secs<br/>    &amp;&amp; inactive_1 &#43; pending_inactive_1 &gt; MAX_U64;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_beneficiary"></a>

### Function `get_beneficiary`


<pre><code>fun get_beneficiary(contract: &amp;vesting::VestingContract, shareholder: address): address<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;spec&#45;3.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
aborts_if false;<br/></code></pre>




<a id="0x1_vesting_SetManagementRoleAbortsIf"></a>


<pre><code>schema SetManagementRoleAbortsIf &#123;<br/>contract_address: address;<br/>admin: signer;<br/>aborts_if !exists&lt;VestingContract&gt;(contract_address);<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>aborts_if signer::address_of(admin) !&#61; vesting_contract.admin;<br/>&#125;<br/></code></pre>




<a id="0x1_vesting_VerifyAdminAbortsIf"></a>


<pre><code>schema VerifyAdminAbortsIf &#123;<br/>contract_address: address;<br/>admin: signer;<br/>aborts_if !exists&lt;VestingContract&gt;(contract_address);<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>aborts_if signer::address_of(admin) !&#61; vesting_contract.admin;<br/>&#125;<br/></code></pre>




<a id="0x1_vesting_ActiveVestingContractAbortsIf"></a>


<pre><code>schema ActiveVestingContractAbortsIf&lt;VestingContract&gt; &#123;<br/>contract_address: address;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;spec&#45;5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
    aborts_if !exists&lt;VestingContract&gt;(contract_address);<br/>let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;spec&#45;8&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 8&lt;/a&gt;:
    aborts_if vesting_contract.state !&#61; VESTING_POOL_ACTIVE;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
