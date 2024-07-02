
<a id="0x1_vesting"></a>

# Module `0x1::vesting`


Simple vesting contract that allows specifying how much APT coins should be vesting in each fixed&#45;size period. The
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
3. After the unlocked rewards become fully withdrawable (as it&apos;s subject to staking lockup), shareholders can call
distribute() to send all withdrawable funds to all shareholders based on the original grant&apos;s shares structure.
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
withdrawable, admin can call admin_withdraw to withdraw all funds to the vesting contract&apos;s withdrawal address.


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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64">0x1::pool_u64</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="staking_contract.md#0x1_staking_contract">0x1::staking_contract</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_vesting_VestingSchedule"></a>

## Struct `VestingSchedule`



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_StakingInfo">StakingInfo</a> <b>has</b> store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> <b>has</b> key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a> <b>has</b> key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> <b>has</b> key<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_CreateVestingContract">CreateVestingContract</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_UpdateOperator">UpdateOperator</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_UpdateVoter">UpdateVoter</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_ResetLockup">ResetLockup</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_SetBeneficiary">SetBeneficiary</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_UnlockRewards">UnlockRewards</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_Vest">Vest</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_Distribute">Distribute</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_Terminate">Terminate</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="vesting.md#0x1_vesting_AdminWithdraw">AdminWithdraw</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_CreateVestingContractEvent">CreateVestingContractEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_UpdateOperatorEvent">UpdateOperatorEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_UpdateVoterEvent">UpdateVoterEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_ResetLockupEvent">ResetLockupEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_SetBeneficiaryEvent">SetBeneficiaryEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_UnlockRewardsEvent">UnlockRewardsEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_VestEvent">VestEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_DistributeEvent">DistributeEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_TerminateEvent">TerminateEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="vesting.md#0x1_vesting_AdminWithdrawEvent">AdminWithdrawEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_vesting_EEMPTY_VESTING_SCHEDULE"></a>

Vesting schedule cannot be empty.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS"></a>

Withdrawal address is invalid.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS">EINVALID_WITHDRAWAL_ADDRESS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_vesting_ENOT_ADMIN"></a>

The signer is not the admin of the vesting contract.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ENOT_ADMIN">ENOT_ADMIN</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_vesting_ENO_SHAREHOLDERS"></a>

Shareholders list cannot be empty.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_vesting_EPENDING_STAKE_FOUND"></a>

Cannot terminate the vesting contract with pending active stake. Need to wait until next epoch.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EPENDING_STAKE_FOUND">EPENDING_STAKE_FOUND</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x1_vesting_EPERMISSION_DENIED"></a>

Account is not admin or does not have the required role to take this action.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EPERMISSION_DENIED">EPERMISSION_DENIED</a>: u64 &#61; 15;<br /></code></pre>



<a id="0x1_vesting_EROLE_NOT_FOUND"></a>

The vesting account has no such management role.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EROLE_NOT_FOUND">EROLE_NOT_FOUND</a>: u64 &#61; 14;<br /></code></pre>



<a id="0x1_vesting_ESHARES_LENGTH_MISMATCH"></a>

The length of shareholders and shares lists don&apos;t match.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ESHARES_LENGTH_MISMATCH">ESHARES_LENGTH_MISMATCH</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION"></a>

Zero items were provided to a &#42;_many function.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>: u64 &#61; 16;<br /></code></pre>



<a id="0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES"></a>

Vesting account has no other management roles beside admin.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES">EVESTING_ACCOUNT_HAS_NO_ROLES</a>: u64 &#61; 13;<br /></code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE"></a>

Vesting contract needs to be in active state.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE">EVESTING_CONTRACT_NOT_ACTIVE</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_NOT_FOUND"></a>

No vesting contract found at provided address.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_FOUND">EVESTING_CONTRACT_NOT_FOUND</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE"></a>

Admin can only withdraw from an inactive (paused or terminated) vesting contract.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE">EVESTING_CONTRACT_STILL_ACTIVE</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_vesting_EVESTING_START_TOO_SOON"></a>

Vesting cannot start before or at the current block timestamp. Has to be in the future.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_START_TOO_SOON">EVESTING_START_TOO_SOON</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_vesting_EZERO_GRANT"></a>

Grant amount cannot be 0.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EZERO_GRANT">EZERO_GRANT</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD"></a>

Vesting period cannot be 0.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD">EZERO_VESTING_SCHEDULE_PERIOD</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_vesting_MAXIMUM_SHAREHOLDERS"></a>

Maximum number of shareholders a vesting pool can support.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_MAXIMUM_SHAREHOLDERS">MAXIMUM_SHAREHOLDERS</a>: u64 &#61; 30;<br /></code></pre>



<a id="0x1_vesting_ROLE_BENEFICIARY_RESETTER"></a>

Roles that can manage certain aspects of the vesting account beyond the main admin.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [82, 79, 76, 69, 95, 66, 69, 78, 69, 70, 73, 67, 73, 65, 82, 89, 95, 82, 69, 83, 69, 84, 84, 69, 82];<br /></code></pre>



<a id="0x1_vesting_VESTING_POOL_ACTIVE"></a>

Vesting contract states.
Vesting contract is active and distributions can be made.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_vesting_VESTING_POOL_SALT"></a>



<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_SALT">VESTING_POOL_SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 118, 101, 115, 116, 105, 110, 103];<br /></code></pre>



<a id="0x1_vesting_VESTING_POOL_TERMINATED"></a>

Vesting contract has been terminated and all funds have been released back to the withdrawal address.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_vesting_stake_pool_address"></a>

## Function `stake_pool_address`

Return the address of the underlying stake pool (separate resource account) of the vesting contract.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_stake_pool_address">stake_pool_address</a>(vesting_contract_address: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_stake_pool_address">stake_pool_address</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.pool_address<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_vesting_start_secs"></a>

## Function `vesting_start_secs`

Return the vesting start timestamp (in seconds) of the vesting contract.
Vesting will start at this time, and once a full period has passed, the first vest will become unlocked.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule.start_timestamp_secs<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_period_duration_secs"></a>

## Function `period_duration_secs`

Return the duration of one vesting period (in seconds).
Each vest is released after one full period has started, starting from the specified start_timestamp_secs.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule.period_duration<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_remaining_grant"></a>

## Function `remaining_grant`

Return the remaining grant, consisting of unvested coins that have not been distributed to shareholders.
Prior to start_timestamp_secs, the remaining grant will always be equal to the original grant.
Once vesting has started, and vested tokens are distributed, the remaining grant will decrease over time,
according to the vesting schedule.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).remaining_grant<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_beneficiary"></a>

## Function `beneficiary`

Return the beneficiary account of the specified shareholder in a vesting contract.
This is the same as the shareholder address by default and only different if it&apos;s been explicitly set.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(<b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address), shareholder)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_operator_commission_percentage"></a>

## Function `operator_commission_percentage`

Return the percentage of accumulated rewards that is paid to the operator as commission.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.commission_percentage<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_vesting_contracts"></a>

## Function `vesting_contracts`

Return all the vesting contracts a given address is an admin of.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> &#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin)) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;()<br />    &#125; <b>else</b> &#123;<br />        <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin).vesting_contracts<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_operator"></a>

## Function `operator`

Return the operator who runs the validator for the vesting contract.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator">operator</a>(vesting_contract_address: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator">operator</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.operator<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_voter"></a>

## Function `voter`

Return the voter who will be voting on on&#45;chain governance proposals on behalf of the vesting contract&apos;s stake
pool.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_voter">voter</a>(vesting_contract_address: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_voter">voter</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.voter<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_vesting_schedule"></a>

## Function `vesting_schedule`

Return the vesting contract&apos;s vesting schedule. The core schedule is represented as a list of u64&#45;based
fractions, where the rightmmost 32 bits can be divided by 2^32 to get the fraction, and anything else is the
whole number.

For example 3/48, or 0.0625, will be represented as 268435456. The fractional portion would be
268435456 / 2^32 &#61; 0.0625. Since there are fewer than 32 bits, the whole number portion is effectively 0.
So 268435456 &#61; 0.0625.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_schedule">vesting_schedule</a>(vesting_contract_address: <b>address</b>): <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_schedule">vesting_schedule</a>(vesting_contract_address: <b>address</b>): <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);<br />    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_total_accumulated_rewards"></a>

## Function `total_accumulated_rewards`

