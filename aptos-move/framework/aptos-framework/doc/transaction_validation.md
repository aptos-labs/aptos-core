
<a id="0x1_transaction_validation"></a>

# Module `0x1::transaction_validation`



-  [Enum `ReplayProtector`](#0x1_transaction_validation_ReplayProtector)
-  [Resource `TransactionValidation`](#0x1_transaction_validation_TransactionValidation)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_transaction_validation_initialize)
-  [Function `check_for_replay_protection_regular_txn`](#0x1_transaction_validation_check_for_replay_protection_regular_txn)
-  [Function `check_for_replay_protection_orderless_txn`](#0x1_transaction_validation_check_for_replay_protection_orderless_txn)
-  [Function `prologue_common`](#0x1_transaction_validation_prologue_common)
-  [Function `script_prologue`](#0x1_transaction_validation_script_prologue)
-  [Function `script_prologue_extended`](#0x1_transaction_validation_script_prologue_extended)
-  [Function `script_prologue_payload_v2_extension`](#0x1_transaction_validation_script_prologue_payload_v2_extension)
-  [Function `multi_agent_script_prologue`](#0x1_transaction_validation_multi_agent_script_prologue)
-  [Function `multi_agent_script_prologue_extended`](#0x1_transaction_validation_multi_agent_script_prologue_extended)
-  [Function `multi_agent_script_prologue_payload_v2_extension`](#0x1_transaction_validation_multi_agent_script_prologue_payload_v2_extension)
-  [Function `multi_agent_common_prologue`](#0x1_transaction_validation_multi_agent_common_prologue)
-  [Function `fee_payer_script_prologue`](#0x1_transaction_validation_fee_payer_script_prologue)
-  [Function `fee_payer_script_prologue_extended`](#0x1_transaction_validation_fee_payer_script_prologue_extended)
-  [Function `fee_payer_script_prologue_payload_v2_extension`](#0x1_transaction_validation_fee_payer_script_prologue_payload_v2_extension)
-  [Function `epilogue`](#0x1_transaction_validation_epilogue)
-  [Function `epilogue_extended`](#0x1_transaction_validation_epilogue_extended)
-  [Function `epilogue_gas_payer`](#0x1_transaction_validation_epilogue_gas_payer)
-  [Function `epilogue_gas_payer_extended`](#0x1_transaction_validation_epilogue_gas_payer_extended)
-  [Function `epilogue_payload_v2_extension`](#0x1_transaction_validation_epilogue_payload_v2_extension)
-  [Function `epilogue_gas_payer_payload_v2_extension`](#0x1_transaction_validation_epilogue_gas_payer_payload_v2_extension)
-  [Function `skip_auth_key_check`](#0x1_transaction_validation_skip_auth_key_check)
-  [Function `skip_gas_payment`](#0x1_transaction_validation_skip_gas_payment)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="nonce_validation.md#0x1_nonce_validation">0x1::nonce_validation</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="transaction_fee.md#0x1_transaction_fee">0x1::transaction_fee</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_transaction_validation_ReplayProtector"></a>

## Enum `ReplayProtector`



<pre><code>enum <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Nonce</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>SequenceNumber</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

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



<a id="0x1_transaction_validation_PROLOGUE_NONCE_ALREADY_USED"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_NONCE_ALREADY_USED">PROLOGUE_NONCE_ALREADY_USED</a>: u64 = 1012;
</code></pre>



<a id="0x1_transaction_validation_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(
    aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    // module_prologue_name is deprecated and not used.
    module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>move_to</b>(aptos_framework, <a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a> {
        module_addr: @aptos_framework,
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

<a id="0x1_transaction_validation_check_for_replay_protection_regular_txn"></a>

## Function `check_for_replay_protection_regular_txn`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_regular_txn">check_for_replay_protection_regular_txn</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, txn_sequence_number: u64, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_regular_txn">check_for_replay_protection_regular_txn</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: <b>address</b>,
    txn_sequence_number: u64,
    txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    is_simulation: bool,
) {
    <b>let</b> transaction_sender = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);
    <b>if</b> (
        transaction_sender == gas_payer
            || <a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)
            || !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_sponsored_automatic_account_creation_enabled">features::sponsored_automatic_account_creation_enabled</a>()
            || txn_sequence_number &gt; 0
    ) {
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>));
        <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_simulation_enhancement_enabled">features::transaction_simulation_enhancement_enabled</a>() ||
                !<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &txn_authentication_key)) {
            <b>assert</b>!(
                txn_authentication_key == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(transaction_sender),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
            )
        };

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

        <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_simulation_enhancement_enabled">features::transaction_simulation_enhancement_enabled</a>() ||
                !<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &txn_authentication_key)) {
            <b>assert</b>!(
                txn_authentication_key == <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&transaction_sender),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
            );
        }
    };
}
</code></pre>



</details>

<a id="0x1_transaction_validation_check_for_replay_protection_orderless_txn"></a>

## Function `check_for_replay_protection_orderless_txn`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_orderless_txn">check_for_replay_protection_orderless_txn</a>(sender: <b>address</b>, nonce: u64, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_expiration_time: u64, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_orderless_txn">check_for_replay_protection_orderless_txn</a>(
    sender: <b>address</b>,
    nonce: u64,
    txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    txn_expiration_time: u64,
    is_simulation: bool,
) {
    // Question: Is this logic correct?
    <b>if</b> (is_simulation) {
        <b>assert</b>!(<a href="nonce_validation.md#0x1_nonce_validation_check_nonce">nonce_validation::check_nonce</a>(sender, nonce, txn_expiration_time), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_NONCE_ALREADY_USED">PROLOGUE_NONCE_ALREADY_USED</a>));
    }
    <b>else</b> {
        <b>assert</b>!(<a href="nonce_validation.md#0x1_nonce_validation_check_and_insert_nonce">nonce_validation::check_and_insert_nonce</a>(sender, nonce, txn_expiration_time), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_NONCE_ALREADY_USED">PROLOGUE_NONCE_ALREADY_USED</a>));
    };

    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_simulation_enhancement_enabled">features::transaction_simulation_enhancement_enabled</a>() ||
        !<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &txn_authentication_key)) {
        <b>assert</b>!(
            txn_authentication_key == <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&sender),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
        );
    }
}
</code></pre>



</details>

<a id="0x1_transaction_validation_prologue_common"></a>

## Function `prologue_common`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: <b>address</b>,
    replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>,
    txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; txn_expiration_time,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED">PROLOGUE_ETRANSACTION_EXPIRED</a>),
    );
    <b>assert</b>!(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() == <a href="chain_id.md#0x1_chain_id">chain_id</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID">PROLOGUE_EBAD_CHAIN_ID</a>));

    match (replay_protector) {
        SequenceNumber(txn_sequence_number) =&gt; {
            <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_regular_txn">check_for_replay_protection_regular_txn</a>(
                sender,
                gas_payer,
                txn_sequence_number,
                txn_authentication_key,
                is_simulation,
            );
        },
        Nonce(nonce) =&gt; {
            <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_orderless_txn">check_for_replay_protection_orderless_txn</a>(
                <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender),
                nonce,
                txn_authentication_key,
                txn_expiration_time,
                is_simulation,
            );
        }
    };

    <b>let</b> max_transaction_fee = txn_gas_price * txn_max_gas_units;

    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_simulation_enhancement_enabled">features::transaction_simulation_enhancement_enabled</a>() || !<a href="transaction_validation.md#0x1_transaction_validation_skip_gas_payment">skip_gas_payment</a>(is_simulation, gas_payer)) {
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
            <b>assert</b>!(
                <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">aptos_account::is_fungible_balance_at_least</a>(gas_payer, max_transaction_fee),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)
            );
        } <b>else</b> {
            <b>assert</b>!(
                <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;AptosCoin&gt;(gas_payer, max_transaction_fee),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)
            );
        }
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
    // prologue_common <b>with</b> is_simulation set <b>to</b> <b>false</b> behaves identically <b>to</b> the original script_prologue function.
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        gas_payer,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        txn_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        <b>false</b>,
    )
}
</code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue_extended"></a>

## Function `script_prologue_extended`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue_extended">script_prologue_extended</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue_extended">script_prologue_extended</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    txn_sequence_number: u64,
    txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    is_simulation: bool,
) {
    <b>let</b> gas_payer = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        gas_payer,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        txn_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    )
}
</code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue_payload_v2_extension"></a>

## Function `script_prologue_payload_v2_extension`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue_payload_v2_extension">script_prologue_payload_v2_extension</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue_payload_v2_extension">script_prologue_payload_v2_extension</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>,
    txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    is_simulation: bool,
) {
    <b>let</b> gas_payer = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        gas_payer,
        replay_protector,
        txn_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    )
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
    // prologue_common and multi_agent_common_prologue <b>with</b> is_simulation set <b>to</b> <b>false</b> behaves identically <b>to</b> the
    // original multi_agent_script_prologue function.
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        sender_addr,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        <b>false</b>,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes, <b>false</b>);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_script_prologue_extended"></a>

## Function `multi_agent_script_prologue_extended`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue_extended">multi_agent_script_prologue_extended</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue_extended">multi_agent_script_prologue_extended</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    txn_sequence_number: u64,
    txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <b>let</b> sender_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        sender_addr,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes, is_simulation);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_script_prologue_payload_v2_extension"></a>

## Function `multi_agent_script_prologue_payload_v2_extension`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue_payload_v2_extension">multi_agent_script_prologue_payload_v2_extension</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue_payload_v2_extension">multi_agent_script_prologue_payload_v2_extension</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>,
    txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <b>let</b> sender_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&sender);
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        sender_addr,
        replay_protector,
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes, is_simulation);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_common_prologue"></a>

