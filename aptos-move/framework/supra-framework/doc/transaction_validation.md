
<a id="0x1_transaction_validation"></a>

# Module `0x1::transaction_validation`



-  [Resource `TransactionValidation`](#0x1_transaction_validation_TransactionValidation)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_transaction_validation_initialize)
-  [Function `prologue_common`](#0x1_transaction_validation_prologue_common)
-  [Function `script_prologue`](#0x1_transaction_validation_script_prologue)
-  [Function `automated_transaction_prologue`](#0x1_transaction_validation_automated_transaction_prologue)
-  [Function `multi_agent_script_prologue`](#0x1_transaction_validation_multi_agent_script_prologue)
-  [Function `multi_agent_common_prologue`](#0x1_transaction_validation_multi_agent_common_prologue)
-  [Function `fee_payer_script_prologue`](#0x1_transaction_validation_fee_payer_script_prologue)
-  [Function `epilogue`](#0x1_transaction_validation_epilogue)
-  [Function `automated_transaction_epilogue`](#0x1_transaction_validation_automated_transaction_epilogue)
-  [Function `epilogue_gas_payer_only`](#0x1_transaction_validation_epilogue_gas_payer_only)
-  [Function `epilogue_gas_payer`](#0x1_transaction_validation_epilogue_gas_payer)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `prologue_common`](#@Specification_1_prologue_common)
    -  [Function `script_prologue`](#@Specification_1_script_prologue)
    -  [Function `automated_transaction_prologue`](#@Specification_1_automated_transaction_prologue)
    -  [Function `multi_agent_script_prologue`](#@Specification_1_multi_agent_script_prologue)
    -  [Function `multi_agent_common_prologue`](#@Specification_1_multi_agent_common_prologue)
    -  [Function `fee_payer_script_prologue`](#@Specification_1_fee_payer_script_prologue)
    -  [Function `epilogue`](#@Specification_1_epilogue)
    -  [Function `automated_transaction_epilogue`](#@Specification_1_automated_transaction_epilogue)
    -  [Function `epilogue_gas_payer_only`](#@Specification_1_epilogue_gas_payer_only)
    -  [Function `epilogue_gas_payer`](#@Specification_1_epilogue_gas_payer)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="automation_registry.md#0x1_automation_registry">0x1::automation_registry</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="supra_account.md#0x1_supra_account">0x1::supra_account</a>;
<b>use</b> <a href="supra_coin.md#0x1_supra_coin">0x1::supra_coin</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="transaction_fee.md#0x1_transaction_fee">0x1::transaction_fee</a>;
</code></pre>



<a id="0x1_transaction_validation_TransactionValidation"></a>

## Resource `TransactionValidation`

This holds information that will be picked up by the VM to call the
correct chain-specific prologue and epilogue functions


<pre><code><b>struct</b> <a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>module_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_transaction_validation_MAX_U64"></a>

MSB is used to indicate a gas payer tx


<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_transaction_validation_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>: u64 = 6;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>: u64 = 1004;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID">PROLOGUE_EBAD_CHAIN_ID</a>: u64 = 1007;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>: u64 = 1005;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED">PROLOGUE_EFEE_PAYER_NOT_ENABLED</a>: u64 = 1010;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY"></a>

Prologue errors. These are separated out from the other errors in this
module since they are mapped separately to major VM statuses, and are
important to the semantics of the system.


<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>: u64 = 1001;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ENO_ACTIVE_AUTOMATED_TASK"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ENO_ACTIVE_AUTOMATED_TASK">PROLOGUE_ENO_ACTIVE_AUTOMATED_TASK</a>: u64 = 1012;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH">PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH</a>: u64 = 1009;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG">PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG</a>: u64 = 1008;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW">PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW</a>: u64 = 1003;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD">PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD</a>: u64 = 1002;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED">PROLOGUE_ETRANSACTION_EXPIRED</a>: u64 = 1006;
</code></pre>



<a id="0x1_transaction_validation_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    // module_prologue_name is deprecated and not used.
    module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);

    <b>move_to</b>(supra_framework, <a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a> {
        module_addr: @supra_framework,
        module_name: b"<a href="transaction_validation.md#0x1_transaction_validation">transaction_validation</a>",
        script_prologue_name,
        // module_prologue_name is deprecated and not used.
        module_prologue_name,
        multi_agent_prologue_name,
        user_epilogue_name,
    });
}
</code></pre>



</details>

<a id="0x1_transaction_validation_prologue_common"></a>

## Function `prologue_common`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, txn_sequence_number: u64, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: <b>address</b>,
    txn_sequence_number: u64,
    txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
) {
    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; txn_expiration_time,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED">PROLOGUE_ETRANSACTION_EXPIRED</a>),
    );
    <b>assert</b>!(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() == <a href="chain_id.md#0x1_chain_id">chain_id</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID">PROLOGUE_EBAD_CHAIN_ID</a>));

    <b>let</b> transaction_sender = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);

    <b>if</b> (
        transaction_sender == gas_payer
            || <a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)
            || !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_sponsored_automatic_account_creation_enabled">features::sponsored_automatic_account_creation_enabled</a>()
            || txn_sequence_number != 0
    ) {
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>));
        <b>assert</b>!(
            txn_authentication_key == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(transaction_sender),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
        );

        <b>let</b> account_sequence_number = <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(transaction_sender);
        <b>assert</b>!(
            txn_sequence_number &lt; (1u64 &lt;&lt; 63),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG">PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG</a>)
        );

        <b>assert</b>!(
            txn_sequence_number &gt;= account_sequence_number,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD">PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD</a>)
        );

        <b>assert</b>!(
            txn_sequence_number == account_sequence_number,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW">PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW</a>)
        );
    } <b>else</b> {
        // In this case, the transaction is sponsored and the <a href="account.md#0x1_account">account</a> does not exist, so ensure
        // the default values match.
        <b>assert</b>!(
            txn_sequence_number == 0,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW">PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW</a>)
        );

        <b>assert</b>!(
            txn_authentication_key == <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&transaction_sender),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
        );
    };

    <b>let</b> max_transaction_fee = txn_gas_price * txn_max_gas_units;

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_supra_store_enabled">features::operations_default_to_fa_supra_store_enabled</a>()) {
        <b>assert</b>!(
            <a href="supra_account.md#0x1_supra_account_is_fungible_balance_at_least">supra_account::is_fungible_balance_at_least</a>(gas_payer, max_transaction_fee),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)
        );
    } <b>else</b> {
        <b>assert</b>!(
            <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;SupraCoin&gt;(gas_payer, max_transaction_fee),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)
        );
    }
}
</code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue"></a>

## Function `script_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    txn_sequence_number: u64,
    txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <b>let</b> gas_payer = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        gas_payer,
        txn_sequence_number,
        txn_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>
    )
}
</code></pre>



</details>

<a id="0x1_transaction_validation_automated_transaction_prologue"></a>

## Function `automated_transaction_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_automated_transaction_prologue">automated_transaction_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_index: u64, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_automated_transaction_prologue">automated_transaction_prologue</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task_index: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
)  {
    <b>let</b> gas_payer = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);

    <b>assert</b>!(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() == <a href="chain_id.md#0x1_chain_id">chain_id</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID">PROLOGUE_EBAD_CHAIN_ID</a>));

    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; txn_expiration_time,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED">PROLOGUE_ETRANSACTION_EXPIRED</a>),
    );

    <b>let</b> max_transaction_fee = txn_gas_price * txn_max_gas_units;

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_supra_store_enabled">features::operations_default_to_fa_supra_store_enabled</a>()) {
        <b>assert</b>!(
            <a href="supra_account.md#0x1_supra_account_is_fungible_balance_at_least">supra_account::is_fungible_balance_at_least</a>(gas_payer, max_transaction_fee),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)
        );
    } <b>else</b> {
        <b>assert</b>!(
            <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;SupraCoin&gt;(gas_payer, max_transaction_fee),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)
        );
    };
    <b>assert</b>!(<a href="automation_registry.md#0x1_automation_registry_has_sender_active_task_with_id">automation_registry::has_sender_active_task_with_id</a>(address_of(&sender), task_index),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ENO_ACTIVE_AUTOMATED_TASK">PROLOGUE_ENO_ACTIVE_AUTOMATED_TASK</a>))
}
</code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_script_prologue"></a>

## Function `multi_agent_script_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue">multi_agent_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue">multi_agent_script_prologue</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    txn_sequence_number: u64,
    txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
) {
    <b>let</b> sender_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        sender_addr,
        txn_sequence_number,
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_common_prologue"></a>

## Function `multi_agent_common_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) {
    <b>let</b> num_secondary_signers = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&secondary_signer_addresses);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&secondary_signer_public_key_hashes) == num_secondary_signers,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH">PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH</a>),
    );

    <b>let</b> i = 0;
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> i &lt;= num_secondary_signers;
            <b>invariant</b> <b>forall</b> j in 0..i:
                <a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[j])
                    && secondary_signer_public_key_hashes[j]
                    == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[j]);
        };
        (i &lt; num_secondary_signers)
    }) {
        <b>let</b> secondary_address = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&secondary_signer_addresses, i);
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>));

        <b>let</b> signer_public_key_hash = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&secondary_signer_public_key_hashes, i);
        <b>assert</b>!(
            signer_public_key_hash == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
        );
        i = i + 1;
    }
}
</code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue"></a>

