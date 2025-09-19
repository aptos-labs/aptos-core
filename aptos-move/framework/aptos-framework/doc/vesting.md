
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
-  [Struct `VestPermission`](#0x1_vesting_VestPermission)
-  [Constants](#@Constants_0)
-  [Function `check_vest_permission`](#0x1_vesting_check_vest_permission)
-  [Function `grant_permission`](#0x1_vesting_grant_permission)
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64">0x1::pool_u64</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="staking_contract.md#0x1_staking_contract">0x1::staking_contract</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_vesting_VestingSchedule"></a>

## Struct `VestingSchedule`



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>&gt;</code>
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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_StakingInfo">StakingInfo</a> <b>has</b> store
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
<code>voter: <b>address</b></code>
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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> <b>has</b> key
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
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>grant_pool: <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a></code>
</dt>
<dd>

</dd>
<dt>
<code>beneficiaries: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_schedule: <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a></code>
</dt>
<dd>

</dd>
<dt>
<code>withdrawal_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking: <a href="vesting.md#0x1_vesting_StakingInfo">vesting::StakingInfo</a></code>
</dt>
<dd>

</dd>
<dt>
<code>remaining_grant: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>

</dd>
<dt>
<code>update_operator_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_UpdateOperatorEvent">vesting::UpdateOperatorEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_voter_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_UpdateVoterEvent">vesting::UpdateVoterEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reset_lockup_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_ResetLockupEvent">vesting::ResetLockupEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>set_beneficiary_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_SetBeneficiaryEvent">vesting::SetBeneficiaryEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_rewards_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_UnlockRewardsEvent">vesting::UnlockRewardsEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vest_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_VestEvent">vesting::VestEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>distribute_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_DistributeEvent">vesting::DistributeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>terminate_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_TerminateEvent">vesting::TerminateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>admin_withdraw_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_AdminWithdrawEvent">vesting::AdminWithdrawEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_VestingAccountManagement"></a>

## Resource `VestingAccountManagement`



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>roles: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_AdminStore"></a>

## Resource `AdminStore`



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>vesting_contracts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>nonce: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>create_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting.md#0x1_vesting_CreateVestingContractEvent">vesting::CreateVestingContractEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_CreateVestingContract"></a>

## Struct `CreateVestingContract`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_CreateVestingContract">CreateVestingContract</a> <b>has</b> drop, store
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
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>grant_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>withdrawal_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_UpdateOperator">UpdateOperator</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
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
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_UpdateVoter"></a>

## Struct `UpdateVoter`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_UpdateVoter">UpdateVoter</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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

<a id="0x1_vesting_ResetLockup"></a>

## Struct `ResetLockup`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_ResetLockup">ResetLockup</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_SetBeneficiary">SetBeneficiary</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>shareholder: <b>address</b></code>
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

<a id="0x1_vesting_UnlockRewards"></a>

## Struct `UnlockRewards`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_UnlockRewards">UnlockRewards</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_Vest">Vest</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_Distribute">Distribute</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
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



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_Terminate">Terminate</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_AdminWithdraw"></a>

## Struct `AdminWithdraw`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="vesting.md#0x1_vesting_AdminWithdraw">AdminWithdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_CreateVestingContractEvent">CreateVestingContractEvent</a> <b>has</b> drop, store
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
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>grant_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>withdrawal_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_UpdateOperatorEvent">UpdateOperatorEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
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
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_UpdateVoterEvent"></a>

## Struct `UpdateVoterEvent`



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_UpdateVoterEvent">UpdateVoterEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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

<a id="0x1_vesting_ResetLockupEvent"></a>

## Struct `ResetLockupEvent`



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_ResetLockupEvent">ResetLockupEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_SetBeneficiaryEvent">SetBeneficiaryEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>shareholder: <b>address</b></code>
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

<a id="0x1_vesting_UnlockRewardsEvent"></a>

## Struct `UnlockRewardsEvent`



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_UnlockRewardsEvent">UnlockRewardsEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestEvent">VestEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>staking_pool_address: <b>address</b></code>
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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_DistributeEvent">DistributeEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_TerminateEvent">TerminateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_AdminWithdrawEvent"></a>

## Struct `AdminWithdrawEvent`



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_AdminWithdrawEvent">AdminWithdrawEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
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

<a id="0x1_vesting_VestPermission"></a>

## Struct `VestPermission`

Permissions to mutate the vesting config for a given account.


<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestPermission">VestPermission</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_vesting_EEMPTY_VESTING_SCHEDULE"></a>

Vesting schedule cannot be empty.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>: u64 = 2;
</code></pre>



<a id="0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS"></a>

Withdrawal address is invalid.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS">EINVALID_WITHDRAWAL_ADDRESS</a>: u64 = 1;
</code></pre>



<a id="0x1_vesting_ENOT_ADMIN"></a>

The signer is not the admin of the vesting contract.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ENOT_ADMIN">ENOT_ADMIN</a>: u64 = 7;
</code></pre>



<a id="0x1_vesting_ENO_SHAREHOLDERS"></a>

Shareholders list cannot be empty.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>: u64 = 4;
</code></pre>



<a id="0x1_vesting_ENO_VESTING_PERMISSION"></a>

Current permissioned signer cannot perform vesting operations.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ENO_VESTING_PERMISSION">ENO_VESTING_PERMISSION</a>: u64 = 17;
</code></pre>



<a id="0x1_vesting_EPENDING_STAKE_FOUND"></a>

Cannot terminate the vesting contract with pending active stake. Need to wait until next epoch.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EPENDING_STAKE_FOUND">EPENDING_STAKE_FOUND</a>: u64 = 11;
</code></pre>



<a id="0x1_vesting_EPERMISSION_DENIED"></a>

Account is not admin or does not have the required role to take this action.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EPERMISSION_DENIED">EPERMISSION_DENIED</a>: u64 = 15;
</code></pre>



<a id="0x1_vesting_EROLE_NOT_FOUND"></a>

The vesting account has no such management role.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EROLE_NOT_FOUND">EROLE_NOT_FOUND</a>: u64 = 14;
</code></pre>



<a id="0x1_vesting_ESHARES_LENGTH_MISMATCH"></a>

The length of shareholders and shares lists don't match.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ESHARES_LENGTH_MISMATCH">ESHARES_LENGTH_MISMATCH</a>: u64 = 5;
</code></pre>



<a id="0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION"></a>

Zero items were provided to a *_many function.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>: u64 = 16;
</code></pre>



<a id="0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES"></a>

Vesting account has no other management roles beside admin.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES">EVESTING_ACCOUNT_HAS_NO_ROLES</a>: u64 = 13;
</code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE"></a>

Vesting contract needs to be in active state.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE">EVESTING_CONTRACT_NOT_ACTIVE</a>: u64 = 8;
</code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_NOT_FOUND"></a>

No vesting contract found at provided address.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_FOUND">EVESTING_CONTRACT_NOT_FOUND</a>: u64 = 10;
</code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE"></a>

Admin can only withdraw from an inactive (paused or terminated) vesting contract.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE">EVESTING_CONTRACT_STILL_ACTIVE</a>: u64 = 9;
</code></pre>



<a id="0x1_vesting_EVESTING_START_TOO_SOON"></a>

Vesting cannot start before or at the current block timestamp. Has to be in the future.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_START_TOO_SOON">EVESTING_START_TOO_SOON</a>: u64 = 6;
</code></pre>



<a id="0x1_vesting_EZERO_GRANT"></a>

Grant amount cannot be 0.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EZERO_GRANT">EZERO_GRANT</a>: u64 = 12;
</code></pre>



<a id="0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD"></a>

Vesting period cannot be 0.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD">EZERO_VESTING_SCHEDULE_PERIOD</a>: u64 = 3;
</code></pre>



<a id="0x1_vesting_MAXIMUM_SHAREHOLDERS"></a>

Maximum number of shareholders a vesting pool can support.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_MAXIMUM_SHAREHOLDERS">MAXIMUM_SHAREHOLDERS</a>: u64 = 30;
</code></pre>



<a id="0x1_vesting_ROLE_BENEFICIARY_RESETTER"></a>

Roles that can manage certain aspects of the vesting account beyond the main admin.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [82, 79, 76, 69, 95, 66, 69, 78, 69, 70, 73, 67, 73, 65, 82, 89, 95, 82, 69, 83, 69, 84, 84, 69, 82];
</code></pre>



<a id="0x1_vesting_VESTING_POOL_ACTIVE"></a>

Vesting contract states.
Vesting contract is active and distributions can be made.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>: u64 = 1;
</code></pre>



<a id="0x1_vesting_VESTING_POOL_SALT"></a>



<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_SALT">VESTING_POOL_SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 118, 101, 115, 116, 105, 110, 103];
</code></pre>



<a id="0x1_vesting_VESTING_POOL_TERMINATED"></a>

Vesting contract has been terminated and all funds have been released back to the withdrawal address.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>: u64 = 2;
</code></pre>



<a id="0x1_vesting_check_vest_permission"></a>

## Function `check_vest_permission`

Permissions


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_check_vest_permission">check_vest_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="vesting.md#0x1_vesting_check_vest_permission">check_vest_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">permissioned_signer::check_permission_exists</a>(s, <a href="vesting.md#0x1_vesting_VestPermission">VestPermission</a> {}),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="vesting.md#0x1_vesting_ENO_VESTING_PERMISSION">ENO_VESTING_PERMISSION</a>),
    );
}
</code></pre>



</details>

<a id="0x1_vesting_grant_permission"></a>

## Function `grant_permission`

Grant permission to perform vesting operations on behalf of the master signer.


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_grant_permission">grant_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_grant_permission">grant_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">permissioned_signer::authorize_unlimited</a>(master, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>, <a href="vesting.md#0x1_vesting_VestPermission">VestPermission</a> {})
}
</code></pre>



</details>

<a id="0x1_vesting_stake_pool_address"></a>

## Function `stake_pool_address`

Return the address of the underlying stake pool (separate resource account) of the vesting contract.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_stake_pool_address">stake_pool_address</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_stake_pool_address">stake_pool_address</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.pool_address
}
</code></pre>