Return the total accumulated rewards that have not been distributed to shareholders of the vesting contract.
This excludes any unpaid commission that the operator has not collected.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);<br /><br />    <b>let</b> vesting_contract &#61; <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br />    <b>let</b> (total_active_stake, _, commission_amount) &#61;<br />        <a href="staking_contract.md#0x1_staking_contract_staking_contract_amounts">staking_contract::staking_contract_amounts</a>(vesting_contract_address, vesting_contract.staking.operator);<br />    total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_accumulated_rewards"></a>

## Function `accumulated_rewards`

Return the accumulated rewards that have not been distributed to the provided shareholder. Caller can also pass
the beneficiary address instead of shareholder address.

This errors out if the vesting contract with the provided address doesn&apos;t exist.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_accumulated_rewards">accumulated_rewards</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_accumulated_rewards">accumulated_rewards</a>(<br />    vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);<br /><br />    <b>let</b> total_accumulated_rewards &#61; <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(vesting_contract_address);<br />    <b>let</b> shareholder &#61; <a href="vesting.md#0x1_vesting_shareholder">shareholder</a>(vesting_contract_address, shareholder_or_beneficiary);<br />    <b>let</b> vesting_contract &#61; <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br />    <b>let</b> shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(&amp;vesting_contract.grant_pool, shareholder);<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">pool_u64::shares_to_amount_with_total_coins</a>(&amp;vesting_contract.grant_pool, shares, total_accumulated_rewards)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_shareholders"></a>

## Function `shareholders`

Return the list of all shareholders in the vesting contract.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholders">shareholders</a>(vesting_contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholders">shareholders</a>(vesting_contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);<br /><br />    <b>let</b> vesting_contract &#61; <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br />    <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders">pool_u64::shareholders</a>(&amp;vesting_contract.grant_pool)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_shareholder"></a>

## Function `shareholder`

Return the shareholder address given the beneficiary address in a given vesting contract. If there are multiple
shareholders with the same beneficiary address, only the first shareholder is returned. If the given beneficiary
address is actually a shareholder address, just return the address back.

This returns 0x0 if no shareholder is found for the given beneficiary / the address is not a shareholder itself.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholder">shareholder</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholder">shareholder</a>(<br />    vesting_contract_address: <b>address</b>,<br />    shareholder_or_beneficiary: <b>address</b><br />): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);<br /><br />    <b>let</b> shareholders &#61; &amp;<a href="vesting.md#0x1_vesting_shareholders">shareholders</a>(vesting_contract_address);<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(shareholders, &amp;shareholder_or_beneficiary)) &#123;<br />        <b>return</b> shareholder_or_beneficiary<br />    &#125;;<br />    <b>let</b> vesting_contract &#61; <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br />    <b>let</b> result &#61; @0x0;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_any">vector::any</a>(shareholders, &#124;shareholder&#124; &#123;<br />        <b>if</b> (shareholder_or_beneficiary &#61;&#61; <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(vesting_contract, &#42;shareholder)) &#123;<br />            result &#61; &#42;shareholder;<br />            <b>true</b><br />        &#125; <b>else</b> &#123;<br />            <b>false</b><br />        &#125;<br />    &#125;);<br /><br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_create_vesting_schedule"></a>

## Function `create_vesting_schedule`

Create a vesting schedule with the given schedule of distributions, a vesting start time and period duration.


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_schedule">create_vesting_schedule</a>(schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>&gt;, start_timestamp_secs: u64, period_duration: u64): <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_schedule">create_vesting_schedule</a>(<br />    schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;FixedPoint32&gt;,<br />    start_timestamp_secs: u64,<br />    period_duration: u64,<br />): <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;schedule) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>));<br />    <b>assert</b>!(period_duration &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD">EZERO_VESTING_SCHEDULE_PERIOD</a>));<br />    <b>assert</b>!(<br />        start_timestamp_secs &gt;&#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EVESTING_START_TOO_SOON">EVESTING_START_TOO_SOON</a>),<br />    );<br /><br />    <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a> &#123;<br />        schedule,<br />        start_timestamp_secs,<br />        period_duration,<br />        last_vested_period: 0,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_create_vesting_contract"></a>

## Function `create_vesting_contract`

Create a vesting contract with a given configurations.


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract">create_vesting_contract</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, shareholders: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, buy_ins: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;&gt;, vesting_schedule: <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a>, withdrawal_address: <b>address</b>, operator: <b>address</b>, voter: <b>address</b>, commission_percentage: u64, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract">create_vesting_contract</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    shareholders: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,<br />    buy_ins: SimpleMap&lt;<b>address</b>, Coin&lt;AptosCoin&gt;&gt;,<br />    vesting_schedule: <a href="vesting.md#0x1_vesting_VestingSchedule">VestingSchedule</a>,<br />    withdrawal_address: <b>address</b>,<br />    operator: <b>address</b>,<br />    voter: <b>address</b>,<br />    commission_percentage: u64,<br />    // Optional seed used when creating the staking contract <a href="account.md#0x1_account">account</a>.<br />    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> &#123;<br />    <b>assert</b>!(<br />        !<a href="system_addresses.md#0x1_system_addresses_is_reserved_address">system_addresses::is_reserved_address</a>(withdrawal_address),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS">EINVALID_WITHDRAWAL_ADDRESS</a>),<br />    );<br />    assert_account_is_registered_for_apt(withdrawal_address);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shareholders) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>));<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_length">simple_map::length</a>(&amp;buy_ins) &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shareholders),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_ESHARES_LENGTH_MISMATCH">ESHARES_LENGTH_MISMATCH</a>),<br />    );<br /><br />    // Create a coins pool <b>to</b> track shareholders and shares of the grant.<br />    <b>let</b> grant &#61; <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;AptosCoin&gt;();<br />    <b>let</b> grant_amount &#61; 0;<br />    <b>let</b> grant_pool &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_create">pool_u64::create</a>(<a href="vesting.md#0x1_vesting_MAXIMUM_SHAREHOLDERS">MAXIMUM_SHAREHOLDERS</a>);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(shareholders, &#124;shareholder&#124; &#123;<br />        <b>let</b> shareholder: <b>address</b> &#61; &#42;shareholder;<br />        <b>let</b> (_, buy_in) &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&amp;<b>mut</b> buy_ins, &amp;shareholder);<br />        <b>let</b> buy_in_amount &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;buy_in);<br />        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&amp;<b>mut</b> grant, buy_in);<br />        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(<br />            &amp;<b>mut</b> grant_pool,<br />            shareholder,<br />            buy_in_amount,<br />        );<br />        grant_amount &#61; grant_amount &#43; buy_in_amount;<br />    &#125;);<br />    <b>assert</b>!(grant_amount &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EZERO_GRANT">EZERO_GRANT</a>));<br /><br />    // If this is the first time this admin <a href="account.md#0x1_account">account</a> <b>has</b> created a <a href="vesting.md#0x1_vesting">vesting</a> contract, initialize the admin store.<br />    <b>let</b> admin_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin_address)) &#123;<br />        <b>move_to</b>(admin, <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> &#123;<br />            vesting_contracts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;(),<br />            nonce: 0,<br />            create_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_CreateVestingContractEvent">CreateVestingContractEvent</a>&gt;(admin),<br />        &#125;);<br />    &#125;;<br /><br />    // Initialize the <a href="vesting.md#0x1_vesting">vesting</a> contract in a new resource <a href="account.md#0x1_account">account</a>. This allows the same admin <b>to</b> create multiple<br />    // pools.<br />    <b>let</b> (contract_signer, contract_signer_cap) &#61; <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(admin, contract_creation_seed);<br />    <b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract_create_staking_contract_with_coins">staking_contract::create_staking_contract_with_coins</a>(<br />        &amp;contract_signer, operator, voter, grant, commission_percentage, contract_creation_seed);<br /><br />    // Add the newly created <a href="vesting.md#0x1_vesting">vesting</a> contract&apos;s <b>address</b> <b>to</b> the admin store.<br />    <b>let</b> contract_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;contract_signer);<br />    <b>let</b> admin_store &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin_address);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> admin_store.vesting_contracts, contract_address);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_CreateVestingContract">CreateVestingContract</a> &#123;<br />                operator,<br />                voter,<br />                withdrawal_address,<br />                grant_amount,<br />                vesting_contract_address: contract_address,<br />                staking_pool_address: pool_address,<br />                commission_percentage,<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> admin_store.create_events,<br />        <a href="vesting.md#0x1_vesting_CreateVestingContractEvent">CreateVestingContractEvent</a> &#123;<br />            operator,<br />            voter,<br />            withdrawal_address,<br />            grant_amount,<br />            vesting_contract_address: contract_address,<br />            staking_pool_address: pool_address,<br />            commission_percentage,<br />        &#125;,<br />    );<br /><br />    <b>move_to</b>(&amp;contract_signer, <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />        state: <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>,<br />        admin: admin_address,<br />        grant_pool,<br />        beneficiaries: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, <b>address</b>&gt;(),<br />        vesting_schedule,<br />        withdrawal_address,<br />        staking: <a href="vesting.md#0x1_vesting_StakingInfo">StakingInfo</a> &#123; pool_address, operator, voter, commission_percentage &#125;,<br />        remaining_grant: grant_amount,<br />        signer_cap: contract_signer_cap,<br />        update_operator_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_UpdateOperatorEvent">UpdateOperatorEvent</a>&gt;(&amp;contract_signer),<br />        update_voter_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_UpdateVoterEvent">UpdateVoterEvent</a>&gt;(&amp;contract_signer),<br />        reset_lockup_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_ResetLockupEvent">ResetLockupEvent</a>&gt;(&amp;contract_signer),<br />        set_beneficiary_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_SetBeneficiaryEvent">SetBeneficiaryEvent</a>&gt;(&amp;contract_signer),<br />        unlock_rewards_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_UnlockRewardsEvent">UnlockRewardsEvent</a>&gt;(&amp;contract_signer),<br />        vest_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_VestEvent">VestEvent</a>&gt;(&amp;contract_signer),<br />        distribute_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_DistributeEvent">DistributeEvent</a>&gt;(&amp;contract_signer),<br />        terminate_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_TerminateEvent">TerminateEvent</a>&gt;(&amp;contract_signer),<br />        admin_withdraw_events: new_event_handle&lt;<a href="vesting.md#0x1_vesting_AdminWithdrawEvent">AdminWithdrawEvent</a>&gt;(&amp;contract_signer),<br />    &#125;);<br /><br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_destroy_empty">simple_map::destroy_empty</a>(buy_ins);<br />    contract_address<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_unlock_rewards"></a>