## Function `fee_payer_script_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    txn_sequence_number: u64,
    txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    fee_payer_address: <b>address</b>,
    fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_fee_payer_enabled">features::fee_payer_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED">PROLOGUE_EFEE_PAYER_NOT_ENABLED</a>));
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        fee_payer_address,
        txn_sequence_number,
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes);
    <b>assert</b>!(
        fee_payer_public_key_hash == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(fee_payer_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
    );
}
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue"></a>

## Function `epilogue`

Epilogue function is run after a transaction is successfully executed.
Called by the Adapter


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64
) {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<a href="account.md#0x1_account">account</a>, addr, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_automated_transaction_epilogue"></a>

## Function `automated_transaction_epilogue`

Epilogue function is run after a automated transaction is successfully executed.
Called by the Adapter


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_automated_transaction_epilogue">automated_transaction_epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_automated_transaction_epilogue">automated_transaction_epilogue</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64
) {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_only">epilogue_gas_payer_only</a>(addr, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer_only"></a>

## Function `epilogue_gas_payer_only`

Epilogue function with explicit gas payer specified, is run after a transaction is successfully executed.
Called by the Adapter.
Only burns spent gas does not increment sequcence number of the sender account.


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_only">epilogue_gas_payer_only</a>(gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_only">epilogue_gas_payer_only</a>(
    gas_payer: <b>address</b>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64
) {
    <b>assert</b>!(txn_max_gas_units &gt;= gas_units_remaining, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>));
    <b>let</b> gas_used = txn_max_gas_units - gas_units_remaining;

    <b>assert</b>!(
        (txn_gas_price <b>as</b> u128) * (gas_used <b>as</b> u128) &lt;= <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>)
    );
    <b>let</b> transaction_fee_amount = txn_gas_price * gas_used;

    // it's important <b>to</b> maintain the <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> <a href="code.md#0x1_code">code</a> consistent <b>with</b> vm
    // <b>to</b> do failed transaction cleanup.
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_supra_store_enabled">features::operations_default_to_fa_supra_store_enabled</a>()) {
        <b>assert</b>!(
            <a href="supra_account.md#0x1_supra_account_is_fungible_balance_at_least">supra_account::is_fungible_balance_at_least</a>(gas_payer, transaction_fee_amount),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>),
        );
    } <b>else</b> {
        <b>assert</b>!(
            <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;SupraCoin&gt;(gas_payer, transaction_fee_amount),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>),
        );
    };

    <b>let</b> amount_to_burn = <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_collect_and_distribute_gas_fees">features::collect_and_distribute_gas_fees</a>()) {
        // TODO(gas): We might want <b>to</b> distinguish the refundable part of the charge and burn it or track
        // it separately, so that we don't increase the total supply by refunding.

        // If transaction fees are redistributed <b>to</b> validators, collect them here for
        // later redistribution.
        <a href="transaction_fee.md#0x1_transaction_fee_collect_fee">transaction_fee::collect_fee</a>(gas_payer, transaction_fee_amount);
        0
    } <b>else</b> {
        // Otherwise, just burn the fee.
        // TODO: this branch should be removed completely when transaction fee collection
        // is tested and is fully proven <b>to</b> work well.
        transaction_fee_amount
    };

    <b>if</b> (amount_to_burn &gt; storage_fee_refunded) {
        <b>let</b> burn_amount = amount_to_burn - storage_fee_refunded;
        <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">transaction_fee::burn_fee</a>(gas_payer, burn_amount);
    } <b>else</b> <b>if</b> (amount_to_burn &lt; storage_fee_refunded) {
        <b>let</b> mint_amount = storage_fee_refunded - amount_to_burn;
        <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">transaction_fee::mint_and_refund</a>(gas_payer, mint_amount)
    };

}
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer"></a>