</details>

<a id="0x1_vesting_vesting_start_secs"></a>

## Function `vesting_start_secs`

Return the vesting start timestamp (in seconds) of the vesting contract.
Vesting will start at this time, and once a full period has passed, the first vest will become unlocked.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule.start_timestamp_secs
}
</code></pre>



</details>

<a id="0x1_vesting_period_duration_secs"></a>

## Function `period_duration_secs`

Return the duration of one vesting period (in seconds).
Each vest is released after one full period has started, starting from the specified start_timestamp_secs.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule.period_duration
}
</code></pre>



</details>

<a id="0x1_vesting_remaining_grant"></a>

## Function `remaining_grant`

Return the remaining grant, consisting of unvested coins that have not been distributed to shareholders.
Prior to start_timestamp_secs, the remaining grant will always be equal to the original grant.
Once vesting has started, and vested tokens are distributed, the remaining grant will decrease over time,
according to the vesting schedule.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).remaining_grant
}
</code></pre>



</details>

<a id="0x1_vesting_beneficiary"></a>

## Function `beneficiary`

Return the beneficiary account of the specified shareholder in a vesting contract.
This is the same as the shareholder address by default and only different if it's been explicitly set.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(<b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address), shareholder)
}
</code></pre>



</details>

<a id="0x1_vesting_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the percentage of accumulated rewards that is paid to the operator as commission.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.commission_percentage
}
</code></pre>



</details>

<a id="0x1_vesting_vesting_contracts"></a>

## Function `vesting_contracts`

Return all the vesting contracts a given address is an admin of.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;()
    } <b>else</b> {
        <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin).vesting_contracts
    }
}
</code></pre>



</details>

<a id="0x1_vesting_operator"></a>

## Function `operator`

Return the operator who runs the validator for the vesting contract.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator">operator</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator">operator</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.operator
}
</code></pre>



</details>

<a id="0x1_vesting_voter"></a>

## Function `voter`

Return the voter who will be voting on on-chain governance proposals on behalf of the vesting contract's stake
pool.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_voter">voter</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_voter">voter</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.voter
}
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


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_schedule">vesting_schedule</a>(vesting_contract_address: <b>address</b>): <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_schedule">vesting_schedule</a>(vesting_contract_address: <b>address</b>): <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule
}
</code></pre>



</details>

<a id="0x1_vesting_total_accumulated_rewards"></a>

## Function `total_accumulated_rewards`

Return the total accumulated rewards that have not been distributed to shareholders of the vesting contract.
This excludes any unpaid commission that the operator has not collected.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);

    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
    <b>let</b> (total_active_stake, _, commission_amount) =
        <a href="staking_contract.md#0x1_staking_contract_staking_contract_amounts">staking_contract::staking_contract_amounts</a>(vesting_contract_address, vesting_contract.staking.operator);
    total_active_stake - vesting_contract.remaining_grant - commission_amount
}
</code></pre>



</details>

<a id="0x1_vesting_accumulated_rewards"></a>

## Function `accumulated_rewards`

Return the accumulated rewards that have not been distributed to the provided shareholder. Caller can also pass
the beneficiary address instead of shareholder address.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_accumulated_rewards">accumulated_rewards</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_accumulated_rewards">accumulated_rewards</a>(
    vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);

    <b>let</b> total_accumulated_rewards = <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(vesting_contract_address);
    <b>let</b> shareholder = <a href="vesting.md#0x1_vesting_shareholder">shareholder</a>(vesting_contract_address, shareholder_or_beneficiary);
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
    <b>let</b> shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(&vesting_contract.grant_pool, shareholder);
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">pool_u64::shares_to_amount_with_total_coins</a>(&vesting_contract.grant_pool, shares, total_accumulated_rewards)
}
</code></pre>



</details>

<a id="0x1_vesting_shareholders"></a>

## Function `shareholders`

Return the list of all shareholders in the vesting contract.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholders">shareholders</a>(vesting_contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholders">shareholders</a>(vesting_contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);

    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders">pool_u64::shareholders</a>(&vesting_contract.grant_pool)
}
</code></pre>



</details>

<a id="0x1_vesting_shareholder"></a>

## Function `shareholder`

Return the shareholder address given the beneficiary address in a given vesting contract. If there are multiple
shareholders with the same beneficiary address, only the first shareholder is returned. If the given beneficiary
address is actually a shareholder address, just return the address back.

This returns 0x0 if no shareholder is found for the given beneficiary / the address is not a shareholder itself.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholder">shareholder</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholder">shareholder</a>(
    vesting_contract_address: <b>address</b>,
    shareholder_or_beneficiary: <b>address</b>
): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);

    <b>let</b> shareholders = &<a href="vesting.md#0x1_vesting_shareholders">shareholders</a>(vesting_contract_address);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(shareholders, &shareholder_or_beneficiary)) {
        <b>return</b> shareholder_or_beneficiary
    };
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
    <b>let</b> result = @0x0;
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_any">vector::any</a>(shareholders, |shareholder| {
        <b>if</b> (shareholder_or_beneficiary == <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(vesting_contract, *shareholder)) {
            result = *shareholder;
            <b>true</b>
        } <b>else</b> {
            <b>false</b>
        }
    });

    result
}
</code></pre>



</details>

<a id="0x1_vesting_create_vesting_schedule"></a>

## Function `create_vesting_schedule`

Create a vesting schedule with the given schedule of distributions, a vesting start time and period duration.


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_schedule">create_vesting_schedule</a>(schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>&gt;, start_timestamp_secs: u64, period_duration: u64): <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_schedule">create_vesting_schedule</a>(
    schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;FixedPoint32&gt;,
    start_timestamp_secs: u64,
    period_duration: u64,
): <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&schedule) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>));
    <b>assert</b>!(period_duration &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD">EZERO_VESTING_SCHEDULE_PERIOD</a>));
    <b>assert</b>!(
        start_timestamp_secs &gt;= <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EVESTING_START_TOO_SOON">EVESTING_START_TOO_SOON</a>),
    );

    <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a> {
        schedule,
        start_timestamp_secs,
        period_duration,
        last_vested_period: 0,
    }
}
</code></pre>