## Function `multi_agent_common_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    is_simulation: bool,
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
                <a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[j]);
            <b>invariant</b> <b>forall</b> j in 0..i:
                secondary_signer_public_key_hashes[j] == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[j]) ||
                    (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_simulation_enhancement_enabled">features::spec_simulation_enhancement_enabled</a>() && is_simulation && <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(secondary_signer_public_key_hashes[j]));
        };
        (i &lt; num_secondary_signers)
    }) {
        <b>let</b> secondary_address = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&secondary_signer_addresses, i);
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>));

        <b>let</b> signer_public_key_hash = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&secondary_signer_public_key_hashes, i);
        <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_simulation_enhancement_enabled">features::transaction_simulation_enhancement_enabled</a>() ||
                !<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &signer_public_key_hash)) {
            <b>assert</b>!(
                signer_public_key_hash == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_address),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
            )
        };
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
    // prologue_common and multi_agent_common_prologue <b>with</b> is_simulation set <b>to</b> <b>false</b> behaves identically <b>to</b> the
    // original fee_payer_script_prologue function.
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        fee_payer_address,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        <b>false</b>,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes, <b>false</b>);
    <b>assert</b>!(
        fee_payer_public_key_hash == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(fee_payer_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
    );
}
</code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue_extended"></a>

