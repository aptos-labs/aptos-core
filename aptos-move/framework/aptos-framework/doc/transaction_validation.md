
<a id="0x1_transaction_validation"></a>

# Module `0x1::transaction_validation`



-  [Enum `ReplayProtector`](#0x1_transaction_validation_ReplayProtector)
-  [Resource `TransactionValidation`](#0x1_transaction_validation_TransactionValidation)
-  [Struct `GasPermission`](#0x1_transaction_validation_GasPermission)
-  [Constants](#@Constants_0)
-  [Function `grant_gas_permission`](#0x1_transaction_validation_grant_gas_permission)
-  [Function `revoke_gas_permission`](#0x1_transaction_validation_revoke_gas_permission)
-  [Function `initialize`](#0x1_transaction_validation_initialize)
-  [Function `allow_missing_txn_authentication_key`](#0x1_transaction_validation_allow_missing_txn_authentication_key)
-  [Function `prologue_common`](#0x1_transaction_validation_prologue_common)
-  [Function `check_for_replay_protection_regular_txn`](#0x1_transaction_validation_check_for_replay_protection_regular_txn)
-  [Function `check_for_replay_protection_orderless_txn`](#0x1_transaction_validation_check_for_replay_protection_orderless_txn)
-  [Function `script_prologue`](#0x1_transaction_validation_script_prologue)
-  [Function `script_prologue_extended`](#0x1_transaction_validation_script_prologue_extended)
-  [Function `multi_agent_script_prologue`](#0x1_transaction_validation_multi_agent_script_prologue)
-  [Function `multi_agent_script_prologue_extended`](#0x1_transaction_validation_multi_agent_script_prologue_extended)
-  [Function `multi_agent_common_prologue`](#0x1_transaction_validation_multi_agent_common_prologue)
-  [Function `fee_payer_script_prologue`](#0x1_transaction_validation_fee_payer_script_prologue)
-  [Function `fee_payer_script_prologue_extended`](#0x1_transaction_validation_fee_payer_script_prologue_extended)
-  [Function `epilogue`](#0x1_transaction_validation_epilogue)
-  [Function `epilogue_extended`](#0x1_transaction_validation_epilogue_extended)
-  [Function `epilogue_gas_payer`](#0x1_transaction_validation_epilogue_gas_payer)
-  [Function `epilogue_gas_payer_extended`](#0x1_transaction_validation_epilogue_gas_payer_extended)
-  [Function `skip_auth_key_check`](#0x1_transaction_validation_skip_auth_key_check)
-  [Function `skip_gas_payment`](#0x1_transaction_validation_skip_gas_payment)
-  [Function `unified_prologue`](#0x1_transaction_validation_unified_prologue)
-  [Function `unified_prologue_fee_payer`](#0x1_transaction_validation_unified_prologue_fee_payer)
-  [Function `unified_epilogue`](#0x1_transaction_validation_unified_epilogue)
-  [Function `unified_prologue_v2`](#0x1_transaction_validation_unified_prologue_v2)
-  [Function `unified_prologue_fee_payer_v2`](#0x1_transaction_validation_unified_prologue_fee_payer_v2)
-  [Function `unified_epilogue_v2`](#0x1_transaction_validation_unified_epilogue_v2)
-  [Function `scheduled_txn_epilogue`](#0x1_transaction_validation_scheduled_txn_epilogue)
-  [Function `scheduled_txn_cleanup`](#0x1_transaction_validation_scheduled_txn_cleanup)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `grant_gas_permission`](#@Specification_1_grant_gas_permission)
    -  [Function `revoke_gas_permission`](#@Specification_1_revoke_gas_permission)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `prologue_common`](#@Specification_1_prologue_common)
    -  [Function `check_for_replay_protection_regular_txn`](#@Specification_1_check_for_replay_protection_regular_txn)
    -  [Function `check_for_replay_protection_orderless_txn`](#@Specification_1_check_for_replay_protection_orderless_txn)
    -  [Function `script_prologue`](#@Specification_1_script_prologue)
    -  [Function `script_prologue_extended`](#@Specification_1_script_prologue_extended)
    -  [Function `multi_agent_script_prologue`](#@Specification_1_multi_agent_script_prologue)
    -  [Function `multi_agent_script_prologue_extended`](#@Specification_1_multi_agent_script_prologue_extended)
    -  [Function `multi_agent_common_prologue`](#@Specification_1_multi_agent_common_prologue)
    -  [Function `fee_payer_script_prologue`](#@Specification_1_fee_payer_script_prologue)
    -  [Function `fee_payer_script_prologue_extended`](#@Specification_1_fee_payer_script_prologue_extended)
    -  [Function `epilogue`](#@Specification_1_epilogue)
    -  [Function `epilogue_extended`](#@Specification_1_epilogue_extended)
    -  [Function `epilogue_gas_payer`](#@Specification_1_epilogue_gas_payer)
    -  [Function `epilogue_gas_payer_extended`](#@Specification_1_epilogue_gas_payer_extended)
    -  [Function `unified_prologue`](#@Specification_1_unified_prologue)
    -  [Function `unified_prologue_fee_payer`](#@Specification_1_unified_prologue_fee_payer)
    -  [Function `unified_epilogue`](#@Specification_1_unified_epilogue)
    -  [Function `unified_prologue_v2`](#@Specification_1_unified_prologue_v2)
    -  [Function `unified_prologue_fee_payer_v2`](#@Specification_1_unified_prologue_fee_payer_v2)
    -  [Function `unified_epilogue_v2`](#@Specification_1_unified_epilogue_v2)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="account_abstraction.md#0x1_account_abstraction">0x1::account_abstraction</a>;
<b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="nonce_validation.md#0x1_nonce_validation">0x1::nonce_validation</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="scheduled_txns.md#0x1_scheduled_txns">0x1::scheduled_txns</a>;
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

<a id="0x1_transaction_validation_GasPermission"></a>

## Struct `GasPermission`



<pre><code><b>struct</b> <a href="transaction_validation.md#0x1_transaction_validation_GasPermission">GasPermission</a> <b>has</b> <b>copy</b>, drop, store
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


<a id="0x1_transaction_validation_MAX_U64"></a>

MSB is used to indicate a gas payer tx


<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_transaction_validation_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>: u64 = 6;
</code></pre>



<a id="0x1_transaction_validation_MAX_EXPIRATION_TIME_SECONDS_FOR_ORDERLESS_TXNS"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_MAX_EXPIRATION_TIME_SECONDS_FOR_ORDERLESS_TXNS">MAX_EXPIRATION_TIME_SECONDS_FOR_ORDERLESS_TXNS</a>: u64 = 65;
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



<a id="0x1_transaction_validation_PROLOGUE_ENONCE_ALREADY_USED"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ENONCE_ALREADY_USED">PROLOGUE_ENONCE_ALREADY_USED</a>: u64 = 1012;
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



<a id="0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE">PROLOGUE_ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE</a>: u64 = 1013;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED">PROLOGUE_ETRANSACTION_EXPIRED</a>: u64 = 1006;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_PERMISSIONED_GAS_LIMIT_INSUFFICIENT"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_PERMISSIONED_GAS_LIMIT_INSUFFICIENT">PROLOGUE_PERMISSIONED_GAS_LIMIT_INSUFFICIENT</a>: u64 = 1011;
</code></pre>



<a id="0x1_transaction_validation_grant_gas_permission"></a>

## Function `grant_gas_permission`

Permission management

Master signer grant permissioned signer ability to consume a given amount of gas in octas.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_grant_gas_permission">grant_gas_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_grant_gas_permission">grant_gas_permission</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_amount: u64
) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_increase">permissioned_signer::authorize_increase</a>(
        master,
        permissioned,
        (gas_amount <b>as</b> u256),
        <a href="transaction_validation.md#0x1_transaction_validation_GasPermission">GasPermission</a> {}
    )
}
</code></pre>



</details>

<a id="0x1_transaction_validation_revoke_gas_permission"></a>

## Function `revoke_gas_permission`

Removing permissions from permissioned signer.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_revoke_gas_permission">revoke_gas_permission</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_revoke_gas_permission">revoke_gas_permission</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission">permissioned_signer::revoke_permission</a>(permissioned, <a href="transaction_validation.md#0x1_transaction_validation_GasPermission">GasPermission</a> {})
}
</code></pre>



</details>

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

<a id="0x1_transaction_validation_allow_missing_txn_authentication_key"></a>

## Function `allow_missing_txn_authentication_key`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_allow_missing_txn_authentication_key">allow_missing_txn_authentication_key</a>(transaction_sender: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_allow_missing_txn_authentication_key">allow_missing_txn_authentication_key</a>(transaction_sender: <b>address</b>): bool {
    // aa verifies authentication itself
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_derivable_account_abstraction_enabled">features::is_derivable_account_abstraction_enabled</a>()
        || (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_account_abstraction_enabled">features::is_account_abstraction_enabled</a>() && <a href="account_abstraction.md#0x1_account_abstraction_using_dispatchable_authenticator">account_abstraction::using_dispatchable_authenticator</a>(transaction_sender))
}
</code></pre>



</details>

<a id="0x1_transaction_validation_prologue_common"></a>

## Function `prologue_common`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>,
    txn_authentication_key: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <b>let</b> sender_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>let</b> gas_payer_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(gas_payer);
    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; txn_expiration_time,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED">PROLOGUE_ETRANSACTION_EXPIRED</a>),
    );
    <b>assert</b>!(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() == <a href="chain_id.md#0x1_chain_id">chain_id</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID">PROLOGUE_EBAD_CHAIN_ID</a>));

    // TODO[Orderless]: Here, we are maintaining the same order of validation steps <b>as</b> before orderless txns were introduced.
    // Ideally, do the replay protection check in the end after the authentication key check and gas payment checks.

    // Check <b>if</b> the authentication key is valid
    <b>if</b> (!<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &txn_authentication_key)) {
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&txn_authentication_key)) {
            <b>assert</b>!(
                txn_authentication_key == <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(sender_address)),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),
            );
        } <b>else</b> {
            <b>assert</b>!(
                <a href="transaction_validation.md#0x1_transaction_validation_allow_missing_txn_authentication_key">allow_missing_txn_authentication_key</a>(sender_address),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>)
            );
        };
    };

    // Check for replay protection
    match (replay_protector) {
        SequenceNumber(txn_sequence_number) =&gt; {
            <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_regular_txn">check_for_replay_protection_regular_txn</a>(
                sender_address,
                gas_payer_address,
                txn_sequence_number,
            );
        },
        Nonce(nonce) =&gt; {
            <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_orderless_txn">check_for_replay_protection_orderless_txn</a>(
                sender_address,
                nonce,
                txn_expiration_time,
            );
        }
    };

    // Check <b>if</b> the gas payer <b>has</b> enough balance <b>to</b> pay for the transaction
    <b>let</b> max_transaction_fee = txn_gas_price * txn_max_gas_units;
    <b>if</b> (!<a href="transaction_validation.md#0x1_transaction_validation_skip_gas_payment">skip_gas_payment</a>(
        is_simulation,
        gas_payer_address
    )) {
        <b>assert</b>!(
            <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">permissioned_signer::check_permission_capacity_above</a>(
                gas_payer,
                (max_transaction_fee <b>as</b> u256),
                <a href="transaction_validation.md#0x1_transaction_validation_GasPermission">GasPermission</a> {}
            ),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_PERMISSIONED_GAS_LIMIT_INSUFFICIENT">PROLOGUE_PERMISSIONED_GAS_LIMIT_INSUFFICIENT</a>)
        );
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
            <b>assert</b>!(
                <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">aptos_account::is_fungible_balance_at_least</a>(gas_payer_address, max_transaction_fee),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)
            );
        } <b>else</b> {
            <b>assert</b>!(
                <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;AptosCoin&gt;(gas_payer_address, max_transaction_fee),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)
            );
        }
    };
}
</code></pre>



</details>

<a id="0x1_transaction_validation_check_for_replay_protection_regular_txn"></a>

## Function `check_for_replay_protection_regular_txn`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_regular_txn">check_for_replay_protection_regular_txn</a>(sender_address: <b>address</b>, gas_payer_address: <b>address</b>, txn_sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_regular_txn">check_for_replay_protection_regular_txn</a>(
    sender_address: <b>address</b>,
    gas_payer_address: <b>address</b>,
    txn_sequence_number: u64,
) {
    <b>if</b> (
        sender_address == gas_payer_address
            || <a href="account.md#0x1_account_exists_at">account::exists_at</a>(sender_address)
            || !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_sponsored_automatic_account_creation_enabled">features::sponsored_automatic_account_creation_enabled</a>()
            || txn_sequence_number &gt; 0
    ) {
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(sender_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>));
        <b>let</b> account_sequence_number = <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(sender_address);
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
    };
}
</code></pre>



</details>

<a id="0x1_transaction_validation_check_for_replay_protection_orderless_txn"></a>

## Function `check_for_replay_protection_orderless_txn`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_orderless_txn">check_for_replay_protection_orderless_txn</a>(sender: <b>address</b>, nonce: u64, txn_expiration_time: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_orderless_txn">check_for_replay_protection_orderless_txn</a>(
    sender: <b>address</b>,
    nonce: u64,
    txn_expiration_time: u64,
) {
    // prologue_common already checks that the current_time &gt; txn_expiration_time
    <b>assert</b>!(
        txn_expiration_time &lt;= <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() + <a href="transaction_validation.md#0x1_transaction_validation_MAX_EXPIRATION_TIME_SECONDS_FOR_ORDERLESS_TXNS">MAX_EXPIRATION_TIME_SECONDS_FOR_ORDERLESS_TXNS</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE">PROLOGUE_ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE</a>),
    );
    <b>assert</b>!(<a href="nonce_validation.md#0x1_nonce_validation_check_and_insert_nonce">nonce_validation::check_and_insert_nonce</a>(sender, nonce, txn_expiration_time), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ENONCE_ALREADY_USED">PROLOGUE_ENONCE_ALREADY_USED</a>));
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
    // prologue_common <b>with</b> is_simulation set <b>to</b> <b>false</b> behaves identically <b>to</b> the original script_prologue function.
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        &sender,
        &sender,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(txn_public_key),
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
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        &sender,
        &sender,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(txn_public_key),
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
    // prologue_common and multi_agent_common_prologue <b>with</b> is_simulation set <b>to</b> <b>false</b> behaves identically <b>to</b> the
    // original multi_agent_script_prologue function.
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        &sender,
        &sender,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(txn_sender_public_key),
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        <b>false</b>,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(
        secondary_signer_addresses,
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map">vector::map</a>(secondary_signer_public_key_hashes, |x| <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(x)),
        <b>false</b>
    );
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
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        &sender,
        &sender,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(txn_sender_public_key),
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(
        secondary_signer_addresses,
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map">vector::map</a>(secondary_signer_public_key_hashes, |x| <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(x)),
        is_simulation
    );
}
</code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_common_prologue"></a>

## Function `multi_agent_common_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    is_simulation: bool,
) {
    <b>let</b> num_secondary_signers = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&secondary_signer_addresses);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&secondary_signer_public_key_hashes) == num_secondary_signers,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH">PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH</a>),
    );

    <b>let</b> i = 0;
    <b>while</b> ({
        // <b>spec</b> {
        //     <b>invariant</b> i &lt;= num_secondary_signers;
        //     <b>invariant</b> <b>forall</b> j in 0..i:
        //         <a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[j]);
        //     <b>invariant</b> <b>forall</b> j in 0..i:
        //         secondary_signer_public_key_hashes[j] == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[j]) ||
        //             (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_simulation_enhancement_enabled">features::spec_simulation_enhancement_enabled</a>() && is_simulation && <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(secondary_signer_public_key_hashes[j]));
        //         account::account_resource_exists_at(secondary_signer_addresses[j])
        //         && secondary_signer_public_key_hashes[j]
        //             == <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[j])
        //             || features::account_abstraction_enabled() && account_abstraction::using_native_authenticator(
        //             secondary_signer_addresses[j]
        //         ) && <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(secondary_signer_public_key_hashes[j]) == account_abstraction::native_authenticator(
        //         <a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[j])
        //         && secondary_signer_public_key_hashes[j]
        //             == <a href="account.md#0x1_account_spec_get_authentication_key">account::spec_get_authentication_key</a>(secondary_signer_addresses[j])
        //             || features::spec_account_abstraction_enabled() && account_abstraction::using_native_authenticator(
        //             secondary_signer_addresses[j]
        //         ) && <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(
        //             secondary_signer_public_key_hashes[j]
        //         ) == account_abstraction::spec_native_authenticator(
        //             secondary_signer_addresses[j]
        //         );
        // };
        (i &lt; num_secondary_signers)
    }) {
        <b>let</b> secondary_address = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&secondary_signer_addresses, i);
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>));
        <b>let</b> signer_public_key_hash = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&secondary_signer_public_key_hashes, i);
        <b>if</b> (!<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &signer_public_key_hash)) {
            <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&signer_public_key_hash)) {
                <b>assert</b>!(
                    signer_public_key_hash == <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_address)),
                    <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>)
                );
            } <b>else</b> {
                <b>assert</b>!(
                    <a href="transaction_validation.md#0x1_transaction_validation_allow_missing_txn_authentication_key">allow_missing_txn_authentication_key</a>(secondary_address),
                    <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>)
                )
            };
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
        &sender,
        &<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(fee_payer_address),
        ReplayProtector::SequenceNumber(txn_sequence_number),
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(txn_sender_public_key),
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        <b>false</b>,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(
        secondary_signer_addresses,
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map">vector::map</a>(secondary_signer_public_key_hashes, |x| <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(x)),
        <b>false</b>
    );
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
        &sender,
        &<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(fee_payer_address),
        ReplayProtector::SequenceNumber(txn_sequence_number),
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(txn_sender_public_key),
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(
        secondary_signer_addresses,
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map">vector::map</a>(secondary_signer_public_key_hashes, |x| <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(x)),
        is_simulation
    );
    <b>if</b> (!<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(fee_payer_public_key_hash))) {
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
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(
        <a href="account.md#0x1_account">account</a>,
        addr,
        storage_fee_refunded,
        txn_gas_price,
        txn_max_gas_units,
        gas_units_remaining
    );
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
    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_extended">epilogue_gas_payer_extended</a>(
        <a href="account.md#0x1_account">account</a>,
        addr,
        storage_fee_refunded,
        txn_gas_price,
        txn_max_gas_units,
        gas_units_remaining,
        is_simulation
    );
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
    <b>assert</b>!(txn_max_gas_units &gt;= gas_units_remaining, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>));
    <b>let</b> gas_used = txn_max_gas_units - gas_units_remaining;

    <b>assert</b>!(
        (txn_gas_price <b>as</b> u128) * (gas_used <b>as</b> u128) &lt;= <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>)
    );
    <b>let</b> transaction_fee_amount = txn_gas_price * gas_used;

    // it's important <b>to</b> maintain the <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> <a href="code.md#0x1_code">code</a> consistent <b>with</b> vm
    // <b>to</b> do failed transaction cleanup.
    <b>if</b> (!<a href="transaction_validation.md#0x1_transaction_validation_skip_gas_payment">skip_gas_payment</a>(is_simulation, gas_payer)) {
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
            <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">transaction_fee::mint_and_refund</a>(gas_payer, mint_amount);
        };
    };

    // Increment sequence number
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_increment_sequence_number">account::increment_sequence_number</a>(addr);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_skip_auth_key_check"></a>

## Function `skip_auth_key_check`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation: bool, auth_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation: bool, auth_key: &Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): bool {
    is_simulation && (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(auth_key) || <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(auth_key)))
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

<a id="0x1_transaction_validation_unified_prologue"></a>

## Function `unified_prologue`

new set of functions


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue">unified_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_sequence_number: u64, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue">unified_prologue</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    // None means no need <b>to</b> check, i.e. either AA (<b>where</b> it is already checked) or simulation
    txn_sender_public_key: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    txn_sequence_number: u64,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_v2">unified_prologue_v2</a>(
        sender,
        txn_sender_public_key,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        secondary_signer_addresses,
        secondary_signer_public_key_hashes,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,

    )
}
</code></pre>



</details>

<a id="0x1_transaction_validation_unified_prologue_fee_payer"></a>

## Function `unified_prologue_fee_payer`

If there is no fee_payer, fee_payer = sender


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_fee_payer">unified_prologue_fee_payer</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fee_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_sequence_number: u64, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_fee_payer">unified_prologue_fee_payer</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    fee_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    // None means no need <b>to</b> check, i.e. either AA (<b>where</b> it is already checked) or simulation
    txn_sender_public_key: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    // None means no need <b>to</b> check, i.e. either AA (<b>where</b> it is already checked) or simulation
    fee_payer_public_key_hash: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    txn_sequence_number: u64,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_fee_payer_v2">unified_prologue_fee_payer_v2</a>(
        sender,
        fee_payer,
        txn_sender_public_key,
        fee_payer_public_key_hash,
        ReplayProtector::SequenceNumber(txn_sequence_number),
        secondary_signer_addresses,
        secondary_signer_public_key_hashes,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    )
}
</code></pre>



</details>

<a id="0x1_transaction_validation_unified_epilogue"></a>

## Function `unified_epilogue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_epilogue">unified_epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_epilogue">unified_epilogue</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64,
    is_simulation: bool,
) {
    <a href="transaction_validation.md#0x1_transaction_validation_unified_epilogue_v2">unified_epilogue_v2</a>(
        <a href="account.md#0x1_account">account</a>,
        gas_payer,
        storage_fee_refunded,
        txn_gas_price,
        txn_max_gas_units,
        gas_units_remaining,
        is_simulation,
        <b>false</b>,
    )
}
</code></pre>



</details>

<a id="0x1_transaction_validation_unified_prologue_v2"></a>

## Function `unified_prologue_v2`

new set of functions to support txn payload v2 format and orderless transactions


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_v2">unified_prologue_v2</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_v2">unified_prologue_v2</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    txn_sender_public_key: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        &sender,
        &sender,
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

<a id="0x1_transaction_validation_unified_prologue_fee_payer_v2"></a>

## Function `unified_prologue_fee_payer_v2`

If there is no fee_payer, fee_payer = sender


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_fee_payer_v2">unified_prologue_fee_payer_v2</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fee_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_fee_payer_v2">unified_prologue_fee_payer_v2</a>(
    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    fee_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    txn_sender_public_key: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    fee_payer_public_key_hash: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>,
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    is_simulation: bool,
) {
    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(
        &sender,
        &fee_payer,
        replay_protector,
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        is_simulation,
    );
    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes, is_simulation);
    <b>if</b> (!<a href="transaction_validation.md#0x1_transaction_validation_skip_auth_key_check">skip_auth_key_check</a>(is_simulation, &fee_payer_public_key_hash)) {
        <b>let</b> fee_payer_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&fee_payer);
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&fee_payer_public_key_hash)) {
            <b>assert</b>!(
                fee_payer_public_key_hash == <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(fee_payer_address)),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>)
            );
        } <b>else</b> {
            <b>assert</b>!(
                <a href="transaction_validation.md#0x1_transaction_validation_allow_missing_txn_authentication_key">allow_missing_txn_authentication_key</a>(fee_payer_address),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>)
            )
        };
    }
}
</code></pre>



</details>

<a id="0x1_transaction_validation_unified_epilogue_v2"></a>

## Function `unified_epilogue_v2`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_epilogue_v2">unified_epilogue_v2</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool, is_orderless_txn: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_epilogue_v2">unified_epilogue_v2</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
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

    <b>let</b> gas_payer_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&gas_payer);
    // it's important <b>to</b> maintain the <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> <a href="code.md#0x1_code">code</a> consistent <b>with</b> vm
    // <b>to</b> do failed transaction cleanup.
    <b>if</b> (!<a href="transaction_validation.md#0x1_transaction_validation_skip_gas_payment">skip_gas_payment</a>(
        is_simulation,
        gas_payer_address
    )) {
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
            <b>assert</b>!(
                <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">aptos_account::is_fungible_balance_at_least</a>(gas_payer_address, transaction_fee_amount),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>),
            );
        } <b>else</b> {
            <b>assert</b>!(
                <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;AptosCoin&gt;(gas_payer_address, transaction_fee_amount),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>),
            );
        };

        <b>if</b> (transaction_fee_amount &gt; storage_fee_refunded) {
            <b>let</b> burn_amount = transaction_fee_amount - storage_fee_refunded;
            <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">transaction_fee::burn_fee</a>(gas_payer_address, burn_amount);
            <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">permissioned_signer::check_permission_consume</a>(
                &gas_payer,
                (burn_amount <b>as</b> u256),
                <a href="transaction_validation.md#0x1_transaction_validation_GasPermission">GasPermission</a> {}
            );
        } <b>else</b> <b>if</b> (transaction_fee_amount &lt; storage_fee_refunded) {
            <b>let</b> mint_amount = storage_fee_refunded - transaction_fee_amount;
            <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">transaction_fee::mint_and_refund</a>(gas_payer_address, mint_amount);
            <a href="permissioned_signer.md#0x1_permissioned_signer_increase_limit">permissioned_signer::increase_limit</a>(
                &gas_payer,
                (mint_amount <b>as</b> u256),
                <a href="transaction_validation.md#0x1_transaction_validation_GasPermission">GasPermission</a> {}
            );
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

<a id="0x1_transaction_validation_scheduled_txn_epilogue"></a>

## Function `scheduled_txn_epilogue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_scheduled_txn_epilogue">scheduled_txn_epilogue</a>(deposit_store_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, txn_key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, scheduling_deposit: u64, gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_scheduled_txn_epilogue">scheduled_txn_epilogue</a>(
    deposit_store_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    txn_key: ScheduleMapKey,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    scheduling_deposit: u64,
    gas_units_remaining: u64
) {
    <b>assert</b>!(txn_max_gas_units &gt;= gas_units_remaining, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>));
    <b>let</b> gas_used = txn_max_gas_units - gas_units_remaining;

    <b>assert</b>!(
        (txn_gas_price <b>as</b> u128) * (gas_used <b>as</b> u128) &lt;= <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>)
    );
    <b>let</b> transaction_fee_amount = txn_gas_price * gas_used;

    <b>assert</b>!(scheduling_deposit &gt;= transaction_fee_amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>));
    <b>let</b> deposit_store_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deposit_store_owner);

    <b>if</b> (transaction_fee_amount &gt; storage_fee_refunded) {
        <b>let</b> burn_amount = transaction_fee_amount - storage_fee_refunded;
        <b>let</b> refund_from_scheduling_deposit = scheduling_deposit - burn_amount;
        <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(
            deposit_store_owner,
            <a href="account.md#0x1_account">account</a>,
            refund_from_scheduling_deposit
        );
        <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">transaction_fee::burn_fee</a>(deposit_store_addr, burn_amount);
    } <b>else</b> <b>if</b> (transaction_fee_amount &lt; storage_fee_refunded) {
        // <b>return</b> the full deposit and mint the remaining
        <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(
            deposit_store_owner,
            <a href="account.md#0x1_account">account</a>,
            scheduling_deposit
        );
        <b>let</b> mint_and_refund_amount = storage_fee_refunded - transaction_fee_amount;
        <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">transaction_fee::mint_and_refund</a>(<a href="account.md#0x1_account">account</a>, mint_and_refund_amount);
    };

    // Increment sequence number
    <a href="account.md#0x1_account_increment_sequence_number">account::increment_sequence_number</a>(<a href="account.md#0x1_account">account</a>);

    <a href="transaction_validation.md#0x1_transaction_validation_scheduled_txn_cleanup">scheduled_txn_cleanup</a>(txn_key);
}
</code></pre>