## Function `epilogue_gas_payer`

Epilogue function with explicit gas payer specified, is run after a transaction is successfully executed.
Called by the Adapter


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: <b>address</b>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64
) {
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_only">epilogue_gas_payer_only</a>(gas_payer, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining);

    // Increment sequence number
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_increment_sequence_number">account::increment_sequence_number</a>(addr);
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
<td>The sender of a transaction should have sufficient coin balance to pay the transaction fee.</td>
<td>High</td>
<td>The prologue_common function asserts that the transaction sender has enough coin balance to be paid as the max_transaction_fee.</td>
<td>Formally verified via <a href="#high-level-req-1">PrologueCommonAbortsIf</a>. Moreover, the native transaction validation patterns have been manually audited.</td>
</tr>

<tr>
<td>2</td>
<td>All secondary signer addresses are verified to be authentic through a validation process.</td>
<td>Critical</td>
<td>The function multi_agent_script_prologue ensures that each secondary signer address undergoes authentication validation, including verification of account existence and authentication key matching, confirming their authenticity.</td>
<td>Formally verified via <a href="#high-level-req-2">multi_agent_script_prologue</a>. Moreover, the native transaction validation patterns have been manually audited.</td>
</tr>

<tr>
<td>3</td>
<td>After successful execution, base the transaction fee on the configuration set by the features library.</td>
<td>High</td>
<td>The epilogue function collects the transaction fee for either redistribution or burning based on the feature::collect_and_distribute_gas_fees result.</td>
<td>Formally Verified via <a href="#high-level-req-3">epilogue</a>. Moreover, the native transaction validation patterns have been manually audited.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


Ensure caller is <code>supra_framework</code>.
Aborts if TransactionValidation already exists.


<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(supra_framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_supra_framework_address">system_addresses::is_supra_framework_address</a>(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a>&gt;(addr);
</code></pre>


Create a schema to reuse some code.
Give some constraints that may abort according to the conditions.


<a id="0x1_transaction_validation_PrologueCommonAbortsIf"></a>


<pre><code><b>schema</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> {
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    gas_payer: <b>address</b>;
    txn_sequence_number: u64;
    txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    txn_gas_price: u64;
    txn_max_gas_units: u64;
    txn_expiration_time: u64;
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8;
    <b>aborts_if</b> !<b>exists</b>&lt;CurrentTimeMicroseconds&gt;(@supra_framework);
    <b>aborts_if</b> !(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; txn_expiration_time);
    <b>aborts_if</b> !<b>exists</b>&lt;ChainId&gt;(@supra_framework);
    <b>aborts_if</b> !(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() == <a href="chain_id.md#0x1_chain_id">chain_id</a>);
    <b>let</b> transaction_sender = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>aborts_if</b> (
        !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>)
            || <a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)
            || transaction_sender == gas_payer
            || txn_sequence_number &gt; 0
    ) && (
        !(txn_sequence_number &gt;= <b>global</b>&lt;Account&gt;(transaction_sender).sequence_number)
            || !(txn_authentication_key == <b>global</b>&lt;Account&gt;(transaction_sender).authentication_key)
            || !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)
            || !(txn_sequence_number == <b>global</b>&lt;Account&gt;(transaction_sender).sequence_number)
    );
    <b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>)
        && transaction_sender != gas_payer
        && txn_sequence_number == 0
        && !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)
        && txn_authentication_key != <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(transaction_sender);
    <b>aborts_if</b> !(txn_sequence_number &lt; (1u64 &lt;&lt; 63));
    <b>let</b> max_transaction_fee = txn_gas_price * txn_max_gas_units;
    <b>aborts_if</b> max_transaction_fee &gt; <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>;
    <b>aborts_if</b> !<b>exists</b>&lt;CoinStore&lt;SupraCoin&gt;&gt;(gas_payer);
    // This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
    <b>aborts_if</b> !(<b>global</b>&lt;CoinStore&lt;SupraCoin&gt;&gt;(gas_payer).<a href="coin.md#0x1_coin">coin</a>.value &gt;= max_transaction_fee);
}
</code></pre>