## Function `unlock_rewards`

Unlock any accumulated rewards.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> accumulated_rewards &#61; <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(contract_address);<br />    <b>let</b> vesting_contract &#61; <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract, accumulated_rewards);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_unlock_rewards_many"></a>

## Function `unlock_rewards_many`

Call <code>unlock_rewards</code> for many vesting contracts.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards_many">unlock_rewards_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards_many">unlock_rewards_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;contract_addresses);<br /><br />    <b>assert</b>!(len !&#61; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>));<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;contract_addresses, &#124;contract_address&#124; &#123;<br />        <b>let</b> contract_address: <b>address</b> &#61; &#42;contract_address;<br />        <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address);<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_vest"></a>

## Function `vest`

Unlock any vested portion of the grant.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest">vest</a>(contract_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest">vest</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    // Unlock all rewards first, <b>if</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a>.<br />    <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address);<br /><br />    // Unlock the vested amount. This amount will become withdrawable when the underlying <a href="stake.md#0x1_stake">stake</a> pool&apos;s lockup<br />    // expires.<br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    // Short&#45;circuit <b>if</b> <a href="vesting.md#0x1_vesting">vesting</a> hasn&apos;t started yet.<br />    <b>if</b> (vesting_contract.vesting_schedule.start_timestamp_secs &gt; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    // Check <b>if</b> the next vested period <b>has</b> already passed. If not, short&#45;circuit since there&apos;s nothing <b>to</b> vest.<br />    <b>let</b> vesting_schedule &#61; &amp;<b>mut</b> vesting_contract.vesting_schedule;<br />    <b>let</b> last_vested_period &#61; vesting_schedule.last_vested_period;<br />    <b>let</b> next_period_to_vest &#61; last_vested_period &#43; 1;<br />    <b>let</b> last_completed_period &#61;<br />        (<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &#45; vesting_schedule.start_timestamp_secs) / vesting_schedule.period_duration;<br />    <b>if</b> (last_completed_period &lt; next_period_to_vest) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    // Calculate how much <b>has</b> vested, excluding rewards.<br />    // Index is 0&#45;based <b>while</b> period is 1&#45;based so we need <b>to</b> subtract 1.<br />    <b>let</b> schedule &#61; &amp;vesting_schedule.schedule;<br />    <b>let</b> schedule_index &#61; next_period_to_vest &#45; 1;<br />    <b>let</b> vesting_fraction &#61; <b>if</b> (schedule_index &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(schedule)) &#123;<br />        &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(schedule, schedule_index)<br />    &#125; <b>else</b> &#123;<br />        // Last <a href="vesting.md#0x1_vesting">vesting</a> schedule fraction will repeat until the grant runs out.<br />        &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(schedule, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(schedule) &#45; 1)<br />    &#125;;<br />    <b>let</b> total_grant &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_total_coins">pool_u64::total_coins</a>(&amp;vesting_contract.grant_pool);<br />    <b>let</b> vested_amount &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_multiply_u64">fixed_point32::multiply_u64</a>(total_grant, vesting_fraction);<br />    // Cap vested amount by the remaining grant amount so we don&apos;t try <b>to</b> distribute more than what&apos;s remaining.<br />    vested_amount &#61; <b>min</b>(vested_amount, vesting_contract.remaining_grant);<br />    vesting_contract.remaining_grant &#61; vesting_contract.remaining_grant &#45; vested_amount;<br />    vesting_schedule.last_vested_period &#61; next_period_to_vest;<br />    <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract, vested_amount);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_Vest">Vest</a> &#123;<br />                admin: vesting_contract.admin,<br />                vesting_contract_address: contract_address,<br />                staking_pool_address: vesting_contract.staking.pool_address,<br />                period_vested: next_period_to_vest,<br />                amount: vested_amount,<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> vesting_contract.vest_events,<br />        <a href="vesting.md#0x1_vesting_VestEvent">VestEvent</a> &#123;<br />            admin: vesting_contract.admin,<br />            vesting_contract_address: contract_address,<br />            staking_pool_address: vesting_contract.staking.pool_address,<br />            period_vested: next_period_to_vest,<br />            amount: vested_amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_vest_many"></a>

## Function `vest_many`

Call <code>vest</code> for many vesting contracts.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest_many">vest_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest_many">vest_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;contract_addresses);<br /><br />    <b>assert</b>!(len !&#61; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>));<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;contract_addresses, &#124;contract_address&#124; &#123;<br />        <b>let</b> contract_address &#61; &#42;contract_address;<br />        <a href="vesting.md#0x1_vesting_vest">vest</a>(contract_address);<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_distribute"></a>

## Function `distribute`

Distribute any withdrawable stake from the stake pool.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);<br /><br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <b>let</b> coins &#61; <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract, contract_address);<br />    <b>let</b> total_distribution_amount &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;coins);<br />    <b>if</b> (total_distribution_amount &#61;&#61; 0) &#123;<br />        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);<br />        <b>return</b><br />    &#125;;<br /><br />    // <a href="vesting.md#0x1_vesting_Distribute">Distribute</a> coins <b>to</b> all shareholders in the <a href="vesting.md#0x1_vesting">vesting</a> contract.<br />    <b>let</b> grant_pool &#61; &amp;vesting_contract.grant_pool;<br />    <b>let</b> shareholders &#61; &amp;<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders">pool_u64::shareholders</a>(grant_pool);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(shareholders, &#124;shareholder&#124; &#123;<br />        <b>let</b> shareholder &#61; &#42;shareholder;<br />        <b>let</b> shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(grant_pool, shareholder);<br />        <b>let</b> amount &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">pool_u64::shares_to_amount_with_total_coins</a>(grant_pool, shares, total_distribution_amount);<br />        <b>let</b> share_of_coins &#61; <a href="coin.md#0x1_coin_extract">coin::extract</a>(&amp;<b>mut</b> coins, amount);<br />        <b>let</b> recipient_address &#61; <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(vesting_contract, shareholder);<br />        <a href="aptos_account.md#0x1_aptos_account_deposit_coins">aptos_account::deposit_coins</a>(recipient_address, share_of_coins);<br />    &#125;);<br /><br />    // Send <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> remaining &quot;dust&quot; (leftover due <b>to</b> rounding <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a>) <b>to</b> the withdrawal <b>address</b>.<br />    <b>if</b> (<a href="coin.md#0x1_coin_value">coin::value</a>(&amp;coins) &gt; 0) &#123;<br />        <a href="aptos_account.md#0x1_aptos_account_deposit_coins">aptos_account::deposit_coins</a>(vesting_contract.withdrawal_address, coins);<br />    &#125; <b>else</b> &#123;<br />        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);<br />    &#125;;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_Distribute">Distribute</a> &#123;<br />                admin: vesting_contract.admin,<br />                vesting_contract_address: contract_address,<br />                amount: total_distribution_amount,<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> vesting_contract.distribute_events,<br />        <a href="vesting.md#0x1_vesting_DistributeEvent">DistributeEvent</a> &#123;<br />            admin: vesting_contract.admin,<br />            vesting_contract_address: contract_address,<br />            amount: total_distribution_amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_distribute_many"></a>

