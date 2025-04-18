
<a id="0x1_vesting_without_staking"></a>

# Module `0x1::vesting_without_staking`


Vesting without staking contract


-  [Struct `VestingSchedule`](#0x1_vesting_without_staking_VestingSchedule)
-  [Struct `VestingRecord`](#0x1_vesting_without_staking_VestingRecord)
-  [Resource `VestingContract`](#0x1_vesting_without_staking_VestingContract)
-  [Resource `VestingAccountManagement`](#0x1_vesting_without_staking_VestingAccountManagement)
-  [Resource `AdminStore`](#0x1_vesting_without_staking_AdminStore)
-  [Struct `CreateVestingContractEvent`](#0x1_vesting_without_staking_CreateVestingContractEvent)
-  [Struct `SetBeneficiaryEvent`](#0x1_vesting_without_staking_SetBeneficiaryEvent)
-  [Struct `VestEvent`](#0x1_vesting_without_staking_VestEvent)
-  [Struct `TerminateEvent`](#0x1_vesting_without_staking_TerminateEvent)
-  [Struct `AdminWithdrawEvent`](#0x1_vesting_without_staking_AdminWithdrawEvent)
-  [Struct `ShareHolderRemovedEvent`](#0x1_vesting_without_staking_ShareHolderRemovedEvent)
-  [Constants](#@Constants_0)
-  [Function `vesting_start_secs`](#0x1_vesting_without_staking_vesting_start_secs)
-  [Function `period_duration_secs`](#0x1_vesting_without_staking_period_duration_secs)
-  [Function `get_withdrawal_addr`](#0x1_vesting_without_staking_get_withdrawal_addr)
-  [Function `get_contract_admin`](#0x1_vesting_without_staking_get_contract_admin)
-  [Function `get_vesting_record`](#0x1_vesting_without_staking_get_vesting_record)
-  [Function `remaining_grant`](#0x1_vesting_without_staking_remaining_grant)
-  [Function `beneficiary`](#0x1_vesting_without_staking_beneficiary)
-  [Function `vesting_contracts`](#0x1_vesting_without_staking_vesting_contracts)
-  [Function `vesting_schedule`](#0x1_vesting_without_staking_vesting_schedule)
-  [Function `shareholders`](#0x1_vesting_without_staking_shareholders)
-  [Function `shareholder`](#0x1_vesting_without_staking_shareholder)
-  [Function `create_vesting_schedule`](#0x1_vesting_without_staking_create_vesting_schedule)
-  [Function `create_vesting_contract_with_amounts`](#0x1_vesting_without_staking_create_vesting_contract_with_amounts)
-  [Function `create_vesting_contract`](#0x1_vesting_without_staking_create_vesting_contract)
-  [Function `vest`](#0x1_vesting_without_staking_vest)
-  [Function `vest_individual`](#0x1_vesting_without_staking_vest_individual)
-  [Function `vest_transfer`](#0x1_vesting_without_staking_vest_transfer)
-  [Function `remove_shareholder`](#0x1_vesting_without_staking_remove_shareholder)
-  [Function `terminate_vesting_contract`](#0x1_vesting_without_staking_terminate_vesting_contract)
-  [Function `admin_withdraw`](#0x1_vesting_without_staking_admin_withdraw)
-  [Function `set_beneficiary`](#0x1_vesting_without_staking_set_beneficiary)
-  [Function `reset_beneficiary`](#0x1_vesting_without_staking_reset_beneficiary)
-  [Function `set_management_role`](#0x1_vesting_without_staking_set_management_role)
-  [Function `set_beneficiary_resetter`](#0x1_vesting_without_staking_set_beneficiary_resetter)
-  [Function `get_role_holder`](#0x1_vesting_without_staking_get_role_holder)
-  [Function `get_vesting_account_signer`](#0x1_vesting_without_staking_get_vesting_account_signer)
-  [Function `get_vesting_account_signer_internal`](#0x1_vesting_without_staking_get_vesting_account_signer_internal)
-  [Function `create_vesting_contract_account`](#0x1_vesting_without_staking_create_vesting_contract_account)
-  [Function `verify_admin`](#0x1_vesting_without_staking_verify_admin)
-  [Function `assert_vesting_contract_exists`](#0x1_vesting_without_staking_assert_vesting_contract_exists)
-  [Function `assert_shareholder_exists`](#0x1_vesting_without_staking_assert_shareholder_exists)
-  [Function `assert_active_vesting_contract`](#0x1_vesting_without_staking_assert_active_vesting_contract)
-  [Function `get_beneficiary`](#0x1_vesting_without_staking_get_beneficiary)
-  [Function `set_terminate_vesting_contract`](#0x1_vesting_without_staking_set_terminate_vesting_contract)
-  [Specification](#@Specification_1)
    -  [Struct `VestingRecord`](#@Specification_1_VestingRecord)
    -  [Function `vesting_start_secs`](#@Specification_1_vesting_start_secs)
    -  [Function `period_duration_secs`](#@Specification_1_period_duration_secs)
    -  [Function `remaining_grant`](#@Specification_1_remaining_grant)
    -  [Function `beneficiary`](#@Specification_1_beneficiary)
    -  [Function `vesting_contracts`](#@Specification_1_vesting_contracts)
    -  [Function `vesting_schedule`](#@Specification_1_vesting_schedule)
    -  [Function `create_vesting_schedule`](#@Specification_1_create_vesting_schedule)
    -  [Function `vest`](#@Specification_1_vest)
    -  [Function `vest_individual`](#@Specification_1_vest_individual)
    -  [Function `vest_transfer`](#@Specification_1_vest_transfer)
    -  [Function `remove_shareholder`](#@Specification_1_remove_shareholder)
    -  [Function `admin_withdraw`](#@Specification_1_admin_withdraw)
    -  [Function `set_beneficiary`](#@Specification_1_set_beneficiary)
    -  [Function `reset_beneficiary`](#@Specification_1_reset_beneficiary)
    -  [Function `set_management_role`](#@Specification_1_set_management_role)
    -  [Function `set_beneficiary_resetter`](#@Specification_1_set_beneficiary_resetter)
    -  [Function `get_role_holder`](#@Specification_1_get_role_holder)
    -  [Function `get_vesting_account_signer`](#@Specification_1_get_vesting_account_signer)
    -  [Function `get_vesting_account_signer_internal`](#@Specification_1_get_vesting_account_signer_internal)
    -  [Function `create_vesting_contract_account`](#@Specification_1_create_vesting_contract_account)
    -  [Function `verify_admin`](#@Specification_1_verify_admin)
    -  [Function `assert_vesting_contract_exists`](#@Specification_1_assert_vesting_contract_exists)
    -  [Function `assert_active_vesting_contract`](#@Specification_1_assert_active_vesting_contract)
    -  [Function `get_beneficiary`](#@Specification_1_get_beneficiary)
    -  [Function `set_terminate_vesting_contract`](#@Specification_1_set_terminate_vesting_contract)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="supra_account.md#0x1_supra_account">0x1::supra_account</a>;
<b>use</b> <a href="supra_coin.md#0x1_supra_coin">0x1::supra_coin</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_vesting_without_staking_VestingSchedule"></a>

## Struct `VestingSchedule`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">VestingSchedule</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_vesting_without_staking_VestingRecord"></a>

## Struct `VestingRecord`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">VestingRecord</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>init_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>left_amount: u64</code>
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

<a id="0x1_vesting_without_staking_VestingContract"></a>

## Resource `VestingContract`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> <b>has</b> key
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
<code>beneficiaries: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>shareholders: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">vesting_without_staking::VestingRecord</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_schedule: <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">vesting_without_staking::VestingSchedule</a></code>
</dt>
<dd>

</dd>
<dt>
<code>withdrawal_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>

</dd>
<dt>
<code>set_beneficiary_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_SetBeneficiaryEvent">vesting_without_staking::SetBeneficiaryEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vest_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestEvent">vesting_without_staking::VestEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>terminate_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_TerminateEvent">vesting_without_staking::TerminateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>admin_withdraw_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminWithdrawEvent">vesting_without_staking::AdminWithdrawEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>shareholder_removed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_ShareHolderRemovedEvent">vesting_without_staking::ShareHolderRemovedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_without_staking_VestingAccountManagement"></a>

## Resource `VestingAccountManagement`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a> <b>has</b> key
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

<a id="0x1_vesting_without_staking_AdminStore"></a>

## Resource `AdminStore`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a> <b>has</b> key
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
<code>create_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_CreateVestingContractEvent">vesting_without_staking::CreateVestingContractEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_without_staking_CreateVestingContractEvent"></a>

## Struct `CreateVestingContractEvent`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_CreateVestingContractEvent">CreateVestingContractEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
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
</dl>


</details>

<a id="0x1_vesting_without_staking_SetBeneficiaryEvent"></a>

## Struct `SetBeneficiaryEvent`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_SetBeneficiaryEvent">SetBeneficiaryEvent</a> <b>has</b> drop, store
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

<a id="0x1_vesting_without_staking_VestEvent"></a>

## Struct `VestEvent`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestEvent">VestEvent</a> <b>has</b> drop, store
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
<code>shareholder_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>period_vested: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_vesting_without_staking_TerminateEvent"></a>

## Struct `TerminateEvent`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_TerminateEvent">TerminateEvent</a> <b>has</b> drop, store
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

<a id="0x1_vesting_without_staking_AdminWithdrawEvent"></a>

## Struct `AdminWithdrawEvent`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminWithdrawEvent">AdminWithdrawEvent</a> <b>has</b> drop, store
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

<a id="0x1_vesting_without_staking_ShareHolderRemovedEvent"></a>

## Struct `ShareHolderRemovedEvent`



<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_ShareHolderRemovedEvent">ShareHolderRemovedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>shareholder: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>beneficiary: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;</code>
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


<a id="0x1_vesting_without_staking_EBALANCE_MISMATCH"></a>

Balance is the same in the contract and the shareholders' left amount.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EBALANCE_MISMATCH">EBALANCE_MISMATCH</a>: u64 = 17;
</code></pre>



<a id="0x1_vesting_without_staking_EEMPTY_VESTING_SCHEDULE"></a>

Vesting schedule cannot be empty.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>: u64 = 2;
</code></pre>



<a id="0x1_vesting_without_staking_EINVALID_WITHDRAWAL_ADDRESS"></a>

Withdrawal address is invalid.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EINVALID_WITHDRAWAL_ADDRESS">EINVALID_WITHDRAWAL_ADDRESS</a>: u64 = 1;
</code></pre>



<a id="0x1_vesting_without_staking_ENOT_ADMIN"></a>

The signer is not the admin of the vesting contract.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_ENOT_ADMIN">ENOT_ADMIN</a>: u64 = 7;
</code></pre>



<a id="0x1_vesting_without_staking_ENO_SHAREHOLDERS"></a>

Shareholders list cannot be empty.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>: u64 = 4;
</code></pre>



<a id="0x1_vesting_without_staking_EPERMISSION_DENIED"></a>

Account is not admin or does not have the required role to take this action.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EPERMISSION_DENIED">EPERMISSION_DENIED</a>: u64 = 15;
</code></pre>



<a id="0x1_vesting_without_staking_EROLE_NOT_FOUND"></a>

The vesting account has no such management role.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EROLE_NOT_FOUND">EROLE_NOT_FOUND</a>: u64 = 14;
</code></pre>



<a id="0x1_vesting_without_staking_ESHAREHOLDER_NOT_EXIST"></a>

Shareholder address is not exist


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_ESHAREHOLDER_NOT_EXIST">ESHAREHOLDER_NOT_EXIST</a>: u64 = 18;
</code></pre>



<a id="0x1_vesting_without_staking_ESHARES_LENGTH_MISMATCH"></a>

The length of shareholders and shares lists don't match.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_ESHARES_LENGTH_MISMATCH">ESHARES_LENGTH_MISMATCH</a>: u64 = 5;
</code></pre>



<a id="0x1_vesting_without_staking_EVEC_EMPTY_FOR_MANY_FUNCTION"></a>

Zero items were provided to a *_many function.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EVEC_EMPTY_FOR_MANY_FUNCTION">EVEC_EMPTY_FOR_MANY_FUNCTION</a>: u64 = 16;
</code></pre>



<a id="0x1_vesting_without_staking_EVESTING_ACCOUNT_HAS_NO_ROLES"></a>

Vesting account has no other management roles beside admin.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_ACCOUNT_HAS_NO_ROLES">EVESTING_ACCOUNT_HAS_NO_ROLES</a>: u64 = 13;
</code></pre>



<a id="0x1_vesting_without_staking_EVESTING_CONTRACT_NOT_ACTIVE"></a>

Vesting contract needs to be in active state.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_CONTRACT_NOT_ACTIVE">EVESTING_CONTRACT_NOT_ACTIVE</a>: u64 = 8;
</code></pre>



<a id="0x1_vesting_without_staking_EVESTING_CONTRACT_NOT_FOUND"></a>

No vesting contract found at provided address.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_CONTRACT_NOT_FOUND">EVESTING_CONTRACT_NOT_FOUND</a>: u64 = 10;
</code></pre>



<a id="0x1_vesting_without_staking_EVESTING_CONTRACT_STILL_ACTIVE"></a>

Admin can only withdraw from an inactive (paused or terminated) vesting contract.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_CONTRACT_STILL_ACTIVE">EVESTING_CONTRACT_STILL_ACTIVE</a>: u64 = 9;
</code></pre>



<a id="0x1_vesting_without_staking_EVESTING_START_TOO_SOON"></a>

Deprecated.

Vesting cannot start before or at the current block timestamp. Has to be in the future.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_START_TOO_SOON">EVESTING_START_TOO_SOON</a>: u64 = 6;
</code></pre>



<a id="0x1_vesting_without_staking_EZERO_GRANT"></a>

Grant amount cannot be 0.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EZERO_GRANT">EZERO_GRANT</a>: u64 = 12;
</code></pre>



<a id="0x1_vesting_without_staking_EZERO_VESTING_SCHEDULE_PERIOD"></a>

Vesting period cannot be 0.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_EZERO_VESTING_SCHEDULE_PERIOD">EZERO_VESTING_SCHEDULE_PERIOD</a>: u64 = 3;
</code></pre>



<a id="0x1_vesting_without_staking_ROLE_BENEFICIARY_RESETTER"></a>

Roles that can manage certain aspects of the vesting account beyond the main admin.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [82, 79, 76, 69, 95, 66, 69, 78, 69, 70, 73, 67, 73, 65, 82, 89, 95, 82, 69, 83, 69, 84, 84, 69, 82];
</code></pre>



<a id="0x1_vesting_without_staking_VESTING_POOL_ACTIVE"></a>

Vesting contract states.
Vesting contract is active and distributions can be made.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>: u64 = 1;
</code></pre>



<a id="0x1_vesting_without_staking_VESTING_POOL_SALT"></a>



<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_SALT">VESTING_POOL_SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [115, 117, 112, 114, 97, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 118, 101, 115, 116, 105, 110, 103];
</code></pre>



<a id="0x1_vesting_without_staking_VESTING_POOL_TERMINATED"></a>

Vesting contract has been terminated and all funds have been released back to the withdrawal address.


<pre><code><b>const</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>: u64 = 2;
</code></pre>



<a id="0x1_vesting_without_staking_vesting_start_secs"></a>

## Function `vesting_start_secs`

Return the vesting start timestamp (in seconds) of the vesting contract.
Vesting will start at this time, and once a full period has passed, the first vest will become unlocked.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule.start_timestamp_secs
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_period_duration_secs"></a>

## Function `period_duration_secs`

Return the duration of one vesting period (in seconds).
Each vest is released after one full period has started, starting from the specified start_timestamp_secs.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64 <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule.period_duration
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_get_withdrawal_addr"></a>

## Function `get_withdrawal_addr`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_withdrawal_addr">get_withdrawal_addr</a>(vesting_contract_addr: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_withdrawal_addr">get_withdrawal_addr</a>(vesting_contract_addr: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_addr).withdrawal_address
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_get_contract_admin"></a>

## Function `get_contract_admin`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_contract_admin">get_contract_admin</a>(vesting_contract_addr: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_contract_admin">get_contract_admin</a>(vesting_contract_addr: <b>address</b>): <b>address</b> <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_addr).admin
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_get_vesting_record"></a>

## Function `get_vesting_record`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_record">get_vesting_record</a>(vesting_contract_address: <b>address</b>, shareholder_address: <b>address</b>): (u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_record">get_vesting_record</a>(
    vesting_contract_address: <b>address</b>, shareholder_address: <b>address</b>
): (u64, u64, u64) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>let</b> vesting_record =
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(
            &<b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address).shareholders,
            &shareholder_address
        );
    (
        vesting_record.init_amount,
        vesting_record.left_amount,
        vesting_record.last_vested_period
    )
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_remaining_grant"></a>

## Function `remaining_grant`

Return the remaining grant of shareholder


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>, shareholder_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_remaining_grant">remaining_grant</a>(
    vesting_contract_address: <b>address</b>, shareholder_address: <b>address</b>
): u64 <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(
        &<b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address).shareholders,
        &shareholder_address
    ).left_amount
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_beneficiary"></a>

## Function `beneficiary`

Return the beneficiary account of the specified shareholder in a vesting contract.
This is the same as the shareholder address by default and only different if it's been explicitly set.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_beneficiary">beneficiary</a>(
    vesting_contract_address: <b>address</b>, shareholder: <b>address</b>
): <b>address</b> <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_beneficiary">get_beneficiary</a>(
        <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address), shareholder
    )
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_vesting_contracts"></a>

## Function `vesting_contracts`

Return all the vesting contracts a given address is an admin of.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;()
    } <b>else</b> {
        <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin).vesting_contracts
    }
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_vesting_schedule"></a>

## Function `vesting_schedule`

Return the vesting contract's vesting schedule. The core schedule is represented as a list of u64-based
fractions, where the rightmmost 32 bits can be divided by 2^32 to get the fraction, and anything else is the
whole number.

For example 3/48, or 0.0625, will be represented as 268435456. The fractional portion would be
268435456 / 2^32 = 0.0625. Since there are fewer than 32 bits, the whole number portion is effectively 0.
So 268435456 = 0.0625.

This errors out if the vesting contract with the provided address doesn't exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_schedule">vesting_schedule</a>(vesting_contract_address: <b>address</b>): <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">vesting_without_staking::VestingSchedule</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_schedule">vesting_schedule</a>(
    vesting_contract_address: <b>address</b>
): <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">VestingSchedule</a> <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(vesting_contract_address);
    <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_shareholders"></a>

## Function `shareholders`

Return the list of all shareholders in the vesting contract.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_shareholders">shareholders</a>(vesting_contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_shareholders">shareholders</a>(
    vesting_contract_address: <b>address</b>
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);

    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
    <b>let</b> shareholders_address = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_keys">simple_map::keys</a>(&vesting_contract.shareholders);
    shareholders_address
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_shareholder"></a>

## Function `shareholder`

Return the shareholder address given the beneficiary address in a given vesting contract. If there are multiple
shareholders with the same beneficiary address, only the first shareholder is returned. If the given beneficiary
address is actually a shareholder address, just return the address back.

This returns 0x0 if no shareholder is found for the given beneficiary / the address is not a shareholder itself.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_shareholder">shareholder</a>(vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_shareholder">shareholder</a>(
    vesting_contract_address: <b>address</b>, shareholder_or_beneficiary: <b>address</b>
): <b>address</b> <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_active_vesting_contract">assert_active_vesting_contract</a>(vesting_contract_address);

    <b>let</b> shareholders = &<a href="vesting_without_staking.md#0x1_vesting_without_staking_shareholders">shareholders</a>(vesting_contract_address);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(shareholders, &shareholder_or_beneficiary)) {
        <b>return</b> shareholder_or_beneficiary
    };
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address);
    <b>let</b> result = @0x0;
    <b>let</b> (sh_vec, ben_vec) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_to_vec_pair">simple_map::to_vec_pair</a>(vesting_contract.beneficiaries);
    <b>let</b> (found, found_index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(
        &ben_vec, &shareholder_or_beneficiary
    );
    <b>if</b> (found) {
        result = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&sh_vec, found_index);
    };
    result
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_create_vesting_schedule"></a>

## Function `create_vesting_schedule`

Create a vesting schedule with the given schedule of distributions, a vesting start time and period duration.


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_schedule">create_vesting_schedule</a>(schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>&gt;, start_timestamp_secs: u64, period_duration: u64): <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">vesting_without_staking::VestingSchedule</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_schedule">create_vesting_schedule</a>(
    schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;FixedPoint32&gt;,
    start_timestamp_secs: u64,
    period_duration: u64
): <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">VestingSchedule</a> {
    <b>let</b> schedule_len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&schedule);
    <b>assert</b>!(schedule_len != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>));
    // If the first <a href="vesting.md#0x1_vesting">vesting</a> fraction is zero, we can replace it <b>with</b> nonzero by increasing start time
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&schedule, 0)) != 0,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>)
    );
    // last <a href="vesting.md#0x1_vesting">vesting</a> fraction must be non zero <b>to</b> ensure that no amount remains unvested forever.
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&schedule, schedule_len - 1))
            != 0,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EEMPTY_VESTING_SCHEDULE">EEMPTY_VESTING_SCHEDULE</a>)
    );
    <b>assert</b>!(
        period_duration != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EZERO_VESTING_SCHEDULE_PERIOD">EZERO_VESTING_SCHEDULE_PERIOD</a>)
    );
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">VestingSchedule</a> {
        schedule,
        start_timestamp_secs,
        period_duration,
        last_vested_period: 0
    }
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_create_vesting_contract_with_amounts"></a>

## Function `create_vesting_contract_with_amounts`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract_with_amounts">create_vesting_contract_with_amounts</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, shareholders: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, shares: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, vesting_numerators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, vesting_denominator: u64, start_timestamp_secs: u64, period_duration: u64, withdrawal_address: <b>address</b>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract_with_amounts">create_vesting_contract_with_amounts</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    shareholders: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    shares: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    vesting_numerators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    vesting_denominator: u64,
    start_timestamp_secs: u64,
    period_duration: u64,
    withdrawal_address: <b>address</b>,
    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a> {
    <b>assert</b>!(
        !<a href="system_addresses.md#0x1_system_addresses_is_reserved_address">system_addresses::is_reserved_address</a>(withdrawal_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EINVALID_WITHDRAWAL_ADDRESS">EINVALID_WITHDRAWAL_ADDRESS</a>)
    );
    assert_account_is_registered_for_supra(withdrawal_address);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&shareholders) != 0,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&shareholders) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&shares),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_ESHARES_LENGTH_MISMATCH">ESHARES_LENGTH_MISMATCH</a>)
    );

    // If this is the first time this admin <a href="account.md#0x1_account">account</a> <b>has</b> created a <a href="vesting.md#0x1_vesting">vesting</a> contract, initialize the admin store.
    <b>let</b> admin_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin);
    <b>if</b> (!<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin_address)) {
        <b>move_to</b>(
            admin,
            <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a> {
                vesting_contracts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;(),
                nonce: 0,
                create_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_CreateVestingContractEvent">CreateVestingContractEvent</a>&gt;(admin)
            }
        );
    };

    // Initialize the <a href="vesting.md#0x1_vesting">vesting</a> contract in a new resource <a href="account.md#0x1_account">account</a>. This allows the same admin <b>to</b> create multiple
    // pools.
    <b>let</b> (contract_signer, contract_signer_cap) =
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract_account">create_vesting_contract_account</a>(admin, contract_creation_seed);
    <b>let</b> contract_signer_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&contract_signer);
    <b>let</b> schedule = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(
        &vesting_numerators,
        |numerator| {
            <b>let</b> <a href="event.md#0x1_event">event</a> =
                <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_rational">fixed_point32::create_from_rational</a>(*numerator, vesting_denominator);
            <a href="event.md#0x1_event">event</a>
        }
    );

    <b>let</b> vesting_schedule =
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_schedule">create_vesting_schedule</a>(schedule, start_timestamp_secs, period_duration);
    <b>let</b> shareholders_map = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">VestingRecord</a>&gt;();
    <b>let</b> grant_amount = 0;
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_reverse">vector::for_each_reverse</a>(
        shares,
        |amount| {
            <b>let</b> shareholder = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> shareholders);
            <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(
                &<b>mut</b> shareholders_map,
                shareholder,
                <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">VestingRecord</a> {
                    init_amount: amount,
                    left_amount: amount,
                    last_vested_period: vesting_schedule.last_vested_period
                }
            );
            grant_amount = grant_amount + amount;
        }
    );
    <b>assert</b>!(grant_amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EZERO_GRANT">EZERO_GRANT</a>));
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(admin, contract_signer_address, grant_amount);

    <b>let</b> admin_store = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin_address);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> admin_store.vesting_contracts, contract_signer_address);
    emit_event(
        &<b>mut</b> admin_store.create_events,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_CreateVestingContractEvent">CreateVestingContractEvent</a> {
            withdrawal_address,
            grant_amount,
            vesting_contract_address: contract_signer_address
        }
    );

    <b>move_to</b>(
        &contract_signer,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
            state: <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>,
            admin: admin_address,
            shareholders: shareholders_map,
            beneficiaries: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, <b>address</b>&gt;(),
            vesting_schedule,
            withdrawal_address,
            signer_cap: contract_signer_cap,
            set_beneficiary_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_SetBeneficiaryEvent">SetBeneficiaryEvent</a>&gt;(
                &contract_signer
            ),
            vest_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestEvent">VestEvent</a>&gt;(&contract_signer),
            terminate_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_TerminateEvent">TerminateEvent</a>&gt;(&contract_signer),
            admin_withdraw_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminWithdrawEvent">AdminWithdrawEvent</a>&gt;(
                &contract_signer
            ),
            shareholder_removed_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_ShareHolderRemovedEvent">ShareHolderRemovedEvent</a>&gt;(
                &contract_signer
            )
        }
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_create_vesting_contract"></a>

## Function `create_vesting_contract`

Create a vesting contract with a given configurations.


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract">create_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, buy_ins: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;&gt;, vesting_schedule: <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">vesting_without_staking::VestingSchedule</a>, withdrawal_address: <b>address</b>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract">create_vesting_contract</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    buy_ins: SimpleMap&lt;<b>address</b>, Coin&lt;SupraCoin&gt;&gt;,
    vesting_schedule: <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">VestingSchedule</a>,
    withdrawal_address: <b>address</b>,
    contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <b>address</b> <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a> {
    <b>assert</b>!(
        !<a href="system_addresses.md#0x1_system_addresses_is_reserved_address">system_addresses::is_reserved_address</a>(withdrawal_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EINVALID_WITHDRAWAL_ADDRESS">EINVALID_WITHDRAWAL_ADDRESS</a>)
    );
    assert_account_is_registered_for_supra(withdrawal_address);
    <b>let</b> shareholders_address = &<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_keys">simple_map::keys</a>(&buy_ins);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shareholders_address) != 0,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>)
    );

    <b>let</b> shareholders = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">VestingRecord</a>&gt;();
    <b>let</b> grant = <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;SupraCoin&gt;();
    <b>let</b> grant_amount = 0;
    <b>let</b> (shareholders_address, buy_ins) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_to_vec_pair">simple_map::to_vec_pair</a>(buy_ins);
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&shareholders_address) != 0) {
        <b>let</b> shareholder = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> shareholders_address);
        <b>let</b> buy_in = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> buy_ins);
        <b>let</b> init = <a href="coin.md#0x1_coin_value">coin::value</a>(&buy_in);
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> grant, buy_in);
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(
            &<b>mut</b> shareholders,
            shareholder,
            <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">VestingRecord</a> {
                init_amount: init,
                left_amount: init,
                last_vested_period: vesting_schedule.last_vested_period
            }
        );
        grant_amount = grant_amount + init;
    };
    <b>assert</b>!(grant_amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EZERO_GRANT">EZERO_GRANT</a>));

    // If this is the first time this admin <a href="account.md#0x1_account">account</a> <b>has</b> created a <a href="vesting.md#0x1_vesting">vesting</a> contract, initialize the admin store.
    <b>let</b> admin_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin);
    <b>if</b> (!<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin_address)) {
        <b>move_to</b>(
            admin,
            <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a> {
                vesting_contracts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;(),
                nonce: 0,
                create_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_CreateVestingContractEvent">CreateVestingContractEvent</a>&gt;(admin)
            }
        );
    };

    // Initialize the <a href="vesting.md#0x1_vesting">vesting</a> contract in a new resource <a href="account.md#0x1_account">account</a>. This allows the same admin <b>to</b> create multiple
    // pools.
    <b>let</b> (contract_signer, contract_signer_cap) =
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract_account">create_vesting_contract_account</a>(admin, contract_creation_seed);
    <b>let</b> contract_signer_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&contract_signer);
    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(contract_signer_address, grant);

    <b>let</b> admin_store = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin_address);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> admin_store.vesting_contracts, contract_signer_address);
    emit_event(
        &<b>mut</b> admin_store.create_events,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_CreateVestingContractEvent">CreateVestingContractEvent</a> {
            withdrawal_address,
            grant_amount,
            vesting_contract_address: contract_signer_address
        }
    );

    <b>move_to</b>(
        &contract_signer,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
            state: <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>,
            admin: admin_address,
            shareholders,
            beneficiaries: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, <b>address</b>&gt;(),
            vesting_schedule,
            withdrawal_address,
            signer_cap: contract_signer_cap,
            set_beneficiary_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_SetBeneficiaryEvent">SetBeneficiaryEvent</a>&gt;(
                &contract_signer
            ),
            vest_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestEvent">VestEvent</a>&gt;(&contract_signer),
            terminate_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_TerminateEvent">TerminateEvent</a>&gt;(&contract_signer),
            admin_withdraw_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminWithdrawEvent">AdminWithdrawEvent</a>&gt;(
                &contract_signer
            ),
            shareholder_removed_events: new_event_handle&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_ShareHolderRemovedEvent">ShareHolderRemovedEvent</a>&gt;(
                &contract_signer
            )
        }
    );

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(buy_ins);
    contract_signer_address
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_vest"></a>

## Function `vest`

Unlock any vested portion of the grant.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest">vest</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest">vest</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    // Short-circuit <b>if</b> <a href="vesting.md#0x1_vesting">vesting</a> hasn't started yet.
    <b>if</b> (vesting_contract.vesting_schedule.start_timestamp_secs
        &gt; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()) { <b>return</b> };

    <b>let</b> shareholders = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_keys">simple_map::keys</a>(&vesting_contract.shareholders);
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&shareholders) != 0) {
        <b>let</b> shareholder = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> shareholders);
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_individual">vest_individual</a>(contract_address, shareholder);
    };
    <b>let</b> total_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(contract_address);
    <b>if</b> (total_balance == 0) {
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_terminate_vesting_contract">set_terminate_vesting_contract</a>(contract_address);
    };
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_vest_individual"></a>

## Function `vest_individual`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_individual">vest_individual</a>(contract_address: <b>address</b>, shareholder_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_individual">vest_individual</a>(
    contract_address: <b>address</b>, shareholder_address: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    //check <b>if</b> contract exist, active and shareholder is a member of the contract
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_shareholder_exists">assert_shareholder_exists</a>(contract_address, shareholder_address);

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> beneficiary = <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_beneficiary">get_beneficiary</a>(vesting_contract, shareholder_address);
    // Short-circuit <b>if</b> <a href="vesting.md#0x1_vesting">vesting</a> hasn't started yet.
    <b>if</b> (vesting_contract.vesting_schedule.start_timestamp_secs
        &gt; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()) { <b>return</b> };

    <b>let</b> vesting_record =
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(
            &<b>mut</b> vesting_contract.shareholders, &shareholder_address
        );
    <b>let</b> signer_cap = &vesting_contract.signer_cap;

    // Check <b>if</b> the next vested period <b>has</b> already passed. If not, short-circuit since there's nothing <b>to</b> vest.
    <b>let</b> vesting_schedule = vesting_contract.vesting_schedule;
    <b>let</b> schedule = &vesting_schedule.schedule;
    <b>let</b> last_vested_period = vesting_record.last_vested_period;
    <b>let</b> next_period_to_vest = last_vested_period + 1;
    <b>let</b> last_completed_period =
        (<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() - vesting_schedule.start_timestamp_secs)
            / vesting_schedule.period_duration;

    // Index is 0-based <b>while</b> period is 1-based so we need <b>to</b> subtract 1.

    <b>while</b> (last_completed_period &gt;= next_period_to_vest && vesting_record.left_amount != 0 && next_period_to_vest &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(schedule)) {
        <b>let</b> schedule_index = next_period_to_vest - 1;
        <b>let</b> vesting_fraction = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(schedule, schedule_index);
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_transfer">vest_transfer</a>(vesting_record, signer_cap, beneficiary, vesting_fraction);
        emit_event(&<b>mut</b> vesting_contract.vest_events,
            <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestEvent">VestEvent</a> {
                admin: vesting_contract.admin,
                shareholder_address,
                vesting_contract_address: contract_address,
                period_vested: next_period_to_vest
            }
        );
        next_period_to_vest = next_period_to_vest + 1;
    };

    <b>if</b> (last_completed_period &gt;= next_period_to_vest && vesting_record.left_amount != 0) {
        <b>let</b> final_fraction = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(schedule, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(schedule) - 1);
        <b>let</b> final_fraction_amount = <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_multiply_u64">fixed_point32::multiply_u64</a>(vesting_record.init_amount, final_fraction);
        // Determine how many periods is needed based on the left_amount
        <b>let</b> added_fraction = <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_multiply_u64_return_fixpoint32">fixed_point32::multiply_u64_return_fixpoint32</a>(last_completed_period - next_period_to_vest + 1, final_fraction);
        // If the added_fraction is greater than or equal <b>to</b> the left_amount, then we can vest all the left_amount
        <b>let</b> periods_need =
            <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_multiply_u64">fixed_point32::multiply_u64</a>(vesting_record.init_amount, added_fraction) &gt;= vesting_record.left_amount){
            <b>let</b> result =  vesting_record.left_amount / final_fraction_amount;
                // check <b>if</b> `left_amount` is perfectly divisible by `final_fraction_amount`
                  <b>if</b> (vesting_record.left_amount == final_fraction_amount*result) {
                   result
                } <b>else</b> {
                   result + 1
                }
        } <b>else</b> {
            last_completed_period - next_period_to_vest + 1
        };

        <b>let</b> total_fraction = <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_multiply_u64_return_fixpoint32">fixed_point32::multiply_u64_return_fixpoint32</a>(periods_need, final_fraction);
        // We don't need <b>to</b> check vesting_record.left_amount &gt; 0 because vest_transfer will handle that.
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_transfer">vest_transfer</a>(vesting_record, signer_cap, beneficiary, total_fraction);
        next_period_to_vest = next_period_to_vest + periods_need;
        emit_event(&<b>mut</b> vesting_contract.vest_events,
            <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestEvent">VestEvent</a> {
                admin: vesting_contract.admin,
                shareholder_address,
                vesting_contract_address: contract_address,
                period_vested: next_period_to_vest,
            },
        );
    };

    //<b>update</b> last_vested_period for the shareholder
    vesting_record.last_vested_period = next_period_to_vest - 1;
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_vest_transfer"></a>

## Function `vest_transfer`



<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_transfer">vest_transfer</a>(vesting_record: &<b>mut</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">vesting_without_staking::VestingRecord</a>, signer_cap: &<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, beneficiary: <b>address</b>, vesting_fraction: <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_transfer">vest_transfer</a>(
    vesting_record: &<b>mut</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">VestingRecord</a>,
    signer_cap: &SignerCapability,
    beneficiary: <b>address</b>,
    vesting_fraction: FixedPoint32
) {
    <b>let</b> vesting_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(signer_cap);

    //amount <b>to</b> be transfer is minimum of what is left and <a href="vesting.md#0x1_vesting">vesting</a> fraction due of init_amount
    <b>let</b> amount =
        <b>min</b>(
            vesting_record.left_amount,
            <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_multiply_u64">fixed_point32::multiply_u64</a>(
                vesting_record.init_amount, vesting_fraction
            )
        );
    //<b>update</b> left_amount for the shareholder
    vesting_record.left_amount = vesting_record.left_amount - amount;
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(&vesting_signer, beneficiary, amount);
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_remove_shareholder"></a>

## Function `remove_shareholder`

Remove the lockup period for the vesting contract. This can only be called by the admin of the vesting contract.
Example usage: If admin find shareholder suspicious, admin can remove it.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_remove_shareholder">remove_shareholder</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_remove_shareholder">remove_shareholder</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder_address: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_shareholder_exists">assert_shareholder_exists</a>(contract_address, shareholder_address);
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> vesting_signer = <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <b>let</b> shareholder_amount =
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&vesting_contract.shareholders, &shareholder_address).left_amount;
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(
        &vesting_signer, vesting_contract.withdrawal_address, shareholder_amount
    );
    emit_event(
        &<b>mut</b> vesting_contract.admin_withdraw_events,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminWithdrawEvent">AdminWithdrawEvent</a> {
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            amount: shareholder_amount
        }
    );

    // remove `shareholder_address`` from `vesting_contract.shareholders`
    <b>let</b> shareholders = &<b>mut</b> vesting_contract.shareholders;
    <b>let</b> (_, shareholders_vesting) =
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(shareholders, &shareholder_address);

    // remove `shareholder_address` from `vesting_contract.beneficiaries`
    <b>let</b> beneficiary = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    <b>let</b> shareholder_beneficiaries = &<b>mut</b> vesting_contract.beneficiaries;
    // Not all shareholders have their beneficiaries, so before removing them, we need <b>to</b> check <b>if</b> the beneficiary <b>exists</b>
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(shareholder_beneficiaries, &shareholder_address)) {
        <b>let</b> (_, shareholder_baneficiary) =
            <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(shareholder_beneficiaries, &shareholder_address);
        beneficiary = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(shareholder_baneficiary);
    };

    // Emit <a href="vesting_without_staking.md#0x1_vesting_without_staking_ShareHolderRemovedEvent">ShareHolderRemovedEvent</a>
    emit_event(
        &<b>mut</b> vesting_contract.shareholder_removed_events,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_ShareHolderRemovedEvent">ShareHolderRemovedEvent</a> {
            shareholder: shareholder_address,
            beneficiary,
            amount: shareholders_vesting.left_amount
        }
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_terminate_vesting_contract"></a>

## Function `terminate_vesting_contract`

Terminate the vesting contract and send all funds back to the withdrawal address.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_terminate_vesting_contract">terminate_vesting_contract</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_terminate_vesting_contract">terminate_vesting_contract</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);

    <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest">vest</a>(contract_address);

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin, vesting_contract);

    // Distribute remaining coins <b>to</b> withdrawal <b>address</b> of <a href="vesting.md#0x1_vesting">vesting</a> contract.
    <b>let</b> shareholders_address = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_keys">simple_map::keys</a>(&vesting_contract.shareholders);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(
        &shareholders_address,
        |shareholder| {
            <b>let</b> shareholder_amount =
                <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(
                    &<b>mut</b> vesting_contract.shareholders, shareholder
                );
            shareholder_amount.left_amount = 0;
        }
    );
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_terminate_vesting_contract">set_terminate_vesting_contract</a>(contract_address);
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_admin_withdraw"></a>

## Function `admin_withdraw`

Withdraw all funds to the preset vesting contract's withdrawal address. This can only be called if the contract
has already been terminated.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_admin_withdraw">admin_withdraw</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_admin_withdraw">admin_withdraw</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>assert</b>!(
        vesting_contract.state == <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_CONTRACT_STILL_ACTIVE">EVESTING_CONTRACT_STILL_ACTIVE</a>)
    );

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin, vesting_contract);
    <b>let</b> total_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(contract_address);
    <b>let</b> vesting_signer = <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(
        &vesting_signer, vesting_contract.withdrawal_address, total_balance
    );

    emit_event(
        &<b>mut</b> vesting_contract.admin_withdraw_events,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminWithdrawEvent">AdminWithdrawEvent</a> {
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            amount: total_balance
        }
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_set_beneficiary"></a>

## Function `set_beneficiary`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_beneficiary">set_beneficiary</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>, new_beneficiary: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_beneficiary">set_beneficiary</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    shareholder: <b>address</b>,
    new_beneficiary: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    // Verify that the beneficiary <a href="account.md#0x1_account">account</a> is set up <b>to</b> receive SUPRA. This is a requirement so distribute() wouldn't
    // fail and <a href="block.md#0x1_block">block</a> all other accounts from receiving SUPRA <b>if</b> one beneficiary is not registered.
    assert_account_is_registered_for_supra(new_beneficiary);

    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin, vesting_contract);

    <b>let</b> old_beneficiary = <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_beneficiary">get_beneficiary</a>(vesting_contract, shareholder);
    <b>let</b> beneficiaries = &<b>mut</b> vesting_contract.beneficiaries;
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(beneficiaries, shareholder, new_beneficiary);

    emit_event(
        &<b>mut</b> vesting_contract.set_beneficiary_events,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_SetBeneficiaryEvent">SetBeneficiaryEvent</a> {
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address,
            shareholder,
            old_beneficiary,
            new_beneficiary
        }
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_reset_beneficiary"></a>

## Function `reset_beneficiary`

Remove the beneficiary for the given shareholder. All distributions will sent directly to the shareholder
account.


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_reset_beneficiary">reset_beneficiary</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_reset_beneficiary">reset_beneficiary</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    shareholder: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(
        addr == vesting_contract.admin
            || addr
                == <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_role_holder">get_role_holder</a>(
                    contract_address, utf8(<a href="vesting_without_staking.md#0x1_vesting_without_staking_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>)
                ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EPERMISSION_DENIED">EPERMISSION_DENIED</a>)
    );

    <b>let</b> beneficiaries = &<b>mut</b> vesting_contract.beneficiaries;
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(beneficiaries, &shareholder)) {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(beneficiaries, &shareholder);
    };
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_set_management_role"></a>

## Function `set_management_role`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_management_role">set_management_role</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, role_holder: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_management_role">set_management_role</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    role: String,
    role_holder: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin, vesting_contract);

    <b>if</b> (!<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address)) {
        <b>let</b> contract_signer = &<a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract);
        <b>move_to</b>(
            contract_signer,
            <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a> {
                roles: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <b>address</b>&gt;()
            }
        )
    };
    <b>let</b> roles =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(roles, role, role_holder);
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_set_beneficiary_resetter"></a>

## Function `set_beneficiary_resetter`



<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_beneficiary_resetter">set_beneficiary_resetter</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, beneficiary_resetter: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_beneficiary_resetter">set_beneficiary_resetter</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    contract_address: <b>address</b>,
    beneficiary_resetter: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a>, <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_management_role">set_management_role</a>(
        admin,
        contract_address,
        utf8(<a href="vesting_without_staking.md#0x1_vesting_without_staking_ROLE_BENEFICIARY_RESETTER">ROLE_BENEFICIARY_RESETTER</a>),
        beneficiary_resetter
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_get_role_holder"></a>

## Function `get_role_holder`



<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_role_holder">get_role_holder</a>(contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_role_holder">get_role_holder</a>(
    contract_address: <b>address</b>, role: String
): <b>address</b> <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_ACCOUNT_HAS_NO_ROLES">EVESTING_ACCOUNT_HAS_NO_ROLES</a>)
    );
    <b>let</b> roles = &<b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingAccountManagement">VestingAccountManagement</a>&gt;(contract_address).roles;
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(roles, &role), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EROLE_NOT_FOUND">EROLE_NOT_FOUND</a>)
    );
    *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(roles, &role)
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_get_vesting_account_signer"></a>

## Function `get_vesting_account_signer`

For emergency use in case the admin needs emergency control of vesting contract account.


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer">get_vesting_account_signer</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin, vesting_contract);
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract)
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_get_vesting_account_signer_internal"></a>

## Function `get_vesting_account_signer_internal`



<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">vesting_without_staking::VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(
    vesting_contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&vesting_contract.signer_cap)
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_create_vesting_contract_account"></a>

## Function `create_vesting_contract_account`

Create a salt for generating the resource accounts that will be holding the VestingContract.
This address should be deterministic for the same admin and vesting contract creation nonce.


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract_account">create_vesting_contract_account</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract_account">create_vesting_contract_account</a>(
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, SignerCapability) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a> {
    <b>let</b> admin_store = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin));
    <b>let</b> seed = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&admin_store.nonce));
    admin_store.nonce = admin_store.nonce + 1;

    // Include a salt <b>to</b> avoid conflicts <b>with</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> other modules out there that might also generate
    // deterministic resource accounts for the same admin <b>address</b> + nonce.
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_SALT">VESTING_POOL_SALT</a>);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, contract_creation_seed);

    <b>let</b> (account_signer, signer_cap) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(admin, seed);
    // Register the <a href="vesting.md#0x1_vesting">vesting</a> contract <a href="account.md#0x1_account">account</a> <b>to</b> receive SUPRA
    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;SupraCoin&gt;(&account_signer);

    (account_signer, signer_cap)
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_verify_admin"></a>