</details>

<a id="0x1_transaction_validation_scheduled_txn_cleanup"></a>

## Function `scheduled_txn_cleanup`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_scheduled_txn_cleanup">scheduled_txn_cleanup</a>(txn_key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_scheduled_txn_cleanup">scheduled_txn_cleanup</a>(txn_key: ScheduleMapKey) {
    <a href="scheduled_txns.md#0x1_scheduled_txns_finish_execution">scheduled_txns::finish_execution</a>(txn_key);
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



<a id="@Specification_1_grant_gas_permission"></a>

### Function `grant_gas_permission`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_grant_gas_permission">grant_gas_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_amount: u64)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
</code></pre>



<a id="@Specification_1_revoke_gas_permission"></a>

### Function `revoke_gas_permission`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_revoke_gas_permission">revoke_gas_permission</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


Ensure caller is <code>aptos_framework</code>.
Aborts if TransactionValidation already exists.


<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a>&gt;(addr);
</code></pre>


Create a schema to reuse some code.
Give some constraints that may abort according to the conditions.


<a id="0x1_transaction_validation_PrologueCommonAbortsIf"></a>


<pre><code><b>schema</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> {
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    gas_payer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">ReplayProtector</a>;
    txn_sequence_number: u64;
    txn_authentication_key: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;;
    txn_gas_price: u64;
    txn_max_gas_units: u64;
    txn_expiration_time: u64;
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8;
    <b>aborts_if</b> !<b>exists</b>&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);
    <b>aborts_if</b> !(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; txn_expiration_time);
    <b>aborts_if</b> !<b>exists</b>&lt;ChainId&gt;(@aptos_framework);
    <b>aborts_if</b> !(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() == <a href="chain_id.md#0x1_chain_id">chain_id</a>);
    <b>let</b> transaction_sender = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>let</b> gas_payer_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(gas_payer);
    <b>aborts_if</b> (
        !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>)
            || <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(transaction_sender)
            || transaction_sender == gas_payer_addr
            || txn_sequence_number &gt; 0
    ) && (
        !(txn_sequence_number &gt;= <b>global</b>&lt;Account&gt;(transaction_sender).sequence_number)
            || !(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(txn_authentication_key) || <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(
            txn_authentication_key
        ) == <b>global</b>&lt;Account&gt;(transaction_sender).authentication_key)
            || !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(transaction_sender)
            || !(txn_sequence_number == <b>global</b>&lt;Account&gt;(transaction_sender).sequence_number)
    );
    <b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>)
        && transaction_sender != gas_payer_addr
        && txn_sequence_number == 0
        && !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(transaction_sender)
        && (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(txn_authentication_key) || <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(
        txn_authentication_key
    ) != <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(transaction_sender));
    <b>aborts_if</b> !(txn_sequence_number &lt; (1u64 &lt;&lt; 63));
    <b>let</b> max_transaction_fee = txn_gas_price * txn_max_gas_units;
    <b>aborts_if</b> max_transaction_fee &gt; <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>;
    <b>aborts_if</b> !<b>exists</b>&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer_addr);
    // This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
    <b>aborts_if</b> !(<b>global</b>&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer_addr).<a href="coin.md#0x1_coin">coin</a>.value &gt;= max_transaction_fee);
}
</code></pre>