</details>

<a id="0x1_vesting_create_vesting_contract"></a>

## Function `create_vesting_contract`

Create a vesting contract with a given configurations.


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract">create_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, shareholders: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, buy_ins: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;&gt;, vesting_schedule: <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a>, withdrawal_address: <b>address</b>, operator: <b>address</b>, voter: <b>address</b>, commission_percentage: u64, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract">create_vesting_contract</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    shareholders: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    buy_ins: SimpleMap&lt;<b>address</b>, Coin&lt;AptosCoin&gt;&gt;,
    vesting_schedule: <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a>,
    withdrawal_address: <b>address</b>,
    operator: <b>address</b>,
    voter: <b>address</b>,
    commission_percentage: u64,
    // Optional seed used when creating the staking contract <a href="account.md#0x1_account">account</a>.
    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> {
    <a href="vesting.md#0x1_vesting_check_vest_permission">check_vest_permission</a>(admin);
    <b>assert</b>!(
        !<a href="system_addresses.md#0x1_system_addresses_is_reserved_address">system_addresses::is_reserved_address</a>(withdrawal_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS">EINVALID_WITHDRAWAL_ADDRESS</a>),
    );
    assert_account_is_registered_for_apt(withdrawal_address);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shareholders) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>));
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_length">simple_map::length</a>(&buy_ins) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shareholders),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_ESHARES_LENGTH_MISMATCH">ESHARES_LENGTH_MISMATCH</a>),
    );

    // Create a coins pool <b>to</b> track shareholders and shares of the grant.
    <b>let</b> grant = <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;AptosCoin&gt;();
    <b>let</b> grant_amount = 0;
    <b>let</b> grant_pool = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create">pool_u64::create</a>(<a href="vesting.md#0x1_vesting_MAXIMUM_SHAREHOLDERS">MAXIMUM_SHAREHOLDERS</a>);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(shareholders, |shareholder| {
        <b>let</b> shareholder: <b>address</b> = *shareholder;
        <b>let</b> (_, buy_in) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> buy_ins, &shareholder);
        <b>let</b> buy_in_amount = <a href="coin.md#0x1_coin_value">coin::value</a>(&buy_in);
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> grant, buy_in);
        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(
            &<b>mut</b> grant_pool,
            shareholder,
            buy_in_amount,
        );
        grant_amount = grant_amount + buy_in_amount;
    });
    <b>assert</b>!(grant_amount &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EZERO_GRANT">EZERO_GRANT</a>));

    // If this is the first time this admin <a href="account.md#0x1_account">account</a> <b>has</b> created a <a href="vesting.md#0x1_vesting">vesting</a> contract, initialize the admin store.
    <b>let</b> admin_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin);
    <b>if</b> (!<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin_address)) {
        <b>move_to</b>(admin, <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> {
            vesting_contracts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;(),
            nonce: 0,
            create_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_CreateVestingContractEvent">CreateVestingContractEvent</a>&gt;(admin),
        });
    };

    // Initialize the <a href="vesting.md#0x1_vesting">vesting</a> contract in a new resource <a href="account.md#0x1_account">account</a>. This allows the same admin <b>to</b> create multiple
    // pools.
    <b>let</b> (contract_signer, contract_signer_cap) = <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(admin, contract_creation_seed);
    <b>let</b> pool_address = <a href="staking_contract.md#0x1_staking_contract_create_staking_contract_with_coins">staking_contract::create_staking_contract_with_coins</a>(
        &contract_signer, operator, voter, grant, commission_percentage, contract_creation_seed);

    // Add the newly created <a href="vesting.md#0x1_vesting">vesting</a> contract's <b>address</b> <b>to</b> the admin store.
    <b>let</b> contract_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&contract_signer);
    <b>let</b> admin_store = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin_address);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> admin_store.vesting_contracts, contract_address);
    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_CreateVestingContract">CreateVestingContract</a> {
                operator,
                voter,
                withdrawal_address,
                grant_amount,
                vesting_contract_address: contract_address,
                staking_pool_address: pool_address,
                commission_percentage,
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> admin_store.create_events,
            <a href="vesting.md#0x1_vesting_CreateVestingContractEvent">CreateVestingContractEvent</a> {
                operator,
                voter,
                withdrawal_address,
                grant_amount,
                vesting_contract_address: contract_address,
                staking_pool_address: pool_address,
                commission_percentage,
            },
        );
    };

    <b>move_to</b>(&contract_signer, <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
        state: <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>,
        admin: admin_address,
        grant_pool,
        beneficiaries: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, <b>address</b>&gt;(),
        vesting_schedule,
        withdrawal_address,
        staking: <a href="vesting.md#0x1_vesting_StakingInfo">StakingInfo</a> { pool_address, operator, voter, commission_percentage },
        remaining_grant: grant_amount,
        signer_cap: contract_signer_cap,
        update_operator_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_UpdateOperatorEvent">UpdateOperatorEvent</a>&gt;(&contract_signer),
        update_voter_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_UpdateVoterEvent">UpdateVoterEvent</a>&gt;(&contract_signer),
        reset_lockup_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_ResetLockupEvent">ResetLockupEvent</a>&gt;(&contract_signer),
        set_beneficiary_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_SetBeneficiaryEvent">SetBeneficiaryEvent</a>&gt;(&contract_signer),
        unlock_rewards_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_UnlockRewardsEvent">UnlockRewardsEvent</a>&gt;(&contract_signer),
        vest_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_VestEvent">VestEvent</a>&gt;(&contract_signer),
        distribute_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_DistributeEvent">DistributeEvent</a>&gt;(&contract_signer),
        terminate_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_TerminateEvent">TerminateEvent</a>&gt;(&contract_signer),
        admin_withdraw_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_AdminWithdrawEvent">AdminWithdrawEvent</a>&gt;(&contract_signer),
    });

    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_destroy_empty">simple_map::destroy_empty</a>(buy_ins);
    contract_address
}
</code></pre>



</details>

<a id="0x1_vesting_unlock_rewards"></a>

## Function `unlock_rewards`

Unlock any accumulated rewards.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> accumulated_rewards = <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(contract_address);
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract, accumulated_rewards);
}
</code></pre>



</details>

<a id="0x1_vesting_unlock_rewards_many"></a>

## Function `unlock_rewards_many`

Call <code>unlock_rewards</code> for many vesting contracts.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards_many">unlock_rewards_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards_many">unlock_rewards_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&contract_addresses);

    <b>assert</b>!(len != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>));

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&contract_addresses, |contract_address| {
        <b>let</b> contract_address: <b>address</b> = *contract_address;
        <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address);
    });
}
</code></pre>



</details>

<a id="0x1_vesting_vest"></a>

## Function `vest`

