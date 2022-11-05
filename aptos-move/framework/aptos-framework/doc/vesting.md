
<a name="0x1_vesting"></a>

# Module `0x1::vesting`



-  [Struct `VestingSchedule`](#0x1_vesting_VestingSchedule)
-  [Struct `StakingInfo`](#0x1_vesting_StakingInfo)
-  [Resource `VestingContract`](#0x1_vesting_VestingContract)
-  [Resource `VestingAccountManagement`](#0x1_vesting_VestingAccountManagement)
-  [Resource `AdminStore`](#0x1_vesting_AdminStore)
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
-  [Function `remaining_grant`](#0x1_vesting_remaining_grant)
-  [Function `beneficiary`](#0x1_vesting_beneficiary)
-  [Function `operator_commission_percentage`](#0x1_vesting_operator_commission_percentage)
-  [Function `vesting_contracts`](#0x1_vesting_vesting_contracts)
-  [Function `operator`](#0x1_vesting_operator)
-  [Function `voter`](#0x1_vesting_voter)
-  [Function `create_vesting_schedule`](#0x1_vesting_create_vesting_schedule)
-  [Function `create_vesting_contract`](#0x1_vesting_create_vesting_contract)
-  [Function `unlock_rewards`](#0x1_vesting_unlock_rewards)
-  [Function `vest`](#0x1_vesting_vest)
-  [Function `distribute`](#0x1_vesting_distribute)
-  [Function `terminate_vesting_contract`](#0x1_vesting_terminate_vesting_contract)
-  [Function `admin_withdraw`](#0x1_vesting_admin_withdraw)
-  [Function `update_operator`](#0x1_vesting_update_operator)
-  [Function `update_operator_with_same_commission`](#0x1_vesting_update_operator_with_same_commission)
-  [Function `update_voter`](#0x1_vesting_update_voter)
-  [Function `reset_lockup`](#0x1_vesting_reset_lockup)
-  [Function `set_beneficiary`](#0x1_vesting_set_beneficiary)
-  [Function `reset_beneficiary`](#0x1_vesting_reset_beneficiary)
-  [Function `set_management_role`](#0x1_vesting_set_management_role)
-  [Function `set_beneficiary_resetter`](#0x1_vesting_set_beneficiary_resetter)
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64">0x1::pool_u64</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="staking_contract.md#0x1_staking_contract">0x1::staking_contract</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_vesting_VestingSchedule"></a>

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

<a name="0x1_vesting_StakingInfo"></a>

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

<a name="0x1_vesting_VestingContract"></a>

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

<a name="0x1_vesting_VestingAccountManagement"></a>

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

<a name="0x1_vesting_AdminStore"></a>

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

<a name="0x1_vesting_CreateVestingContractEvent"></a>

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

<a name="0x1_vesting_UpdateOperatorEvent"></a>

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

<a name="0x1_vesting_UpdateVoterEvent"></a>

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

<a name="0x1_vesting_ResetLockupEvent"></a>

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

<a name="0x1_vesting_SetBeneficiaryEvent"></a>

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

<a name="0x1_vesting_UnlockRewardsEvent"></a>

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

<a name="0x1_vesting_VestEvent"></a>

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

<a name="0x1_vesting_DistributeEvent"></a>

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

<a name="0x1_vesting_TerminateEvent"></a>

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

<a name="0x1_vesting_AdminWithdrawEvent"></a>

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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_vesting_EEMPTY_VESTING_SCHEDULE"></a>

Vesting schedule cannot be empty.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>: u64 = 2;
</code></pre>



<a name="0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS"></a>

Withdrawal address is invalid.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EINVALID_WITHDRAWAL_ADDRESS">EINVALID_WITHDRAWAL_ADDRESS</a>: u64 = 1;
</code></pre>



<a name="0x1_vesting_ENOT_ADMIN"></a>

The signer is not the admin of the vesting contract.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ENOT_ADMIN">ENOT_ADMIN</a>: u64 = 7;
</code></pre>



<a name="0x1_vesting_ENO_SHAREHOLDERS"></a>

Shareholders list cannot be empty.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>: u64 = 4;
</code></pre>



<a name="0x1_vesting_EPENDING_STAKE_FOUND"></a>

Cannot terminate the vesting contract with pending active stake. Need to wait until next epoch.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EPENDING_STAKE_FOUND">EPENDING_STAKE_FOUND</a>: u64 = 11;
</code></pre>



<a name="0x1_vesting_EPERMISSION_DENIED"></a>

Account is not admin or does not have the required role to take this action.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EPERMISSION_DENIED">EPERMISSION_DENIED</a>: u64 = 15;
</code></pre>



<a name="0x1_vesting_EROLE_NOT_FOUND"></a>

The vesting account has no such management role.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EROLE_NOT_FOUND">EROLE_NOT_FOUND</a>: u64 = 14;
</code></pre>



<a name="0x1_vesting_ESHARES_LENGTH_MISMATCH"></a>

The length of shareholders and shares lists don't match.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ESHARES_LENGTH_MISMATCH">ESHARES_LENGTH_MISMATCH</a>: u64 = 5;
</code></pre>



<a name="0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES"></a>

Vesting account has no other management roles beside admin.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_ACCOUNT_HAS_NO_ROLES">EVESTING_ACCOUNT_HAS_NO_ROLES</a>: u64 = 13;
</code></pre>



<a name="0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE"></a>

Vesting contract needs to be in active state.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_ACTIVE">EVESTING_CONTRACT_NOT_ACTIVE</a>: u64 = 8;
</code></pre>



<a name="0x1_vesting_EVESTING_CONTRACT_NOT_FOUND"></a>

No vesting contract found at provided address.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_NOT_FOUND">EVESTING_CONTRACT_NOT_FOUND</a>: u64 = 10;
</code></pre>



<a name="0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE"></a>

Admin can only withdraw from an inactive (paused or terminated) vesting contract.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE">EVESTING_CONTRACT_STILL_ACTIVE</a>: u64 = 9;
</code></pre>



<a name="0x1_vesting_EVESTING_START_TOO_SOON"></a>

Vesting cannot start before or at the current block timestamp. Has to be in the future.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EVESTING_START_TOO_SOON">EVESTING_START_TOO_SOON</a>: u64 = 6;
</code></pre>



<a name="0x1_vesting_EZERO_GRANT"></a>

Grant amount cannot be 0.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EZERO_GRANT">EZERO_GRANT</a>: u64 = 12;
</code></pre>



<a name="0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD"></a>

Vesting period cannot be 0.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_EZERO_VESTING_SCHEDULE_PERIOD">EZERO_VESTING_SCHEDULE_PERIOD</a>: u64 = 3;
</code></pre>



<a name="0x1_vesting_MAXIMUM_SHAREHOLDERS"></a>

Maximum number of shareholders a vesting pool can support.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_MAXIMUM_SHAREHOLDERS">MAXIMUM_SHAREHOLDERS</a>: u64 = 30;
</code></pre>



<a name="0x1_vesting_ROLE_BENEFICIARY_RESETTER"></a>

Roles that can manage certain aspects of the vesting account beyond the main admin.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [82, 79, 76, 69, 95, 66, 69, 78, 69, 70, 73, 67, 73, 65, 82, 89, 95, 82, 69, 83, 69, 84, 84, 69, 82];
</code></pre>



<a name="0x1_vesting_VESTING_POOL_ACTIVE"></a>

Vesting contract states.
Vesting contract is active and distributions can be made.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>: u64 = 1;
</code></pre>



<a name="0x1_vesting_VESTING_POOL_SALT"></a>



<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_SALT">VESTING_POOL_SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 118, 101, 115, 116, 105, 110, 103];
</code></pre>



<a name="0x1_vesting_VESTING_POOL_TERMINATED"></a>

Vesting contract has been terminated and all funds have been released back to the withdrawal address.


<pre><code><b>const</b> <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>: u64 = 2;
</code></pre>



<a name="0x1_vesting_stake_pool_address"></a>

## Function `stake_pool_address`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_stake_pool_address">stake_pool_address</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_stake_pool_address">stake_pool_address</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.pool_address
}
</code></pre>



</details>

<a name="0x1_vesting_vesting_start_secs"></a>

## Function `vesting_start_secs`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule.start_timestamp_secs
}
</code></pre>



</details>

<a name="0x1_vesting_remaining_grant"></a>

## Function `remaining_grant`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).remaining_grant
}
</code></pre>



</details>

<a name="0x1_vesting_beneficiary"></a>

## Function `beneficiary`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(<b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address), shareholder)
}
</code></pre>



</details>

<a name="0x1_vesting_operator_commission_percentage"></a>

## Function `operator_commission_percentage`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator_commission_percentage">operator_commission_percentage</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.commission_percentage
}
</code></pre>



</details>

<a name="0x1_vesting_vesting_contracts"></a>

## Function `vesting_contracts`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
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

<a name="0x1_vesting_operator"></a>

## Function `operator`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator">operator</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_operator">operator</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.operator
}
</code></pre>



</details>

<a name="0x1_vesting_voter"></a>

## Function `voter`



<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_voter">voter</a>(vesting_contract_address: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_voter">voter</a>(vesting_contract_address: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(vesting_contract_address).staking.voter
}
</code></pre>



</details>

<a name="0x1_vesting_create_vesting_schedule"></a>

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

<a name="0x1_vesting_create_vesting_contract"></a>

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
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shareholders);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        <b>let</b> shareholder = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shareholders, i);
        <b>let</b> (_, buy_in) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> buy_ins, &shareholder);
        <b>let</b> buy_in_amount = <a href="coin.md#0x1_coin_value">coin::value</a>(&buy_in);
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> grant, buy_in);
        <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_buy_in">pool_u64::buy_in</a>(
            &<b>mut</b> grant_pool,
            *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shareholders, i),
            buy_in_amount,
        );
        grant_amount = grant_amount + buy_in_amount;

        i = i + 1;
    };
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

<a name="0x1_vesting_unlock_rewards"></a>

## Function `unlock_rewards`

Unlock any accumulated rewards.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_unlock_rewards">unlock_rewards</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> contract_signer = &<a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <a href="staking_contract.md#0x1_staking_contract_unlock_rewards">staking_contract::unlock_rewards</a>(contract_signer, vesting_contract.staking.operator);
}
</code></pre>



</details>

<a name="0x1_vesting_vest"></a>

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
}
</code></pre>



</details>

<a name="0x1_vesting_distribute"></a>

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

    // Distribute coins <b>to</b> all shareholders in the <a href="vesting.md#0x1_vesting">vesting</a> contract.
    <b>let</b> grant_pool = &vesting_contract.grant_pool;
    <b>let</b> shareholders = &<a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shareholders">pool_u64::shareholders</a>(grant_pool);
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shareholders);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        <b>let</b> shareholder = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shareholders, i);
        <b>let</b> shares = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares">pool_u64::shares</a>(grant_pool, shareholder);
        <b>let</b> amount = <a href="../../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">pool_u64::shares_to_amount_with_total_coins</a>(grant_pool, shares, total_distribution_amount);
        <b>let</b> share_of_coins = <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> coins, amount);
        <b>let</b> recipient_address = <a href="vesting.md#0x1_vesting_get_beneficiary">get_beneficiary</a>(vesting_contract, shareholder);
        <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(recipient_address, share_of_coins);

        i = i + 1;
    };

    // Send <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> remaining "dust" (leftover due <b>to</b> rounding <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a>) <b>to</b> the withdrawal <b>address</b>.
    <b>if</b> (<a href="coin.md#0x1_coin_value">coin::value</a>(&coins) &gt; 0) {
        <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(vesting_contract.withdrawal_address, coins);
    } <b>else</b> {
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);
    };

    emit_event(
        &<b>mut</b> vesting_contract.distribute_events,
        <a href="vesting.md#0x1_vesting_DistributeEvent">DistributeEvent</a> {
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            amount: total_distribution_amount,
        },
    );
}
</code></pre>



</details>

<a name="0x1_vesting_terminate_vesting_contract"></a>

## Function `terminate_vesting_contract`

Terminate the vesting contract and send all funds back to the withdrawal address.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <a href="vesting.md#0x1_vesting_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);

    // Distribute all withdrawable coins, which should have been from previous rewards withdrawal or vest.
    <a href="vesting.md#0x1_vesting_distribute">distribute</a>(contract_address);

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> (active_stake, _, pending_active_stake, _) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(vesting_contract.staking.pool_address);
    <b>assert</b>!(pending_active_stake == 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting.md#0x1_vesting_EPENDING_STAKE_FOUND">EPENDING_STAKE_FOUND</a>));

    // Unlock all remaining active <a href="stake.md#0x1_stake">stake</a>.
    vesting_contract.state = <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>;
    vesting_contract.remaining_grant = 0;
    <a href="vesting.md#0x1_vesting_unlock_stake">unlock_stake</a>(vesting_contract, active_stake);

    emit_event(
        &<b>mut</b> vesting_contract.terminate_events,
        <a href="vesting.md#0x1_vesting_TerminateEvent">TerminateEvent</a> {
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
        },
    );
}
</code></pre>



</details>

<a name="0x1_vesting_admin_withdraw"></a>

## Function `admin_withdraw`

Withdraw all funds to the preset vesting contract's withdrawal address. This can only be called if the contract
has already been terminated.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_admin_withdraw">admin_withdraw</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting.md#0x1_vesting_admin_withdraw">admin_withdraw</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>) <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>assert</b>!(vesting_contract.state == <a href="vesting.md#0x1_vesting_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting.md#0x1_vesting_EVESTING_CONTRACT_STILL_ACTIVE">EVESTING_CONTRACT_STILL_ACTIVE</a>));

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> coins = <a href="vesting.md#0x1_vesting_withdraw_stake">withdraw_stake</a>(vesting_contract, contract_address);
    <b>let</b> amount = <a href="coin.md#0x1_coin_value">coin::value</a>(&coins);
    <b>if</b> (amount == 0) {
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(coins);
        <b>return</b>
    };
    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(vesting_contract.withdrawal_address, coins);

    emit_event(
        &<b>mut</b> vesting_contract.admin_withdraw_events,
        <a href="vesting.md#0x1_vesting_AdminWithdrawEvent">AdminWithdrawEvent</a> {
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            amount,
        },
    );
}
</code></pre>



</details>

<a name="0x1_vesting_update_operator"></a>

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
}
</code></pre>



</details>

<a name="0x1_vesting_update_operator_with_same_commission"></a>

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

<a name="0x1_vesting_update_voter"></a>

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
</code></pre>



</details>

<a name="0x1_vesting_reset_lockup"></a>

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

    emit_event(
        &<b>mut</b> vesting_contract.reset_lockup_events,
        <a href="vesting.md#0x1_vesting_ResetLockupEvent">ResetLockupEvent</a> {
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            staking_pool_address: vesting_contract.staking.pool_address,
            new_lockup_expiration_secs: <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(vesting_contract.staking.pool_address),
        },
    );
}
</code></pre>