<a id="@Specification_1_prologue_common"></a>

### Function `prologue_common`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a>;
</code></pre>



<a id="@Specification_1_check_for_replay_protection_regular_txn"></a>

### Function `check_for_replay_protection_regular_txn`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_regular_txn">check_for_replay_protection_regular_txn</a>(sender_address: <b>address</b>, gas_payer_address: <b>address</b>, txn_sequence_number: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_check_for_replay_protection_orderless_txn"></a>

### Function `check_for_replay_protection_orderless_txn`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_check_for_replay_protection_orderless_txn">check_for_replay_protection_orderless_txn</a>(sender: <b>address</b>, nonce: u64, txn_expiration_time: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_script_prologue"></a>

### Function `script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf"></a>


<pre><code><b>schema</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> {
    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;;
    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;;
    is_simulation: bool;
    <b>let</b> num_secondary_signers = len(secondary_signer_addresses);
    <b>aborts_if</b> len(secondary_signer_public_key_hashes) != num_secondary_signers;
    // This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
    <b>aborts_if</b> <b>exists</b> i in 0..num_secondary_signers:
        !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(secondary_signer_addresses[i]);
    <b>aborts_if</b> <b>exists</b> i in 0..num_secondary_signers:
        !<a href="transaction_validation.md#0x1_transaction_validation_can_skip">can_skip</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_simulation_enhancement_enabled">features::spec_simulation_enhancement_enabled</a>(), is_simulation, secondary_signer_public_key_hashes[i]) &&
            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(secondary_signer_public_key_hashes[i]) && <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(
            secondary_signer_public_key_hashes[i]
        ) !=
                <a href="account.md#0x1_account_spec_get_authentication_key">account::spec_get_authentication_key</a>(secondary_signer_addresses[i]);
    <b>ensures</b> <b>forall</b> i in 0..num_secondary_signers:
        <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(secondary_signer_addresses[i]);
    <b>ensures</b> <b>forall</b> i in 0..num_secondary_signers:
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(secondary_signer_public_key_hashes[i]) || <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(
            secondary_signer_public_key_hashes[i]
        ) ==
            <a href="account.md#0x1_account_spec_get_authentication_key">account::spec_get_authentication_key</a>(secondary_signer_addresses[i])
            || <a href="transaction_validation.md#0x1_transaction_validation_can_skip">can_skip</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_simulation_enhancement_enabled">features::spec_simulation_enhancement_enabled</a>(), is_simulation, secondary_signer_public_key_hashes[i]);
}
</code></pre>




