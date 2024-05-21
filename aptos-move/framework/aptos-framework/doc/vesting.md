
<a id="0x1_vesting"></a>

# Module `0x1::vesting`


Simple vesting contract that allows specifying how much APT coins should be vesting in each fixed-size period. The
vesting contract also comes with staking and allows shareholders to withdraw rewards anytime.

Vesting schedule is represented as a vector of distributions. For example, a vesting schedule of
[3/48, 3/48, 1/48] means that after the vesting starts:
1. The first and second periods will vest 3/48 of the total original grant.
2. The third period will vest 1/48.
3. All subsequent periods will also vest 1/48 (last distribution in the schedule) until the original grant runs out.

Shareholder flow:
1. Admin calls create_vesting_contract with a schedule of [3/48, 3/48, 1/48] with a vesting cliff of 1 year and
vesting period of 1 month.
2. After a month, a shareholder calls unlock_rewards to request rewards. They can also call vest() which would also
unlocks rewards but since the 1 year cliff has not passed (vesting has not started), vest() would not release any of
the original grant.
3. After the unlocked rewards become fully withdrawable (as it's subject to staking lockup), shareholders can call
distribute() to send all withdrawable funds to all shareholders based on the original grant's shares structure.
4. After 1 year and 1 month, the vesting schedule now starts. Shareholders call vest() to unlock vested coins. vest()
checks the schedule and unlocks 3/48 of the original grant in addition to any accumulated rewards since last
unlock_rewards(). Once the unlocked coins become withdrawable, shareholders can call distribute().
5. Assuming the shareholders forgot to call vest() for 2 months, when they call vest() again, they will unlock vested
tokens for the next period since last vest. This would be for the first month they missed. They can call vest() a
second time to unlock for the second month they missed.

Admin flow:
1. After creating the vesting contract, admin cannot change the vesting schedule.
2. Admin can call update_voter, update_operator, or reset_lockup at any time to update the underlying staking
contract.
3. Admin can also call update_beneficiary for any shareholder. This would send all distributions (rewards, vested
coins) of that shareholder to the beneficiary account. By defalt, if a beneficiary is not set, the distributions are
send directly to the shareholder account.
4. Admin can call terminate_vesting_contract to terminate the vesting. This would first finish any distribution but
will prevent any further rewards or vesting distributions from being created. Once the locked up stake becomes
withdrawable, admin can call admin_withdraw to withdraw all funds to the vesting contract's withdrawal address.


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


<pre><code>use 0x1::account;
use 0x1::aptos_account;
use 0x1::aptos_coin;
use 0x1::bcs;
use 0x1::coin;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::fixed_point32;
use 0x1::math64;
use 0x1::pool_u64;
use 0x1::signer;
use 0x1::simple_map;
use 0x1::stake;
use 0x1::staking_contract;
use 0x1::string;
use 0x1::system_addresses;
use 0x1::timestamp;
use 0x1::vector;
</code></pre>



<a id="0x1_vesting_VestingSchedule"></a>

## Struct `VestingSchedule`



<pre><code>struct VestingSchedule has copy, drop, store
</code></pre>



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



<pre><code>struct StakingInfo has store
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



<pre><code>struct VestingContract has key
</code></pre>



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



<pre><code>struct VestingAccountManagement has key
</code></pre>



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



<pre><code>struct AdminStore has key
</code></pre>



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



<pre><code>&#35;[event]
struct CreateVestingContract has drop, store
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



<pre><code>&#35;[event]
struct UpdateOperator has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct UpdateVoter has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct ResetLockup has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct SetBeneficiary has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct UnlockRewards has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct Vest has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct Distribute has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct Terminate has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct AdminWithdraw has drop, store
</code></pre>



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



<pre><code>struct CreateVestingContractEvent has drop, store
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



<pre><code>struct UpdateOperatorEvent has drop, store
</code></pre>



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



<pre><code>struct UpdateVoterEvent has drop, store
</code></pre>



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



<pre><code>struct ResetLockupEvent has drop, store
</code></pre>



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



<pre><code>struct SetBeneficiaryEvent has drop, store
</code></pre>



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



<pre><code>struct UnlockRewardsEvent has drop, store
</code></pre>



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



<pre><code>struct VestEvent has drop, store
</code></pre>



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



<pre><code>struct DistributeEvent has drop, store
</code></pre>



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



<pre><code>struct TerminateEvent has drop, store
</code></pre>



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



<pre><code>struct AdminWithdrawEvent has drop, store
</code></pre>



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


<pre><code>const EEMPTY_VESTING_SCHEDULE: u64 &#61; 2;
</code></pre>



<a id="0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS"></a>

Withdrawal address is invalid.


<pre><code>const EINVALID_WITHDRAWAL_ADDRESS: u64 &#61; 1;
</code></pre>



<a id="0x1_vesting_ENOT_ADMIN"></a>

The signer is not the admin of the vesting contract.


<pre><code>const ENOT_ADMIN: u64 &#61; 7;
</code></pre>



<a id="0x1_vesting_ENO_SHAREHOLDERS"></a>

Shareholders list cannot be empty.


<pre><code>const ENO_SHAREHOLDERS: u64 &#61; 4;
</code></pre>



<a id="0x1_vesting_EPENDING_STAKE_FOUND"></a>

Cannot terminate the vesting contract with pending active stake. Need to wait until next epoch.


<pre><code>const EPENDING_STAKE_FOUND: u64 &#61; 11;
</code></pre>



<a id="0x1_vesting_EPERMISSION_DENIED"></a>

Account is not admin or does not have the required role to take this action.


<pre><code>const EPERMISSION_DENIED: u64 &#61; 15;
</code></pre>



<a id="0x1_vesting_EROLE_NOT_FOUND"></a>

The vesting account has no such management role.


<pre><code>const EROLE_NOT_FOUND: u64 &#61; 14;
</code></pre>



<a id="0x1_vesting_ESHARES_LENGTH_MISMATCH"></a>

The length of shareholders and shares lists don't match.


<pre><code>const ESHARES_LENGTH_MISMATCH: u64 &#61; 5;
</code></pre>



<a id="0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION"></a>

Zero items were provided to a *_many function.


<pre><code>const EVEC_EMPTY_FOR_MANY_FUNCTION: u64 &#61; 16;
</code></pre>



<a id="0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES"></a>

Vesting account has no other management roles beside admin.


<pre><code>const EVESTING_ACCOUNT_HAS_NO_ROLES: u64 &#61; 13;
</code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE"></a>

Vesting contract needs to be in active state.


<pre><code>const EVESTING_CONTRACT_NOT_ACTIVE: u64 &#61; 8;
</code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_NOT_FOUND"></a>

No vesting contract found at provided address.


<pre><code>const EVESTING_CONTRACT_NOT_FOUND: u64 &#61; 10;
</code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE"></a>

Admin can only withdraw from an inactive (paused or terminated) vesting contract.


<pre><code>const EVESTING_CONTRACT_STILL_ACTIVE: u64 &#61; 9;
</code></pre>



<a id="0x1_vesting_EVESTING_START_TOO_SOON"></a>

Vesting cannot start before or at the current block timestamp. Has to be in the future.


<pre><code>const EVESTING_START_TOO_SOON: u64 &#61; 6;
</code></pre>



<a id="0x1_vesting_EZERO_GRANT"></a>

Grant amount cannot be 0.


<pre><code>const EZERO_GRANT: u64 &#61; 12;
</code></pre>



<a id="0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD"></a>

Vesting period cannot be 0.


<pre><code>const EZERO_VESTING_SCHEDULE_PERIOD: u64 &#61; 3;
</code></pre>



<a id="0x1_vesting_MAXIMUM_SHAREHOLDERS"></a>

Maximum number of shareholders a vesting pool can support.


<pre><code>const MAXIMUM_SHAREHOLDERS: u64 &#61; 30;
</code></pre>



<a id="0x1_vesting_ROLE_BENEFICIARY_RESETTER"></a>

Roles that can manage certain aspects of the vesting account beyond the main admin.


<pre><code>const ROLE_BENEFICIARY_RESETTER: vector&lt;u8&gt; &#61; [82, 79, 76, 69, 95, 66, 69, 78, 69, 70, 73, 67, 73, 65, 82, 89, 95, 82, 69, 83, 69, 84, 84, 69, 82];
</code></pre>



<a id="0x1_vesting_VESTING_POOL_ACTIVE"></a>

Vesting contract states.
Vesting contract is active and distributions can be made.


<pre><code>const VESTING_POOL_ACTIVE: u64 &#61; 1;
</code></pre>



<a id="0x1_vesting_VESTING_POOL_SALT"></a>



<pre><code>const VESTING_POOL_SALT: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 118, 101, 115, 116, 105, 110, 103];
</code></pre>



<a id="0x1_vesting_VESTING_POOL_TERMINATED"></a>

Vesting contract has been terminated and all funds have been released back to the withdrawal address.


<pre><code>const VESTING_POOL_TERMINATED: u64 &#61; 2;
</code></pre>



<a id="0x1_vesting_stake_pool_address"></a>

## Function `stake_pool_address`