## Function `distribute_many`

Call <code>distribute</code> for many vesting contracts.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute_many">distribute_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute_many">distribute_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;contract_addresses);<br /><br />    <b>assert</b>!(len !&#61; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting.md#0x1_vesting_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>));<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;contract_addresses, &#124;contract_address&#124; &#123;<br />        <b>let</b> contract_address &#61; &#42;contract_address;<br />        <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address);<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_terminate_vesting_contract"></a>

## Function `terminate_vesting_contract`

Terminate the vesting contract and send all funds back to the withdrawal address.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);<br /><br />    // <a href="vesting.md#0x1_vesting_Distribute">Distribute</a> all withdrawable coins, which should have been from previous rewards withdrawal or vest.<br />    <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address);<br /><br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br />    <b>let</b> (active_stake, _, pending_active_stake, _) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(vesting_contract.staking.pool_address);<br />    <b>assert</b>!(pending_active_stake &#61;&#61; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting.md#0x1_vesting_EPENDING_STAKE_FOUND">EPENDING_STAKE_FOUND</a>));<br /><br />    // Unlock all remaining active <a href="stake.md#0x1_stake">stake</a>.<br />    vesting_contract.state &#61; <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>;<br />    vesting_contract.remaining_grant &#61; 0;<br />    <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract, active_stake);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_Terminate">Terminate</a> &#123;<br />                admin: vesting_contract.admin,<br />                vesting_contract_address: contract_address,<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> vesting_contract.terminate_events,<br />        <a href="vesting.md#0x1_vesting_TerminateEvent">TerminateEvent</a> &#123;<br />            admin: vesting_contract.admin,<br />            vesting_contract_address: contract_address,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_admin_withdraw"></a>

## Function `admin_withdraw`

Withdraw all funds to the preset vesting contract&apos;s withdrawal address. This can only be called if the contract
has already been terminated.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_admin_withdraw">admin_withdraw</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_admin_withdraw">admin_withdraw</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> vesting_contract &#61; <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <b>assert</b>!(<br />        vesting_contract.state &#61;&#61; <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE">EVESTING_CONTRACT_STILL_ACTIVE</a>)<br />    );<br /><br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br />    <b>let</b> coins &#61; <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract, contract_address);<br />    <b>let</b> amount &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(&amp;coins);<br />    <b>if</b> (amount &#61;&#61; 0) &#123;<br />        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);<br />        <b>return</b><br />    &#125;;<br />    <a href="aptos_account.md#0x1_aptos_account_deposit_coins">aptos_account::deposit_coins</a>(vesting_contract.withdrawal_address, coins);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_AdminWithdraw">AdminWithdraw</a> &#123;<br />                admin: vesting_contract.admin,<br />                vesting_contract_address: contract_address,<br />                amount,<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> vesting_contract.admin_withdraw_events,<br />        <a href="vesting.md#0x1_vesting_AdminWithdrawEvent">AdminWithdrawEvent</a> &#123;<br />            admin: vesting_contract.admin,<br />            vesting_contract_address: contract_address,<br />            amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_update_operator"></a>

## Function `update_operator`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator">update_operator</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_operator: <b>address</b>, commission_percentage: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator">update_operator</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />    new_operator: <b>address</b>,<br />    commission_percentage: u64,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br />    <b>let</b> contract_signer &#61; &amp;<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);<br />    <b>let</b> old_operator &#61; vesting_contract.staking.operator;<br />    <a href="staking_contract.md#0x1_staking_contract_switch_operator">staking_contract::switch_operator</a>(contract_signer, old_operator, new_operator, commission_percentage);<br />    vesting_contract.staking.operator &#61; new_operator;<br />    vesting_contract.staking.commission_percentage &#61; commission_percentage;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_UpdateOperator">UpdateOperator</a> &#123;<br />                admin: vesting_contract.admin,<br />                vesting_contract_address: contract_address,<br />                staking_pool_address: vesting_contract.staking.pool_address,<br />                old_operator,<br />                new_operator,<br />                commission_percentage,<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> vesting_contract.update_operator_events,<br />        <a href="vesting.md#0x1_vesting_UpdateOperatorEvent">UpdateOperatorEvent</a> &#123;<br />            admin: vesting_contract.admin,<br />            vesting_contract_address: contract_address,<br />            staking_pool_address: vesting_contract.staking.pool_address,<br />            old_operator,<br />            new_operator,<br />            commission_percentage,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_update_operator_with_same_commission"></a>

## Function `update_operator_with_same_commission`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator_with_same_commission">update_operator_with_same_commission</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator_with_same_commission">update_operator_with_same_commission</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />    new_operator: <b>address</b>,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> commission_percentage &#61; <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(contract_address);<br />    <a href="vesting.md#0x1_vesting_update_operator">update_operator</a>(admin, contract_address, new_operator, commission_percentage);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_update_commission_percentage"></a>

## Function `update_commission_percentage`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_commission_percentage">update_commission_percentage</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_commission_percentage: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_commission_percentage">update_commission_percentage</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />    new_commission_percentage: u64,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> operator &#61; <a href="vesting.md#0x1_vesting_operator">operator</a>(contract_address);<br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br />    <b>let</b> contract_signer &#61; &amp;<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);<br />    <a href="staking_contract.md#0x1_staking_contract_update_commision">staking_contract::update_commision</a>(contract_signer, operator, new_commission_percentage);<br />    vesting_contract.staking.commission_percentage &#61; new_commission_percentage;<br />    // This function does not emit an <a href="event.md#0x1_event">event</a>. Instead, `staking_contract::update_commission_percentage`<br />    // <b>emits</b> the <a href="event.md#0x1_event">event</a> for this commission percentage <b>update</b>.<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_update_voter"></a>

## Function `update_voter`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_voter">update_voter</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_voter">update_voter</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />    new_voter: <b>address</b>,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br />    <b>let</b> contract_signer &#61; &amp;<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);<br />    <b>let</b> old_voter &#61; vesting_contract.staking.voter;<br />    <a href="staking_contract.md#0x1_staking_contract_update_voter">staking_contract::update_voter</a>(contract_signer, vesting_contract.staking.operator, new_voter);<br />    vesting_contract.staking.voter &#61; new_voter;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_UpdateVoter">UpdateVoter</a> &#123;<br />                admin: vesting_contract.admin,<br />                vesting_contract_address: contract_address,<br />                staking_pool_address: vesting_contract.staking.pool_address,<br />                old_voter,<br />                new_voter,<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> vesting_contract.update_voter_events,<br />        <a href="vesting.md#0x1_vesting_UpdateVoterEvent">UpdateVoterEvent</a> &#123;<br />            admin: vesting_contract.admin,<br />            vesting_contract_address: contract_address,<br />            staking_pool_address: vesting_contract.staking.pool_address,<br />            old_voter,<br />            new_voter,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_reset_lockup"></a>

## Function `reset_lockup`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_lockup">reset_lockup</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_lockup">reset_lockup</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br />    <b>let</b> contract_signer &#61; &amp;<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);<br />    <a href="staking_contract.md#0x1_staking_contract_reset_lockup">staking_contract::reset_lockup</a>(contract_signer, vesting_contract.staking.operator);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_ResetLockup">ResetLockup</a> &#123;<br />                admin: vesting_contract.admin,<br />                vesting_contract_address: contract_address,<br />                staking_pool_address: vesting_contract.staking.pool_address,<br />                new_lockup_expiration_secs: <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(vesting_contract.staking.pool_address),<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> vesting_contract.reset_lockup_events,<br />        <a href="vesting.md#0x1_vesting_ResetLockupEvent">ResetLockupEvent</a> &#123;<br />            admin: vesting_contract.admin,<br />            vesting_contract_address: contract_address,<br />            staking_pool_address: vesting_contract.staking.pool_address,<br />            new_lockup_expiration_secs: <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(vesting_contract.staking.pool_address),<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_set_beneficiary"></a>