## Function `fee_payer_script_prologue_extended`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue_extended">fee_payer_script_prologue_extended</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue_extended">fee_payer_script_prologue_extended</a>(
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
    is_simulation: bool,
) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_fee_payer_enabled">features::fee_payer_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED">PROLOGUE_EFEE_PAYER_NOT_ENABLED</a>));
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        fee_payer_address,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes, is_simulation);
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_simulation_enhancement_enabled">features::transaction_simulation_enhancement_enabled</a>() ||
        !<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &fee_payer_public_key_hash)) {
        <b>assert</b>!(
            fee_payer_public_key_hash == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(fee_payer_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
        )
    }
}
</code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue_payload_v2_extension"></a>

## Function `fee_payer_script_prologue_payload_v2_extension`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue_payload_v2_extension">fee_payer_script_prologue_payload_v2_extension</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue_payload_v2_extension">fee_payer_script_prologue_payload_v2_extension</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>,
    txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    fee_payer_address: <b>address</b>,
    fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_fee_payer_enabled">features::fee_payer_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED">PROLOGUE_EFEE_PAYER_NOT_ENABLED</a>));
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        sender,
        fee_payer_address,
        replay_protector,
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes, is_simulation);
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_simulation_enhancement_enabled">features::transaction_simulation_enhancement_enabled</a>() ||
        !<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &fee_payer_public_key_hash)) {
        <b>assert</b>!(
            fee_payer_public_key_hash == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(fee_payer_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
        )
    }
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
    gas_units_remaining: u64,
) {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<a href="account.md#0x1_account">account</a>, addr, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_extended"></a>

## Function `epilogue_extended`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_extended">epilogue_extended</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_extended">epilogue_extended</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64,
    is_simulation: bool,
) {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_extended">epilogue_gas_payer_extended</a>(<a href="account.md#0x1_account">account</a>, addr, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining, is_simulation);
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
    gas_units_remaining: u64,
) {
    // epilogue_gas_payer_extended <b>with</b> is_simulation set <b>to</b> <b>false</b> behaves identically <b>to</b> the original
    // epilogue_gas_payer function.
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_extended">epilogue_gas_payer_extended</a>(
        <a href="account.md#0x1_account">account</a>,
        gas_payer,
        storage_fee_refunded,
        txn_gas_price,
        txn_max_gas_units,
        gas_units_remaining,
        <b>false</b>,
    );
}
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer_extended"></a>