Unlock any vested portion of the grant.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest">vest</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest">vest</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    // Unlock all rewards first, <b>if</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a>.
    <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address);

    // Unlock the vested amount. This amount will become withdrawable when the underlying <a href="stake.md#0x1_stake">stake</a> pool's lockup
    // expires.
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    // Short-circuit <b>if</b> <a href="vesting.md#0x1_vesting">vesting</a> hasn't started yet.
    <b>if</b> (vesting_contract.vesting_schedule.start_timestamp_secs &gt; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()) {
        <b>return</b>
    };

    // Check <b>if</b> the next vested period <b>has</b> already passed. If not, short-circuit since there's nothing <b>to</b> vest.
    <b>let</b> vesting_schedule = &<b>mut</b> vesting_contract.vesting_schedule;
    <b>let</b> last_vested_period = vesting_schedule.last_vested_period;
    <b>let</b> next_period_to_vest = last_vested_period + 1;
    <b>let</b> last_completed_period =
        (<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() - vesting_schedule.start_timestamp_secs) / vesting_schedule.period_duration;
    <b>if</b> (last_completed_period &lt; next_period_to_vest) {
        <b>return</b>
    };

    // Calculate how much <b>has</b> vested, excluding rewards.
    // Index is 0-based <b>while</b> period is 1-based so we need <b>to</b> subtract 1.
    <b>let</b> schedule = &vesting_schedule.schedule;
    <b>let</b> schedule_index = next_period_to_vest - 1;
    <b>let</b> vesting_fraction = <b>if</b> (schedule_index &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(schedule)) {
        *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(schedule, schedule_index)
    } <b>else</b> {
        // Last <a href="vesting.md#0x1_vesting">vesting</a> schedule fraction will repeat until the grant runs out.
        *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(schedule, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(schedule) - 1)
    };
    <b>let</b> total_grant = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_total_coins">pool_u64::total_coins</a>(&vesting_contract.grant_pool);
    <b>let</b> vested_amount = <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_multiply_u64">fixed_point32::multiply_u64</a>(total_grant, vesting_fraction);
    // Cap vested amount by the remaining grant amount so we don't try <b>to</b> distribute more than what's remaining.
    vested_amount = <b>min</b>(vested_amount, vesting_contract.remaining_grant);
    vesting_contract.remaining_grant = vesting_contract.remaining_grant - vested_amount;
    vesting_schedule.last_vested_period = next_period_to_vest;
    <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract, vested_amount);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_Vest">Vest</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                period_vested: next_period_to_vest,
                amount: vested_amount,
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> vesting_contract.vest_events,
            <a href="vesting.md#0x1_vesting_VestEvent">VestEvent</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                period_vested: next_period_to_vest,
                amount: vested_amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_vesting_vest_many"></a>

## Function `vest_many`

Call <code>vest</code> for many vesting contracts.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest_many">vest_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest_many">vest_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&contract_addresses);

    <b>assert</b>!(len != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>));

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&contract_addresses, |contract_address| {
        <b>let</b> contract_address = *contract_address;
        <a href="vesting.md#0x1_vesting_vest">vest</a>(contract_address);
    });
}
</code></pre>



</details>

<a id="0x1_vesting_distribute"></a>

## Function `distribute`

Distribute any withdrawable stake from the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> coins = <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract, contract_address);
    <b>let</b> total_distribution_amount = <a href="coin.md#0x1_coin_value">coin::value</a>(&coins);
    <b>if</b> (total_distribution_amount == 0) {
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);
        <b>return</b>
    };

    // <a href="vesting.md#0x1_vesting_Distribute">Distribute</a> coins <b>to</b> all shareholders in the <a href="vesting.md#0x1_vesting">vesting</a> contract.
    <b>let</b> grant_pool = &vesting_contract.grant_pool;
    <b>let</b> shareholders = &<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders">pool_u64::shareholders</a>(grant_pool);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(shareholders, |shareholder| {
        <b>let</b> shareholder = *shareholder;
        <b>let</b> shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(grant_pool, shareholder);
        <b>let</b> amount = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">pool_u64::shares_to_amount_with_total_coins</a>(grant_pool, shares, total_distribution_amount);
        <b>let</b> share_of_coins = <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> coins, amount);
        <b>let</b> recipient_address = <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(vesting_contract, shareholder);
        <a href="aptos_account.md#0x1_aptos_account_deposit_coins">aptos_account::deposit_coins</a>(recipient_address, share_of_coins);
    });

    // Send <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> remaining "dust" (leftover due <b>to</b> rounding <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a>) <b>to</b> the withdrawal <b>address</b>.
    <b>if</b> (<a href="coin.md#0x1_coin_value">coin::value</a>(&coins) &gt; 0) {
        <a href="aptos_account.md#0x1_aptos_account_deposit_coins">aptos_account::deposit_coins</a>(vesting_contract.withdrawal_address, coins);
    } <b>else</b> {
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);
    };

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_Distribute">Distribute</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                amount: total_distribution_amount,
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> vesting_contract.distribute_events,
            <a href="vesting.md#0x1_vesting_DistributeEvent">DistributeEvent</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                amount: total_distribution_amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_vesting_distribute_many"></a>

## Function `distribute_many`

Call <code>distribute</code> for many vesting contracts.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute_many">distribute_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute_many">distribute_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&contract_addresses);

    <b>assert</b>!(len != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>));

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&contract_addresses, |contract_address| {
        <b>let</b> contract_address = *contract_address;
        <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address);
    });
}
</code></pre>



</details>

<a id="0x1_vesting_terminate_vesting_contract"></a>

## Function `terminate_vesting_contract`

Terminate the vesting contract and send all funds back to the withdrawal address.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);

    // <a href="vesting.md#0x1_vesting_Distribute">Distribute</a> all withdrawable coins, which should have been from previous rewards withdrawal or vest.
    <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address);

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> (active_stake, _, pending_active_stake, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(vesting_contract.staking.pool_address);
    <b>assert</b>!(pending_active_stake == 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting.md#0x1_vesting_EPENDING_STAKE_FOUND">EPENDING_STAKE_FOUND</a>));

    // Unlock all remaining active <a href="stake.md#0x1_stake">stake</a>.
    vesting_contract.state = <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>;
    vesting_contract.remaining_grant = 0;
    <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract, active_stake);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_Terminate">Terminate</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> vesting_contract.terminate_events,
            <a href="vesting.md#0x1_vesting_TerminateEvent">TerminateEvent</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_vesting_admin_withdraw"></a>

## Function `admin_withdraw`

Withdraw all funds to the preset vesting contract's withdrawal address. This can only be called if the contract
has already been terminated.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_admin_withdraw">admin_withdraw</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_admin_withdraw">admin_withdraw</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>assert</b>!(
        vesting_contract.state == <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE">EVESTING_CONTRACT_STILL_ACTIVE</a>)
    );

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> coins = <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract, contract_address);
    <b>let</b> amount = <a href="coin.md#0x1_coin_value">coin::value</a>(&coins);
    <b>if</b> (amount == 0) {
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);
        <b>return</b>
    };
    <a href="aptos_account.md#0x1_aptos_account_deposit_coins">aptos_account::deposit_coins</a>(vesting_contract.withdrawal_address, coins);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_AdminWithdraw">AdminWithdraw</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                amount,
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> vesting_contract.admin_withdraw_events,
            <a href="vesting.md#0x1_vesting_AdminWithdrawEvent">AdminWithdrawEvent</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_vesting_update_operator"></a>

## Function `update_operator`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator">update_operator</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_operator: <b>address</b>, commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator">update_operator</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    new_operator: <b>address</b>,
    commission_percentage: u64,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> contract_signer = &<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <b>let</b> old_operator = vesting_contract.staking.operator;
    <a href="staking_contract.md#0x1_staking_contract_switch_operator">staking_contract::switch_operator</a>(contract_signer, old_operator, new_operator, commission_percentage);
    vesting_contract.staking.operator = new_operator;
    vesting_contract.staking.commission_percentage = commission_percentage;

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_UpdateOperator">UpdateOperator</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                old_operator,
                new_operator,
                commission_percentage,
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> vesting_contract.update_operator_events,
            <a href="vesting.md#0x1_vesting_UpdateOperatorEvent">UpdateOperatorEvent</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                old_operator,
                new_operator,
                commission_percentage,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_vesting_update_operator_with_same_commission"></a>

## Function `update_operator_with_same_commission`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator_with_same_commission">update_operator_with_same_commission</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_operator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator_with_same_commission">update_operator_with_same_commission</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    new_operator: <b>address</b>,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> commission_percentage = <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(contract_address);
    <a href="vesting.md#0x1_vesting_update_operator">update_operator</a>(admin, contract_address, new_operator, commission_percentage);
}
</code></pre>



</details>

<a id="0x1_vesting_update_commission_percentage"></a>