<a id="0x1_transaction_validation_can_skip"></a>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_can_skip">can_skip</a>(feature_flag: bool, is_simulation: bool, auth_key: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): bool {
   <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_simulation_enhancement_enabled">features::spec_simulation_enhancement_enabled</a>() && is_simulation && <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(auth_key)
}
</code></pre>



<a id="@Specification_1_script_prologue_extended"></a>

### Function `script_prologue_extended`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue_extended">script_prologue_extended</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> {
    gas_payer: sender,
    txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(txn_public_key),
    replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
};
</code></pre>



<a id="@Specification_1_multi_agent_script_prologue"></a>

### Function `multi_agent_script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue">multi_agent_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_multi_agent_script_prologue_extended"></a>

### Function `multi_agent_script_prologue_extended`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue_extended">multi_agent_script_prologue_extended</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>


Aborts if length of public key hashed vector
not equal the number of singers.


<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>let</b> gas_payer = sender;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_multi_agent_common_prologue"></a>

### Function `multi_agent_common_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
</code></pre>



<a id="@Specification_1_fee_payer_script_prologue"></a>

### Function `fee_payer_script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_fee_payer_script_prologue_extended"></a>

### Function `fee_payer_script_prologue_extended`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue_extended">fee_payer_script_prologue_extended</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>pragma</b> verify_duration_estimate = 120;
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_FEE_PAYER_ENABLED">features::FEE_PAYER_ENABLED</a>);
<b>let</b> gas_payer = <a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(fee_payer_address);
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> {
    gas_payer,
    replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
    txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(txn_sender_public_key),
};
<b>aborts_if</b> !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(fee_payer_address);
<b>aborts_if</b> !(fee_payer_public_key_hash == <a href="account.md#0x1_account_spec_get_authentication_key">account::spec_get_authentication_key</a>(fee_payer_address));
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_fee_payer_enabled">features::spec_fee_payer_enabled</a>();
</code></pre>