## Function `verify_admin`



<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">vesting_without_staking::VestingContract</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>) {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) == vesting_contract.admin,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_ENOT_ADMIN">ENOT_ADMIN</a>)
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_assert_vesting_contract_exists"></a>

## Function `assert_vesting_contract_exists`



<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>) {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_CONTRACT_NOT_FOUND">EVESTING_CONTRACT_NOT_FOUND</a>)
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_assert_shareholder_exists"></a>

## Function `assert_shareholder_exists`



<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_shareholder_exists">assert_shareholder_exists</a>(contract_address: <b>address</b>, shareholder_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_shareholder_exists">assert_shareholder_exists</a>(
    contract_address: <b>address</b>, shareholder_address: <b>address</b>
) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(
            &<b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address).shareholders,
            &shareholder_address
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_ESHAREHOLDER_NOT_EXIST">ESHAREHOLDER_NOT_EXIST</a>)
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_assert_active_vesting_contract"></a>

## Function `assert_active_vesting_contract`



<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address);
    <b>let</b> vesting_contract = <b>borrow_global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>assert</b>!(
        vesting_contract.state == <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="vesting_without_staking.md#0x1_vesting_without_staking_EVESTING_CONTRACT_NOT_ACTIVE">EVESTING_CONTRACT_NOT_ACTIVE</a>)
    );
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_get_beneficiary"></a>