## Function `update_commission_percentage`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_commission_percentage">update_commission_percentage</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_commission_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_commission_percentage">update_commission_percentage</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    new_commission_percentage: u64,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> operator = <a href="vesting.md#0x1_vesting_operator">operator</a>(contract_address);
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> contract_signer = &<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <a href="staking_contract.md#0x1_staking_contract_update_commision">staking_contract::update_commision</a>(contract_signer, operator, new_commission_percentage);
    vesting_contract.staking.commission_percentage = new_commission_percentage;
    // This function does not emit an <a href="event.md#0x1_event">event</a>. Instead, `staking_contract::update_commission_percentage`
    // <b>emits</b> the <a href="event.md#0x1_event">event</a> for this commission percentage <b>update</b>.
}
</code></pre>



</details>

<a id="0x1_vesting_update_voter"></a>

## Function `update_voter`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_voter">update_voter</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_voter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_voter">update_voter</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    new_voter: <b>address</b>,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> contract_signer = &<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <b>let</b> old_voter = vesting_contract.staking.voter;
    <a href="staking_contract.md#0x1_staking_contract_update_voter">staking_contract::update_voter</a>(contract_signer, vesting_contract.staking.operator, new_voter);
    vesting_contract.staking.voter = new_voter;

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_UpdateVoter">UpdateVoter</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                old_voter,
                new_voter,
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> vesting_contract.update_voter_events,
            <a href="vesting.md#0x1_vesting_UpdateVoterEvent">UpdateVoterEvent</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                old_voter,
                new_voter,
            },
        );
    }
}
</code></pre>



</details>

<a id="0x1_vesting_reset_lockup"></a>

## Function `reset_lockup`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_lockup">reset_lockup</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_lockup">reset_lockup</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> contract_signer = &<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <a href="staking_contract.md#0x1_staking_contract_reset_lockup">staking_contract::reset_lockup</a>(contract_signer, vesting_contract.staking.operator);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_ResetLockup">ResetLockup</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                new_lockup_expiration_secs: <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(vesting_contract.staking.pool_address),
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> vesting_contract.reset_lockup_events,
            <a href="vesting.md#0x1_vesting_ResetLockupEvent">ResetLockupEvent</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                staking_pool_address: vesting_contract.staking.pool_address,
                new_lockup_expiration_secs: <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(vesting_contract.staking.pool_address),
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_vesting_set_beneficiary"></a>

## Function `set_beneficiary`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary">set_beneficiary</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>, new_beneficiary: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary">set_beneficiary</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    shareholder: <b>address</b>,
    new_beneficiary: <b>address</b>,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    // Verify that the beneficiary <a href="account.md#0x1_account">account</a> is set up <b>to</b> receive APT. This is a requirement so <a href="vesting.md#0x1_vesting_distribute">distribute</a>() wouldn't
    // fail and <a href="block.md#0x1_block">block</a> all other accounts from receiving APT <b>if</b> one beneficiary is not registered.
    assert_account_is_registered_for_apt(new_beneficiary);

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);

    <b>let</b> old_beneficiary = <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(vesting_contract, shareholder);
    <b>let</b> beneficiaries = &<b>mut</b> vesting_contract.beneficiaries;
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(beneficiaries, &shareholder)) {
        <b>let</b> beneficiary = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(beneficiaries, &shareholder);
        *beneficiary = new_beneficiary;
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(beneficiaries, shareholder, new_beneficiary);
    };

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="vesting.md#0x1_vesting_SetBeneficiary">SetBeneficiary</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                shareholder,
                old_beneficiary,
                new_beneficiary,
            },
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> vesting_contract.set_beneficiary_events,
            <a href="vesting.md#0x1_vesting_SetBeneficiaryEvent">SetBeneficiaryEvent</a> {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                shareholder,
                old_beneficiary,
                new_beneficiary,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x1_vesting_reset_beneficiary"></a>

## Function `reset_beneficiary`

Remove the beneficiary for the given shareholder. All distributions will sent directly to the shareholder
account.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_beneficiary">reset_beneficiary</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_beneficiary">reset_beneficiary</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    shareholder: <b>address</b>,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_check_vest_permission">check_vest_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(
        addr == vesting_contract.admin ||
            addr == <a href="vesting.md#0x1_vesting_get_role_holder">get_role_holder</a>(contract_address, utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>)),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="vesting.md#0x1_vesting_EPERMISSION_DENIED">EPERMISSION_DENIED</a>),
    );

    <b>let</b> beneficiaries = &<b>mut</b> vesting_contract.beneficiaries;
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(beneficiaries, &shareholder)) {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(beneficiaries, &shareholder);
    };
}
</code></pre>



</details>

<a id="0x1_vesting_set_management_role"></a>

## Function `set_management_role`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_management_role">set_management_role</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, role_holder: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_management_role">set_management_role</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    role: String,
    role_holder: <b>address</b>,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);

    <b>if</b> (!<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address)) {
        <b>let</b> contract_signer = &<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
        <b>move_to</b>(contract_signer, <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a> {
            roles: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <b>address</b>&gt;(),
        })
    };
    <b>let</b> roles = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(roles, &role)) {
        *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(roles, &role) = role_holder;
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(roles, role, role_holder);
    };
}
</code></pre>



</details>

<a id="0x1_vesting_set_beneficiary_resetter"></a>

## Function `set_beneficiary_resetter`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_resetter">set_beneficiary_resetter</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, beneficiary_resetter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_resetter">set_beneficiary_resetter</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    beneficiary_resetter: <b>address</b>,
) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_set_management_role">set_management_role</a>(admin, contract_address, utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>), beneficiary_resetter);
}
</code></pre>



</details>

<a id="0x1_vesting_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Set the beneficiary for the operator.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(operator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_beneficiary: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(
    operator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_beneficiary: <b>address</b>,
) {
    <a href="staking_contract.md#0x1_staking_contract_set_beneficiary_for_operator">staking_contract::set_beneficiary_for_operator</a>(operator, new_beneficiary);
}
</code></pre>



</details>

<a id="0x1_vesting_get_role_holder"></a>

## Function `get_role_holder`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_role_holder">get_role_holder</a>(contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_role_holder">get_role_holder</a>(contract_address: <b>address</b>, role: String): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting.md#0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES">EVESTING_ACCOUNT_HAS_NO_ROLES</a>));
    <b>let</b> roles = &<b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(roles, &role), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting.md#0x1_vesting_EROLE_NOT_FOUND">EROLE_NOT_FOUND</a>));
    *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(roles, &role)
}
</code></pre>



</details>

<a id="0x1_vesting_get_vesting_account_signer"></a>

## Function `get_vesting_account_signer`

For emergency use in case the admin needs emergency control of vesting contract account.
This doesn't give the admin total power as the admin would still need to follow the rules set by
staking_contract and stake modules.


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract)
}
</code></pre>



</details>

<a id="0x1_vesting_get_vesting_account_signer_internal"></a>

## Function `get_vesting_account_signer_internal`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&vesting_contract.signer_cap)
}
</code></pre>



</details>

<a id="0x1_vesting_create_vesting_contract_account"></a>

## Function `create_vesting_contract_account`

Create a salt for generating the resource accounts that will be holding the VestingContract.
This address should be deterministic for the same admin and vesting contract creation nonce.


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, SignerCapability) <b>acquires</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> {
    <a href="vesting.md#0x1_vesting_check_vest_permission">check_vest_permission</a>(admin);
    <b>let</b> admin_store = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin));
    <b>let</b> seed = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&admin_store.nonce));
    admin_store.nonce = admin_store.nonce + 1;

    // Include a salt <b>to</b> avoid conflicts <b>with</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> other modules out there that might also generate
    // deterministic resource accounts for the same admin <b>address</b> + nonce.
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, <a href="vesting.md#0x1_vesting_VESTING_POOL_SALT">VESTING_POOL_SALT</a>);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, contract_creation_seed);

    <b>let</b> (account_signer, signer_cap) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(admin, seed);
    // Register the <a href="vesting.md#0x1_vesting">vesting</a> contract <a href="account.md#0x1_account">account</a> <b>to</b> receive APT <b>as</b> it'll be sent <b>to</b> it when claiming unlocked <a href="stake.md#0x1_stake">stake</a> from
    // the underlying staking contract.
    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&account_signer);

    (account_signer, signer_cap)
}
</code></pre>