</details>

<a name="0x1_vesting_set_beneficiary"></a>

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
}
</code></pre>



</details>

<a name="0x1_vesting_reset_beneficiary"></a>

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

<a name="0x1_vesting_set_management_role"></a>

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
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
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

<a name="0x1_vesting_set_beneficiary_resetter"></a>

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

<a name="0x1_vesting_get_role_holder"></a>

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

<a name="0x1_vesting_get_vesting_account_signer"></a>

## Function `get_vesting_account_signer`

For emergency use in case the admin needs emergency control of vesting contract account.
This doesn't give the admin total power as the admin would still need to follow the rules set by
staking_contract and stake modules.


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting.md#0x1_vesting_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin, vesting_contract);
    <a href="vesting.md#0x1_vesting_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract)
}
</code></pre>



</details>

<a name="0x1_vesting_get_vesting_account_signer_internal"></a>

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

<a name="0x1_vesting_create_vesting_contract_account"></a>

## Function `create_vesting_contract_account`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_create_vesting_contract_account">create_vesting_contract_account</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, SignerCapability) <b>acquires</b> <a href="vesting.md#0x1_vesting_AdminStore">AdminStore</a> {
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

<a name="0x1_vesting_verify_admin"></a>

## Function `verify_admin`



<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">vesting::VestingContract</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting.md#0x1_vesting_verify_admin">verify_admin</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &<a href="vesting.md#0x1_vesting_VestingContract">VestingContract</a>) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) == vesting_contract.admin, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="vesting.md#0x1_vesting_ENOT_ADMIN">ENOT_ADMIN</a>));
}
</code></pre>



</details>

<a name="0x1_vesting_assert_vesting_contract_exists"></a>

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

<a name="0x1_vesting_assert_active_vesting_contract"></a>

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

<a name="0x1_vesting_unlock_stake"></a>

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

<a name="0x1_vesting_withdraw_stake"></a>

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

<a name="0x1_vesting_get_beneficiary"></a>

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

<a name="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