## Function `get_beneficiary`



<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_beneficiary">get_beneficiary</a>(contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">vesting_without_staking::VestingContract</a>, shareholder: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_beneficiary">get_beneficiary</a>(contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>, shareholder: <b>address</b>): <b>address</b> {
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&contract.beneficiaries, &shareholder)) {
        *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&contract.beneficiaries, &shareholder)
    } <b>else</b> {
        shareholder
    }
}
</code></pre>



</details>

<a id="0x1_vesting_without_staking_set_terminate_vesting_contract"></a>

## Function `set_terminate_vesting_contract`



<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_terminate_vesting_contract">set_terminate_vesting_contract</a>(contract_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_terminate_vesting_contract">set_terminate_vesting_contract</a>(contract_address: <b>address</b>) <b>acquires</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a> {
    <b>let</b> vesting_contract = <b>borrow_global_mut</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    vesting_contract.state = <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>;
    emit_event(
        &<b>mut</b> vesting_contract.terminate_events,
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_TerminateEvent">TerminateEvent</a> {
            admin: vesting_contract.admin,
            vesting_contract_address: contract_address
        }
    );
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_VestingRecord"></a>

### Struct `VestingRecord`


<pre><code><b>struct</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">VestingRecord</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<dl>
<dt>
<code>init_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>left_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>last_vested_period: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> init_amount &gt;= left_amount;
</code></pre>