<a id="@Specification_1_prologue_common"></a>

### Function `prologue_common`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, txn_sequence_number: u64, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a>;
</code></pre>



<a id="@Specification_1_script_prologue"></a>

### Function `script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> {
    gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
    txn_authentication_key: txn_public_key
};
</code></pre>




<a id="0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf"></a>


<pre><code><b>schema</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> {
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;;
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;;
    <b>let</b> num_secondary_signers = len(secondary_signer_addresses);
    <b>aborts_if</b> len(secondary_signer_public_key_hashes) != num_secondary_signers;
    // This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
    <b>aborts_if</b> <b>exists</b> i in 0..num_secondary_signers:
        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[i])
            || secondary_signer_public_key_hashes[i] !=
            <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[i]);
    <b>ensures</b> <b>forall</b> i in 0..num_secondary_signers:
        <a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[i])
            && secondary_signer_public_key_hashes[i] ==
            <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[i]);
}
</code></pre>



<a id="@Specification_1_automated_transaction_prologue"></a>

### Function `automated_transaction_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_automated_transaction_prologue">automated_transaction_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_index: u64, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_multi_agent_script_prologue"></a>

### Function `multi_agent_script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue">multi_agent_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>


Aborts if length of public key hashed vector
not equal the number of singers.


<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>let</b> gas_payer = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
<b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> {
    gas_payer,
    txn_sequence_number,
    txn_authentication_key: txn_sender_public_key,
};
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> {
    secondary_signer_addresses,
    secondary_signer_public_key_hashes,
};
</code></pre>