Return the address of the underlying stake pool (separate resource account) of the vesting contract.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun stake_pool_address(vesting_contract_address: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun stake_pool_address(vesting_contract_address: address): address acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    borrow_global&lt;VestingContract&gt;(vesting_contract_address).staking.pool_address
&#125;
</code></pre>



</details>

<a id="0x1_vesting_vesting_start_secs"></a>

## Function `vesting_start_secs`

Return the vesting start timestamp (in seconds) of the vesting contract.
Vesting will start at this time, and once a full period has passed, the first vest will become unlocked.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun vesting_start_secs(vesting_contract_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vesting_start_secs(vesting_contract_address: address): u64 acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    borrow_global&lt;VestingContract&gt;(vesting_contract_address).vesting_schedule.start_timestamp_secs
&#125;
</code></pre>



</details>

<a id="0x1_vesting_period_duration_secs"></a>

## Function `period_duration_secs`

Return the duration of one vesting period (in seconds).
Each vest is released after one full period has started, starting from the specified start_timestamp_secs.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun period_duration_secs(vesting_contract_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun period_duration_secs(vesting_contract_address: address): u64 acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    borrow_global&lt;VestingContract&gt;(vesting_contract_address).vesting_schedule.period_duration
&#125;
</code></pre>



</details>

<a id="0x1_vesting_remaining_grant"></a>

## Function `remaining_grant`

Return the remaining grant, consisting of unvested coins that have not been distributed to shareholders.
Prior to start_timestamp_secs, the remaining grant will always be equal to the original grant.
Once vesting has started, and vested tokens are distributed, the remaining grant will decrease over time,
according to the vesting schedule.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun remaining_grant(vesting_contract_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remaining_grant(vesting_contract_address: address): u64 acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    borrow_global&lt;VestingContract&gt;(vesting_contract_address).remaining_grant
&#125;
</code></pre>



</details>

<a id="0x1_vesting_beneficiary"></a>

## Function `beneficiary`

Return the beneficiary account of the specified shareholder in a vesting contract.
This is the same as the shareholder address by default and only different if it's been explicitly set.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun beneficiary(vesting_contract_address: address, shareholder: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun beneficiary(vesting_contract_address: address, shareholder: address): address acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    get_beneficiary(borrow_global&lt;VestingContract&gt;(vesting_contract_address), shareholder)
&#125;
</code></pre>



</details>

<a id="0x1_vesting_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the percentage of accumulated rewards that is paid to the operator as commission.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun operator_commission_percentage(vesting_contract_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator_commission_percentage(vesting_contract_address: address): u64 acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    borrow_global&lt;VestingContract&gt;(vesting_contract_address).staking.commission_percentage
&#125;
</code></pre>



</details>

<a id="0x1_vesting_vesting_contracts"></a>

## Function `vesting_contracts`

Return all the vesting contracts a given address is an admin of.


<pre><code>&#35;[view]
public fun vesting_contracts(admin: address): vector&lt;address&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vesting_contracts(admin: address): vector&lt;address&gt; acquires AdminStore &#123;
    if (!exists&lt;AdminStore&gt;(admin)) &#123;
        vector::empty&lt;address&gt;()
    &#125; else &#123;
        borrow_global&lt;AdminStore&gt;(admin).vesting_contracts
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_vesting_operator"></a>

## Function `operator`

Return the operator who runs the validator for the vesting contract.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun operator(vesting_contract_address: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator(vesting_contract_address: address): address acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    borrow_global&lt;VestingContract&gt;(vesting_contract_address).staking.operator
&#125;
</code></pre>



</details>

<a id="0x1_vesting_voter"></a>

## Function `voter`

Return the voter who will be voting on on-chain governance proposals on behalf of the vesting contract's stake
pool.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun voter(vesting_contract_address: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun voter(vesting_contract_address: address): address acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    borrow_global&lt;VestingContract&gt;(vesting_contract_address).staking.voter
&#125;
</code></pre>



</details>

<a id="0x1_vesting_vesting_schedule"></a>

## Function `vesting_schedule`

Return the vesting contract's vesting schedule. The core schedule is represented as a list of u64-based
fractions, where the rightmmost 32 bits can be divided by 2^32 to get the fraction, and anything else is the
whole number.

For example 3/48, or 0.0625, will be represented as 268435456. The fractional portion would be
268435456 / 2^32 = 0.0625. Since there are fewer than 32 bits, the whole number portion is effectively 0.
So 268435456 = 0.0625.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun vesting_schedule(vesting_contract_address: address): vesting::VestingSchedule
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vesting_schedule(vesting_contract_address: address): VestingSchedule acquires VestingContract &#123;
    assert_vesting_contract_exists(vesting_contract_address);
    borrow_global&lt;VestingContract&gt;(vesting_contract_address).vesting_schedule
&#125;
</code></pre>



</details>

<a id="0x1_vesting_total_accumulated_rewards"></a>

## Function `total_accumulated_rewards`

Return the total accumulated rewards that have not been distributed to shareholders of the vesting contract.
This excludes any unpaid commission that the operator has not collected.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun total_accumulated_rewards(vesting_contract_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun total_accumulated_rewards(vesting_contract_address: address): u64 acquires VestingContract &#123;
    assert_active_vesting_contract(vesting_contract_address);

    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(vesting_contract_address);
    let (total_active_stake, _, commission_amount) &#61;
        staking_contract::staking_contract_amounts(vesting_contract_address, vesting_contract.staking.operator);
    total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount
&#125;
</code></pre>



</details>

<a id="0x1_vesting_accumulated_rewards"></a>

## Function `accumulated_rewards`

Return the accumulated rewards that have not been distributed to the provided shareholder. Caller can also pass
the beneficiary address instead of shareholder address.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>&#35;[view]
public fun accumulated_rewards(vesting_contract_address: address, shareholder_or_beneficiary: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun accumulated_rewards(
    vesting_contract_address: address, shareholder_or_beneficiary: address): u64 acquires VestingContract &#123;
    assert_active_vesting_contract(vesting_contract_address);

    let total_accumulated_rewards &#61; total_accumulated_rewards(vesting_contract_address);
    let shareholder &#61; shareholder(vesting_contract_address, shareholder_or_beneficiary);
    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(vesting_contract_address);
    let shares &#61; pool_u64::shares(&amp;vesting_contract.grant_pool, shareholder);
    pool_u64::shares_to_amount_with_total_coins(&amp;vesting_contract.grant_pool, shares, total_accumulated_rewards)
&#125;
</code></pre>



</details>

<a id="0x1_vesting_shareholders"></a>

## Function `shareholders`

Return the list of all shareholders in the vesting contract.


<pre><code>&#35;[view]
public fun shareholders(vesting_contract_address: address): vector&lt;address&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shareholders(vesting_contract_address: address): vector&lt;address&gt; acquires VestingContract &#123;
    assert_active_vesting_contract(vesting_contract_address);

    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(vesting_contract_address);
    pool_u64::shareholders(&amp;vesting_contract.grant_pool)
&#125;
</code></pre>



</details>

<a id="0x1_vesting_shareholder"></a>

## Function `shareholder`

Return the shareholder address given the beneficiary address in a given vesting contract. If there are multiple
shareholders with the same beneficiary address, only the first shareholder is returned. If the given beneficiary
address is actually a shareholder address, just return the address back.

This returns 0x0 if no shareholder is found for the given beneficiary / the address is not a shareholder itself.


<pre><code>&#35;[view]
public fun shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shareholder(
    vesting_contract_address: address,
    shareholder_or_beneficiary: address
): address acquires VestingContract &#123;
    assert_active_vesting_contract(vesting_contract_address);

    let shareholders &#61; &amp;shareholders(vesting_contract_address);
    if (vector::contains(shareholders, &amp;shareholder_or_beneficiary)) &#123;
        return shareholder_or_beneficiary
    &#125;;
    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(vesting_contract_address);
    let result &#61; @0x0;
    vector::any(shareholders, &#124;shareholder&#124; &#123;
        if (shareholder_or_beneficiary &#61;&#61; get_beneficiary(vesting_contract, &#42;shareholder)) &#123;
            result &#61; &#42;shareholder;
            true
        &#125; else &#123;
            false
        &#125;
    &#125;);

    result
&#125;
</code></pre>



</details>

<a id="0x1_vesting_create_vesting_schedule"></a>

## Function `create_vesting_schedule`

Create a vesting schedule with the given schedule of distributions, a vesting start time and period duration.


<pre><code>public fun create_vesting_schedule(schedule: vector&lt;fixed_point32::FixedPoint32&gt;, start_timestamp_secs: u64, period_duration: u64): vesting::VestingSchedule
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_vesting_schedule(
    schedule: vector&lt;FixedPoint32&gt;,
    start_timestamp_secs: u64,
    period_duration: u64,
): VestingSchedule &#123;
    assert!(vector::length(&amp;schedule) &gt; 0, error::invalid_argument(EEMPTY_VESTING_SCHEDULE));
    assert!(period_duration &gt; 0, error::invalid_argument(EZERO_VESTING_SCHEDULE_PERIOD));
    assert!(
        start_timestamp_secs &gt;&#61; timestamp::now_seconds(),
        error::invalid_argument(EVESTING_START_TOO_SOON),
    );

    VestingSchedule &#123;
        schedule,
        start_timestamp_secs,
        period_duration,
        last_vested_period: 0,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_vesting_create_vesting_contract"></a>

## Function `create_vesting_contract`

Create a vesting contract with a given configurations.


<pre><code>public fun create_vesting_contract(admin: &amp;signer, shareholders: &amp;vector&lt;address&gt;, buy_ins: simple_map::SimpleMap&lt;address, coin::Coin&lt;aptos_coin::AptosCoin&gt;&gt;, vesting_schedule: vesting::VestingSchedule, withdrawal_address: address, operator: address, voter: address, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_vesting_contract(
    admin: &amp;signer,
    shareholders: &amp;vector&lt;address&gt;,
    buy_ins: SimpleMap&lt;address, Coin&lt;AptosCoin&gt;&gt;,
    vesting_schedule: VestingSchedule,
    withdrawal_address: address,
    operator: address,
    voter: address,
    commission_percentage: u64,
    // Optional seed used when creating the staking contract account.
    contract_creation_seed: vector&lt;u8&gt;,
): address acquires AdminStore &#123;
    assert!(
        !system_addresses::is_reserved_address(withdrawal_address),
        error::invalid_argument(EINVALID_WITHDRAWAL_ADDRESS),
    );
    assert_account_is_registered_for_apt(withdrawal_address);
    assert!(vector::length(shareholders) &gt; 0, error::invalid_argument(ENO_SHAREHOLDERS));
    assert!(
        simple_map::length(&amp;buy_ins) &#61;&#61; vector::length(shareholders),
        error::invalid_argument(ESHARES_LENGTH_MISMATCH),
    );

    // Create a coins pool to track shareholders and shares of the grant.
    let grant &#61; coin::zero&lt;AptosCoin&gt;();
    let grant_amount &#61; 0;
    let grant_pool &#61; pool_u64::create(MAXIMUM_SHAREHOLDERS);
    vector::for_each_ref(shareholders, &#124;shareholder&#124; &#123;
        let shareholder: address &#61; &#42;shareholder;
        let (_, buy_in) &#61; simple_map::remove(&amp;mut buy_ins, &amp;shareholder);
        let buy_in_amount &#61; coin::value(&amp;buy_in);
        coin::merge(&amp;mut grant, buy_in);
        pool_u64::buy_in(
            &amp;mut grant_pool,
            shareholder,
            buy_in_amount,
        );
        grant_amount &#61; grant_amount &#43; buy_in_amount;
    &#125;);
    assert!(grant_amount &gt; 0, error::invalid_argument(EZERO_GRANT));

    // If this is the first time this admin account has created a vesting contract, initialize the admin store.
    let admin_address &#61; signer::address_of(admin);
    if (!exists&lt;AdminStore&gt;(admin_address)) &#123;
        move_to(admin, AdminStore &#123;
            vesting_contracts: vector::empty&lt;address&gt;(),
            nonce: 0,
            create_events: new_event_handle&lt;CreateVestingContractEvent&gt;(admin),
        &#125;);
    &#125;;

    // Initialize the vesting contract in a new resource account. This allows the same admin to create multiple
    // pools.
    let (contract_signer, contract_signer_cap) &#61; create_vesting_contract_account(admin, contract_creation_seed);
    let pool_address &#61; staking_contract::create_staking_contract_with_coins(
        &amp;contract_signer, operator, voter, grant, commission_percentage, contract_creation_seed);

    // Add the newly created vesting contract&apos;s address to the admin store.
    let contract_address &#61; signer::address_of(&amp;contract_signer);
    let admin_store &#61; borrow_global_mut&lt;AdminStore&gt;(admin_address);
    vector::push_back(&amp;mut admin_store.vesting_contracts, contract_address);
    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            CreateVestingContract &#123;
                operator,
                voter,
                withdrawal_address,
                grant_amount,
                vesting_contract_address: contract_address,
                staking_pool_address: pool_address,
                commission_percentage,
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut admin_store.create_events,
        CreateVestingContractEvent &#123;
            operator,
            voter,
            withdrawal_address,
            grant_amount,
            vesting_contract_address: contract_address,
            staking_pool_address: pool_address,
            commission_percentage,
        &#125;,
    );

    move_to(&amp;contract_signer, VestingContract &#123;
        state: VESTING_POOL_ACTIVE,
        admin: admin_address,
        grant_pool,
        beneficiaries: simple_map::create&lt;address, address&gt;(),
        vesting_schedule,
        withdrawal_address,
        staking: StakingInfo &#123; pool_address, operator, voter, commission_percentage &#125;,
        remaining_grant: grant_amount,
        signer_cap: contract_signer_cap,
        update_operator_events: new_event_handle&lt;UpdateOperatorEvent&gt;(&amp;contract_signer),
        update_voter_events: new_event_handle&lt;UpdateVoterEvent&gt;(&amp;contract_signer),
        reset_lockup_events: new_event_handle&lt;ResetLockupEvent&gt;(&amp;contract_signer),
        set_beneficiary_events: new_event_handle&lt;SetBeneficiaryEvent&gt;(&amp;contract_signer),
        unlock_rewards_events: new_event_handle&lt;UnlockRewardsEvent&gt;(&amp;contract_signer),
        vest_events: new_event_handle&lt;VestEvent&gt;(&amp;contract_signer),
        distribute_events: new_event_handle&lt;DistributeEvent&gt;(&amp;contract_signer),
        terminate_events: new_event_handle&lt;TerminateEvent&gt;(&amp;contract_signer),
        admin_withdraw_events: new_event_handle&lt;AdminWithdrawEvent&gt;(&amp;contract_signer),
    &#125;);

    simple_map::destroy_empty(buy_ins);
    contract_address
&#125;
</code></pre>



</details>

<a id="0x1_vesting_unlock_rewards"></a>

## Function `unlock_rewards`

Unlock any accumulated rewards.


<pre><code>public entry fun unlock_rewards(contract_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock_rewards(contract_address: address) acquires VestingContract &#123;
    let accumulated_rewards &#61; total_accumulated_rewards(contract_address);
    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(contract_address);
    unlock_stake(vesting_contract, accumulated_rewards);
&#125;
</code></pre>



</details>

<a id="0x1_vesting_unlock_rewards_many"></a>

## Function `unlock_rewards_many`

Call <code>unlock_rewards</code> for many vesting contracts.


<pre><code>public entry fun unlock_rewards_many(contract_addresses: vector&lt;address&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unlock_rewards_many(contract_addresses: vector&lt;address&gt;) acquires VestingContract &#123;
    let len &#61; vector::length(&amp;contract_addresses);

    assert!(len !&#61; 0, error::invalid_argument(EVEC_EMPTY_FOR_MANY_FUNCTION));

    vector::for_each_ref(&amp;contract_addresses, &#124;contract_address&#124; &#123;
        let contract_address: address &#61; &#42;contract_address;
        unlock_rewards(contract_address);
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_vesting_vest"></a>

## Function `vest`

Unlock any vested portion of the grant.


<pre><code>public entry fun vest(contract_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vest(contract_address: address) acquires VestingContract &#123;
    // Unlock all rewards first, if any.
    unlock_rewards(contract_address);

    // Unlock the vested amount. This amount will become withdrawable when the underlying stake pool&apos;s lockup
    // expires.
    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    // Short&#45;circuit if vesting hasn&apos;t started yet.
    if (vesting_contract.vesting_schedule.start_timestamp_secs &gt; timestamp::now_seconds()) &#123;
        return
    &#125;;

    // Check if the next vested period has already passed. If not, short&#45;circuit since there&apos;s nothing to vest.
    let vesting_schedule &#61; &amp;mut vesting_contract.vesting_schedule;
    let last_vested_period &#61; vesting_schedule.last_vested_period;
    let next_period_to_vest &#61; last_vested_period &#43; 1;
    let last_completed_period &#61;
        (timestamp::now_seconds() &#45; vesting_schedule.start_timestamp_secs) / vesting_schedule.period_duration;
    if (last_completed_period &lt; next_period_to_vest) &#123;
        return
    &#125;;

    // Calculate how much has vested, excluding rewards.
    // Index is 0&#45;based while period is 1&#45;based so we need to subtract 1.
    let schedule &#61; &amp;vesting_schedule.schedule;
    let schedule_index &#61; next_period_to_vest &#45; 1;
    let vesting_fraction &#61; if (schedule_index &lt; vector::length(schedule)) &#123;
        &#42;vector::borrow(schedule, schedule_index)
    &#125; else &#123;
        // Last vesting schedule fraction will repeat until the grant runs out.
        &#42;vector::borrow(schedule, vector::length(schedule) &#45; 1)
    &#125;;
    let total_grant &#61; pool_u64::total_coins(&amp;vesting_contract.grant_pool);
    let vested_amount &#61; fixed_point32::multiply_u64(total_grant, vesting_fraction);
    // Cap vested amount by the remaining grant amount so we don&apos;t try to distribute more than what&apos;s remaining.
    vested_amount &#61; min(vested_amount, vesting_contract.remaining_grant);
    vesting_contract.remaining_grant &#61; vesting_contract.remaining_grant &#45; vested_amount;
    vesting_schedule.last_vested_period &#61; next_period_to_vest;
    unlock_stake(vesting_contract, vested_amount);

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            Vest &#123;
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                period_vested: next_period_to_vest,
                amount: vested_amount,
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut vesting_contract.vest_events,
        VestEvent &#123;
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            staking_pool_address: vesting_contract.staking.pool_address,
            period_vested: next_period_to_vest,
            amount: vested_amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_vesting_vest_many"></a>

## Function `vest_many`

Call <code>vest</code> for many vesting contracts.


<pre><code>public entry fun vest_many(contract_addresses: vector&lt;address&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vest_many(contract_addresses: vector&lt;address&gt;) acquires VestingContract &#123;
    let len &#61; vector::length(&amp;contract_addresses);

    assert!(len !&#61; 0, error::invalid_argument(EVEC_EMPTY_FOR_MANY_FUNCTION));

    vector::for_each_ref(&amp;contract_addresses, &#124;contract_address&#124; &#123;
        let contract_address &#61; &#42;contract_address;
        vest(contract_address);
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_vesting_distribute"></a>

## Function `distribute`

Distribute any withdrawable stake from the stake pool.


<pre><code>public entry fun distribute(contract_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun distribute(contract_address: address) acquires VestingContract &#123;
    assert_active_vesting_contract(contract_address);

    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    let coins &#61; withdraw_stake(vesting_contract, contract_address);
    let total_distribution_amount &#61; coin::value(&amp;coins);
    if (total_distribution_amount &#61;&#61; 0) &#123;
        coin::destroy_zero(coins);
        return
    &#125;;

    // Distribute coins to all shareholders in the vesting contract.
    let grant_pool &#61; &amp;vesting_contract.grant_pool;
    let shareholders &#61; &amp;pool_u64::shareholders(grant_pool);
    vector::for_each_ref(shareholders, &#124;shareholder&#124; &#123;
        let shareholder &#61; &#42;shareholder;
        let shares &#61; pool_u64::shares(grant_pool, shareholder);
        let amount &#61; pool_u64::shares_to_amount_with_total_coins(grant_pool, shares, total_distribution_amount);
        let share_of_coins &#61; coin::extract(&amp;mut coins, amount);
        let recipient_address &#61; get_beneficiary(vesting_contract, shareholder);
        aptos_account::deposit_coins(recipient_address, share_of_coins);
    &#125;);

    // Send any remaining &quot;dust&quot; (leftover due to rounding error) to the withdrawal address.
    if (coin::value(&amp;coins) &gt; 0) &#123;
        aptos_account::deposit_coins(vesting_contract.withdrawal_address, coins);
    &#125; else &#123;
        coin::destroy_zero(coins);
    &#125;;

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            Distribute &#123;
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                amount: total_distribution_amount,
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut vesting_contract.distribute_events,
        DistributeEvent &#123;
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            amount: total_distribution_amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_vesting_distribute_many"></a>

## Function `distribute_many`

Call <code>distribute</code> for many vesting contracts.


<pre><code>public entry fun distribute_many(contract_addresses: vector&lt;address&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun distribute_many(contract_addresses: vector&lt;address&gt;) acquires VestingContract &#123;
    let len &#61; vector::length(&amp;contract_addresses);

    assert!(len !&#61; 0, error::invalid_argument(EVEC_EMPTY_FOR_MANY_FUNCTION));

    vector::for_each_ref(&amp;contract_addresses, &#124;contract_address&#124; &#123;
        let contract_address &#61; &#42;contract_address;
        distribute(contract_address);
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_vesting_terminate_vesting_contract"></a>

## Function `terminate_vesting_contract`

Terminate the vesting contract and send all funds back to the withdrawal address.


<pre><code>public entry fun terminate_vesting_contract(admin: &amp;signer, contract_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun terminate_vesting_contract(admin: &amp;signer, contract_address: address) acquires VestingContract &#123;
    assert_active_vesting_contract(contract_address);

    // Distribute all withdrawable coins, which should have been from previous rewards withdrawal or vest.
    distribute(contract_address);

    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);
    let (active_stake, _, pending_active_stake, _) &#61; stake::get_stake(vesting_contract.staking.pool_address);
    assert!(pending_active_stake &#61;&#61; 0, error::invalid_state(EPENDING_STAKE_FOUND));

    // Unlock all remaining active stake.
    vesting_contract.state &#61; VESTING_POOL_TERMINATED;
    vesting_contract.remaining_grant &#61; 0;
    unlock_stake(vesting_contract, active_stake);

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            Terminate &#123;
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut vesting_contract.terminate_events,
        TerminateEvent &#123;
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_vesting_admin_withdraw"></a>

## Function `admin_withdraw`

Withdraw all funds to the preset vesting contract's withdrawal address. This can only be called if the contract
has already been terminated.


<pre><code>public entry fun admin_withdraw(admin: &amp;signer, contract_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun admin_withdraw(admin: &amp;signer, contract_address: address) acquires VestingContract &#123;
    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(contract_address);
    assert!(
        vesting_contract.state &#61;&#61; VESTING_POOL_TERMINATED,
        error::invalid_state(EVESTING_CONTRACT_STILL_ACTIVE)
    );

    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);
    let coins &#61; withdraw_stake(vesting_contract, contract_address);
    let amount &#61; coin::value(&amp;coins);
    if (amount &#61;&#61; 0) &#123;
        coin::destroy_zero(coins);
        return
    &#125;;
    aptos_account::deposit_coins(vesting_contract.withdrawal_address, coins);

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            AdminWithdraw &#123;
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                amount,
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut vesting_contract.admin_withdraw_events,
        AdminWithdrawEvent &#123;
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_vesting_update_operator"></a>

## Function `update_operator`



<pre><code>public entry fun update_operator(admin: &amp;signer, contract_address: address, new_operator: address, commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_operator(
    admin: &amp;signer,
    contract_address: address,
    new_operator: address,
    commission_percentage: u64,
) acquires VestingContract &#123;
    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);
    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);
    let old_operator &#61; vesting_contract.staking.operator;
    staking_contract::switch_operator(contract_signer, old_operator, new_operator, commission_percentage);
    vesting_contract.staking.operator &#61; new_operator;
    vesting_contract.staking.commission_percentage &#61; commission_percentage;

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            UpdateOperator &#123;
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                old_operator,
                new_operator,
                commission_percentage,
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut vesting_contract.update_operator_events,
        UpdateOperatorEvent &#123;
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            staking_pool_address: vesting_contract.staking.pool_address,
            old_operator,
            new_operator,
            commission_percentage,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_vesting_update_operator_with_same_commission"></a>

## Function `update_operator_with_same_commission`



<pre><code>public entry fun update_operator_with_same_commission(admin: &amp;signer, contract_address: address, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_operator_with_same_commission(
    admin: &amp;signer,
    contract_address: address,
    new_operator: address,
) acquires VestingContract &#123;
    let commission_percentage &#61; operator_commission_percentage(contract_address);
    update_operator(admin, contract_address, new_operator, commission_percentage);
&#125;
</code></pre>



</details>

<a id="0x1_vesting_update_commission_percentage"></a>

## Function `update_commission_percentage`



<pre><code>public entry fun update_commission_percentage(admin: &amp;signer, contract_address: address, new_commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_commission_percentage(
    admin: &amp;signer,
    contract_address: address,
    new_commission_percentage: u64,
) acquires VestingContract &#123;
    let operator &#61; operator(contract_address);
    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);
    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);
    staking_contract::update_commision(contract_signer, operator, new_commission_percentage);
    vesting_contract.staking.commission_percentage &#61; new_commission_percentage;
    // This function does not emit an event. Instead, `staking_contract::update_commission_percentage`
    // emits the event for this commission percentage update.
&#125;
</code></pre>



</details>

<a id="0x1_vesting_update_voter"></a>

## Function `update_voter`



<pre><code>public entry fun update_voter(admin: &amp;signer, contract_address: address, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_voter(
    admin: &amp;signer,
    contract_address: address,
    new_voter: address,
) acquires VestingContract &#123;
    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);
    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);
    let old_voter &#61; vesting_contract.staking.voter;
    staking_contract::update_voter(contract_signer, vesting_contract.staking.operator, new_voter);
    vesting_contract.staking.voter &#61; new_voter;

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            UpdateVoter &#123;
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                old_voter,
                new_voter,
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut vesting_contract.update_voter_events,
        UpdateVoterEvent &#123;
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            staking_pool_address: vesting_contract.staking.pool_address,
            old_voter,
            new_voter,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_vesting_reset_lockup"></a>

## Function `reset_lockup`



<pre><code>public entry fun reset_lockup(admin: &amp;signer, contract_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reset_lockup(
    admin: &amp;signer,
    contract_address: address,
) acquires VestingContract &#123;
    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);
    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);
    staking_contract::reset_lockup(contract_signer, vesting_contract.staking.operator);

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            ResetLockup &#123;
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                new_lockup_expiration_secs: stake::get_lockup_secs(vesting_contract.staking.pool_address),
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut vesting_contract.reset_lockup_events,
        ResetLockupEvent &#123;
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            staking_pool_address: vesting_contract.staking.pool_address,
            new_lockup_expiration_secs: stake::get_lockup_secs(vesting_contract.staking.pool_address),
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_vesting_set_beneficiary"></a>

## Function `set_beneficiary`



<pre><code>public entry fun set_beneficiary(admin: &amp;signer, contract_address: address, shareholder: address, new_beneficiary: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary(
    admin: &amp;signer,
    contract_address: address,
    shareholder: address,
    new_beneficiary: address,
) acquires VestingContract &#123;
    // Verify that the beneficiary account is set up to receive APT. This is a requirement so distribute() wouldn&apos;t
    // fail and block all other accounts from receiving APT if one beneficiary is not registered.
    assert_account_is_registered_for_apt(new_beneficiary);

    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);

    let old_beneficiary &#61; get_beneficiary(vesting_contract, shareholder);
    let beneficiaries &#61; &amp;mut vesting_contract.beneficiaries;
    if (simple_map::contains_key(beneficiaries, &amp;shareholder)) &#123;
        let beneficiary &#61; simple_map::borrow_mut(beneficiaries, &amp;shareholder);
        &#42;beneficiary &#61; new_beneficiary;
    &#125; else &#123;
        simple_map::add(beneficiaries, shareholder, new_beneficiary);
    &#125;;

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            SetBeneficiary &#123;
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                shareholder,
                old_beneficiary,
                new_beneficiary,
            &#125;,
        );
    &#125;;
    emit_event(
        &amp;mut vesting_contract.set_beneficiary_events,
        SetBeneficiaryEvent &#123;
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            shareholder,
            old_beneficiary,
            new_beneficiary,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_vesting_reset_beneficiary"></a>

## Function `reset_beneficiary`

Remove the beneficiary for the given shareholder. All distributions will sent directly to the shareholder
account.


<pre><code>public entry fun reset_beneficiary(account: &amp;signer, contract_address: address, shareholder: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reset_beneficiary(
    account: &amp;signer,
    contract_address: address,
    shareholder: address,
) acquires VestingAccountManagement, VestingContract &#123;
    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    let addr &#61; signer::address_of(account);
    assert!(
        addr &#61;&#61; vesting_contract.admin &#124;&#124;
            addr &#61;&#61; get_role_holder(contract_address, utf8(ROLE_BENEFICIARY_RESETTER)),
        error::permission_denied(EPERMISSION_DENIED),
    );

    let beneficiaries &#61; &amp;mut vesting_contract.beneficiaries;
    if (simple_map::contains_key(beneficiaries, &amp;shareholder)) &#123;
        simple_map::remove(beneficiaries, &amp;shareholder);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_vesting_set_management_role"></a>

## Function `set_management_role`



<pre><code>public entry fun set_management_role(admin: &amp;signer, contract_address: address, role: string::String, role_holder: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_management_role(
    admin: &amp;signer,
    contract_address: address,
    role: String,
    role_holder: address,
) acquires VestingAccountManagement, VestingContract &#123;
    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);

    if (!exists&lt;VestingAccountManagement&gt;(contract_address)) &#123;
        let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);
        move_to(contract_signer, VestingAccountManagement &#123;
            roles: simple_map::create&lt;String, address&gt;(),
        &#125;)
    &#125;;
    let roles &#61; &amp;mut borrow_global_mut&lt;VestingAccountManagement&gt;(contract_address).roles;
    if (simple_map::contains_key(roles, &amp;role)) &#123;
        &#42;simple_map::borrow_mut(roles, &amp;role) &#61; role_holder;
    &#125; else &#123;
        simple_map::add(roles, role, role_holder);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_vesting_set_beneficiary_resetter"></a>

## Function `set_beneficiary_resetter`



<pre><code>public entry fun set_beneficiary_resetter(admin: &amp;signer, contract_address: address, beneficiary_resetter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary_resetter(
    admin: &amp;signer,
    contract_address: address,
    beneficiary_resetter: address,
) acquires VestingAccountManagement, VestingContract &#123;
    set_management_role(admin, contract_address, utf8(ROLE_BENEFICIARY_RESETTER), beneficiary_resetter);
&#125;
</code></pre>



</details>

<a id="0x1_vesting_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Set the beneficiary for the operator.


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_beneficiary_for_operator(
    operator: &amp;signer,
    new_beneficiary: address,
) &#123;
    staking_contract::set_beneficiary_for_operator(operator, new_beneficiary);
&#125;
</code></pre>



</details>

<a id="0x1_vesting_get_role_holder"></a>

## Function `get_role_holder`



<pre><code>public fun get_role_holder(contract_address: address, role: string::String): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_role_holder(contract_address: address, role: String): address acquires VestingAccountManagement &#123;
    assert!(exists&lt;VestingAccountManagement&gt;(contract_address), error::not_found(EVESTING_ACCOUNT_HAS_NO_ROLES));
    let roles &#61; &amp;borrow_global&lt;VestingAccountManagement&gt;(contract_address).roles;
    assert!(simple_map::contains_key(roles, &amp;role), error::not_found(EROLE_NOT_FOUND));
    &#42;simple_map::borrow(roles, &amp;role)
&#125;
</code></pre>



</details>

<a id="0x1_vesting_get_vesting_account_signer"></a>

## Function `get_vesting_account_signer`

For emergency use in case the admin needs emergency control of vesting contract account.
This doesn't give the admin total power as the admin would still need to follow the rules set by
staking_contract and stake modules.


<pre><code>public fun get_vesting_account_signer(admin: &amp;signer, contract_address: address): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_vesting_account_signer(admin: &amp;signer, contract_address: address): signer acquires VestingContract &#123;
    let vesting_contract &#61; borrow_global_mut&lt;VestingContract&gt;(contract_address);
    verify_admin(admin, vesting_contract);
    get_vesting_account_signer_internal(vesting_contract)
&#125;
</code></pre>



</details>

<a id="0x1_vesting_get_vesting_account_signer_internal"></a>

## Function `get_vesting_account_signer_internal`



<pre><code>fun get_vesting_account_signer_internal(vesting_contract: &amp;vesting::VestingContract): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_vesting_account_signer_internal(vesting_contract: &amp;VestingContract): signer &#123;
    account::create_signer_with_capability(&amp;vesting_contract.signer_cap)
&#125;
</code></pre>



</details>

<a id="0x1_vesting_create_vesting_contract_account"></a>

## Function `create_vesting_contract_account`

Create a salt for generating the resource accounts that will be holding the VestingContract.
This address should be deterministic for the same admin and vesting contract creation nonce.


<pre><code>fun create_vesting_contract_account(admin: &amp;signer, contract_creation_seed: vector&lt;u8&gt;): (signer, account::SignerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_vesting_contract_account(
    admin: &amp;signer,
    contract_creation_seed: vector&lt;u8&gt;,
): (signer, SignerCapability) acquires AdminStore &#123;
    let admin_store &#61; borrow_global_mut&lt;AdminStore&gt;(signer::address_of(admin));
    let seed &#61; bcs::to_bytes(&amp;signer::address_of(admin));
    vector::append(&amp;mut seed, bcs::to_bytes(&amp;admin_store.nonce));
    admin_store.nonce &#61; admin_store.nonce &#43; 1;

    // Include a salt to avoid conflicts with any other modules out there that might also generate
    // deterministic resource accounts for the same admin address &#43; nonce.
    vector::append(&amp;mut seed, VESTING_POOL_SALT);
    vector::append(&amp;mut seed, contract_creation_seed);

    let (account_signer, signer_cap) &#61; account::create_resource_account(admin, seed);
    // Register the vesting contract account to receive APT as it&apos;ll be sent to it when claiming unlocked stake from
    // the underlying staking contract.
    coin::register&lt;AptosCoin&gt;(&amp;account_signer);

    (account_signer, signer_cap)
&#125;
</code></pre>



</details>

<a id="0x1_vesting_verify_admin"></a>

## Function `verify_admin`



<pre><code>fun verify_admin(admin: &amp;signer, vesting_contract: &amp;vesting::VestingContract)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun verify_admin(admin: &amp;signer, vesting_contract: &amp;VestingContract) &#123;
    assert!(signer::address_of(admin) &#61;&#61; vesting_contract.admin, error::unauthenticated(ENOT_ADMIN));
&#125;
</code></pre>



</details>

<a id="0x1_vesting_assert_vesting_contract_exists"></a>

## Function `assert_vesting_contract_exists`



<pre><code>fun assert_vesting_contract_exists(contract_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_vesting_contract_exists(contract_address: address) &#123;
    assert!(exists&lt;VestingContract&gt;(contract_address), error::not_found(EVESTING_CONTRACT_NOT_FOUND));
&#125;
</code></pre>



</details>

<a id="0x1_vesting_assert_active_vesting_contract"></a>

## Function `assert_active_vesting_contract`



<pre><code>fun assert_active_vesting_contract(contract_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_active_vesting_contract(contract_address: address) acquires VestingContract &#123;
    assert_vesting_contract_exists(contract_address);
    let vesting_contract &#61; borrow_global&lt;VestingContract&gt;(contract_address);
    assert!(vesting_contract.state &#61;&#61; VESTING_POOL_ACTIVE, error::invalid_state(EVESTING_CONTRACT_NOT_ACTIVE));
&#125;
</code></pre>



</details>

<a id="0x1_vesting_unlock_stake"></a>

## Function `unlock_stake`



<pre><code>fun unlock_stake(vesting_contract: &amp;vesting::VestingContract, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun unlock_stake(vesting_contract: &amp;VestingContract, amount: u64) &#123;
    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);
    staking_contract::unlock_stake(contract_signer, vesting_contract.staking.operator, amount);
&#125;
</code></pre>



</details>

<a id="0x1_vesting_withdraw_stake"></a>

## Function `withdraw_stake`



<pre><code>fun withdraw_stake(vesting_contract: &amp;vesting::VestingContract, contract_address: address): coin::Coin&lt;aptos_coin::AptosCoin&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun withdraw_stake(vesting_contract: &amp;VestingContract, contract_address: address): Coin&lt;AptosCoin&gt; &#123;
    // Claim any withdrawable distribution from the staking contract. The withdrawn coins will be sent directly to
    // the vesting contract&apos;s account.
    staking_contract::distribute(contract_address, vesting_contract.staking.operator);
    let withdrawn_coins &#61; coin::balance&lt;AptosCoin&gt;(contract_address);
    let contract_signer &#61; &amp;get_vesting_account_signer_internal(vesting_contract);
    coin::withdraw&lt;AptosCoin&gt;(contract_signer, withdrawn_coins)
&#125;
</code></pre>



</details>

<a id="0x1_vesting_get_beneficiary"></a>

## Function `get_beneficiary`



<pre><code>fun get_beneficiary(contract: &amp;vesting::VestingContract, shareholder: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_beneficiary(contract: &amp;VestingContract, shareholder: address): address &#123;
    if (simple_map::contains_key(&amp;contract.beneficiaries, &amp;shareholder)) &#123;
        &#42;simple_map::borrow(&amp;contract.beneficiaries, &amp;shareholder)
    &#125; else &#123;
        shareholder
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
<td>In order to retrieve the address of the underlying stake pool, the vesting start timestamp of the vesting contract, the duration of the vesting period, the remaining grant of a vesting contract, the beneficiary account of a shareholder in a vesting contract, the percentage of accumulated rewards that is paid to the operator as commission, the operator who runs the validator, the voter who will be voting on-chain, and the vesting schedule of a vesting contract, the supplied vesting contract should exist.</td>
<td>Low</td>
<td>The vesting_start_secs, period_duration_secs, remaining_grant, beneficiary, operator_commission_percentage, operator, voter, and vesting_schedule functions ensure that the supplied vesting contract address exists by calling the assert_vesting_contract_exists function.</td>
<td>Formally verified via <a href="#high-level-req-1">assert_vesting_contract_exists</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The vesting pool should not exceed a maximum of 30 shareholders.</td>
<td>Medium</td>
<td>The maximum number of shareholders a vesting pool can support is stored as a constant in MAXIMUM_SHAREHOLDERS which is passed to the pool_u64::create function.</td>
<td>Formally verified via a <a href="#high-level-spec-2">global invariant</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Retrieving all the vesting contracts of a given address and retrieving the list of beneficiaries from a vesting contract should never fail.</td>
<td>Medium</td>
<td>The function vesting_contracts checks if the supplied admin address contains an AdminStore resource and returns all the vesting contracts as a vector<address>. Otherwise it returns an empty vector. The function get_beneficiary checks for a given vesting contract, a specific shareholder exists, and if so, the beneficiary will be returned, otherwise it will simply return the address of the shareholder.</td>
<td>Formally verified via <a href="#high-level-spec-3.1">vesting_contracts</a> and <a href="#high-level-spec-3.2">get_beneficiary</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The shareholders should be able to start vesting only after the vesting cliff and the first vesting period have transpired.</td>
<td>High</td>
<td>The end of the vesting cliff is stored under VestingContract.vesting_schedule.start_timestamp_secs. The vest function always checks that timestamp::now_seconds is greater or equal to the end of the vesting cliff period.</td>
<td>Audited the check for the end of vesting cliff: <a href="https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/vesting.move#L566">vest</a> module.</td>
</tr>

<tr>
<td>5</td>
<td>In order to retrieve the total accumulated rewards that have not been distributed, the accumulated rewards of a given beneficiary, the list of al shareholders in a vesting contract,the shareholder address given the beneficiary address in a given vesting contract, to terminate a vesting contract and to distribute any withdrawable stake from the stake pool, the supplied vesting contract should exist and be active.</td>
<td>Low</td>
<td>The distribute, terminate_vesting_contract, shareholder, shareholders, accumulated_rewards, and total_accumulated_rewards functions ensure that the supplied vesting contract address exists and is active by calling the assert_active_vesting_contract function.</td>
<td>Formally verified via <a href="#high-level-spec-5">ActiveVestingContractAbortsIf</a>.</td>
</tr>

<tr>
<td>6</td>
<td>A new vesting schedule should not be allowed to start vesting in the past or to supply an empty schedule or for the period duration to be zero.</td>
<td>High</td>
<td>The create_vesting_schedule function ensures that the length of the schedule vector is greater than 0, that the period duration is greater than 0 and that the start_timestamp_secs is greater or equal to timestamp::now_seconds.</td>
<td>Formally verified via <a href="#high-level-req-6">create_vesting_schedule</a>.</td>
</tr>

<tr>
<td>7</td>
<td>The shareholders should be able to vest the tokens from previous periods.</td>
<td>High</td>
<td>When vesting, the last_completed_period is checked against the next period to vest. This allows to unlock vested tokens for the next period since last vested, in case they didn't call vest for some periods.</td>
<td>Audited that vesting doesn't skip periods, but gradually increments to allow shareholders to retrieve all the vested tokens.</td>
</tr>

<tr>
<td>8</td>
<td>Actions such as obtaining a list of shareholders, calculating accrued rewards, distributing withdrawable stake, and terminating the vesting contract should be accessible exclusively while the vesting contract remains active.</td>
<td>Low</td>
<td>Restricting access to inactive vesting contracts is achieved through the assert_active_vesting_contract function.</td>
<td>Formally verified via <a href="#high-level-spec-8">ActiveVestingContractAbortsIf</a>.</td>
</tr>

<tr>
<td>9</td>
<td>The ability to terminate a vesting contract should only be available to the owner.</td>
<td>High</td>
<td>Limiting the access of accounts to specific function, is achieved by asserting that the signer matches the admin of the VestingContract.</td>
<td>Formally verified via <a href="#high-level-req-9">verify_admin</a>.</td>
</tr>

<tr>
<td>10</td>
<td>A new vesting contract should not be allowed to have an empty list of shareholders, have a different amount of shareholders than buy-ins, and provide a withdrawal address which is either reserved or not registered for apt.</td>
<td>High</td>
<td>The create_vesting_contract function ensures that the withdrawal_address is not a reserved address, that it is registered for apt, that the list of shareholders is non-empty, and that the amount of shareholders matches the amount of buy_ins.</td>
<td>Formally verified via <a href="#high-level-req-10">create_vesting_contract</a>.</td>
</tr>

<tr>
<td>11</td>
<td>Creating a vesting contract account should require the signer (admin) to own an admin store and should enforce that the seed of the resource account is composed of the admin store's nonce, the vesting pool salt, and the custom contract creation seed.</td>
<td>Medium</td>
<td>The create_vesting_contract_account concatenates to the seed first the admin_store.nonce then the VESTING_POOL_SALT then the contract_creation_seed and then it is passed to the create_resource_account function.</td>
<td>Enforced via <a href="#high-level-req-11">create_vesting_contract_account</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
// This enforces <a id="high-level-spec-2" href="#high-level-req">high-level requirement 2</a>:
invariant forall a: address where exists&lt;VestingContract&gt;(a):
    global&lt;VestingContract&gt;(a).grant_pool.shareholders_limit &lt;&#61; MAXIMUM_SHAREHOLDERS;
</code></pre>



<a id="@Specification_1_stake_pool_address"></a>

### Function `stake_pool_address`


<pre><code>&#35;[view]
public fun stake_pool_address(vesting_contract_address: address): address
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_vesting_start_secs"></a>

### Function `vesting_start_secs`


<pre><code>&#35;[view]
public fun vesting_start_secs(vesting_contract_address: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_period_duration_secs"></a>

### Function `period_duration_secs`


<pre><code>&#35;[view]
public fun period_duration_secs(vesting_contract_address: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_remaining_grant"></a>

### Function `remaining_grant`


<pre><code>&#35;[view]
public fun remaining_grant(vesting_contract_address: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_beneficiary"></a>

### Function `beneficiary`


<pre><code>&#35;[view]
public fun beneficiary(vesting_contract_address: address, shareholder: address): address
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_operator_commission_percentage"></a>

### Function `operator_commission_percentage`


<pre><code>&#35;[view]
public fun operator_commission_percentage(vesting_contract_address: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_vesting_contracts"></a>

### Function `vesting_contracts`


<pre><code>&#35;[view]
public fun vesting_contracts(admin: address): vector&lt;address&gt;
</code></pre>




<pre><code>// This enforces <a id="high-level-spec-3.1" href="#high-level-req">high-level requirement 3</a>:
aborts_if false;
</code></pre>



<a id="@Specification_1_operator"></a>

### Function `operator`


<pre><code>&#35;[view]
public fun operator(vesting_contract_address: address): address
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_voter"></a>

### Function `voter`


<pre><code>&#35;[view]
public fun voter(vesting_contract_address: address): address
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_vesting_schedule"></a>

### Function `vesting_schedule`


<pre><code>&#35;[view]
public fun vesting_schedule(vesting_contract_address: address): vesting::VestingSchedule
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_total_accumulated_rewards"></a>

### Function `total_accumulated_rewards`


<pre><code>&#35;[view]
public fun total_accumulated_rewards(vesting_contract_address: address): u64
</code></pre>




<pre><code>pragma verify &#61; false;
include TotalAccumulatedRewardsAbortsIf;
</code></pre>




<a id="0x1_vesting_TotalAccumulatedRewardsAbortsIf"></a>


<pre><code>schema TotalAccumulatedRewardsAbortsIf &#123;
    vesting_contract_address: address;
    requires staking_contract.commission_percentage &gt;&#61; 0 &amp;&amp; staking_contract.commission_percentage &lt;&#61; 100;
    include ActiveVestingContractAbortsIf&lt;VestingContract&gt;&#123;contract_address: vesting_contract_address&#125;;
    let vesting_contract &#61; global&lt;VestingContract&gt;(vesting_contract_address);
    let staker &#61; vesting_contract_address;
    let operator &#61; vesting_contract.staking.operator;
    let staking_contracts &#61; global&lt;staking_contract::Store&gt;(staker).staking_contracts;
    let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);
    aborts_if !exists&lt;staking_contract::Store&gt;(staker);
    aborts_if !simple_map::spec_contains_key(staking_contracts, operator);
    let pool_address &#61; staking_contract.pool_address;
    let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);
    let active &#61; coin::value(stake_pool.active);
    let pending_active &#61; coin::value(stake_pool.pending_active);
    let total_active_stake &#61; active &#43; pending_active;
    let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;
    let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;
    aborts_if !exists&lt;stake::StakePool&gt;(pool_address);
    aborts_if active &#43; pending_active &gt; MAX_U64;
    aborts_if total_active_stake &lt; staking_contract.principal;
    aborts_if accumulated_rewards &#42; staking_contract.commission_percentage &gt; MAX_U64;
    aborts_if (vesting_contract.remaining_grant &#43; commission_amount) &gt; total_active_stake;
    aborts_if total_active_stake &lt; vesting_contract.remaining_grant;
&#125;
</code></pre>



<a id="@Specification_1_accumulated_rewards"></a>

### Function `accumulated_rewards`


<pre><code>&#35;[view]
public fun accumulated_rewards(vesting_contract_address: address, shareholder_or_beneficiary: address): u64
</code></pre>




<pre><code>pragma verify &#61; false;
include TotalAccumulatedRewardsAbortsIf;
let vesting_contract &#61; global&lt;VestingContract&gt;(vesting_contract_address);
let operator &#61; vesting_contract.staking.operator;
let staking_contracts &#61; global&lt;staking_contract::Store&gt;(vesting_contract_address).staking_contracts;
let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);
let pool_address &#61; staking_contract.pool_address;
let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);
let active &#61; coin::value(stake_pool.active);
let pending_active &#61; coin::value(stake_pool.pending_active);
let total_active_stake &#61; active &#43; pending_active;
let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;
let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;
let total_accumulated_rewards &#61; total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount;
let shareholder &#61; spec_shareholder(vesting_contract_address, shareholder_or_beneficiary);
let pool &#61; vesting_contract.grant_pool;
let shares &#61; pool_u64::spec_shares(pool, shareholder);
aborts_if pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0
    &amp;&amp; (shares &#42; total_accumulated_rewards) / pool.total_shares &gt; MAX_U64;
ensures result &#61;&#61; pool_u64::spec_shares_to_amount_with_total_coins(pool, shares, total_accumulated_rewards);
</code></pre>



<a id="@Specification_1_shareholders"></a>

### Function `shareholders`


<pre><code>&#35;[view]
public fun shareholders(vesting_contract_address: address): vector&lt;address&gt;
</code></pre>




<pre><code>include ActiveVestingContractAbortsIf&lt;VestingContract&gt;&#123;contract_address: vesting_contract_address&#125;;
</code></pre>




<a id="0x1_vesting_spec_shareholder"></a>


<pre><code>fun spec_shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address;
</code></pre>



<a id="@Specification_1_shareholder"></a>

### Function `shareholder`


<pre><code>&#35;[view]
public fun shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address
</code></pre>




<pre><code>pragma opaque;
include ActiveVestingContractAbortsIf&lt;VestingContract&gt;&#123;contract_address: vesting_contract_address&#125;;
ensures [abstract] result &#61;&#61; spec_shareholder(vesting_contract_address, shareholder_or_beneficiary);
</code></pre>



<a id="@Specification_1_create_vesting_schedule"></a>

### Function `create_vesting_schedule`


<pre><code>public fun create_vesting_schedule(schedule: vector&lt;fixed_point32::FixedPoint32&gt;, start_timestamp_secs: u64, period_duration: u64): vesting::VestingSchedule
</code></pre>




<pre><code>// This enforces <a id="high-level-req-6" href="#high-level-req">high-level requirement 6</a>:
aborts_if !(len(schedule) &gt; 0);
aborts_if !(period_duration &gt; 0);
aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
aborts_if !(start_timestamp_secs &gt;&#61; timestamp::now_seconds());
</code></pre>



<a id="@Specification_1_create_vesting_contract"></a>

### Function `create_vesting_contract`


<pre><code>public fun create_vesting_contract(admin: &amp;signer, shareholders: &amp;vector&lt;address&gt;, buy_ins: simple_map::SimpleMap&lt;address, coin::Coin&lt;aptos_coin::AptosCoin&gt;&gt;, vesting_schedule: vesting::VestingSchedule, withdrawal_address: address, operator: address, voter: address, commission_percentage: u64, contract_creation_seed: vector&lt;u8&gt;): address
</code></pre>




<pre><code>pragma verify &#61; false;
// This enforces <a id="high-level-req-10" href="#high-level-req">high-level requirement 10</a>:
aborts_if withdrawal_address &#61;&#61; @aptos_framework &#124;&#124; withdrawal_address &#61;&#61; @vm_reserved;
aborts_if !exists&lt;account::Account&gt;(withdrawal_address);
aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(withdrawal_address);
aborts_if len(shareholders) &#61;&#61; 0;
aborts_if simple_map::spec_len(buy_ins) !&#61; len(shareholders);
ensures global&lt;VestingContract&gt;(result).grant_pool.shareholders_limit &#61;&#61; 30;
</code></pre>



<a id="@Specification_1_unlock_rewards"></a>

### Function `unlock_rewards`


<pre><code>public entry fun unlock_rewards(contract_address: address)
</code></pre>




<pre><code>pragma verify &#61; false;
include UnlockRewardsAbortsIf;
</code></pre>




<a id="0x1_vesting_UnlockRewardsAbortsIf"></a>


<pre><code>schema UnlockRewardsAbortsIf &#123;
    contract_address: address;
    include TotalAccumulatedRewardsAbortsIf &#123; vesting_contract_address: contract_address &#125;;
    let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
    let operator &#61; vesting_contract.staking.operator;
    let staking_contracts &#61; global&lt;staking_contract::Store&gt;(contract_address).staking_contracts;
    let staking_contract &#61; simple_map::spec_get(staking_contracts, operator);
    let pool_address &#61; staking_contract.pool_address;
    let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);
    let active &#61; coin::value(stake_pool.active);
    let pending_active &#61; coin::value(stake_pool.pending_active);
    let total_active_stake &#61; active &#43; pending_active;
    let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;
    let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;
    let amount &#61; total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount;
    include UnlockStakeAbortsIf &#123; vesting_contract, amount &#125;;
&#125;
</code></pre>



<a id="@Specification_1_unlock_rewards_many"></a>

### Function `unlock_rewards_many`


<pre><code>public entry fun unlock_rewards_many(contract_addresses: vector&lt;address&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
aborts_if len(contract_addresses) &#61;&#61; 0;
include PreconditionAbortsIf;
</code></pre>



<a id="@Specification_1_vest"></a>

### Function `vest`


<pre><code>public entry fun vest(contract_address: address)
</code></pre>




<pre><code>pragma verify &#61; false;
include UnlockRewardsAbortsIf;
</code></pre>



<a id="@Specification_1_vest_many"></a>

### Function `vest_many`


<pre><code>public entry fun vest_many(contract_addresses: vector&lt;address&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
aborts_if len(contract_addresses) &#61;&#61; 0;
include PreconditionAbortsIf;
</code></pre>



<a id="@Specification_1_distribute"></a>

### Function `distribute`


<pre><code>public entry fun distribute(contract_address: address)
</code></pre>




<pre><code>pragma verify &#61; false;
include ActiveVestingContractAbortsIf&lt;VestingContract&gt;;
let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
include WithdrawStakeAbortsIf &#123; vesting_contract &#125;;
</code></pre>



<a id="@Specification_1_distribute_many"></a>

### Function `distribute_many`


<pre><code>public entry fun distribute_many(contract_addresses: vector&lt;address&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
aborts_if len(contract_addresses) &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_terminate_vesting_contract"></a>

### Function `terminate_vesting_contract`


<pre><code>public entry fun terminate_vesting_contract(admin: &amp;signer, contract_address: address)
</code></pre>




<pre><code>pragma verify &#61; false;
include ActiveVestingContractAbortsIf&lt;VestingContract&gt;;
let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
include WithdrawStakeAbortsIf &#123; vesting_contract &#125;;
</code></pre>



<a id="@Specification_1_admin_withdraw"></a>

### Function `admin_withdraw`


<pre><code>public entry fun admin_withdraw(admin: &amp;signer, contract_address: address)
</code></pre>




<pre><code>pragma verify &#61; false;
let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
aborts_if vesting_contract.state !&#61; VESTING_POOL_TERMINATED;
include VerifyAdminAbortsIf;
include WithdrawStakeAbortsIf &#123; vesting_contract &#125;;
</code></pre>



<a id="@Specification_1_update_operator"></a>

### Function `update_operator`


<pre><code>public entry fun update_operator(admin: &amp;signer, contract_address: address, new_operator: address, commission_percentage: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
include VerifyAdminAbortsIf;
let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
let acc &#61; vesting_contract.signer_cap.account;
let old_operator &#61; vesting_contract.staking.operator;
include staking_contract::ContractExistsAbortsIf &#123; staker: acc, operator: old_operator &#125;;
let store &#61; global&lt;staking_contract::Store&gt;(acc);
let staking_contracts &#61; store.staking_contracts;
aborts_if simple_map::spec_contains_key(staking_contracts, new_operator);
let staking_contract &#61; simple_map::spec_get(staking_contracts, old_operator);
include DistributeInternalAbortsIf &#123; staker: acc, operator: old_operator, staking_contract, distribute_events: store.distribute_events &#125;;
</code></pre>



<a id="@Specification_1_update_operator_with_same_commission"></a>

### Function `update_operator_with_same_commission`


<pre><code>public entry fun update_operator_with_same_commission(admin: &amp;signer, contract_address: address, new_operator: address)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_update_commission_percentage"></a>

### Function `update_commission_percentage`


<pre><code>public entry fun update_commission_percentage(admin: &amp;signer, contract_address: address, new_commission_percentage: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_update_voter"></a>

### Function `update_voter`


<pre><code>public entry fun update_voter(admin: &amp;signer, contract_address: address, new_voter: address)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
include VerifyAdminAbortsIf;
let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
let operator &#61; vesting_contract.staking.operator;
let staker &#61; vesting_contract.signer_cap.account;
include staking_contract::UpdateVoterSchema;
</code></pre>



<a id="@Specification_1_reset_lockup"></a>

### Function `reset_lockup`


<pre><code>public entry fun reset_lockup(admin: &amp;signer, contract_address: address)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
aborts_if !exists&lt;VestingContract&gt;(contract_address);
let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
aborts_if signer::address_of(admin) !&#61; vesting_contract.admin;
let operator &#61; vesting_contract.staking.operator;
let staker &#61; vesting_contract.signer_cap.account;
include staking_contract::ContractExistsAbortsIf &#123;staker, operator&#125;;
include staking_contract::IncreaseLockupWithCapAbortsIf &#123;staker, operator&#125;;
let store &#61; global&lt;staking_contract::Store&gt;(staker);
let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);
let pool_address &#61; staking_contract.owner_cap.pool_address;
aborts_if !exists&lt;stake::StakePool&gt;(vesting_contract.staking.pool_address);
</code></pre>



<a id="@Specification_1_set_beneficiary"></a>

### Function `set_beneficiary`


<pre><code>public entry fun set_beneficiary(admin: &amp;signer, contract_address: address, shareholder: address, new_beneficiary: address)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
pragma aborts_if_is_partial;
aborts_if !account::exists_at(new_beneficiary);
aborts_if !coin::spec_is_account_registered&lt;AptosCoin&gt;(new_beneficiary);
include VerifyAdminAbortsIf;
let post vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
ensures simple_map::spec_contains_key(vesting_contract.beneficiaries,shareholder);
</code></pre>



<a id="@Specification_1_reset_beneficiary"></a>

### Function `reset_beneficiary`


<pre><code>public entry fun reset_beneficiary(account: &amp;signer, contract_address: address, shareholder: address)
</code></pre>




<pre><code>aborts_if !exists&lt;VestingContract&gt;(contract_address);
let addr &#61; signer::address_of(account);
let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
aborts_if addr !&#61; vesting_contract.admin &amp;&amp; !std::string::spec_internal_check_utf8(ROLE_BENEFICIARY_RESETTER);
aborts_if addr !&#61; vesting_contract.admin &amp;&amp; !exists&lt;VestingAccountManagement&gt;(contract_address);
let roles &#61; global&lt;VestingAccountManagement&gt;(contract_address).roles;
let role &#61; std::string::spec_utf8(ROLE_BENEFICIARY_RESETTER);
aborts_if addr !&#61; vesting_contract.admin &amp;&amp; !simple_map::spec_contains_key(roles, role);
aborts_if addr !&#61; vesting_contract.admin &amp;&amp; addr !&#61; simple_map::spec_get(roles, role);
let post post_vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
ensures !simple_map::spec_contains_key(post_vesting_contract.beneficiaries,shareholder);
</code></pre>



<a id="@Specification_1_set_management_role"></a>

### Function `set_management_role`


<pre><code>public entry fun set_management_role(admin: &amp;signer, contract_address: address, role: string::String, role_holder: address)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
include SetManagementRoleAbortsIf;
</code></pre>



<a id="@Specification_1_set_beneficiary_resetter"></a>

### Function `set_beneficiary_resetter`


<pre><code>public entry fun set_beneficiary_resetter(admin: &amp;signer, contract_address: address, beneficiary_resetter: address)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
aborts_if !std::string::spec_internal_check_utf8(ROLE_BENEFICIARY_RESETTER);
include SetManagementRoleAbortsIf;
</code></pre>



<a id="@Specification_1_set_beneficiary_for_operator"></a>

### Function `set_beneficiary_for_operator`


<pre><code>public entry fun set_beneficiary_for_operator(operator: &amp;signer, new_beneficiary: address)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_get_role_holder"></a>

### Function `get_role_holder`


<pre><code>public fun get_role_holder(contract_address: address, role: string::String): address
</code></pre>




<pre><code>aborts_if !exists&lt;VestingAccountManagement&gt;(contract_address);
let roles &#61; global&lt;VestingAccountManagement&gt;(contract_address).roles;
aborts_if !simple_map::spec_contains_key(roles,role);
</code></pre>



<a id="@Specification_1_get_vesting_account_signer"></a>

### Function `get_vesting_account_signer`


<pre><code>public fun get_vesting_account_signer(admin: &amp;signer, contract_address: address): signer
</code></pre>




<pre><code>include VerifyAdminAbortsIf;
</code></pre>



<a id="@Specification_1_get_vesting_account_signer_internal"></a>

### Function `get_vesting_account_signer_internal`


<pre><code>fun get_vesting_account_signer_internal(vesting_contract: &amp;vesting::VestingContract): signer
</code></pre>




<pre><code>aborts_if false;
</code></pre>




<a id="0x1_vesting_spec_get_vesting_account_signer"></a>


<pre><code>fun spec_get_vesting_account_signer(vesting_contract: VestingContract): signer;
</code></pre>



<a id="@Specification_1_create_vesting_contract_account"></a>

### Function `create_vesting_contract_account`


<pre><code>fun create_vesting_contract_account(admin: &amp;signer, contract_creation_seed: vector&lt;u8&gt;): (signer, account::SignerCapability)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
let admin_addr &#61; signer::address_of(admin);
let admin_store &#61; global&lt;AdminStore&gt;(admin_addr);
let seed &#61; bcs::to_bytes(admin_addr);
let nonce &#61; bcs::to_bytes(admin_store.nonce);
let first &#61; concat(seed, nonce);
let second &#61; concat(first, VESTING_POOL_SALT);
let end &#61; concat(second, contract_creation_seed);
// This enforces <a id="high-level-req-11" href="#high-level-req">high-level requirement 11</a>:
let resource_addr &#61; account::spec_create_resource_address(admin_addr, end);
aborts_if !exists&lt;AdminStore&gt;(admin_addr);
aborts_if len(account::ZERO_AUTH_KEY) !&#61; 32;
aborts_if admin_store.nonce &#43; 1 &gt; MAX_U64;
let ea &#61; account::exists_at(resource_addr);
include if (ea) account::CreateResourceAccountAbortsIf else account::CreateAccountAbortsIf &#123;addr: resource_addr&#125;;
let acc &#61; global&lt;account::Account&gt;(resource_addr);
let post post_acc &#61; global&lt;account::Account&gt;(resource_addr);
aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; !aptos_std::type_info::spec_is_struct&lt;AptosCoin&gt;();
aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; ea &amp;&amp; acc.guid_creation_num &#43; 2 &gt; MAX_U64;
aborts_if !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; ea &amp;&amp; acc.guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;
ensures exists&lt;account::Account&gt;(resource_addr) &amp;&amp; post_acc.authentication_key &#61;&#61; account::ZERO_AUTH_KEY &amp;&amp;
        exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr);
ensures signer::address_of(result_1) &#61;&#61; resource_addr;
ensures result_2.account &#61;&#61; resource_addr;
</code></pre>



<a id="@Specification_1_verify_admin"></a>

### Function `verify_admin`


<pre><code>fun verify_admin(admin: &amp;signer, vesting_contract: &amp;vesting::VestingContract)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-9" href="#high-level-req">high-level requirement 9</a>:
aborts_if signer::address_of(admin) !&#61; vesting_contract.admin;
</code></pre>



<a id="@Specification_1_assert_vesting_contract_exists"></a>

### Function `assert_vesting_contract_exists`


<pre><code>fun assert_vesting_contract_exists(contract_address: address)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
aborts_if !exists&lt;VestingContract&gt;(contract_address);
</code></pre>



<a id="@Specification_1_assert_active_vesting_contract"></a>

### Function `assert_active_vesting_contract`


<pre><code>fun assert_active_vesting_contract(contract_address: address)
</code></pre>




<pre><code>include ActiveVestingContractAbortsIf&lt;VestingContract&gt;;
</code></pre>



<a id="@Specification_1_unlock_stake"></a>

### Function `unlock_stake`


<pre><code>fun unlock_stake(vesting_contract: &amp;vesting::VestingContract, amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
include UnlockStakeAbortsIf;
</code></pre>




<a id="0x1_vesting_UnlockStakeAbortsIf"></a>


<pre><code>schema UnlockStakeAbortsIf &#123;
    vesting_contract: &amp;VestingContract;
    amount: u64;
    let acc &#61; vesting_contract.signer_cap.account;
    let operator &#61; vesting_contract.staking.operator;
    include amount !&#61; 0 &#61;&#61;&gt; staking_contract::ContractExistsAbortsIf &#123; staker: acc, operator &#125;;
    let store &#61; global&lt;staking_contract::Store&gt;(acc);
    let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);
    include amount !&#61; 0 &#61;&#61;&gt; DistributeInternalAbortsIf &#123; staker: acc, operator, staking_contract, distribute_events: store.distribute_events &#125;;
&#125;
</code></pre>



<a id="@Specification_1_withdraw_stake"></a>

### Function `withdraw_stake`


<pre><code>fun withdraw_stake(vesting_contract: &amp;vesting::VestingContract, contract_address: address): coin::Coin&lt;aptos_coin::AptosCoin&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
include WithdrawStakeAbortsIf;
</code></pre>




<a id="0x1_vesting_WithdrawStakeAbortsIf"></a>


<pre><code>schema WithdrawStakeAbortsIf &#123;
    vesting_contract: &amp;VestingContract;
    contract_address: address;
    let operator &#61; vesting_contract.staking.operator;
    include staking_contract::ContractExistsAbortsIf &#123; staker: contract_address, operator &#125;;
    let store &#61; global&lt;staking_contract::Store&gt;(contract_address);
    let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);
    include DistributeInternalAbortsIf &#123; staker: contract_address, operator, staking_contract, distribute_events: store.distribute_events &#125;;
&#125;
</code></pre>




<a id="0x1_vesting_DistributeInternalAbortsIf"></a>


<pre><code>schema DistributeInternalAbortsIf &#123;
    staker: address;
    operator: address;
    staking_contract: staking_contract::StakingContract;
    distribute_events: EventHandle&lt;staking_contract::DistributeEvent&gt;;
    let pool_address &#61; staking_contract.pool_address;
    aborts_if !exists&lt;stake::StakePool&gt;(pool_address);
    let stake_pool &#61; global&lt;stake::StakePool&gt;(pool_address);
    let inactive &#61; stake_pool.inactive.value;
    let pending_inactive &#61; stake_pool.pending_inactive.value;
    aborts_if inactive &#43; pending_inactive &gt; MAX_U64;
    let total_potential_withdrawable &#61; inactive &#43; pending_inactive;
    let pool_address_1 &#61; staking_contract.owner_cap.pool_address;
    aborts_if !exists&lt;stake::StakePool&gt;(pool_address_1);
    let stake_pool_1 &#61; global&lt;stake::StakePool&gt;(pool_address_1);
    aborts_if !exists&lt;stake::ValidatorSet&gt;(@aptos_framework);
    let validator_set &#61; global&lt;stake::ValidatorSet&gt;(@aptos_framework);
    let inactive_state &#61; !stake::spec_contains(validator_set.pending_active, pool_address_1)
        &amp;&amp; !stake::spec_contains(validator_set.active_validators, pool_address_1)
        &amp;&amp; !stake::spec_contains(validator_set.pending_inactive, pool_address_1);
    let inactive_1 &#61; stake_pool_1.inactive.value;
    let pending_inactive_1 &#61; stake_pool_1.pending_inactive.value;
    let new_inactive_1 &#61; inactive_1 &#43; pending_inactive_1;
    aborts_if inactive_state &amp;&amp; timestamp::spec_now_seconds() &gt;&#61; stake_pool_1.locked_until_secs
        &amp;&amp; inactive_1 &#43; pending_inactive_1 &gt; MAX_U64;
&#125;
</code></pre>



<a id="@Specification_1_get_beneficiary"></a>

### Function `get_beneficiary`


<pre><code>fun get_beneficiary(contract: &amp;vesting::VestingContract, shareholder: address): address
</code></pre>




<pre><code>// This enforces <a id="high-level-spec-3.2" href="#high-level-req">high-level requirement 3</a>:
aborts_if false;
</code></pre>




<a id="0x1_vesting_SetManagementRoleAbortsIf"></a>


<pre><code>schema SetManagementRoleAbortsIf &#123;
    contract_address: address;
    admin: signer;
    aborts_if !exists&lt;VestingContract&gt;(contract_address);
    let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
    aborts_if signer::address_of(admin) !&#61; vesting_contract.admin;
&#125;
</code></pre>




<a id="0x1_vesting_VerifyAdminAbortsIf"></a>


<pre><code>schema VerifyAdminAbortsIf &#123;
    contract_address: address;
    admin: signer;
    aborts_if !exists&lt;VestingContract&gt;(contract_address);
    let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
    aborts_if signer::address_of(admin) !&#61; vesting_contract.admin;
&#125;
</code></pre>




<a id="0x1_vesting_ActiveVestingContractAbortsIf"></a>


<pre><code>schema ActiveVestingContractAbortsIf&lt;VestingContract&gt; &#123;
    contract_address: address;
    // This enforces <a id="high-level-spec-5" href="#high-level-req">high-level requirement 5</a>:
    aborts_if !exists&lt;VestingContract&gt;(contract_address);
    let vesting_contract &#61; global&lt;VestingContract&gt;(contract_address);
    // This enforces <a id="high-level-spec-8" href="#high-level-req">high-level requirement 8</a>:
    aborts_if vesting_contract.state !&#61; VESTING_POOL_ACTIVE;
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