<a id="@Specification_1_vesting_start_secs"></a>

### Function `vesting_start_secs`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_start_secs">vesting_start_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractExists">VestingContractExists</a>{contract_address: vesting_contract_address};
</code></pre>



<a id="@Specification_1_period_duration_secs"></a>

### Function `period_duration_secs`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_period_duration_secs">period_duration_secs</a>(vesting_contract_address: <b>address</b>): u64
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractExists">VestingContractExists</a>{contract_address: vesting_contract_address};
</code></pre>



<a id="@Specification_1_remaining_grant"></a>

### Function `remaining_grant`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_remaining_grant">remaining_grant</a>(vesting_contract_address: <b>address</b>, shareholder_address: <b>address</b>): u64
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractExists">VestingContractExists</a>{contract_address: vesting_contract_address};
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address).shareholders, shareholder_address);
<b>ensures</b> result == <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(<b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address).shareholders, shareholder_address).left_amount;
</code></pre>



<a id="@Specification_1_beneficiary"></a>

### Function `beneficiary`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_beneficiary">beneficiary</a>(vesting_contract_address: <b>address</b>, shareholder: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractExists">VestingContractExists</a>{contract_address: vesting_contract_address};
</code></pre>



<a id="@Specification_1_vesting_contracts"></a>