<a id="@Specification_1_epilogue"></a>

### Function `epilogue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_epilogue_extended"></a>

### Function `epilogue_extended`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_extended">epilogue_extended</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool)
</code></pre>


Abort according to the conditions.
<code>AptosCoinCapabilities</code> and <code>CoinInfo</code> should exists.
Skip transaction_fee::burn_fee verification.


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_EpilogueGasPayerAbortsIf">EpilogueGasPayerAbortsIf</a> { gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>) };
</code></pre>



<a id="@Specification_1_epilogue_gas_payer"></a>

### Function `epilogue_gas_payer`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_epilogue_gas_payer_extended"></a>

### Function `epilogue_gas_payer_extended`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_extended">epilogue_gas_payer_extended</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool)
</code></pre>


Abort according to the conditions.
<code>AptosCoinCapabilities</code> and <code>CoinInfo</code> should exist.
Skip transaction_fee::burn_fee verification.


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_EpilogueGasPayerAbortsIf">EpilogueGasPayerAbortsIf</a>;
</code></pre>



<a id="@Specification_1_unified_prologue"></a>

### Function `unified_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue">unified_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_sequence_number: u64, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_unified_prologue_fee_payer"></a>

### Function `unified_prologue_fee_payer`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_fee_payer">unified_prologue_fee_payer</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fee_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_sequence_number: u64, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_unified_epilogue"></a>