## Function `set_beneficiary`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary">set_beneficiary</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>, new_beneficiary: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary">set_beneficiary</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />    shareholder: <b>address</b>,<br />    new_beneficiary: <b>address</b>,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    // Verify that the beneficiary <a href="account.md#0x1_account">account</a> is set up <b>to</b> receive APT. This is a requirement so <a href="vesting.md#0x1_vesting_distribute">distribute</a>() wouldn&apos;t<br />    // fail and <a href="block.md#0x1_block">block</a> all other accounts from receiving APT <b>if</b> one beneficiary is not registered.<br />    assert_account_is_registered_for_apt(new_beneficiary);<br /><br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br /><br />    <b>let</b> old_beneficiary &#61; <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(vesting_contract, shareholder);<br />    <b>let</b> beneficiaries &#61; &amp;<b>mut</b> vesting_contract.beneficiaries;<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(beneficiaries, &amp;shareholder)) &#123;<br />        <b>let</b> beneficiary &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(beneficiaries, &amp;shareholder);<br />        &#42;beneficiary &#61; new_beneficiary;<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(beneficiaries, shareholder, new_beneficiary);<br />    &#125;;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        emit(<br />            <a href="vesting.md#0x1_vesting_SetBeneficiary">SetBeneficiary</a> &#123;<br />                admin: vesting_contract.admin,<br />                vesting_contract_address: contract_address,<br />                shareholder,<br />                old_beneficiary,<br />                new_beneficiary,<br />            &#125;,<br />        );<br />    &#125;;<br />    emit_event(<br />        &amp;<b>mut</b> vesting_contract.set_beneficiary_events,<br />        <a href="vesting.md#0x1_vesting_SetBeneficiaryEvent">SetBeneficiaryEvent</a> &#123;<br />            admin: vesting_contract.admin,<br />            vesting_contract_address: contract_address,<br />            shareholder,<br />            old_beneficiary,<br />            new_beneficiary,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_reset_beneficiary"></a>

## Function `reset_beneficiary`

Remove the beneficiary for the given shareholder. All distributions will sent directly to the shareholder
account.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_beneficiary">reset_beneficiary</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_beneficiary">reset_beneficiary</a>(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />    shareholder: <b>address</b>,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<br />        addr &#61;&#61; vesting_contract.admin &#124;&#124;<br />            addr &#61;&#61; <a href="vesting.md#0x1_vesting_get_role_holder">get_role_holder</a>(contract_address, utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>)),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="vesting.md#0x1_vesting_EPERMISSION_DENIED">EPERMISSION_DENIED</a>),<br />    );<br /><br />    <b>let</b> beneficiaries &#61; &amp;<b>mut</b> vesting_contract.beneficiaries;<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(beneficiaries, &amp;shareholder)) &#123;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(beneficiaries, &amp;shareholder);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_set_management_role"></a>

## Function `set_management_role`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_management_role">set_management_role</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, role_holder: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_management_role">set_management_role</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />    role: String,<br />    role_holder: <b>address</b>,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br /><br />    <b>if</b> (!<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address)) &#123;<br />        <b>let</b> contract_signer &#61; &amp;<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);<br />        <b>move_to</b>(contract_signer, <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a> &#123;<br />            roles: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <b>address</b>&gt;(),<br />        &#125;)<br />    &#125;;<br />    <b>let</b> roles &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(roles, &amp;role)) &#123;<br />        &#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(roles, &amp;role) &#61; role_holder;<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(roles, role, role_holder);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_set_beneficiary_resetter"></a>

## Function `set_beneficiary_resetter`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_resetter">set_beneficiary_resetter</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, beneficiary_resetter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_resetter">set_beneficiary_resetter</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_address: <b>address</b>,<br />    beneficiary_resetter: <b>address</b>,<br />) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_set_management_role">set_management_role</a>(admin, contract_address, utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>), beneficiary_resetter);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_set_beneficiary_for_operator"></a>

## Function `set_beneficiary_for_operator`

Set the beneficiary for the operator.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_beneficiary: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(<br />    operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    new_beneficiary: <b>address</b>,<br />) &#123;<br />    <a href="staking_contract.md#0x1_staking_contract_set_beneficiary_for_operator">staking_contract::set_beneficiary_for_operator</a>(operator, new_beneficiary);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_get_role_holder"></a>

## Function `get_role_holder`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_role_holder">get_role_holder</a>(contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_role_holder">get_role_holder</a>(contract_address: <b>address</b>, role: String): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting.md#0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES">EVESTING_ACCOUNT_HAS_NO_ROLES</a>));<br />    <b>let</b> roles &#61; &amp;<b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(roles, &amp;role), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting.md#0x1_vesting_EROLE_NOT_FOUND">EROLE_NOT_FOUND</a>));<br />    &#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(roles, &amp;role)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_get_vesting_account_signer"></a>

## Function `get_vesting_account_signer`

For emergency use in case the admin needs emergency control of vesting contract account.
This doesn&apos;t give the admin total power as the admin would still need to follow the rules set by
staking_contract and stake modules.


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <b>let</b> vesting_contract &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);<br />    <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_get_vesting_account_signer_internal"></a>

## Function `get_vesting_account_signer_internal`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#123;<br />    <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&amp;vesting_contract.signer_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_create_vesting_contract_account"></a>

## Function `create_vesting_contract_account`

Create a salt for generating the resource accounts that will be holding the VestingContract.
This address should be deterministic for the same admin and vesting contract creation nonce.


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(<br />    admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, SignerCapability) <b>acquires</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> &#123;<br />    <b>let</b> admin_store &#61; <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin));<br />    <b>let</b> seed &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;admin_store.nonce));<br />    admin_store.nonce &#61; admin_store.nonce &#43; 1;<br /><br />    // Include a salt <b>to</b> avoid conflicts <b>with</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> other modules out there that might also generate<br />    // deterministic resource accounts for the same admin <b>address</b> &#43; nonce.<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, <a href="vesting.md#0x1_vesting_VESTING_POOL_SALT">VESTING_POOL_SALT</a>);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, contract_creation_seed);<br /><br />    <b>let</b> (account_signer, signer_cap) &#61; <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(admin, seed);<br />    // Register the <a href="vesting.md#0x1_vesting">vesting</a> contract <a href="account.md#0x1_account">account</a> <b>to</b> receive APT <b>as</b> it&apos;ll be sent <b>to</b> it when claiming unlocked <a href="stake.md#0x1_stake">stake</a> from<br />    // the underlying staking contract.<br />    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&amp;account_signer);<br /><br />    (account_signer, signer_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_verify_admin"></a>

## Function `verify_admin`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>) &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) &#61;&#61; vesting_contract.admin, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="vesting.md#0x1_vesting_ENOT_ADMIN">ENOT_ADMIN</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_assert_vesting_contract_exists"></a>

## Function `assert_vesting_contract_exists`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>) &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_FOUND">EVESTING_CONTRACT_NOT_FOUND</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_assert_active_vesting_contract"></a>

## Function `assert_active_vesting_contract`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> &#123;<br />    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address);<br />    <b>let</b> vesting_contract &#61; <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />    <b>assert</b>!(vesting_contract.state &#61;&#61; <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE">EVESTING_CONTRACT_NOT_ACTIVE</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_unlock_stake"></a>

## Function `unlock_stake`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>, amount: u64) &#123;<br />    <b>let</b> contract_signer &#61; &amp;<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);<br />    <a href="staking_contract.md#0x1_staking_contract_unlock_stake">staking_contract::unlock_stake</a>(contract_signer, vesting_contract.staking.operator, amount);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_withdraw_stake"></a>

## Function `withdraw_stake`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, contract_address: <b>address</b>): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>, contract_address: <b>address</b>): Coin&lt;AptosCoin&gt; &#123;<br />    // Claim <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> withdrawable distribution from the staking contract. The withdrawn coins will be sent directly <b>to</b><br />    // the <a href="vesting.md#0x1_vesting">vesting</a> contract&apos;s <a href="account.md#0x1_account">account</a>.<br />    <a href="staking_contract.md#0x1_staking_contract_distribute">staking_contract::distribute</a>(contract_address, vesting_contract.staking.operator);<br />    <b>let</b> withdrawn_coins &#61; <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(contract_address);<br />    <b>let</b> contract_signer &#61; &amp;<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);<br />    <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;AptosCoin&gt;(contract_signer, withdrawn_coins)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vesting_get_beneficiary"></a>