### Function `vesting_contracts`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_contracts">vesting_contracts</a>(admin: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> !<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin) ==&gt; result == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;();
<b>ensures</b> <b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin) ==&gt; result == <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(admin).vesting_contracts;
</code></pre>



<a id="@Specification_1_vesting_schedule"></a>

### Function `vesting_schedule`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vesting_schedule">vesting_schedule</a>(vesting_contract_address: <b>address</b>): <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">vesting_without_staking::VestingSchedule</a>
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractExists">VestingContractExists</a>{contract_address: vesting_contract_address};
<b>ensures</b> result == <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(vesting_contract_address).vesting_schedule;
</code></pre>



<a id="@Specification_1_create_vesting_schedule"></a>

### Function `create_vesting_schedule`


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_schedule">create_vesting_schedule</a>(schedule: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>&gt;, start_timestamp_secs: u64, period_duration: u64): <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingSchedule">vesting_without_staking::VestingSchedule</a>
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(schedule) == 0;
<b>aborts_if</b> period_duration &lt;= 0;
<b>aborts_if</b> start_timestamp_secs &lt; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();
</code></pre>



<a id="@Specification_1_vest"></a>

### Function `vest`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest">vest</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractActive">VestingContractActive</a>;
<b>let</b> vesting_contract_pre = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> <b>post</b> vesting_contract_post = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> vesting_schedule = vesting_contract_pre.vesting_schedule;
<b>let</b> last_vested_period = vesting_schedule.last_vested_period;
<b>let</b> next_period_to_vest = last_vested_period + 1;
<b>let</b> last_completed_period =
    (<a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() - vesting_schedule.start_timestamp_secs) / vesting_schedule.period_duration;