</details>

<a id="0x1_vesting_verify_admin"></a>

## Function `verify_admin`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>) {
    <a href="vesting.md#0x1_vesting_check_vest_permission">check_vest_permission</a>(admin);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) == vesting_contract.admin, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="vesting.md#0x1_vesting_ENOT_ADMIN">ENOT_ADMIN</a>));
}
</code></pre>



</details>

<a id="0x1_vesting_assert_vesting_contract_exists"></a>

## Function `assert_vesting_contract_exists`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>) {
    <b>assert</b>!(<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_FOUND">EVESTING_CONTRACT_NOT_FOUND</a>));
}
</code></pre>



</details>

<a id="0x1_vesting_assert_active_vesting_contract"></a>

## Function `assert_active_vesting_contract`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address);
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>assert</b>!(vesting_contract.state == <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE">EVESTING_CONTRACT_NOT_ACTIVE</a>));
}
</code></pre>



</details>

<a id="0x1_vesting_unlock_stake"></a>

## Function `unlock_stake`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>, amount: u64) {
    <b>let</b> contract_signer = &<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <a href="staking_contract.md#0x1_staking_contract_unlock_stake">staking_contract::unlock_stake</a>(contract_signer, vesting_contract.staking.operator, amount);
}
</code></pre>



</details>

<a id="0x1_vesting_withdraw_stake"></a>

## Function `withdraw_stake`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, contract_address: <b>address</b>): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>, contract_address: <b>address</b>): Coin&lt;AptosCoin&gt; {
    // Claim <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> withdrawable distribution from the staking contract. The withdrawn coins will be sent directly <b>to</b>
    // the <a href="vesting.md#0x1_vesting">vesting</a> contract's <a href="account.md#0x1_account">account</a>.
    <a href="staking_contract.md#0x1_staking_contract_distribute">staking_contract::distribute</a>(contract_address, vesting_contract.staking.operator);
    <b>let</b> withdrawn_coins = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(contract_address);
    <b>let</b> contract_signer = &<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;AptosCoin&gt;(contract_signer, withdrawn_coins)
}
</code></pre>



</details>

<a id="0x1_vesting_get_beneficiary"></a>

## Function `get_beneficiary`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, shareholder: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(contract: &<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>, shareholder: <b>address</b>): <b>address</b> {
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&contract.beneficiaries, &shareholder)) {
        *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&contract.beneficiaries, &shareholder)
    } <b>else</b> {
        shareholder
    }
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


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial;
// This enforces <a id="high-level-spec-2" href="#high-level-req">high-level requirement 2</a>:
<b>invariant</b> <b>forall</b> a: <b>address</b> <b>where</b> <b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(a):
    <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(a).grant_pool.shareholders_limit &lt;= <a href="vesting.md#0x1_vesting_MAXIMUM_SHAREHOLDERS">MAXIMUM_SHAREHOLDERS</a>;
</code></pre>




<a id="0x1_vesting_AbortsIfPermissionedSigner"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_AbortsIfPermissionedSigner">AbortsIfPermissionedSigner</a> {
    s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>let</b> perm = <a href="vesting.md#0x1_vesting_VestPermission">VestPermission</a> {};
    <b>aborts_if</b> !<a href="permissioned_signer.md#0x1_permissioned_signer_spec_check_permission_exists">permissioned_signer::spec_check_permission_exists</a>(s, perm);
}
</code></pre>



<a id="@Specification_1_stake_pool_address"></a>

### Function `stake_pool_address`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_stake_pool_address">stake_pool_address</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_vesting_start_secs"></a>

### Function `vesting_start_secs`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_period_duration_secs"></a>

### Function `period_duration_secs`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_remaining_grant"></a>

### Function `remaining_grant`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_beneficiary"></a>

### Function `beneficiary`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_operator_commission_percentage"></a>

### Function `operator_commission_percentage`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_vesting_contracts"></a>

### Function `vesting_contracts`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>




<pre><code>// This enforces <a id="high-level-spec-3.1" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_operator"></a>

### Function `operator`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator">operator</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_voter"></a>

### Function `voter`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_voter">voter</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_vesting_schedule"></a>

### Function `vesting_schedule`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_schedule">vesting_schedule</a>(vesting_contract_address: <b>address</b>): <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
</code></pre>



<a id="@Specification_1_total_accumulated_rewards"></a>

### Function `total_accumulated_rewards`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_TotalAccumulatedRewardsAbortsIf">TotalAccumulatedRewardsAbortsIf</a>;
</code></pre>




<a id="0x1_vesting_TotalAccumulatedRewardsAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_TotalAccumulatedRewardsAbortsIf">TotalAccumulatedRewardsAbortsIf</a> {
    vesting_contract_address: <b>address</b>;
    <b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>{contract_address: vesting_contract_address};
    <b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
    <b>let</b> staker = vesting_contract_address;
    <b>let</b> operator = vesting_contract.staking.operator;
    <b>let</b> staking_contracts = <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(staker).staking_contracts;
    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(staker);
    <b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(staking_contracts, operator);
    <b>let</b> pool_address = <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;
    <b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);
    <b>let</b> active = <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);
    <b>let</b> pending_active = <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);
    <b>let</b> total_active_stake = active + pending_active;
    <b>let</b> accumulated_rewards = total_active_stake - <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;
    <b>let</b> commission_amount = accumulated_rewards * <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);
    <b>aborts_if</b> active + pending_active &gt; MAX_U64;
    <b>aborts_if</b> total_active_stake &lt; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;
    <b>aborts_if</b> accumulated_rewards * <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &gt; MAX_U64;
    <b>aborts_if</b> (vesting_contract.remaining_grant + commission_amount) &gt; total_active_stake;
    <b>aborts_if</b> total_active_stake &lt; vesting_contract.remaining_grant;
}
</code></pre>



<a id="@Specification_1_accumulated_rewards"></a>

### Function `accumulated_rewards`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_accumulated_rewards">accumulated_rewards</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): u64
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_TotalAccumulatedRewardsAbortsIf">TotalAccumulatedRewardsAbortsIf</a>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
<b>let</b> operator = vesting_contract.staking.operator;
<b>let</b> staking_contracts = <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(vesting_contract_address).staking_contracts;
<b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);
<b>let</b> pool_address = <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;
<b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);
<b>let</b> active = <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);
<b>let</b> pending_active = <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);
<b>let</b> total_active_stake = active + pending_active;
<b>let</b> accumulated_rewards = total_active_stake - <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;
<b>let</b> commission_amount = accumulated_rewards * <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;
<b>let</b> total_accumulated_rewards = total_active_stake - vesting_contract.remaining_grant - commission_amount;
<b>let</b> shareholder = <a href="vesting.md#0x1_vesting_spec_shareholder">spec_shareholder</a>(vesting_contract_address, shareholder_or_beneficiary);
<b>let</b> pool = vesting_contract.grant_pool;
<b>let</b> shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_spec_shares">pool_u64::spec_shares</a>(pool, shareholder);
<b>aborts_if</b> pool.total_coins &gt; 0 && pool.total_shares &gt; 0
    && (shares * total_accumulated_rewards) / pool.total_shares &gt; MAX_U64;
<b>ensures</b> result == <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_spec_shares_to_amount_with_total_coins">pool_u64::spec_shares_to_amount_with_total_coins</a>(pool, shares, total_accumulated_rewards);
</code></pre>



<a id="@Specification_1_shareholders"></a>

### Function `shareholders`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholders">shareholders</a>(vesting_contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>




<pre><code><b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>{contract_address: vesting_contract_address};
</code></pre>




<a id="0x1_vesting_spec_shareholder"></a>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_spec_shareholder">spec_shareholder</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): <b>address</b>;
</code></pre>