<a id="@Specification_1_multi_agent_common_prologue"></a>

### Function `multi_agent_common_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>




<pre><code><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> {
    secondary_signer_addresses,
    secondary_signer_public_key_hashes,
};
</code></pre>



<a id="@Specification_1_fee_payer_script_prologue"></a>

### Function `fee_payer_script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_FEE_PAYER_ENABLED">features::FEE_PAYER_ENABLED</a>);
<b>let</b> gas_payer = fee_payer_address;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> {
    gas_payer,
    txn_sequence_number,
    txn_authentication_key: txn_sender_public_key,
};
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> {
    secondary_signer_addresses,
    secondary_signer_public_key_hashes,
};
<b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(gas_payer);
<b>aborts_if</b> !(fee_payer_public_key_hash == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(gas_payer));
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_fee_payer_enabled">features::spec_fee_payer_enabled</a>();
</code></pre>



<a id="@Specification_1_epilogue"></a>

### Function `epilogue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>


Abort according to the conditions.
<code>SupraCoinCapabilities</code> and <code>CoinInfo</code> should exists.
Skip transaction_fee::burn_fee verification.


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_EpilogueGasPayerAbortsIf">EpilogueGasPayerAbortsIf</a> { gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>) };
</code></pre>



<a id="@Specification_1_automated_transaction_epilogue"></a>

### Function `automated_transaction_epilogue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_automated_transaction_epilogue">automated_transaction_epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_epilogue_gas_payer_only"></a>

### Function `epilogue_gas_payer_only`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_only">epilogue_gas_payer_only</a>(gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_epilogue_gas_payer"></a>

### Function `epilogue_gas_payer`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>


Abort according to the conditions.
<code>SupraCoinCapabilities</code> and <code>CoinInfo</code> should exist.
Skip transaction_fee::burn_fee verification.


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_EpilogueGasPayerAbortsIf">EpilogueGasPayerAbortsIf</a>;
</code></pre>




<a id="0x1_transaction_validation_EpilogueGasPayerAbortsIf"></a>