<b>ensures</b> vesting_contract_pre.vesting_schedule.start_timestamp_secs &gt; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() ==&gt; vesting_contract_pre == vesting_contract_post;
<b>ensures</b> last_completed_period &lt; next_period_to_vest ==&gt; vesting_contract_pre == vesting_contract_post;
</code></pre>



<a id="@Specification_1_vest_individual"></a>

### Function `vest_individual`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_individual">vest_individual</a>(contract_address: <b>address</b>, shareholder_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractActive">VestingContractActive</a>;
<b>let</b> vesting_contract_pre = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> <b>post</b> vesting_contract_post = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>ensures</b> vesting_contract_pre.vesting_schedule.start_timestamp_secs &gt; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() ==&gt; vesting_contract_pre == vesting_contract_post;
</code></pre>



<a id="@Specification_1_vest_transfer"></a>

### Function `vest_transfer`


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_vest_transfer">vest_transfer</a>(vesting_record: &<b>mut</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingRecord">vesting_without_staking::VestingRecord</a>, signer_cap: &<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, beneficiary: <b>address</b>, vesting_fraction: <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>let</b> amount = <b>min</b>(vesting_record.left_amount, <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_spec_multiply_u64">fixed_point32::spec_multiply_u64</a>(vesting_record.init_amount, vesting_fraction));
<b>ensures</b> vesting_record.left_amount == <b>old</b>(vesting_record.left_amount) - amount;
<b>let</b> address_from = signer_cap.<a href="account.md#0x1_account">account</a>;
<b>ensures</b> beneficiary != address_from ==&gt;
    (<a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(beneficiary) == <b>old</b>(<a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(beneficiary)) + amount
    && <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(address_from) == <b>old</b>(<a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(address_from)) - amount);
</code></pre>



<a id="@Specification_1_remove_shareholder"></a>

### Function `remove_shareholder`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_remove_shareholder">remove_shareholder</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminAborts">AdminAborts</a>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> <b>post</b> vesting_contract_post = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> balance_pre = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(vesting_contract.withdrawal_address);
<b>let</b> <b>post</b> balance_post = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(vesting_contract_post.withdrawal_address);
<b>let</b> shareholder_amount = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(vesting_contract.shareholders, shareholder_address).left_amount;
<b>ensures</b> vesting_contract_post.withdrawal_address != vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a> ==&gt; balance_post == balance_pre + shareholder_amount;
<b>ensures</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(vesting_contract_post.shareholders, shareholder_address);
<b>ensures</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(vesting_contract_post.beneficiaries, shareholder_address);
</code></pre>



<a id="@Specification_1_admin_withdraw"></a>

### Function `admin_withdraw`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_admin_withdraw">admin_withdraw</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> balance_pre = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(vesting_contract.withdrawal_address);
<b>let</b> <b>post</b> balance_post = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(vesting_contract.withdrawal_address);
<b>let</b> <b>post</b> balance_contract = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(contract_address);
<b>aborts_if</b> !(<b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address).state == <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>);
</code></pre>