## Function `get_beneficiary`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, shareholder: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>, shareholder: <b>address</b>): <b>address</b> &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;contract.beneficiaries, &amp;shareholder)) &#123;<br />        &#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;contract.beneficiaries, &amp;shareholder)<br />    &#125; <b>else</b> &#123;<br />        shareholder<br />    &#125;<br />&#125;<br /></code></pre>



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
<td>In order to retrieve the address of the underlying stake pool, the vesting start timestamp of the vesting contract, the duration of the vesting period, the remaining grant of a vesting contract, the beneficiary account of a shareholder in a vesting contract, the percentage of accumulated rewards that is paid to the operator as commission, the operator who runs the validator, the voter who will be voting on&#45;chain, and the vesting schedule of a vesting contract, the supplied vesting contract should exist.</td>
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
<td>The function vesting_contracts checks if the supplied admin address contains an AdminStore resource and returns all the vesting contracts as a vector&lt;address&gt;. Otherwise it returns an empty vector. The function get_beneficiary checks for a given vesting contract, a specific shareholder exists, and if so, the beneficiary will be returned, otherwise it will simply return the address of the shareholder.</td>
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
<td>When vesting, the last_completed_period is checked against the next period to vest. This allows to unlock vested tokens for the next period since last vested, in case they didn&apos;t call vest for some periods.</td>
<td>Audited that vesting doesn&apos;t skip periods, but gradually increments to allow shareholders to retrieve all the vested tokens.</td>
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
<td>A new vesting contract should not be allowed to have an empty list of shareholders, have a different amount of shareholders than buy&#45;ins, and provide a withdrawal address which is either reserved or not registered for apt.</td>
<td>High</td>
<td>The create_vesting_contract function ensures that the withdrawal_address is not a reserved address, that it is registered for apt, that the list of shareholders is non&#45;empty, and that the amount of shareholders matches the amount of buy_ins.</td>
<td>Formally verified via <a href="#high-level-req-10">create_vesting_contract</a>.</td>
</tr>

<tr>
<td>11</td>
<td>Creating a vesting contract account should require the signer (admin) to own an admin store and should enforce that the seed of the resource account is composed of the admin store&apos;s nonce, the vesting pool salt, and the custom contract creation seed.</td>
<td>Medium</td>
<td>The create_vesting_contract_account concatenates to the seed first the admin_store.nonce then the VESTING_POOL_SALT then the contract_creation_seed and then it is passed to the create_resource_account function.</td>
<td>Enforced via <a href="#high-level-req-11">create_vesting_contract_account</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br />// This enforces <a id="high-level-spec-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>invariant</b> <b>forall</b> a: <b>address</b> <b>where</b> <b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(a):<br />    <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(a).grant_pool.shareholders_limit &lt;&#61; <a href="vesting.md#0x1_vesting_MAXIMUM_SHAREHOLDERS">MAXIMUM_SHAREHOLDERS</a>;<br /></code></pre>



<a id="@Specification_1_stake_pool_address"></a>

### Function `stake_pool_address`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_stake_pool_address">stake_pool_address</a>(vesting_contract_address: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_vesting_start_secs"></a>

### Function `vesting_start_secs`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_period_duration_secs"></a>

### Function `period_duration_secs`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_remaining_grant"></a>

### Function `remaining_grant`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_beneficiary"></a>

### Function `beneficiary`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_operator_commission_percentage"></a>

### Function `operator_commission_percentage`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_vesting_contracts"></a>

### Function `vesting_contracts`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;<br /></code></pre>




<pre><code>// This enforces <a id="high-level-spec-3.1" href="#high-level-req">high&#45;level requirement 3</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_operator"></a>

### Function `operator`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator">operator</a>(vesting_contract_address: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_voter"></a>

### Function `voter`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_voter">voter</a>(vesting_contract_address: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_vesting_schedule"></a>

### Function `vesting_schedule`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_schedule">vesting_schedule</a>(vesting_contract_address: <b>address</b>): <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /></code></pre>



<a id="@Specification_1_total_accumulated_rewards"></a>

### Function `total_accumulated_rewards`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_total_accumulated_rewards">total_accumulated_rewards</a>(vesting_contract_address: <b>address</b>): u64<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_TotalAccumulatedRewardsAbortsIf">TotalAccumulatedRewardsAbortsIf</a>;<br /></code></pre>




<a id="0x1_vesting_TotalAccumulatedRewardsAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_TotalAccumulatedRewardsAbortsIf">TotalAccumulatedRewardsAbortsIf</a> &#123;<br />vesting_contract_address: <b>address</b>;<br /><b>requires</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &gt;&#61; 0 &amp;&amp; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &lt;&#61; 100;<br /><b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;&#123;contract_address: vesting_contract_address&#125;;<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /><b>let</b> staker &#61; vesting_contract_address;<br /><b>let</b> operator &#61; vesting_contract.staking.operator;<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(staker).staking_contracts;<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(staker);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(staking_contracts, operator);<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);<br /><b>let</b> pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);<br /><b>let</b> total_active_stake &#61; active &#43; pending_active;<br /><b>let</b> accumulated_rewards &#61; total_active_stake &#45; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>let</b> commission_amount &#61; accumulated_rewards &#42; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> active &#43; pending_active &gt; MAX_U64;<br /><b>aborts_if</b> total_active_stake &lt; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>aborts_if</b> accumulated_rewards &#42; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage &gt; MAX_U64;<br /><b>aborts_if</b> (vesting_contract.remaining_grant &#43; commission_amount) &gt; total_active_stake;<br /><b>aborts_if</b> total_active_stake &lt; vesting_contract.remaining_grant;<br />&#125;<br /></code></pre>



<a id="@Specification_1_accumulated_rewards"></a>

### Function `accumulated_rewards`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_accumulated_rewards">accumulated_rewards</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): u64<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_TotalAccumulatedRewardsAbortsIf">TotalAccumulatedRewardsAbortsIf</a>;<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address);<br /><b>let</b> operator &#61; vesting_contract.staking.operator;<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(vesting_contract_address).staking_contracts;<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);<br /><b>let</b> pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);<br /><b>let</b> total_active_stake &#61; active &#43; pending_active;<br /><b>let</b> accumulated_rewards &#61; total_active_stake &#45; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>let</b> commission_amount &#61; accumulated_rewards &#42; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;<br /><b>let</b> total_accumulated_rewards &#61; total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount;<br /><b>let</b> shareholder &#61; <a href="vesting.md#0x1_vesting_spec_shareholder">spec_shareholder</a>(vesting_contract_address, shareholder_or_beneficiary);<br /><b>let</b> pool &#61; vesting_contract.grant_pool;<br /><b>let</b> shares &#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_spec_shares">pool_u64::spec_shares</a>(pool, shareholder);<br /><b>aborts_if</b> pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0<br />    &amp;&amp; (shares &#42; total_accumulated_rewards) / pool.total_shares &gt; MAX_U64;<br /><b>ensures</b> result &#61;&#61; <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_spec_shares_to_amount_with_total_coins">pool_u64::spec_shares_to_amount_with_total_coins</a>(pool, shares, total_accumulated_rewards);<br /></code></pre>



<a id="@Specification_1_shareholders"></a>

### Function `shareholders`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholders">shareholders</a>(vesting_contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;<br /></code></pre>




<pre><code><b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;&#123;contract_address: vesting_contract_address&#125;;<br /></code></pre>




<a id="0x1_vesting_spec_shareholder"></a>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_spec_shareholder">spec_shareholder</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): <b>address</b>;<br /></code></pre>



<a id="@Specification_1_shareholder"></a>

### Function `shareholder`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_shareholder">shareholder</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;&#123;contract_address: vesting_contract_address&#125;;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="vesting.md#0x1_vesting_spec_shareholder">spec_shareholder</a>(vesting_contract_address, shareholder_or_beneficiary);<br /></code></pre>



<a id="@Specification_1_create_vesting_schedule"></a>

