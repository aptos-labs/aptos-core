
<a name="0x1_ExperimentalAccount"></a>

# Module `0x1::ExperimentalAccount`

The <code><a href="ExperimentalAccount.md#0x1_ExperimentalAccount">ExperimentalAccount</a></code> module manages experimental accounts.
It also defines the prolog and epilog that run before and after every
transaction in addition to the core prologue and epilogue.


-  [Resource `DiemWriteSetManager`](#0x1_ExperimentalAccount_DiemWriteSetManager)
-  [Struct `AdminTransactionEvent`](#0x1_ExperimentalAccount_AdminTransactionEvent)
-  [Struct `ExperimentalAccountMarker`](#0x1_ExperimentalAccount_ExperimentalAccountMarker)
-  [Constants](#@Constants_0)
-  [Function `create_core_account`](#0x1_ExperimentalAccount_create_core_account)
-  [Function `initialize`](#0x1_ExperimentalAccount_initialize)
-  [Function `create_account`](#0x1_ExperimentalAccount_create_account)
-  [Function `exists_at`](#0x1_ExperimentalAccount_exists_at)
-  [Function `create_diem_root_account`](#0x1_ExperimentalAccount_create_diem_root_account)
-  [Function `create_validator_account`](#0x1_ExperimentalAccount_create_validator_account)
-  [Function `create_validator_operator_account`](#0x1_ExperimentalAccount_create_validator_operator_account)
-  [Function `rotate_authentication_key`](#0x1_ExperimentalAccount_rotate_authentication_key)
-  [Function `module_prologue`](#0x1_ExperimentalAccount_module_prologue)
-  [Function `script_prologue`](#0x1_ExperimentalAccount_script_prologue)
-  [Function `writeset_prologue`](#0x1_ExperimentalAccount_writeset_prologue)
-  [Function `multi_agent_script_prologue`](#0x1_ExperimentalAccount_multi_agent_script_prologue)
-  [Function `epilogue`](#0x1_ExperimentalAccount_epilogue)
-  [Function `writeset_epilogue`](#0x1_ExperimentalAccount_writeset_epilogue)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account">0x1::Account</a>;
<b>use</b> <a href="DiemConfig.md#0x1_DiemConfig">0x1::DiemConfig</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp">0x1::DiemTimestamp</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event">0x1::Event</a>;
<b>use</b> <a href="Roles.md#0x1_Roles">0x1::Roles</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
<b>use</b> <a href="ValidatorConfig.md#0x1_ValidatorConfig">0x1::ValidatorConfig</a>;
<b>use</b> <a href="ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig">0x1::ValidatorOperatorConfig</a>;
</code></pre>



<a name="0x1_ExperimentalAccount_DiemWriteSetManager"></a>

## Resource `DiemWriteSetManager`

A resource that holds the event handle for all the past WriteSet transactions that have been committed on chain.


<pre><code><b>struct</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_DiemWriteSetManager">DiemWriteSetManager</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>upgrade_events: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_AdminTransactionEvent">ExperimentalAccount::AdminTransactionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_ExperimentalAccount_AdminTransactionEvent"></a>

## Struct `AdminTransactionEvent`

Message for committed WriteSet transaction.


<pre><code><b>struct</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_AdminTransactionEvent">AdminTransactionEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committed_timestamp_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_ExperimentalAccount_ExperimentalAccountMarker"></a>

## Struct `ExperimentalAccountMarker`



<pre><code><b>struct</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ExperimentalAccountMarker">ExperimentalAccountMarker</a> <b>has</b> drop
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_ExperimentalAccount_MAX_U64"></a>



<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a name="0x1_ExperimentalAccount_EACCOUNT"></a>

The <code><a href="ExperimentalAccount.md#0x1_ExperimentalAccount">ExperimentalAccount</a></code> is not in the required state


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EACCOUNT">EACCOUNT</a>: u64 = 0;
</code></pre>



<a name="0x1_ExperimentalAccount_EACCOUNT_OPERATIONS_CAPABILITY"></a>

The <code>AccountOperationsCapability</code> was not in the required state


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EACCOUNT_OPERATIONS_CAPABILITY">EACCOUNT_OPERATIONS_CAPABILITY</a>: u64 = 22;
</code></pre>



<a name="0x1_ExperimentalAccount_EADD_EXISTING_CURRENCY"></a>

Tried to add a balance in a currency that this account already has


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EADD_EXISTING_CURRENCY">EADD_EXISTING_CURRENCY</a>: u64 = 15;
</code></pre>



<a name="0x1_ExperimentalAccount_ECANNOT_CREATE_AT_CORE_CODE"></a>

An account cannot be created at the reserved core code address of 0x1


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ECANNOT_CREATE_AT_CORE_CODE">ECANNOT_CREATE_AT_CORE_CODE</a>: u64 = 24;
</code></pre>



<a name="0x1_ExperimentalAccount_ECANNOT_CREATE_AT_VM_RESERVED"></a>

An account cannot be created at the reserved VM address of 0x0


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ECANNOT_CREATE_AT_VM_RESERVED">ECANNOT_CREATE_AT_VM_RESERVED</a>: u64 = 10;
</code></pre>



<a name="0x1_ExperimentalAccount_ECOIN_DEPOSIT_IS_ZERO"></a>

Tried to deposit a coin whose value was zero


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ECOIN_DEPOSIT_IS_ZERO">ECOIN_DEPOSIT_IS_ZERO</a>: u64 = 2;
</code></pre>



<a name="0x1_ExperimentalAccount_EDEPOSIT_EXCEEDS_LIMITS"></a>

Tried to deposit funds that would have surpassed the account's limits


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EDEPOSIT_EXCEEDS_LIMITS">EDEPOSIT_EXCEEDS_LIMITS</a>: u64 = 3;
</code></pre>



<a name="0x1_ExperimentalAccount_EGAS"></a>

An invalid amount of gas units was provided for execution of the transaction


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EGAS">EGAS</a>: u64 = 20;
</code></pre>



<a name="0x1_ExperimentalAccount_EINSUFFICIENT_BALANCE"></a>

The account does not hold a large enough balance in the specified currency


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 5;
</code></pre>



<a name="0x1_ExperimentalAccount_EKEY_ROTATION_CAPABILITY_ALREADY_EXTRACTED"></a>

The <code>KeyRotationCapability</code> for this account has already been extracted


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EKEY_ROTATION_CAPABILITY_ALREADY_EXTRACTED">EKEY_ROTATION_CAPABILITY_ALREADY_EXTRACTED</a>: u64 = 9;
</code></pre>



<a name="0x1_ExperimentalAccount_EMALFORMED_AUTHENTICATION_KEY"></a>

The provided authentication had an invalid length


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>: u64 = 8;
</code></pre>



<a name="0x1_ExperimentalAccount_EPAYEE_CANT_ACCEPT_CURRENCY_TYPE"></a>

Attempted to send funds in a currency that the receiving account does not hold.
e.g., <code>Diem&lt;XDX&gt;</code> to an account that exists, but does not have a <code>Balance&lt;XDX&gt;</code> resource


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EPAYEE_CANT_ACCEPT_CURRENCY_TYPE">EPAYEE_CANT_ACCEPT_CURRENCY_TYPE</a>: u64 = 18;
</code></pre>



<a name="0x1_ExperimentalAccount_EPAYEE_DOES_NOT_EXIST"></a>

Attempted to send funds to an account that does not exist


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EPAYEE_DOES_NOT_EXIST">EPAYEE_DOES_NOT_EXIST</a>: u64 = 17;
</code></pre>



<a name="0x1_ExperimentalAccount_EPAYER_DOESNT_HOLD_CURRENCY"></a>

Tried to withdraw funds in a currency that the account does hold


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EPAYER_DOESNT_HOLD_CURRENCY">EPAYER_DOESNT_HOLD_CURRENCY</a>: u64 = 19;
</code></pre>



<a name="0x1_ExperimentalAccount_EROLE_CANT_STORE_BALANCE"></a>

Tried to create a balance for an account whose role does not allow holding balances


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EROLE_CANT_STORE_BALANCE">EROLE_CANT_STORE_BALANCE</a>: u64 = 4;
</code></pre>



<a name="0x1_ExperimentalAccount_EWITHDRAWAL_EXCEEDS_LIMITS"></a>

The withdrawal of funds would have exceeded the the account's limits


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EWITHDRAWAL_EXCEEDS_LIMITS">EWITHDRAWAL_EXCEEDS_LIMITS</a>: u64 = 6;
</code></pre>



<a name="0x1_ExperimentalAccount_EWITHDRAW_CAPABILITY_ALREADY_EXTRACTED"></a>

The <code>WithdrawCapability</code> for this account has already been extracted


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EWITHDRAW_CAPABILITY_ALREADY_EXTRACTED">EWITHDRAW_CAPABILITY_ALREADY_EXTRACTED</a>: u64 = 7;
</code></pre>



<a name="0x1_ExperimentalAccount_EWITHDRAW_CAPABILITY_NOT_EXTRACTED"></a>

The <code>WithdrawCapability</code> for this account is not extracted


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EWITHDRAW_CAPABILITY_NOT_EXTRACTED">EWITHDRAW_CAPABILITY_NOT_EXTRACTED</a>: u64 = 11;
</code></pre>



<a name="0x1_ExperimentalAccount_EWRITESET_MANAGER"></a>

The <code><a href="ExperimentalAccount.md#0x1_ExperimentalAccount_DiemWriteSetManager">DiemWriteSetManager</a></code> was not in the required state


<pre><code><b>const</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EWRITESET_MANAGER">EWRITESET_MANAGER</a>: u64 = 23;
</code></pre>



<a name="0x1_ExperimentalAccount_create_core_account"></a>

## Function `create_core_account`



<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_core_account">create_core_account</a>(account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;): (signer, vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_core_account">create_core_account</a>(account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;): (signer, vector&lt;u8&gt;) {
    <b>assert</b>!(
        account_address != @VMReserved,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ECANNOT_CREATE_AT_VM_RESERVED">ECANNOT_CREATE_AT_VM_RESERVED</a>)
    );
    <b>assert</b>!(
        account_address != @CoreFramework,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ECANNOT_CREATE_AT_CORE_CODE">ECANNOT_CREATE_AT_CORE_CODE</a>)
    );
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_create_account">Account::create_account</a>(account_address, auth_key_prefix, &<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ExperimentalAccountMarker">ExperimentalAccountMarker</a>{})
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_initialize"></a>

## Function `initialize`

Initialize this module. This is only callable from genesis.


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_initialize">initialize</a>(dr_account: &signer, dummy_auth_key_prefix: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_initialize">initialize</a>(
    dr_account: &signer,
    dummy_auth_key_prefix: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp_assert_genesis">DiemTimestamp::assert_genesis</a>();
    // Operational constraint, not a privilege constraint.
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(dr_account);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_initialize">Account::initialize</a>&lt;<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ExperimentalAccountMarker">ExperimentalAccountMarker</a>&gt;(
        dr_account,
        @DiemFramework,
        b"<a href="ExperimentalAccount.md#0x1_ExperimentalAccount">ExperimentalAccount</a>",
        b"script_prologue",
        b"module_prologue",
        b"writeset_prologue",
        b"script_prologue",
        b"epilogue",
        b"writeset_epilogue",
        <b>false</b>,
    );

    // TODO: For legacy reasons. Remove
    <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_diem_root_account">create_diem_root_account</a>(
        <b>copy</b> dummy_auth_key_prefix,
    );
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_create_account"></a>

## Function `create_account`

Basic account creation method: no roles attached, no conditions checked.


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_account">create_account</a>(new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_account">create_account</a>(
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
) {
    <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_core_account">create_core_account</a>(new_account_address, auth_key_prefix);
    // No role attached
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_exists_at"></a>

## Function `exists_at`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_exists_at">exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_exists_at">exists_at</a>(addr: <b>address</b>): bool {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_exists_at">Account::exists_at</a>(addr)
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_create_diem_root_account"></a>

## Function `create_diem_root_account`

Creates the diem root account (during genesis). Publishes the Diem root role,
Publishes a SlidingNonce resource, sets up event generator, publishes
AccountOperationsCapability, WriteSetManager, and finally makes the account.


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_diem_root_account">create_diem_root_account</a>(auth_key_prefix: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_diem_root_account">create_diem_root_account</a>(
    auth_key_prefix: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp_assert_genesis">DiemTimestamp::assert_genesis</a>();
    <b>let</b> (dr_account, _) = <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_core_account">create_core_account</a>(@CoreResources, auth_key_prefix);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(&dr_account);
    <a href="Roles.md#0x1_Roles_grant_diem_root_role">Roles::grant_diem_root_role</a>(&dr_account);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_DiemWriteSetManager">DiemWriteSetManager</a>&gt;(@CoreResources),
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_EWRITESET_MANAGER">EWRITESET_MANAGER</a>)
    );
    <b>move_to</b>(
        &dr_account,
        <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_DiemWriteSetManager">DiemWriteSetManager</a> {
            upgrade_events: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_AdminTransactionEvent">AdminTransactionEvent</a>&gt;(&dr_account),
        }
    );
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_create_validator_account"></a>

## Function `create_validator_account`

Create a Validator account


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_validator_account">create_validator_account</a>(dr_account: &signer, new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_validator_account">create_validator_account</a>(
    dr_account: &signer,
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
    human_name: vector&lt;u8&gt;,
) {
    // TODO: Remove this role check when the core configs refactor lands
    <a href="Roles.md#0x1_Roles_assert_diem_root">Roles::assert_diem_root</a>(dr_account);
    <b>let</b> (new_account, _) = <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_core_account">create_core_account</a>(new_account_address, auth_key_prefix);
    // The dr_account account is verified <b>to</b> have the diem root role in `<a href="Roles.md#0x1_Roles_new_validator_role">Roles::new_validator_role</a>`
    <a href="Roles.md#0x1_Roles_new_validator_role">Roles::new_validator_role</a>(dr_account, &new_account);
    <a href="ValidatorConfig.md#0x1_ValidatorConfig_publish">ValidatorConfig::publish</a>(&new_account, dr_account, human_name);
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_create_validator_operator_account"></a>

## Function `create_validator_operator_account`

Create a Validator Operator account


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_validator_operator_account">create_validator_operator_account</a>(dr_account: &signer, new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_validator_operator_account">create_validator_operator_account</a>(
    dr_account: &signer,
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
    human_name: vector&lt;u8&gt;,
) {
    // TODO: Remove this role check when the core configs refactor lands
    <a href="Roles.md#0x1_Roles_assert_diem_root">Roles::assert_diem_root</a>(dr_account);
    <b>let</b> (new_account, _) = <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_core_account">create_core_account</a>(new_account_address, auth_key_prefix);
    // The dr_account is verified <b>to</b> have the diem root role in `<a href="Roles.md#0x1_Roles_new_validator_operator_role">Roles::new_validator_operator_role</a>`
    <a href="Roles.md#0x1_Roles_new_validator_operator_role">Roles::new_validator_operator_role</a>(dr_account, &new_account);
    <a href="ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig_publish">ValidatorOperatorConfig::publish</a>(&new_account, dr_account, human_name);
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_rotate_authentication_key"></a>

## Function `rotate_authentication_key`

Rotate the authentication key for the account under cap.account_address


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_rotate_authentication_key">rotate_authentication_key</a>(account: &signer, new_authentication_key: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_rotate_authentication_key">rotate_authentication_key</a>(
    account: &signer,
    new_authentication_key: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_rotate_authentication_key">Account::rotate_authentication_key</a>(account, new_authentication_key)
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_module_prologue"></a>

## Function `module_prologue`



<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_module_prologue">module_prologue</a>(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, _txn_gas_price: u64, _txn_max_gas_units: u64, _txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_module_prologue">module_prologue</a>(
    sender: signer,
    txn_sequence_number: u64,
    txn_public_key: vector&lt;u8&gt;,
    _txn_gas_price: u64,
    _txn_max_gas_units: u64,
    _txn_expiration_time: u64,
    chain_id: u8,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_prologue">Account::prologue</a>(&sender, txn_sequence_number, txn_public_key, chain_id)
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_script_prologue"></a>

## Function `script_prologue`



<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_script_prologue">script_prologue</a>(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, _txn_gas_price: u64, _txn_max_gas_units: u64, _txn_expiration_time: u64, chain_id: u8, _script_hash: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_script_prologue">script_prologue</a>(
    sender: signer,
    txn_sequence_number: u64,
    txn_public_key: vector&lt;u8&gt;,
    _txn_gas_price: u64,
    _txn_max_gas_units: u64,
    _txn_expiration_time: u64,
    chain_id: u8,
    _script_hash: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_prologue">Account::prologue</a>(&sender, txn_sequence_number, txn_public_key, chain_id)
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_writeset_prologue"></a>

## Function `writeset_prologue`



<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_writeset_prologue">writeset_prologue</a>(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, _txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_writeset_prologue">writeset_prologue</a>(
    sender: signer,
    txn_sequence_number: u64,
    txn_public_key: vector&lt;u8&gt;,
    _txn_expiration_time: u64,
    chain_id: u8,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_prologue">Account::prologue</a>(&sender, txn_sequence_number, txn_public_key, chain_id)
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_multi_agent_script_prologue"></a>

## Function `multi_agent_script_prologue`



<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_multi_agent_script_prologue">multi_agent_script_prologue</a>(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, _secondary_signer_addresses: vector&lt;<b>address</b>&gt;, _secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, _txn_gas_price: u64, _txn_max_gas_units: u64, _txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_multi_agent_script_prologue">multi_agent_script_prologue</a>(
    sender: signer,
    txn_sequence_number: u64,
    txn_sender_public_key: vector&lt;u8&gt;,
    _secondary_signer_addresses: vector&lt;<b>address</b>&gt;,
    _secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,
    _txn_gas_price: u64,
    _txn_max_gas_units: u64,
    _txn_expiration_time: u64,
    chain_id: u8,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_prologue">Account::prologue</a>(&sender, txn_sequence_number, txn_sender_public_key, chain_id)
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_epilogue"></a>

## Function `epilogue`



<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_epilogue">epilogue</a>(account: signer, _txn_sequence_number: u64, _txn_gas_price: u64, _txn_max_gas_units: u64, _gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_epilogue">epilogue</a>(
    account: signer,
    _txn_sequence_number: u64,
    _txn_gas_price: u64,
    _txn_max_gas_units: u64,
    _gas_units_remaining: u64
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_epilogue">Account::epilogue</a>(&account, &<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ExperimentalAccountMarker">ExperimentalAccountMarker</a>{});
}
</code></pre>



</details>

<a name="0x1_ExperimentalAccount_writeset_epilogue"></a>

## Function `writeset_epilogue`



<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_writeset_epilogue">writeset_epilogue</a>(dr_account: signer, _txn_sequence_number: u64, should_trigger_reconfiguration: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_writeset_epilogue">writeset_epilogue</a>(
    dr_account: signer,
    _txn_sequence_number: u64,
    should_trigger_reconfiguration: bool,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_epilogue">Account::epilogue</a>(&dr_account, &<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_ExperimentalAccountMarker">ExperimentalAccountMarker</a>{});
    <b>if</b> (should_trigger_reconfiguration) <a href="DiemConfig.md#0x1_DiemConfig_reconfigure">DiemConfig::reconfigure</a>(&dr_account);
}
</code></pre>



</details>