<a id="@Specification_1_shareholder"></a>

### Function `shareholder`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholder">shareholder</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>{contract_address: vesting_contract_address};
<b>ensures</b> [abstract] result == <a href="vesting.md#0x1_vesting_spec_shareholder">spec_shareholder</a>(vesting_contract_address, shareholder_or_beneficiary);
</code></pre>



<a id="@Specification_1_create_vesting_schedule"></a>

### Function `create_vesting_schedule`


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_schedule">create_vesting_schedule</a>(schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>&gt;, start_timestamp_secs: u64, period_duration: u64): <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a>
</code></pre>




<pre><code>// This enforces <a id="high-level-req-6" href="#high-level-req">high-level requirement 6</a>:
<b>aborts_if</b> !(len(schedule) &gt; 0);
<b>aborts_if</b> !(period_duration &gt; 0);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);
<b>aborts_if</b> !(start_timestamp_secs &gt;= <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>());
</code></pre>



<a id="@Specification_1_create_vesting_contract"></a>

### Function `create_vesting_contract`


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract">create_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, shareholders: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, buy_ins: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;&gt;, vesting_schedule: <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a>, withdrawal_address: <b>address</b>, operator: <b>address</b>, voter: <b>address</b>, commission_percentage: u64, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
// This enforces <a id="high-level-req-10" href="#high-level-req">high-level requirement 10</a>:
<b>aborts_if</b> withdrawal_address == @aptos_framework || withdrawal_address == @vm_reserved;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(withdrawal_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(withdrawal_address);
<b>aborts_if</b> len(shareholders) == 0;
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_len">simple_map::spec_len</a>(buy_ins) != len(shareholders);
<b>ensures</b> <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(result).grant_pool.shareholders_limit == 30;
</code></pre>



<a id="@Specification_1_unlock_rewards"></a>

### Function `unlock_rewards`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_UnlockRewardsAbortsIf">UnlockRewardsAbortsIf</a>;
</code></pre>




<a id="0x1_vesting_UnlockRewardsAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_UnlockRewardsAbortsIf">UnlockRewardsAbortsIf</a> {
    contract_address: <b>address</b>;
    <b>include</b> <a href="vesting.md#0x1_vesting_TotalAccumulatedRewardsAbortsIf">TotalAccumulatedRewardsAbortsIf</a> { vesting_contract_address: contract_address };
    <b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> operator = vesting_contract.staking.operator;
    <b>let</b> staking_contracts = <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(contract_address).staking_contracts;
    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);
    <b>let</b> pool_address = <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;
    <b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);
    <b>let</b> active = <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);
    <b>let</b> pending_active = <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);
    <b>let</b> total_active_stake = active + pending_active;
    <b>let</b> accumulated_rewards = total_active_stake - <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;
    <b>let</b> commission_amount = accumulated_rewards * <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;
    <b>let</b> amount = total_active_stake - vesting_contract.remaining_grant - commission_amount;
    <b>include</b> <a href="vesting.md#0x1_vesting_UnlockStakeAbortsIf">UnlockStakeAbortsIf</a> { vesting_contract, amount };
}
</code></pre>



<a id="@Specification_1_unlock_rewards_many"></a>

### Function `unlock_rewards_many`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards_many">unlock_rewards_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>aborts_if</b> len(contract_addresses) == 0;
</code></pre>



<a id="@Specification_1_vest"></a>

### Function `vest`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest">vest</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_UnlockRewardsAbortsIf">UnlockRewardsAbortsIf</a>;
</code></pre>



<a id="@Specification_1_vest_many"></a>

### Function `vest_many`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest_many">vest_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>aborts_if</b> len(contract_addresses) == 0;
</code></pre>



<a id="@Specification_1_distribute"></a>

### Function `distribute`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>include</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a> { vesting_contract };
</code></pre>



<a id="@Specification_1_distribute_many"></a>

### Function `distribute_many`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute_many">distribute_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>aborts_if</b> len(contract_addresses) == 0;
</code></pre>



<a id="@Specification_1_terminate_vesting_contract"></a>

### Function `terminate_vesting_contract`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>include</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a> { vesting_contract };
</code></pre>



<a id="@Specification_1_admin_withdraw"></a>

### Function `admin_withdraw`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_admin_withdraw">admin_withdraw</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>aborts_if</b> vesting_contract.state != <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>;
<b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;
<b>include</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a> { vesting_contract };
</code></pre>



<a id="@Specification_1_update_operator"></a>

### Function `update_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator">update_operator</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_operator: <b>address</b>, commission_percentage: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> acc = vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;
<b>let</b> old_operator = vesting_contract.staking.operator;
<b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">staking_contract::ContractExistsAbortsIf</a> { staker: acc, operator: old_operator };
<b>let</b> store = <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(acc);
<b>let</b> staking_contracts = store.staking_contracts;
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(staking_contracts, new_operator);
<b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, old_operator);
<b>include</b> <a href="vesting.md#0x1_vesting_DistributeInternalAbortsIf">DistributeInternalAbortsIf</a> { staker: acc, operator: old_operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, distribute_events: store.distribute_events };
</code></pre>



<a id="@Specification_1_update_operator_with_same_commission"></a>

### Function `update_operator_with_same_commission`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator_with_same_commission">update_operator_with_same_commission</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_operator: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_update_commission_percentage"></a>

### Function `update_commission_percentage`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_commission_percentage">update_commission_percentage</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_commission_percentage: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_update_voter"></a>

### Function `update_voter`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_voter">update_voter</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_voter: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> operator = vesting_contract.staking.operator;
<b>let</b> staker = vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;
<b>include</b> <a href="staking_contract.md#0x1_staking_contract_UpdateVoterSchema">staking_contract::UpdateVoterSchema</a>;
</code></pre>



<a id="@Specification_1_reset_lockup"></a>

### Function `reset_lockup`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_lockup">reset_lockup</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) != vesting_contract.admin;
<b>let</b> operator = vesting_contract.staking.operator;
<b>let</b> staker = vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;
<b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">staking_contract::ContractExistsAbortsIf</a> {staker, operator};
<b>include</b> <a href="staking_contract.md#0x1_staking_contract_IncreaseLockupWithCapAbortsIf">staking_contract::IncreaseLockupWithCapAbortsIf</a> {staker, operator};
<b>let</b> store = <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(staker);
<b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);
<b>let</b> pool_address = <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(vesting_contract.staking.pool_address);
</code></pre>



<a id="@Specification_1_set_beneficiary"></a>

### Function `set_beneficiary`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary">set_beneficiary</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>, new_beneficiary: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>pragma</b> aborts_if_is_partial;
<b>aborts_if</b> !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(new_beneficiary);
<b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;
<b>let</b> <b>post</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(vesting_contract.beneficiaries,shareholder);
</code></pre>



<a id="@Specification_1_reset_beneficiary"></a>

### Function `reset_beneficiary`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_beneficiary">reset_beneficiary</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>aborts_if</b> addr != vesting_contract.admin && !std::string::spec_internal_check_utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>);
<b>aborts_if</b> addr != vesting_contract.admin && !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address);
<b>let</b> roles = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;
<b>let</b> role = std::string::spec_utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>);
<b>aborts_if</b> addr != vesting_contract.admin && !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(roles, role);
<b>aborts_if</b> addr != vesting_contract.admin && addr != <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(roles, role);
<b>let</b> <b>post</b> post_vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
<b>ensures</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_vesting_contract.beneficiaries,shareholder);
</code></pre>



<a id="@Specification_1_set_management_role"></a>

### Function `set_management_role`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_management_role">set_management_role</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, role_holder: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="vesting.md#0x1_vesting_SetManagementRoleAbortsIf">SetManagementRoleAbortsIf</a>;
</code></pre>



<a id="@Specification_1_set_beneficiary_resetter"></a>