<a id="@Specification_1_set_beneficiary"></a>

### Function `set_beneficiary`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_beneficiary">set_beneficiary</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>, new_beneficiary: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>let</b> vesting_contract_pre = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> <b>post</b> vesting_contract_post = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminAborts">AdminAborts</a>{vesting_contract: vesting_contract_pre};
<b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(vesting_contract_post.beneficiaries, shareholder) == new_beneficiary;
</code></pre>



<a id="@Specification_1_reset_beneficiary"></a>

### Function `reset_beneficiary`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_reset_beneficiary">reset_beneficiary</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, shareholder: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>let</b> <b>post</b> vesting_contract = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>ensures</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(vesting_contract.beneficiaries, shareholder);
</code></pre>



<a id="@Specification_1_set_management_role"></a>

### Function `set_management_role`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_management_role">set_management_role</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, role_holder: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
</code></pre>



<a id="@Specification_1_set_beneficiary_resetter"></a>

### Function `set_beneficiary_resetter`


<pre><code><b>public</b> entry <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_beneficiary_resetter">set_beneficiary_resetter</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>, beneficiary_resetter: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
</code></pre>



<a id="@Specification_1_get_role_holder"></a>

### Function `get_role_holder`


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_role_holder">get_role_holder</a>(contract_address: <b>address</b>, role: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
</code></pre>



