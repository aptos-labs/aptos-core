
<a name="0x1_AptosAccount"></a>

# Module `0x1::AptosAccount`

The <code><a href="AptosAccount.md#0x1_AptosAccount">AptosAccount</a></code> module manages experimental accounts.
It also defines the prolog and epilog that run before and after every
transaction in addition to the core prologue and epilogue.


-  [Constants](#@Constants_0)
-  [Function `create_account_internal`](#0x1_AptosAccount_create_account_internal)
-  [Function `initialize`](#0x1_AptosAccount_initialize)
-  [Function `create_account`](#0x1_AptosAccount_create_account)
-  [Function `exists_at`](#0x1_AptosAccount_exists_at)
-  [Function `create_validator_account`](#0x1_AptosAccount_create_validator_account)
-  [Function `create_validator_operator_account`](#0x1_AptosAccount_create_validator_operator_account)
-  [Function `rotate_authentication_key`](#0x1_AptosAccount_rotate_authentication_key)
-  [Function `module_prologue`](#0x1_AptosAccount_module_prologue)
-  [Function `script_prologue`](#0x1_AptosAccount_script_prologue)
-  [Function `writeset_prologue`](#0x1_AptosAccount_writeset_prologue)
-  [Function `multi_agent_script_prologue`](#0x1_AptosAccount_multi_agent_script_prologue)
-  [Function `epilogue`](#0x1_AptosAccount_epilogue)
-  [Function `writeset_epilogue`](#0x1_AptosAccount_writeset_epilogue)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account">0x1::Account</a>;
<b>use</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig">0x1::AptosValidatorConfig</a>;
<b>use</b> <a href="AptosValidatorOperatorConfig.md#0x1_AptosValidatorOperatorConfig">0x1::AptosValidatorOperatorConfig</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp">0x1::DiemTimestamp</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
<b>use</b> <a href="TestCoin.md#0x1_TestCoin">0x1::TestCoin</a>;
<b>use</b> <a href="TransactionFee.md#0x1_TransactionFee">0x1::TransactionFee</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x1_AptosAccount_MAX_U64"></a>



<pre><code><b>const</b> <a href="AptosAccount.md#0x1_AptosAccount_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a name="0x1_AptosAccount_ECANNOT_CREATE_AT_CORE_CODE"></a>



<pre><code><b>const</b> <a href="AptosAccount.md#0x1_AptosAccount_ECANNOT_CREATE_AT_CORE_CODE">ECANNOT_CREATE_AT_CORE_CODE</a>: u64 = 2;
</code></pre>



<a name="0x1_AptosAccount_ECANNOT_CREATE_AT_VM_RESERVED"></a>



<pre><code><b>const</b> <a href="AptosAccount.md#0x1_AptosAccount_ECANNOT_CREATE_AT_VM_RESERVED">ECANNOT_CREATE_AT_VM_RESERVED</a>: u64 = 0;
</code></pre>



<a name="0x1_AptosAccount_EGAS"></a>



<pre><code><b>const</b> <a href="AptosAccount.md#0x1_AptosAccount_EGAS">EGAS</a>: u64 = 1;
</code></pre>



<a name="0x1_AptosAccount_create_account_internal"></a>

## Function `create_account_internal`



<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_create_account_internal">create_account_internal</a>(account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;): (signer, vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_create_account_internal">create_account_internal</a>(account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;): (signer, vector&lt;u8&gt;) {
    <b>assert</b>!(
        account_address != @VMReserved,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="AptosAccount.md#0x1_AptosAccount_ECANNOT_CREATE_AT_VM_RESERVED">ECANNOT_CREATE_AT_VM_RESERVED</a>)
    );
    <b>assert</b>!(
        account_address != @CoreFramework,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="AptosAccount.md#0x1_AptosAccount_ECANNOT_CREATE_AT_CORE_CODE">ECANNOT_CREATE_AT_CORE_CODE</a>)
    );
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_create_account">Account::create_account</a>(account_address, auth_key_prefix, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
}
</code></pre>



</details>

<a name="0x1_AptosAccount_initialize"></a>

## Function `initialize`

Initialize this module. This is only callable from genesis.


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_initialize">initialize</a>(core_resource: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_initialize">initialize</a>(core_resource: &signer) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp_assert_genesis">DiemTimestamp::assert_genesis</a>();
    // Operational constraint, not a privilege constraint.
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(core_resource);
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_initialize">Account::initialize</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(
        core_resource,
        @CoreFramework,
        b"<a href="AptosAccount.md#0x1_AptosAccount">AptosAccount</a>",
        b"script_prologue",
        b"module_prologue",
        b"writeset_prologue",
        b"script_prologue",
        b"epilogue",
        b"writeset_epilogue",
        <b>false</b>,
    );
}
</code></pre>



</details>

<a name="0x1_AptosAccount_create_account"></a>

## Function `create_account`

Basic account creation method: no roles attached, no conditions checked.


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_create_account">create_account</a>(new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_create_account">create_account</a>(
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
): signer {
    <b>let</b> (signer, _) = <a href="AptosAccount.md#0x1_AptosAccount_create_account_internal">create_account_internal</a>(new_account_address, auth_key_prefix);
    signer
}
</code></pre>



</details>

<a name="0x1_AptosAccount_exists_at"></a>

## Function `exists_at`



<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_exists_at">exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_exists_at">exists_at</a>(addr: <b>address</b>): bool {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_exists_at">Account::exists_at</a>(addr)
}
</code></pre>



</details>

<a name="0x1_AptosAccount_create_validator_account"></a>

## Function `create_validator_account`

Create a Validator account


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_create_validator_account">create_validator_account</a>(core_resource: &signer, new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_create_validator_account">create_validator_account</a>(
    core_resource: &signer,
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
    human_name: vector&lt;u8&gt;,
) {
    <b>let</b> (new_account, _) = <a href="AptosAccount.md#0x1_AptosAccount_create_account_internal">create_account_internal</a>(new_account_address, auth_key_prefix);
    <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_publish">AptosValidatorConfig::publish</a>(core_resource, &new_account, human_name);
}
</code></pre>



</details>

<a name="0x1_AptosAccount_create_validator_operator_account"></a>

## Function `create_validator_operator_account`

Create a Validator Operator account


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_create_validator_operator_account">create_validator_operator_account</a>(core_resource: &signer, new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_create_validator_operator_account">create_validator_operator_account</a>(
    core_resource: &signer,
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
    human_name: vector&lt;u8&gt;,
) {
    <b>let</b> (new_account, _) = <a href="AptosAccount.md#0x1_AptosAccount_create_account_internal">create_account_internal</a>(new_account_address, auth_key_prefix);
    <a href="AptosValidatorOperatorConfig.md#0x1_AptosValidatorOperatorConfig_publish">AptosValidatorOperatorConfig::publish</a>(core_resource, &new_account, human_name);
}
</code></pre>



</details>

<a name="0x1_AptosAccount_rotate_authentication_key"></a>

## Function `rotate_authentication_key`

Rotate the authentication key for the account under cap.account_address


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_rotate_authentication_key">rotate_authentication_key</a>(account: &signer, new_authentication_key: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_rotate_authentication_key">rotate_authentication_key</a>(
    account: &signer,
    new_authentication_key: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_rotate_authentication_key">Account::rotate_authentication_key</a>(account, new_authentication_key)
}
</code></pre>



</details>

<a name="0x1_AptosAccount_module_prologue"></a>

## Function `module_prologue`



<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_module_prologue">module_prologue</a>(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, _txn_gas_price: u64, _txn_max_gas_units: u64, _txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_module_prologue">module_prologue</a>(
    sender: signer,
    txn_sequence_number: u64,
    txn_public_key: vector&lt;u8&gt;,
    _txn_gas_price: u64,
    _txn_max_gas_units: u64,
    _txn_expiration_time: u64,
    chain_id: u8,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_prologue">Account::prologue</a>(&sender, txn_sequence_number, txn_public_key, chain_id)
}
</code></pre>



</details>

<a name="0x1_AptosAccount_script_prologue"></a>

## Function `script_prologue`



<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_script_prologue">script_prologue</a>(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, _txn_gas_price: u64, _txn_max_gas_units: u64, _txn_expiration_time: u64, chain_id: u8, _script_hash: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_script_prologue">script_prologue</a>(
    sender: signer,
    txn_sequence_number: u64,
    txn_public_key: vector&lt;u8&gt;,
    _txn_gas_price: u64,
    _txn_max_gas_units: u64,
    _txn_expiration_time: u64,
    chain_id: u8,
    _script_hash: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_prologue">Account::prologue</a>(&sender, txn_sequence_number, txn_public_key, chain_id)
}
</code></pre>



</details>

<a name="0x1_AptosAccount_writeset_prologue"></a>

## Function `writeset_prologue`



<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_writeset_prologue">writeset_prologue</a>(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, _txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_writeset_prologue">writeset_prologue</a>(
    sender: signer,
    txn_sequence_number: u64,
    txn_public_key: vector&lt;u8&gt;,
    _txn_expiration_time: u64,
    chain_id: u8,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_prologue">Account::prologue</a>(&sender, txn_sequence_number, txn_public_key, chain_id)
}
</code></pre>



</details>

<a name="0x1_AptosAccount_multi_agent_script_prologue"></a>

## Function `multi_agent_script_prologue`



<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_multi_agent_script_prologue">multi_agent_script_prologue</a>(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, _secondary_signer_addresses: vector&lt;<b>address</b>&gt;, _secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, _txn_gas_price: u64, _txn_max_gas_units: u64, _txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_multi_agent_script_prologue">multi_agent_script_prologue</a>(
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
     <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_prologue">Account::prologue</a>(&sender, txn_sequence_number, txn_sender_public_key, chain_id)
}
</code></pre>



</details>

<a name="0x1_AptosAccount_epilogue"></a>

## Function `epilogue`



<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_epilogue">epilogue</a>(account: signer, _txn_sequence_number: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_epilogue">epilogue</a>(
    account: signer,
    _txn_sequence_number: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64
) {
    // [EA1; Invariant]: Make sure that the transaction's `max_gas_units` is greater
    // than the number of gas units remaining after execution.
    <b>assert</b>!(txn_max_gas_units &gt;= gas_units_remaining, <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="AptosAccount.md#0x1_AptosAccount_EGAS">EGAS</a>));
    <b>let</b> gas_used = txn_max_gas_units - gas_units_remaining;

    // [EA2; Invariant]: Make sure that the transaction fee would not overflow maximum
    // number representable in a u64. Already checked in [PCA5].
    <b>assert</b>!(
        (txn_gas_price <b>as</b> u128) * (gas_used <b>as</b> u128) &lt;= <a href="AptosAccount.md#0x1_AptosAccount_MAX_U64">MAX_U64</a>,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_limit_exceeded">Errors::limit_exceeded</a>(<a href="AptosAccount.md#0x1_AptosAccount_EGAS">EGAS</a>)
    );
    <b>let</b> transaction_fee_amount = txn_gas_price * gas_used;
    <b>let</b> coin = <a href="TestCoin.md#0x1_TestCoin_withdraw">TestCoin::withdraw</a>(&account, transaction_fee_amount);
    <a href="TransactionFee.md#0x1_TransactionFee_burn_fee">TransactionFee::burn_fee</a>(coin);

    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_epilogue">Account::epilogue</a>(&account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>());
}
</code></pre>



</details>

<a name="0x1_AptosAccount_writeset_epilogue"></a>

## Function `writeset_epilogue`



<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_writeset_epilogue">writeset_epilogue</a>(core_resource: signer, _txn_sequence_number: u64, should_trigger_reconfiguration: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="AptosAccount.md#0x1_AptosAccount_writeset_epilogue">writeset_epilogue</a>(
    core_resource: signer,
    _txn_sequence_number: u64,
    should_trigger_reconfiguration: bool,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/Account.md#0x1_Account_writeset_epilogue">Account::writeset_epilogue</a>(&core_resource, should_trigger_reconfiguration, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>());
}
</code></pre>



</details>