### Function `create_vesting_schedule`


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_schedule">create_vesting_schedule</a>(schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>&gt;, start_timestamp_secs: u64, period_duration: u64): <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a><br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-6" href="#high-level-req">high&#45;level requirement 6</a>:
<b>aborts_if</b> !(len(schedule) &gt; 0);<br /><b>aborts_if</b> !(period_duration &gt; 0);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !(start_timestamp_secs &gt;&#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>());<br /></code></pre>



<a id="@Specification_1_create_vesting_contract"></a>

### Function `create_vesting_contract`


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract">create_vesting_contract</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, shareholders: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, buy_ins: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;&gt;, vesting_schedule: <a href="vesting.md#0x1_vesting_VestingSchedule">vesting::VestingSchedule</a>, withdrawal_address: <b>address</b>, operator: <b>address</b>, voter: <b>address</b>, commission_percentage: u64, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b><br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br />// This enforces <a id="high-level-req-10" href="#high-level-req">high&#45;level requirement 10</a>:
<b>aborts_if</b> withdrawal_address &#61;&#61; @aptos_framework &#124;&#124; withdrawal_address &#61;&#61; @vm_reserved;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(withdrawal_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(withdrawal_address);<br /><b>aborts_if</b> len(shareholders) &#61;&#61; 0;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_len">simple_map::spec_len</a>(buy_ins) !&#61; len(shareholders);<br /><b>ensures</b> <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(result).grant_pool.shareholders_limit &#61;&#61; 30;<br /></code></pre>



<a id="@Specification_1_unlock_rewards"></a>

### Function `unlock_rewards`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_UnlockRewardsAbortsIf">UnlockRewardsAbortsIf</a>;<br /></code></pre>




<a id="0x1_vesting_UnlockRewardsAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_UnlockRewardsAbortsIf">UnlockRewardsAbortsIf</a> &#123;<br />contract_address: <b>address</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_TotalAccumulatedRewardsAbortsIf">TotalAccumulatedRewardsAbortsIf</a> &#123; vesting_contract_address: contract_address &#125;;<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>let</b> operator &#61; vesting_contract.staking.operator;<br /><b>let</b> staking_contracts &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(contract_address).staking_contracts;<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, operator);<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);<br /><b>let</b> pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);<br /><b>let</b> total_active_stake &#61; active &#43; pending_active;<br /><b>let</b> accumulated_rewards &#61; total_active_stake &#45; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>let</b> commission_amount &#61; accumulated_rewards &#42; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;<br /><b>let</b> amount &#61; total_active_stake &#45; vesting_contract.remaining_grant &#45; commission_amount;<br /><b>include</b> <a href="vesting.md#0x1_vesting_UnlockStakeAbortsIf">UnlockStakeAbortsIf</a> &#123; vesting_contract, amount &#125;;<br />&#125;<br /></code></pre>



<a id="@Specification_1_unlock_rewards_many"></a>

### Function `unlock_rewards_many`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards_many">unlock_rewards_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>aborts_if</b> len(contract_addresses) &#61;&#61; 0;<br /><b>include</b> <a href="vesting.md#0x1_vesting_PreconditionAbortsIf">PreconditionAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_vest"></a>

### Function `vest`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest">vest</a>(contract_address: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_UnlockRewardsAbortsIf">UnlockRewardsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_vest_many"></a>

### Function `vest_many`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_vest_many">vest_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>aborts_if</b> len(contract_addresses) &#61;&#61; 0;<br /><b>include</b> <a href="vesting.md#0x1_vesting_PreconditionAbortsIf">PreconditionAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_distribute"></a>

### Function `distribute`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;;<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>include</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a> &#123; vesting_contract &#125;;<br /></code></pre>



<a id="@Specification_1_distribute_many"></a>

### Function `distribute_many`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_distribute_many">distribute_many</a>(contract_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>aborts_if</b> len(contract_addresses) &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_terminate_vesting_contract"></a>

### Function `terminate_vesting_contract`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;;<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>include</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a> &#123; vesting_contract &#125;;<br /></code></pre>



<a id="@Specification_1_admin_withdraw"></a>

### Function `admin_withdraw`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_admin_withdraw">admin_withdraw</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>aborts_if</b> vesting_contract.state !&#61; <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a> &#123; vesting_contract &#125;;<br /></code></pre>



<a id="@Specification_1_update_operator"></a>

### Function `update_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator">update_operator</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_operator: <b>address</b>, commission_percentage: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>let</b> acc &#61; vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;<br /><b>let</b> old_operator &#61; vesting_contract.staking.operator;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">staking_contract::ContractExistsAbortsIf</a> &#123; staker: acc, operator: old_operator &#125;;<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(acc);<br /><b>let</b> staking_contracts &#61; store.staking_contracts;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(staking_contracts, new_operator);<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(staking_contracts, old_operator);<br /><b>include</b> <a href="vesting.md#0x1_vesting_DistributeInternalAbortsIf">DistributeInternalAbortsIf</a> &#123; staker: acc, operator: old_operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, distribute_events: store.distribute_events &#125;;<br /></code></pre>



<a id="@Specification_1_update_operator_with_same_commission"></a>

### Function `update_operator_with_same_commission`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_operator_with_same_commission">update_operator_with_same_commission</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_update_commission_percentage"></a>

### Function `update_commission_percentage`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_commission_percentage">update_commission_percentage</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_commission_percentage: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_update_voter"></a>

### Function `update_voter`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_update_voter">update_voter</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>let</b> operator &#61; vesting_contract.staking.operator;<br /><b>let</b> staker &#61; vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_UpdateVoterSchema">staking_contract::UpdateVoterSchema</a>;<br /></code></pre>



<a id="@Specification_1_reset_lockup"></a>

### Function `reset_lockup`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_lockup">reset_lockup</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) !&#61; vesting_contract.admin;<br /><b>let</b> operator &#61; vesting_contract.staking.operator;<br /><b>let</b> staker &#61; vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">staking_contract::ContractExistsAbortsIf</a> &#123;staker, operator&#125;;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_IncreaseLockupWithCapAbortsIf">staking_contract::IncreaseLockupWithCapAbortsIf</a> &#123;staker, operator&#125;;<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(staker);<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(vesting_contract.staking.pool_address);<br /></code></pre>



<a id="@Specification_1_set_beneficiary"></a>

### Function `set_beneficiary`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary">set_beneficiary</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>, new_beneficiary: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(new_beneficiary);<br /><b>aborts_if</b> !<a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;AptosCoin&gt;(new_beneficiary);<br /><b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;<br /><b>let</b> <b>post</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(vesting_contract.beneficiaries,shareholder);<br /></code></pre>



<a id="@Specification_1_reset_beneficiary"></a>

### Function `reset_beneficiary`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_reset_beneficiary">reset_beneficiary</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>aborts_if</b> addr !&#61; vesting_contract.admin &amp;&amp; !std::string::spec_internal_check_utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>);<br /><b>aborts_if</b> addr !&#61; vesting_contract.admin &amp;&amp; !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address);<br /><b>let</b> roles &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;<br /><b>let</b> role &#61; std::string::spec_utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>);<br /><b>aborts_if</b> addr !&#61; vesting_contract.admin &amp;&amp; !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(roles, role);<br /><b>aborts_if</b> addr !&#61; vesting_contract.admin &amp;&amp; addr !&#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(roles, role);<br /><b>let</b> <b>post</b> post_vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>ensures</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_vesting_contract.beneficiaries,shareholder);<br /></code></pre>



<a id="@Specification_1_set_management_role"></a>

### Function `set_management_role`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_management_role">set_management_role</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, role_holder: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="vesting.md#0x1_vesting_SetManagementRoleAbortsIf">SetManagementRoleAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_set_beneficiary_resetter"></a>

### Function `set_beneficiary_resetter`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_resetter">set_beneficiary_resetter</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, beneficiary_resetter: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>);<br /><b>include</b> <a href="vesting.md#0x1_vesting_SetManagementRoleAbortsIf">SetManagementRoleAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_set_beneficiary_for_operator"></a>

### Function `set_beneficiary_for_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_set_beneficiary_for_operator">set_beneficiary_for_operator</a>(operator: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_beneficiary: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_get_role_holder"></a>

### Function `get_role_holder`


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_role_holder">get_role_holder</a>(contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address);<br /><b>let</b> roles &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(roles,role);<br /></code></pre>



<a id="@Specification_1_get_vesting_account_signer"></a>

### Function `get_vesting_account_signer`


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>




<pre><code><b>include</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_get_vesting_account_signer_internal"></a>

### Function `get_vesting_account_signer_internal`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>




<a id="0x1_vesting_spec_get_vesting_account_signer"></a>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_spec_get_vesting_account_signer">spec_get_vesting_account_signer</a>(vesting_contract: <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /></code></pre>



<a id="@Specification_1_create_vesting_contract_account"></a>

### Function `create_vesting_contract_account`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>let</b> admin_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin);<br /><b>let</b> admin_store &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin_addr);<br /><b>let</b> seed &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(admin_addr);<br /><b>let</b> nonce &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(admin_store.nonce);<br /><b>let</b> first &#61; concat(seed, nonce);<br /><b>let</b> second &#61; concat(first, <a href="vesting.md#0x1_vesting_VESTING_POOL_SALT">VESTING_POOL_SALT</a>);<br /><b>let</b> end &#61; concat(second, contract_creation_seed);<br />// This enforces <a id="high-level-req-11" href="#high-level-req">high&#45;level requirement 11</a>:
<b>let</b> resource_addr &#61; <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(admin_addr, end);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a>&gt;(admin_addr);<br /><b>aborts_if</b> len(<a href="account.md#0x1_account_ZERO_AUTH_KEY">account::ZERO_AUTH_KEY</a>) !&#61; 32;<br /><b>aborts_if</b> admin_store.nonce &#43; 1 &gt; MAX_U64;<br /><b>let</b> ea &#61; <a href="account.md#0x1_account_exists_at">account::exists_at</a>(resource_addr);<br /><b>include</b> <b>if</b> (ea) <a href="account.md#0x1_account_CreateResourceAccountAbortsIf">account::CreateResourceAccountAbortsIf</a> <b>else</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">account::CreateAccountAbortsIf</a> &#123;addr: resource_addr&#125;;<br /><b>let</b> acc &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);<br /><b>let</b> <b>post</b> post_acc &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; !aptos_std::type_info::spec_is_struct&lt;AptosCoin&gt;();<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; ea &amp;&amp; acc.guid_creation_num &#43; 2 &gt; MAX_U64;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr) &amp;&amp; ea &amp;&amp; acc.guid_creation_num &#43; 2 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr) &amp;&amp; post_acc.authentication_key &#61;&#61; <a href="account.md#0x1_account_ZERO_AUTH_KEY">account::ZERO_AUTH_KEY</a> &amp;&amp;<br />        <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr);<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result_1) &#61;&#61; resource_addr;<br /><b>ensures</b> result_2.<a href="account.md#0x1_account">account</a> &#61;&#61; resource_addr;<br /></code></pre>



<a id="@Specification_1_verify_admin"></a>

### Function `verify_admin`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>)<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-9" href="#high-level-req">high&#45;level requirement 9</a>:
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) !&#61; vesting_contract.admin;<br /></code></pre>