## Function `epilogue_gas_payer_extended`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_extended">epilogue_gas_payer_extended</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_extended">epilogue_gas_payer_extended</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: <b>address</b>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64,
    is_simulation: bool,
) {
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_payload_v2_extension">epilogue_gas_payer_payload_v2_extension</a>(
        <a href="account.md#0x1_account">account</a>,
        gas_payer,
        storage_fee_refunded,
        txn_gas_price,
        txn_max_gas_units,
        gas_units_remaining,
        is_simulation,
        <b>false</b>
    )
}
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_payload_v2_extension"></a>

## Function `epilogue_payload_v2_extension`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_payload_v2_extension">epilogue_payload_v2_extension</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool, is_orderless_txn: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_payload_v2_extension">epilogue_payload_v2_extension</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64,
    is_simulation: bool,
    is_orderless_txn: bool,
) {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_payload_v2_extension">epilogue_gas_payer_payload_v2_extension</a>(<a href="account.md#0x1_account">account</a>, addr, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining, is_simulation, is_orderless_txn);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer_payload_v2_extension"></a>

## Function `epilogue_gas_payer_payload_v2_extension`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_payload_v2_extension">epilogue_gas_payer_payload_v2_extension</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool, is_orderless_txn: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_payload_v2_extension">epilogue_gas_payer_payload_v2_extension</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: <b>address</b>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64,
    is_simulation: bool,
    is_orderless_txn: bool,
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
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_simulation_enhancement_enabled">features::transaction_simulation_enhancement_enabled</a>() || !<a href="transaction_validation.md#0x1_transaction_validation_skip_gas_payment">skip_gas_payment</a>(is_simulation, gas_payer)) {
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
            <b>assert</b>!(
                <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">aptos_account::is_fungible_balance_at_least</a>(gas_payer, transaction_fee_amount),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>),
            );
        } <b>else</b> {
            <b>assert</b>!(
                <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;AptosCoin&gt;(gas_payer, transaction_fee_amount),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>),
            );
        };

        <b>if</b> (transaction_fee_amount &gt; storage_fee_refunded) {
            <b>let</b> burn_amount = transaction_fee_amount - storage_fee_refunded;
            <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">transaction_fee::burn_fee</a>(gas_payer, burn_amount);
        } <b>else</b> <b>if</b> (transaction_fee_amount &lt; storage_fee_refunded) {
            <b>let</b> mint_amount = storage_fee_refunded - transaction_fee_amount;
            <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">transaction_fee::mint_and_refund</a>(gas_payer, mint_amount)
        };
    };

    <b>if</b> (!is_orderless_txn) {
        // Increment sequence number
        <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
        <a href="account.md#0x1_account_increment_sequence_number">account::increment_sequence_number</a>(addr);
    }
}
</code></pre>



</details>

<a id="0x1_transaction_validation_skip_auth_key_check"></a>

## Function `skip_auth_key_check`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation: bool, auth_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation: bool, auth_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    is_simulation && <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(auth_key)
}
</code></pre>



</details>

<a id="0x1_transaction_validation_skip_gas_payment"></a>

## Function `skip_gas_payment`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_skip_gas_payment">skip_gas_payment</a>(is_simulation: bool, gas_payer: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_skip_gas_payment">skip_gas_payment</a>(is_simulation: bool, gas_payer: <b>address</b>): bool {
    is_simulation && gas_payer == @0x0
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