### Function `set_beneficiary_resetter`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_resetter">set_beneficiary_resetter</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, beneficiary_resetter: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>);
<b>include</b> <a href="vesting.md#0x1_vesting_SetManagementRoleAbortsIf">SetManagementRoleAbortsIf</a>;
</code></pre>



<a id="@Specification_1_set_beneficiary_for_operator"></a>

### Function `set_beneficiary_for_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(operator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_beneficiary: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_get_role_holder"></a>

### Function `get_role_holder`


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_role_holder">get_role_holder</a>(contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address);
<b>let</b> roles = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(roles,role);
</code></pre>



<a id="@Specification_1_get_vesting_account_signer"></a>

### Function `get_vesting_account_signer`


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;
</code></pre>



<a id="@Specification_1_get_vesting_account_signer_internal"></a>

### Function `get_vesting_account_signer_internal`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>




<a id="0x1_vesting_spec_get_vesting_account_signer"></a>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_spec_get_vesting_account_signer">spec_get_vesting_account_signer</a>(vesting_contract: <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



<a id="@Specification_1_create_vesting_contract_account"></a>

### Function `create_vesting_contract_account`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>let</b> admin_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin);
<b>let</b> admin_store = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin_addr);
<b>let</b> seed = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(admin_addr);
<b>let</b> nonce = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(admin_store.nonce);
<b>let</b> first = concat(seed, nonce);
<b>let</b> second = concat(first, <a href="vesting.md#0x1_vesting_VESTING_POOL_SALT">VESTING_POOL_SALT</a>);
<b>let</b> end = concat(second, contract_creation_seed);
// This enforces <a id="high-level-req-11" href="#high-level-req">high-level requirement 11</a>:
<b>let</b> resource_addr = <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(admin_addr, end);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin_addr);
<b>aborts_if</b> len(<a href="account.md#0x1_account_ZERO_AUTH_KEY">account::ZERO_AUTH_KEY</a>) != 32;
<b>aborts_if</b> admin_store.nonce + 1 &gt; MAX_U64;
<b>let</b> ea = <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(resource_addr);
<b>include</b> <b>if</b> (ea) <a href="account.md#0x1_account_CreateResourceAccountAbortsIf">account::CreateResourceAccountAbortsIf</a> <b>else</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">account::CreateAccountAbortsIf</a> {addr: resource_addr};
<b>let</b> acc = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);
<b>let</b> <b>post</b> post_acc = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr) && !aptos_std::type_info::spec_is_struct&lt;AptosCoin&gt;();
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr) && ea && acc.guid_creation_num + 2 &gt; MAX_U64;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr) && ea && acc.guid_creation_num + 2 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr) && post_acc.authentication_key == <a href="account.md#0x1_account_ZERO_AUTH_KEY">account::ZERO_AUTH_KEY</a> &&
        <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr);
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result_1) == resource_addr;
<b>ensures</b> result_2.<a href="account.md#0x1_account">account</a> == resource_addr;
</code></pre>



<a id="@Specification_1_verify_admin"></a>

### Function `verify_admin`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>aborts_if</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">permissioned_signer::spec_is_permissioned_signer</a>(admin);
// This enforces <a id="high-level-req-9" href="#high-level-req">high-level requirement 9</a>:
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) != vesting_contract.admin;
</code></pre>



<a id="@Specification_1_assert_vesting_contract_exists"></a>

### Function `assert_vesting_contract_exists`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
</code></pre>



<a id="@Specification_1_assert_active_vesting_contract"></a>

### Function `assert_active_vesting_contract`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code><b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>;
</code></pre>



<a id="@Specification_1_unlock_stake"></a>

### Function `unlock_stake`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_UnlockStakeAbortsIf">UnlockStakeAbortsIf</a>;
</code></pre>




<a id="0x1_vesting_UnlockStakeAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_UnlockStakeAbortsIf">UnlockStakeAbortsIf</a> {
    vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>;
    amount: u64;
    <b>let</b> acc = vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;
    <b>let</b> operator = vesting_contract.staking.operator;
    <b>include</b> amount != 0 ==&gt; <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">staking_contract::ContractExistsAbortsIf</a> { staker: acc, operator };
    <b>let</b> store = <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(acc);
    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);
    <b>include</b> amount != 0 ==&gt; <a href="vesting.md#0x1_vesting_DistributeInternalAbortsIf">DistributeInternalAbortsIf</a> { staker: acc, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, distribute_events: store.distribute_events };
}
</code></pre>



<a id="@Specification_1_withdraw_stake"></a>

### Function `withdraw_stake`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, contract_address: <b>address</b>): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a>;
</code></pre>




<a id="0x1_vesting_WithdrawStakeAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a> {
    vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>;
    contract_address: <b>address</b>;
    <b>let</b> operator = vesting_contract.staking.operator;
    <b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">staking_contract::ContractExistsAbortsIf</a> { staker: contract_address, operator };
    <b>let</b> store = <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(contract_address);
    <b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);
    <b>include</b> <a href="vesting.md#0x1_vesting_DistributeInternalAbortsIf">DistributeInternalAbortsIf</a> { staker: contract_address, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, distribute_events: store.distribute_events };
}
</code></pre>




<a id="0x1_vesting_DistributeInternalAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_DistributeInternalAbortsIf">DistributeInternalAbortsIf</a> {
    staker: <b>address</b>;
    operator: <b>address</b>;
    <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>;
    distribute_events: EventHandle&lt;<a href="staking_contract.md#0x1_staking_contract_DistributeEvent">staking_contract::DistributeEvent</a>&gt;;
    <b>let</b> pool_address = <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);
    <b>let</b> stake_pool = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);
    <b>let</b> inactive = stake_pool.inactive.value;
    <b>let</b> pending_inactive = stake_pool.pending_inactive.value;
    <b>aborts_if</b> inactive + pending_inactive &gt; MAX_U64;
    <b>let</b> total_potential_withdrawable = inactive + pending_inactive;
    <b>let</b> pool_address_1 = <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address_1);
    <b>let</b> stake_pool_1 = <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address_1);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@aptos_framework);
    <b>let</b> validator_set = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@aptos_framework);
    <b>let</b> inactive_state = !<a href="stake.md#0x1_stake_spec_contains">stake::spec_contains</a>(validator_set.pending_active, pool_address_1)
        && !<a href="stake.md#0x1_stake_spec_contains">stake::spec_contains</a>(validator_set.active_validators, pool_address_1)
        && !<a href="stake.md#0x1_stake_spec_contains">stake::spec_contains</a>(validator_set.pending_inactive, pool_address_1);
    <b>let</b> inactive_1 = stake_pool_1.inactive.value;
    <b>let</b> pending_inactive_1 = stake_pool_1.pending_inactive.value;
    <b>let</b> new_inactive_1 = inactive_1 + pending_inactive_1;
    <b>aborts_if</b> inactive_state && <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt;= stake_pool_1.locked_until_secs
        && inactive_1 + pending_inactive_1 &gt; MAX_U64;
}
</code></pre>



<a id="@Specification_1_get_beneficiary"></a>

### Function `get_beneficiary`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, shareholder: <b>address</b>): <b>address</b>
</code></pre>




<pre><code>// This enforces <a id="high-level-spec-3.2" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>




<a id="0x1_vesting_SetManagementRoleAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_SetManagementRoleAbortsIf">SetManagementRoleAbortsIf</a> {
    contract_address: <b>address</b>;
    admin: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) != vesting_contract.admin;
}
</code></pre>




<a id="0x1_vesting_VerifyAdminAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a> {
    contract_address: <b>address</b>;
    admin: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>aborts_if</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">permissioned_signer::spec_is_permissioned_signer</a>(admin);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) != vesting_contract.admin;
}
</code></pre>




<a id="0x1_vesting_ActiveVestingContractAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a> {
    contract_address: <b>address</b>;
    // This enforces <a id="high-level-spec-5" href="#high-level-req">high-level requirement 5</a>:
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    // This enforces <a id="high-level-spec-8" href="#high-level-req">high-level requirement 8</a>:
    <b>aborts_if</b> vesting_contract.state != <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>;
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