<pre><code><b>schema</b> <a href="transaction_validation.md#0x1_transaction_validation_EpilogueGasPayerAbortsIf">EpilogueGasPayerAbortsIf</a> {
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    gas_payer: <b>address</b>;
    storage_fee_refunded: u64;
    txn_gas_price: u64;
    txn_max_gas_units: u64;
    gas_units_remaining: u64;
    <b>aborts_if</b> !(txn_max_gas_units &gt;= gas_units_remaining);
    <b>let</b> gas_used = txn_max_gas_units - gas_units_remaining;
    <b>aborts_if</b> !(txn_gas_price * gas_used &lt;= <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>);
    <b>let</b> transaction_fee_amount = txn_gas_price * gas_used;
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> pre_account = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);
    <b>let</b> <b>post</b> <a href="account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);
    <b>aborts_if</b> !<b>exists</b>&lt;CoinStore&lt;SupraCoin&gt;&gt;(gas_payer);
    <b>aborts_if</b> !<b>exists</b>&lt;Account&gt;(addr);
    <b>aborts_if</b> !(<b>global</b>&lt;Account&gt;(addr).sequence_number &lt; <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>);
    <b>ensures</b> <a href="account.md#0x1_account">account</a>.sequence_number == pre_account.sequence_number + 1;
    <b>let</b> collect_fee_enabled = <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES">features::COLLECT_AND_DISTRIBUTE_GAS_FEES</a>);
    <b>let</b> collected_fees = <b>global</b>&lt;CollectedFeesPerBlock&gt;(@supra_framework).amount;
    <b>let</b> aggr = collected_fees.value;
    <b>let</b> aggr_val = <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr);
    <b>let</b> aggr_lim = <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(aggr);
    // This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    <b>aborts_if</b> collect_fee_enabled && !<b>exists</b>&lt;CollectedFeesPerBlock&gt;(@supra_framework);
    <b>aborts_if</b> collect_fee_enabled && transaction_fee_amount &gt; 0 && aggr_val + transaction_fee_amount &gt; aggr_lim;
    <b>let</b> amount_to_burn = <b>if</b> (collect_fee_enabled) {
        0
    } <b>else</b> {
        transaction_fee_amount - storage_fee_refunded
    };
    <b>let</b> apt_addr = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;SupraCoin&gt;().account_address;
    <b>let</b> maybe_apt_supply = <b>global</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(apt_addr).supply;
    <b>let</b> total_supply_enabled = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_apt_supply);
    <b>let</b> apt_supply = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(maybe_apt_supply);
    <b>let</b> apt_supply_value = <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(apt_supply);
    <b>let</b> <b>post</b> post_maybe_apt_supply = <b>global</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(apt_addr).supply;
    <b>let</b> <b>post</b> post_apt_supply = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(post_maybe_apt_supply);
    <b>let</b> <b>post</b> post_apt_supply_value = <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(post_apt_supply);
    <b>aborts_if</b> amount_to_burn &gt; 0 && !<b>exists</b>&lt;SupraCoinCapabilities&gt;(@supra_framework);
    <b>aborts_if</b> amount_to_burn &gt; 0 && !<b>exists</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(apt_addr);
    <b>aborts_if</b> amount_to_burn &gt; 0 && total_supply_enabled && apt_supply_value &lt; amount_to_burn;
    <b>ensures</b> total_supply_enabled ==&gt; apt_supply_value - amount_to_burn == post_apt_supply_value;
    <b>let</b> amount_to_mint = <b>if</b> (collect_fee_enabled) {
        storage_fee_refunded
    } <b>else</b> {
        storage_fee_refunded - transaction_fee_amount
    };
    <b>let</b> total_supply = <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;SupraCoin&gt;;
    <b>let</b> <b>post</b> post_total_supply = <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;SupraCoin&gt;;
    <b>aborts_if</b> amount_to_mint &gt; 0 && !<b>exists</b>&lt;CoinStore&lt;SupraCoin&gt;&gt;(addr);
    <b>aborts_if</b> amount_to_mint &gt; 0 && !<b>exists</b>&lt;SupraCoinMintCapability&gt;(@supra_framework);
    <b>aborts_if</b> amount_to_mint &gt; 0 && total_supply + amount_to_mint &gt; MAX_U128;
    <b>ensures</b> amount_to_mint &gt; 0 ==&gt; post_total_supply == total_supply + amount_to_mint;
    <b>let</b> aptos_addr = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;SupraCoin&gt;().account_address;
    <b>aborts_if</b> (amount_to_mint != 0) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;SupraCoin&gt;&gt;(aptos_addr);
    <b>include</b> <a href="coin.md#0x1_coin_CoinAddAbortsIf">coin::CoinAddAbortsIf</a>&lt;SupraCoin&gt; { amount: amount_to_mint };
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