<a id="@Specification_1_assert_vesting_contract_exists"></a>

### Function `assert_vesting_contract_exists`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>)<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /></code></pre>



<a id="@Specification_1_assert_active_vesting_contract"></a>

### Function `assert_active_vesting_contract`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>)<br /></code></pre>




<pre><code><b>include</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;;<br /></code></pre>



<a id="@Specification_1_unlock_stake"></a>

### Function `unlock_stake`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_UnlockStakeAbortsIf">UnlockStakeAbortsIf</a>;<br /></code></pre>




<a id="0x1_vesting_UnlockStakeAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_UnlockStakeAbortsIf">UnlockStakeAbortsIf</a> &#123;<br />vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>;<br />amount: u64;<br /><b>let</b> acc &#61; vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;<br /><b>let</b> operator &#61; vesting_contract.staking.operator;<br /><b>include</b> amount !&#61; 0 &#61;&#61;&gt; <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">staking_contract::ContractExistsAbortsIf</a> &#123; staker: acc, operator &#125;;<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(acc);<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);<br /><b>include</b> amount !&#61; 0 &#61;&#61;&gt; <a href="vesting.md#0x1_vesting_DistributeInternalAbortsIf">DistributeInternalAbortsIf</a> &#123; staker: acc, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, distribute_events: store.distribute_events &#125;;<br />&#125;<br /></code></pre>



<a id="@Specification_1_withdraw_stake"></a>

### Function `withdraw_stake`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, contract_address: <b>address</b>): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a>;<br /></code></pre>




<a id="0x1_vesting_WithdrawStakeAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_WithdrawStakeAbortsIf">WithdrawStakeAbortsIf</a> &#123;<br />vesting_contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>;<br />contract_address: <b>address</b>;<br /><b>let</b> operator &#61; vesting_contract.staking.operator;<br /><b>include</b> <a href="staking_contract.md#0x1_staking_contract_ContractExistsAbortsIf">staking_contract::ContractExistsAbortsIf</a> &#123; staker: contract_address, operator &#125;;<br /><b>let</b> store &#61; <b>global</b>&lt;<a href="staking_contract.md#0x1_staking_contract_Store">staking_contract::Store</a>&gt;(contract_address);<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);<br /><b>include</b> <a href="vesting.md#0x1_vesting_DistributeInternalAbortsIf">DistributeInternalAbortsIf</a> &#123; staker: contract_address, operator, <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>, distribute_events: store.distribute_events &#125;;<br />&#125;<br /></code></pre>




<a id="0x1_vesting_DistributeInternalAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_DistributeInternalAbortsIf">DistributeInternalAbortsIf</a> &#123;<br />staker: <b>address</b>;<br />operator: <b>address</b>;<br /><a href="staking_contract.md#0x1_staking_contract">staking_contract</a>: <a href="staking_contract.md#0x1_staking_contract_StakingContract">staking_contract::StakingContract</a>;<br />distribute_events: EventHandle&lt;<a href="staking_contract.md#0x1_staking_contract_DistributeEvent">staking_contract::DistributeEvent</a>&gt;;<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>let</b> inactive &#61; stake_pool.inactive.value;<br /><b>let</b> pending_inactive &#61; stake_pool.pending_inactive.value;<br /><b>aborts_if</b> inactive &#43; pending_inactive &gt; MAX_U64;<br /><b>let</b> total_potential_withdrawable &#61; inactive &#43; pending_inactive;<br /><b>let</b> pool_address_1 &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address_1);<br /><b>let</b> stake_pool_1 &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address_1);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> validator_set &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@aptos_framework);<br /><b>let</b> inactive_state &#61; !<a href="stake.md#0x1_stake_spec_contains">stake::spec_contains</a>(validator_set.pending_active, pool_address_1)<br />    &amp;&amp; !<a href="stake.md#0x1_stake_spec_contains">stake::spec_contains</a>(validator_set.active_validators, pool_address_1)<br />    &amp;&amp; !<a href="stake.md#0x1_stake_spec_contains">stake::spec_contains</a>(validator_set.pending_inactive, pool_address_1);<br /><b>let</b> inactive_1 &#61; stake_pool_1.inactive.value;<br /><b>let</b> pending_inactive_1 &#61; stake_pool_1.pending_inactive.value;<br /><b>let</b> new_inactive_1 &#61; inactive_1 &#43; pending_inactive_1;<br /><b>aborts_if</b> inactive_state &amp;&amp; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt;&#61; stake_pool_1.locked_until_secs<br />    &amp;&amp; inactive_1 &#43; pending_inactive_1 &gt; MAX_U64;<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_beneficiary"></a>

### Function `get_beneficiary`


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(contract: &amp;<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>, shareholder: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code>// This enforces <a id="high-level-spec-3.2" href="#high-level-req">high&#45;level requirement 3</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>




<a id="0x1_vesting_SetManagementRoleAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_SetManagementRoleAbortsIf">SetManagementRoleAbortsIf</a> &#123;<br />contract_address: <b>address</b>;<br />admin: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) !&#61; vesting_contract.admin;<br />&#125;<br /></code></pre>




<a id="0x1_vesting_VerifyAdminAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_VerifyAdminAbortsIf">VerifyAdminAbortsIf</a> &#123;<br />contract_address: <b>address</b>;<br />admin: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) !&#61; vesting_contract.admin;<br />&#125;<br /></code></pre>




<a id="0x1_vesting_ActiveVestingContractAbortsIf"></a>


<pre><code><b>schema</b> <a href="vesting.md#0x1_vesting_ActiveVestingContractAbortsIf">ActiveVestingContractAbortsIf</a>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt; &#123;<br />contract_address: <b>address</b>;<br />// This enforces <a id="high-level-spec-5" href="#high-level-req">high&#45;level requirement 5</a>:
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br /><b>let</b> vesting_contract &#61; <b>global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);<br />// This enforces <a id="high-level-spec-8" href="#high-level-req">high&#45;level requirement 8</a>:
    <b>aborts_if</b> vesting_contract.state !&#61; <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>;<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