### Function `unified_epilogue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_epilogue">unified_epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_unified_prologue_v2"></a>

### Function `unified_prologue_v2`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_v2">unified_prologue_v2</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_unified_prologue_fee_payer_v2"></a>

### Function `unified_prologue_fee_payer_v2`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_prologue_fee_payer_v2">unified_prologue_fee_payer_v2</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fee_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, replay_protector: <a href="transaction_validation.md#0x1_transaction_validation_ReplayProtector">transaction_validation::ReplayProtector</a>, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, is_simulation: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_unified_epilogue_v2"></a>

### Function `unified_epilogue_v2`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_unified_epilogue_v2">unified_epilogue_v2</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, is_simulation: bool, is_orderless_txn: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
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
    <b>aborts_if</b> !<b>exists</b>&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer);
    <b>aborts_if</b> !<b>exists</b>&lt;Account&gt;(addr);
    <b>aborts_if</b> !(<b>global</b>&lt;Account&gt;(addr).sequence_number &lt; <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>);
    <b>ensures</b> <a href="account.md#0x1_account">account</a>.sequence_number == pre_account.sequence_number + 1;
    <b>let</b> amount_to_burn = transaction_fee_amount - storage_fee_refunded;
    <b>let</b> apt_addr = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;().account_address;
    <b>let</b> maybe_apt_supply = <b>global</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr).supply;
    <b>let</b> total_supply_enabled = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_apt_supply);
    <b>let</b> apt_supply = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(maybe_apt_supply);
    <b>let</b> apt_supply_value = <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(apt_supply);
    <b>let</b> <b>post</b> post_maybe_apt_supply = <b>global</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr).supply;
    <b>let</b> <b>post</b> post_apt_supply = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(post_maybe_apt_supply);
    <b>let</b> <b>post</b> post_apt_supply_value = <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(post_apt_supply);
    <b>aborts_if</b> amount_to_burn &gt; 0 && !<b>exists</b>&lt;AptosCoinCapabilities&gt;(@aptos_framework);
    <b>aborts_if</b> amount_to_burn &gt; 0 && !<b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr);
    <b>aborts_if</b> amount_to_burn &gt; 0 && total_supply_enabled && apt_supply_value &lt; amount_to_burn;
    <b>ensures</b> total_supply_enabled ==&gt; apt_supply_value - amount_to_burn == post_apt_supply_value;
    <b>let</b> amount_to_mint = storage_fee_refunded - transaction_fee_amount;
    <b>let</b> total_supply = <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;AptosCoin&gt;;
    <b>let</b> <b>post</b> post_total_supply = <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;AptosCoin&gt;;
    <b>aborts_if</b> amount_to_mint &gt; 0 && !<b>exists</b>&lt;CoinStore&lt;AptosCoin&gt;&gt;(addr);
    <b>aborts_if</b> amount_to_mint &gt; 0 && !<b>exists</b>&lt;AptosCoinMintCapability&gt;(@aptos_framework);
    <b>aborts_if</b> amount_to_mint &gt; 0 && total_supply + amount_to_mint &gt; MAX_U128;
    <b>ensures</b> amount_to_mint &gt; 0 ==&gt; post_total_supply == total_supply + amount_to_mint;
    <b>let</b> aptos_addr = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;().account_address;
    <b>aborts_if</b> (amount_to_mint != 0) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;AptosCoin&gt;&gt;(aptos_addr);
    <b>include</b> <a href="coin.md#0x1_coin_CoinAddAbortsIf">coin::CoinAddAbortsIf</a>&lt;AptosCoin&gt; { amount: amount_to_mint };
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