<a id="@Specification_1_get_vesting_account_signer"></a>

### Function `get_vesting_account_signer`


<pre><code><b>public</b> <b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer">get_vesting_account_signer</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminAborts">AdminAborts</a>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
</code></pre>



<a id="@Specification_1_get_vesting_account_signer_internal"></a>

### Function `get_vesting_account_signer_internal`


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_vesting_account_signer_internal">get_vesting_account_signer_internal</a>(vesting_contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">vesting_without_staking::VestingContract</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>let</b> <b>address</b> = vesting_contract.signer_cap.<a href="account.md#0x1_account">account</a>;
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) == <b>address</b>;
</code></pre>



<a id="@Specification_1_create_vesting_contract_account"></a>

### Function `create_vesting_contract_account`


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract_account">create_vesting_contract_account</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, contract_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminStore">AdminStore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin));
</code></pre>



<a id="@Specification_1_verify_admin"></a>

### Function `verify_admin`


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_verify_admin">verify_admin</a>(admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vesting_contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">vesting_without_staking::VestingContract</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminAborts">AdminAborts</a>;
</code></pre>




<a id="0x1_vesting_without_staking_AdminAborts"></a>


<pre><code><b>schema</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_AdminAborts">AdminAborts</a> {
    admin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    vesting_contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>;
    <b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(admin) != vesting_contract.admin;
}
</code></pre>



<a id="@Specification_1_assert_vesting_contract_exists"></a>

### Function `assert_vesting_contract_exists`


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_vesting_contract_exists">assert_vesting_contract_exists</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractExists">VestingContractExists</a>;
</code></pre>




<a id="0x1_vesting_without_staking_VestingContractExists"></a>


<pre><code><b>schema</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractExists">VestingContractExists</a> {
    contract_address: <b>address</b>;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
}
</code></pre>



<a id="@Specification_1_assert_active_vesting_contract"></a>

### Function `assert_active_vesting_contract`


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_assert_active_vesting_contract">assert_active_vesting_contract</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractActive">VestingContractActive</a>;
</code></pre>




<a id="0x1_vesting_without_staking_VestingContractActive"></a>


<pre><code><b>schema</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractActive">VestingContractActive</a> {
    <b>include</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContractExists">VestingContractExists</a>;
    contract_address: <b>address</b>;
    <b>let</b> vesting_contract = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
    <b>aborts_if</b> !(vesting_contract.state == <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_ACTIVE">VESTING_POOL_ACTIVE</a>);
}
</code></pre>



<a id="@Specification_1_get_beneficiary"></a>

### Function `get_beneficiary`


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_get_beneficiary">get_beneficiary</a>(contract: &<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">vesting_without_staking::VestingContract</a>, shareholder: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(contract.beneficiaries, shareholder) ==&gt; result == <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(contract.beneficiaries, shareholder);
<b>ensures</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(contract.beneficiaries, shareholder) ==&gt; result == shareholder;
</code></pre>



<a id="@Specification_1_set_terminate_vesting_contract"></a>

### Function `set_terminate_vesting_contract`


<pre><code><b>fun</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking_set_terminate_vesting_contract">set_terminate_vesting_contract</a>(contract_address: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>let</b> <b>post</b> vesting_contract_post = <b>global</b>&lt;<a href="vesting_without_staking.md#0x1_vesting_without_staking_VestingContract">VestingContract</a>&gt;(contract_address);
<b>ensures</b> vesting_contract_post.state == <a href="vesting_without_staking.md#0x1_vesting_without_staking_VESTING_POOL_TERMINATED">VESTING_POOL_TERMINATED</a>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
