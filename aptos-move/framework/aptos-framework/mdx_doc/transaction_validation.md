
<a id="0x1_transaction_validation"></a>

# Module `0x1::transaction_validation`



-  [Resource `TransactionValidation`](#0x1_transaction_validation_TransactionValidation)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_transaction_validation_initialize)
-  [Function `collect_deposit`](#0x1_transaction_validation_collect_deposit)
-  [Function `return_deposit`](#0x1_transaction_validation_return_deposit)
-  [Function `prologue_common`](#0x1_transaction_validation_prologue_common)
-  [Function `script_prologue`](#0x1_transaction_validation_script_prologue)
-  [Function `script_prologue_collect_deposit`](#0x1_transaction_validation_script_prologue_collect_deposit)
-  [Function `multi_agent_script_prologue`](#0x1_transaction_validation_multi_agent_script_prologue)
-  [Function `multi_agent_common_prologue`](#0x1_transaction_validation_multi_agent_common_prologue)
-  [Function `fee_payer_script_prologue`](#0x1_transaction_validation_fee_payer_script_prologue)
-  [Function `fee_payer_script_prologue_collect_deposit`](#0x1_transaction_validation_fee_payer_script_prologue_collect_deposit)
-  [Function `epilogue`](#0x1_transaction_validation_epilogue)
-  [Function `epilogue_return_deposit`](#0x1_transaction_validation_epilogue_return_deposit)
-  [Function `epilogue_gas_payer`](#0x1_transaction_validation_epilogue_gas_payer)
-  [Function `epilogue_gas_payer_return_deposit`](#0x1_transaction_validation_epilogue_gas_payer_return_deposit)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `collect_deposit`](#@Specification_1_collect_deposit)
    -  [Function `return_deposit`](#@Specification_1_return_deposit)
    -  [Function `prologue_common`](#@Specification_1_prologue_common)
    -  [Function `script_prologue`](#@Specification_1_script_prologue)
    -  [Function `script_prologue_collect_deposit`](#@Specification_1_script_prologue_collect_deposit)
    -  [Function `multi_agent_script_prologue`](#@Specification_1_multi_agent_script_prologue)
    -  [Function `multi_agent_common_prologue`](#@Specification_1_multi_agent_common_prologue)
    -  [Function `fee_payer_script_prologue`](#@Specification_1_fee_payer_script_prologue)
    -  [Function `fee_payer_script_prologue_collect_deposit`](#@Specification_1_fee_payer_script_prologue_collect_deposit)
    -  [Function `epilogue`](#@Specification_1_epilogue)
    -  [Function `epilogue_return_deposit`](#@Specification_1_epilogue_return_deposit)
    -  [Function `epilogue_gas_payer`](#@Specification_1_epilogue_gas_payer)
    -  [Function `epilogue_gas_payer_return_deposit`](#@Specification_1_epilogue_gas_payer_return_deposit)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="transaction_fee.md#0x1_transaction_fee">0x1::transaction_fee</a>;<br /></code></pre>



<a id="0x1_transaction_validation_TransactionValidation"></a>

## Resource `TransactionValidation`

This holds information that will be picked up by the VM to call the
correct chain&#45;specific prologue and epilogue functions


<pre><code><b>struct</b> <a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a> <b>has</b> key<br /></code></pre>



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


<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>: u128 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_transaction_validation_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>: u64 &#61; 1004;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID">PROLOGUE_EBAD_CHAIN_ID</a>: u64 &#61; 1007;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>: u64 &#61; 1005;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED">PROLOGUE_EFEE_PAYER_NOT_ENABLED</a>: u64 &#61; 1010;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT">PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT</a>: u64 &#61; 1011;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY"></a>

Prologue errors. These are separated out from the other errors in this
module since they are mapped separately to major VM statuses, and are
important to the semantics of the system.


<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>: u64 &#61; 1001;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH">PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH</a>: u64 &#61; 1009;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG">PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG</a>: u64 &#61; 1008;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW">PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW</a>: u64 &#61; 1003;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD">PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD</a>: u64 &#61; 1002;<br /></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED"></a>



<pre><code><b>const</b> <a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED">PROLOGUE_ETRANSACTION_EXPIRED</a>: u64 &#61; 1006;<br /></code></pre>



<a id="0x1_transaction_validation_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    // module_prologue_name is deprecated and not used.<br />    module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    <b>move_to</b>(aptos_framework, <a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a> &#123;<br />        module_addr: @aptos_framework,<br />        module_name: b&quot;<a href="transaction_validation.md#0x1_transaction_validation">transaction_validation</a>&quot;,<br />        script_prologue_name,<br />        // module_prologue_name is deprecated and not used.<br />        module_prologue_name,<br />        multi_agent_prologue_name,<br />        user_epilogue_name,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_collect_deposit"></a>

## Function `collect_deposit`

Called in prologue to optionally hold some amount for special txns (e.g. randomness txns).
<code><a href="transaction_validation.md#0x1_transaction_validation_return_deposit">return_deposit</a>()</code> should be invoked in the corresponding epilogue with the same arguments.


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_collect_deposit">collect_deposit</a>(gas_payer: <b>address</b>, amount: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_collect_deposit">collect_deposit</a>(gas_payer: <b>address</b>, amount: Option&lt;u64&gt;) &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;amount)) &#123;<br />        <b>let</b> amount &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> amount);<br />        <b>let</b> balance &#61; <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(gas_payer);<br />        <b>assert</b>!(balance &gt;&#61; amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT">PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT</a>));<br />        <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">transaction_fee::burn_fee</a>(gas_payer, amount);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_return_deposit"></a>

## Function `return_deposit`

Called in epilogue to optionally released the amount held in prologue for special txns (e.g. randomness txns).


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_return_deposit">return_deposit</a>(gas_payer: <b>address</b>, amount: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_return_deposit">return_deposit</a>(gas_payer: <b>address</b>, amount: Option&lt;u64&gt;) &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;amount)) &#123;<br />        <b>let</b> amount &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> amount);<br />        <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">transaction_fee::mint_and_refund</a>(gas_payer, amount);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_prologue_common"></a>

## Function `prologue_common`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, txn_sequence_number: u64, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(<br />    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    gas_payer: <b>address</b>,<br />    txn_sequence_number: u64,<br />    txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    txn_expiration_time: u64,<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,<br />) &#123;<br />    <b>assert</b>!(<br />        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; txn_expiration_time,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED">PROLOGUE_ETRANSACTION_EXPIRED</a>),<br />    );<br />    <b>assert</b>!(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() &#61;&#61; <a href="chain_id.md#0x1_chain_id">chain_id</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID">PROLOGUE_EBAD_CHAIN_ID</a>));<br /><br />    <b>let</b> transaction_sender &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;sender);<br /><br />    <b>if</b> (<br />        transaction_sender &#61;&#61; gas_payer<br />        &#124;&#124; <a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)<br />        &#124;&#124; !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_sponsored_automatic_account_creation_enabled">features::sponsored_automatic_account_creation_enabled</a>()<br />        &#124;&#124; txn_sequence_number &gt; 0<br />    ) &#123;<br />        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>));<br />        <b>assert</b>!(<br />            txn_authentication_key &#61;&#61; <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(transaction_sender),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),<br />        );<br /><br />        <b>let</b> account_sequence_number &#61; <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(transaction_sender);<br />        <b>assert</b>!(<br />            txn_sequence_number &lt; (1u64 &lt;&lt; 63),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG">PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG</a>)<br />        );<br /><br />        <b>assert</b>!(<br />            txn_sequence_number &gt;&#61; account_sequence_number,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD">PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD</a>)<br />        );<br /><br />        <b>assert</b>!(<br />            txn_sequence_number &#61;&#61; account_sequence_number,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW">PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW</a>)<br />        );<br />    &#125; <b>else</b> &#123;<br />        // In this case, the transaction is sponsored and the <a href="account.md#0x1_account">account</a> does not exist, so ensure<br />        // the default values match.<br />        <b>assert</b>!(<br />            txn_sequence_number &#61;&#61; 0,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW">PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW</a>)<br />        );<br /><br />        <b>assert</b>!(<br />            txn_authentication_key &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;transaction_sender),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),<br />        );<br />    &#125;;<br /><br />    <b>let</b> max_transaction_fee &#61; txn_gas_price &#42; txn_max_gas_units;<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) &#123;<br />        <b>assert</b>!(<br />            <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">aptos_account::is_fungible_balance_at_least</a>(gas_payer, max_transaction_fee),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)<br />        );<br />    &#125; <b>else</b> &#123;<br />        <b>assert</b>!(<br />            <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;AptosCoin&gt;(gas_payer, max_transaction_fee),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>)<br />        );<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue"></a>

## Function `script_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>(<br />    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    txn_sequence_number: u64,<br />    txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    txn_expiration_time: u64,<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,<br />    _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) &#123;<br />    <b>let</b> gas_payer &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;sender);<br />    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(sender, gas_payer, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, <a href="chain_id.md#0x1_chain_id">chain_id</a>)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue_collect_deposit"></a>

## Function `script_prologue_collect_deposit`

<code><a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>()</code> then collect an optional deposit depending on the txn.

Deposit collection goes last so <code><a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>()</code> doesn&apos;t have to be aware of the deposit logic.


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue_collect_deposit">script_prologue_collect_deposit</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, required_deposit: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue_collect_deposit">script_prologue_collect_deposit</a>(<br />    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    txn_sequence_number: u64,<br />    txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    txn_expiration_time: u64,<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,<br />    script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    required_deposit: Option&lt;u64&gt;,<br />) &#123;<br />    <b>let</b> gas_payer &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;sender);<br />    <a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>(sender, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, <a href="chain_id.md#0x1_chain_id">chain_id</a>, script_hash);<br />    <a href="transaction_validation.md#0x1_transaction_validation_collect_deposit">collect_deposit</a>(gas_payer, required_deposit);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_script_prologue"></a>

## Function `multi_agent_script_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue">multi_agent_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue">multi_agent_script_prologue</a>(<br />    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    txn_sequence_number: u64,<br />    txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,<br />    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    txn_expiration_time: u64,<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,<br />) &#123;<br />    <b>let</b> sender_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;sender);<br />    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(<br />        sender,<br />        sender_addr,<br />        txn_sequence_number,<br />        txn_sender_public_key,<br />        txn_gas_price,<br />        txn_max_gas_units,<br />        txn_expiration_time,<br />        <a href="chain_id.md#0x1_chain_id">chain_id</a>,<br />    );<br />    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_common_prologue"></a>

## Function `multi_agent_common_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(<br />    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,<br />    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />) &#123;<br />    <b>let</b> num_secondary_signers &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;secondary_signer_addresses);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;secondary_signer_public_key_hashes) &#61;&#61; num_secondary_signers,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH">PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH</a>),<br />    );<br /><br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> i &lt;&#61; num_secondary_signers;<br />            <b>invariant</b> <b>forall</b> j in 0..i:<br />                <a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[j])<br />                &amp;&amp; secondary_signer_public_key_hashes[j]<br />                   &#61;&#61; <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[j]);<br />        &#125;;<br />        (i &lt; num_secondary_signers)<br />    &#125;) &#123;<br />        <b>let</b> secondary_address &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;secondary_signer_addresses, i);<br />        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST">PROLOGUE_EACCOUNT_DOES_NOT_EXIST</a>));<br /><br />        <b>let</b> signer_public_key_hash &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;secondary_signer_public_key_hashes, i);<br />        <b>assert</b>!(<br />            signer_public_key_hash &#61;&#61; <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_address),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),<br />        );<br />        i &#61; i &#43; 1;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue"></a>

## Function `fee_payer_script_prologue`



<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>(<br />    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    txn_sequence_number: u64,<br />    txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,<br />    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    fee_payer_address: <b>address</b>,<br />    fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    txn_expiration_time: u64,<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,<br />) &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_fee_payer_enabled">features::fee_payer_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED">PROLOGUE_EFEE_PAYER_NOT_ENABLED</a>));<br />    <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(<br />        sender,<br />        fee_payer_address,<br />        txn_sequence_number,<br />        txn_sender_public_key,<br />        txn_gas_price,<br />        txn_max_gas_units,<br />        txn_expiration_time,<br />        <a href="chain_id.md#0x1_chain_id">chain_id</a>,<br />    );<br />    <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses, secondary_signer_public_key_hashes);<br />    <b>assert</b>!(<br />        fee_payer_public_key_hash &#61;&#61; <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(fee_payer_address),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY">PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY</a>),<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue_collect_deposit"></a>

## Function `fee_payer_script_prologue_collect_deposit`

<code><a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>()</code> then collect an optional deposit depending on the txn.

Deposit collection goes last so <code><a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>()</code> doesn&apos;t have to be aware of the deposit logic.


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue_collect_deposit">fee_payer_script_prologue_collect_deposit</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, required_deposit: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue_collect_deposit">fee_payer_script_prologue_collect_deposit</a>(<br />    sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    txn_sequence_number: u64,<br />    txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,<br />    secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    fee_payer_address: <b>address</b>,<br />    fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    txn_expiration_time: u64,<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,<br />    required_deposit: Option&lt;u64&gt;,<br />) &#123;<br />    <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>(<br />        sender,<br />        txn_sequence_number,<br />        txn_sender_public_key,<br />        secondary_signer_addresses,<br />        secondary_signer_public_key_hashes,<br />        fee_payer_address,<br />        fee_payer_public_key_hash,<br />        txn_gas_price,<br />        txn_max_gas_units,<br />        txn_expiration_time,<br />        <a href="chain_id.md#0x1_chain_id">chain_id</a>,<br />    );<br />    <a href="transaction_validation.md#0x1_transaction_validation_collect_deposit">collect_deposit</a>(fee_payer_address, required_deposit);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_epilogue"></a>

## Function `epilogue`

Epilogue function is run after a transaction is successfully executed.
Called by the Adapter


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>(<br />    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    storage_fee_refunded: u64,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    gas_units_remaining: u64<br />) &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;<a href="account.md#0x1_account">account</a>);<br />    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<a href="account.md#0x1_account">account</a>, addr, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_return_deposit"></a>

## Function `epilogue_return_deposit`

Return the deposit held in prologue, then <code><a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>()</code>.

Deposit return goes first so <code><a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>()</code> doesn&apos;t have to be aware of this change.


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_return_deposit">epilogue_return_deposit</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_return_deposit">epilogue_return_deposit</a>(<br />    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    storage_fee_refunded: u64,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    gas_units_remaining: u64,<br />    required_deposit: Option&lt;u64&gt;,<br />) &#123;<br />    <b>let</b> gas_payer &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;<a href="account.md#0x1_account">account</a>);<br />    <a href="transaction_validation.md#0x1_transaction_validation_return_deposit">return_deposit</a>(gas_payer, required_deposit);<br />    <a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>(<br />        <a href="account.md#0x1_account">account</a>,<br />        storage_fee_refunded,<br />        txn_gas_price,<br />        txn_max_gas_units,<br />        gas_units_remaining,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer"></a>

## Function `epilogue_gas_payer`

Epilogue function with explicit gas payer specified, is run after a transaction is successfully executed.
Called by the Adapter


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<br />    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    gas_payer: <b>address</b>,<br />    storage_fee_refunded: u64,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    gas_units_remaining: u64<br />) &#123;<br />    <b>assert</b>!(txn_max_gas_units &gt;&#61; gas_units_remaining, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>));<br />    <b>let</b> gas_used &#61; txn_max_gas_units &#45; gas_units_remaining;<br /><br />    <b>assert</b>!(<br />        (txn_gas_price <b>as</b> u128) &#42; (gas_used <b>as</b> u128) &lt;&#61; <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_EOUT_OF_GAS">EOUT_OF_GAS</a>)<br />    );<br />    <b>let</b> transaction_fee_amount &#61; txn_gas_price &#42; gas_used;<br /><br />    // it&apos;s important <b>to</b> maintain the <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> <a href="code.md#0x1_code">code</a> consistent <b>with</b> vm<br />    // <b>to</b> do failed transaction cleanup.<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) &#123;<br />        <b>assert</b>!(<br />            <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">aptos_account::is_fungible_balance_at_least</a>(gas_payer, transaction_fee_amount),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>),<br />        );<br />    &#125; <b>else</b> &#123;<br />        <b>assert</b>!(<br />            <a href="coin.md#0x1_coin_is_balance_at_least">coin::is_balance_at_least</a>&lt;AptosCoin&gt;(gas_payer, transaction_fee_amount),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_validation.md#0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT">PROLOGUE_ECANT_PAY_GAS_DEPOSIT</a>),<br />        );<br />    &#125;;<br /><br />    <b>let</b> amount_to_burn &#61; <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_collect_and_distribute_gas_fees">features::collect_and_distribute_gas_fees</a>()) &#123;<br />        // TODO(gas): We might want <b>to</b> distinguish the refundable part of the charge and burn it or track<br />        // it separately, so that we don&apos;t increase the total supply by refunding.<br /><br />        // If transaction fees are redistributed <b>to</b> validators, collect them here for<br />        // later redistribution.<br />        <a href="transaction_fee.md#0x1_transaction_fee_collect_fee">transaction_fee::collect_fee</a>(gas_payer, transaction_fee_amount);<br />        0<br />    &#125; <b>else</b> &#123;<br />        // Otherwise, just burn the fee.<br />        // TODO: this branch should be removed completely when transaction fee collection<br />        // is tested and is fully proven <b>to</b> work well.<br />        transaction_fee_amount<br />    &#125;;<br /><br />    <b>if</b> (amount_to_burn &gt; storage_fee_refunded) &#123;<br />        <b>let</b> burn_amount &#61; amount_to_burn &#45; storage_fee_refunded;<br />        <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">transaction_fee::burn_fee</a>(gas_payer, burn_amount);<br />    &#125; <b>else</b> <b>if</b> (amount_to_burn &lt; storage_fee_refunded) &#123;<br />        <b>let</b> mint_amount &#61; storage_fee_refunded &#45; amount_to_burn;<br />        <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">transaction_fee::mint_and_refund</a>(gas_payer, mint_amount)<br />    &#125;;<br /><br />    // Increment sequence number<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;<a href="account.md#0x1_account">account</a>);<br />    <a href="account.md#0x1_account_increment_sequence_number">account::increment_sequence_number</a>(addr);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer_return_deposit"></a>

## Function `epilogue_gas_payer_return_deposit`

Return the deposit held in prologue to the gas payer, then <code><a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>()</code>.

Deposit return should go first so <code><a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>()</code> doesn&apos;t have to be aware of this change.


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_return_deposit">epilogue_gas_payer_return_deposit</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_return_deposit">epilogue_gas_payer_return_deposit</a>(<br />    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    gas_payer: <b>address</b>,<br />    storage_fee_refunded: u64,<br />    txn_gas_price: u64,<br />    txn_max_gas_units: u64,<br />    gas_units_remaining: u64,<br />    required_deposit: Option&lt;u64&gt;,<br />) &#123;<br />    <a href="transaction_validation.md#0x1_transaction_validation_return_deposit">return_deposit</a>(gas_payer, required_deposit);<br />    <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<br />        <a href="account.md#0x1_account">account</a>,<br />        gas_payer,<br />        storage_fee_refunded,<br />        txn_gas_price,<br />        txn_max_gas_units,<br />        gas_units_remaining,<br />    );<br />&#125;<br /></code></pre>



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


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, script_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, module_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, multi_agent_prologue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, user_epilogue_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>


Ensure caller is <code>aptos_framework</code>.
Aborts if TransactionValidation already exists.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a>&gt;(addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="transaction_validation.md#0x1_transaction_validation_TransactionValidation">TransactionValidation</a>&gt;(addr);<br /></code></pre>


Create a schema to reuse some code.
Give some constraints that may abort according to the conditions.


<a id="0x1_transaction_validation_PrologueCommonAbortsIf"></a>


<pre><code><b>schema</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> &#123;<br />sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />gas_payer: <b>address</b>;<br />txn_sequence_number: u64;<br />txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br />txn_gas_price: u64;<br />txn_max_gas_units: u64;<br />txn_expiration_time: u64;<br /><a href="chain_id.md#0x1_chain_id">chain_id</a>: u8;<br /><b>aborts_if</b> !<b>exists</b>&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);<br /><b>aborts_if</b> !(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; txn_expiration_time);<br /><b>aborts_if</b> !<b>exists</b>&lt;ChainId&gt;(@aptos_framework);<br /><b>aborts_if</b> !(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() &#61;&#61; <a href="chain_id.md#0x1_chain_id">chain_id</a>);<br /><b>let</b> transaction_sender &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);<br /><b>aborts_if</b> (<br />    !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>)<br />    &#124;&#124; <a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)<br />    &#124;&#124; transaction_sender &#61;&#61; gas_payer<br />    &#124;&#124; txn_sequence_number &gt; 0<br />) &amp;&amp; (<br />    !(txn_sequence_number &gt;&#61; <b>global</b>&lt;Account&gt;(transaction_sender).sequence_number)<br />    &#124;&#124; !(txn_authentication_key &#61;&#61; <b>global</b>&lt;Account&gt;(transaction_sender).authentication_key)<br />    &#124;&#124; !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)<br />    &#124;&#124; !(txn_sequence_number &#61;&#61; <b>global</b>&lt;Account&gt;(transaction_sender).sequence_number)<br />);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>)<br />    &amp;&amp; transaction_sender !&#61; gas_payer<br />    &amp;&amp; txn_sequence_number &#61;&#61; 0<br />    &amp;&amp; !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(transaction_sender)<br />    &amp;&amp; txn_authentication_key !&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(transaction_sender);<br /><b>aborts_if</b> !(txn_sequence_number &lt; (1u64 &lt;&lt; 63));<br /><b>let</b> max_transaction_fee &#61; txn_gas_price &#42; txn_max_gas_units;<br /><b>aborts_if</b> max_transaction_fee &gt; <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
    <b>aborts_if</b> !(<b>global</b>&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer).<a href="coin.md#0x1_coin">coin</a>.value &gt;&#61; max_transaction_fee);<br />&#125;<br /></code></pre>



<a id="@Specification_1_collect_deposit"></a>

### Function `collect_deposit`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_collect_deposit">collect_deposit</a>(gas_payer: <b>address</b>, amount: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_return_deposit"></a>

### Function `return_deposit`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_return_deposit">return_deposit</a>(gas_payer: <b>address</b>, amount: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_prologue_common"></a>

### Function `prologue_common`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_prologue_common">prologue_common</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, txn_sequence_number: u64, txn_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_script_prologue"></a>

### Function `script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue">script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, _script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> &#123;<br />    gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),<br />    txn_authentication_key: txn_public_key<br />&#125;;<br /></code></pre>




<a id="0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf"></a>


<pre><code><b>schema</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> &#123;<br />secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;;<br />secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;;<br /><b>let</b> num_secondary_signers &#61; len(secondary_signer_addresses);<br /><b>aborts_if</b> len(secondary_signer_public_key_hashes) !&#61; num_secondary_signers;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
    <b>aborts_if</b> <b>exists</b> i in 0..num_secondary_signers:<br />    !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[i])<br />        &#124;&#124; secondary_signer_public_key_hashes[i] !&#61;<br />        <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[i]);<br /><b>ensures</b> <b>forall</b> i in 0..num_secondary_signers:<br />    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(secondary_signer_addresses[i])<br />        &amp;&amp; secondary_signer_public_key_hashes[i] &#61;&#61;<br />            <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(secondary_signer_addresses[i]);<br />&#125;<br /></code></pre>



<a id="@Specification_1_script_prologue_collect_deposit"></a>

### Function `script_prologue_collect_deposit`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_script_prologue_collect_deposit">script_prologue_collect_deposit</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, script_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, required_deposit: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_multi_agent_script_prologue"></a>

### Function `multi_agent_script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_script_prologue">multi_agent_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)<br /></code></pre>


Aborts if length of public key hashed vector
not equal the number of singers.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>let</b> gas_payer &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);<br /><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> &#123;<br />    gas_payer,<br />    txn_sequence_number,<br />    txn_authentication_key: txn_sender_public_key,<br />&#125;;<br /><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> &#123;<br />    secondary_signer_addresses,<br />    secondary_signer_public_key_hashes,<br />&#125;;<br /></code></pre>



<a id="@Specification_1_multi_agent_common_prologue"></a>

### Function `multi_agent_common_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_multi_agent_common_prologue">multi_agent_common_prologue</a>(secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>




<pre><code><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> &#123;<br />    secondary_signer_addresses,<br />    secondary_signer_public_key_hashes,<br />&#125;;<br /></code></pre>



<a id="@Specification_1_fee_payer_script_prologue"></a>

### Function `fee_payer_script_prologue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue">fee_payer_script_prologue</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_FEE_PAYER_ENABLED">features::FEE_PAYER_ENABLED</a>);<br /><b>let</b> gas_payer &#61; fee_payer_address;<br /><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_PrologueCommonAbortsIf">PrologueCommonAbortsIf</a> &#123;<br />    gas_payer,<br />    txn_sequence_number,<br />    txn_authentication_key: txn_sender_public_key,<br />&#125;;<br /><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf">MultiAgentPrologueCommonAbortsIf</a> &#123;<br />    secondary_signer_addresses,<br />    secondary_signer_public_key_hashes,<br />&#125;;<br /><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(gas_payer);<br /><b>aborts_if</b> !(fee_payer_public_key_hash &#61;&#61; <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(gas_payer));<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_fee_payer_enabled">features::spec_fee_payer_enabled</a>();<br /></code></pre>



<a id="@Specification_1_fee_payer_script_prologue_collect_deposit"></a>

### Function `fee_payer_script_prologue_collect_deposit`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_fee_payer_script_prologue_collect_deposit">fee_payer_script_prologue_collect_deposit</a>(sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_sequence_number: u64, txn_sender_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secondary_signer_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, secondary_signer_public_key_hashes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, fee_payer_address: <b>address</b>, fee_payer_public_key_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, required_deposit: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_epilogue"></a>

### Function `epilogue`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue">epilogue</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)<br /></code></pre>


Abort according to the conditions.
<code>AptosCoinCapabilities</code> and <code>CoinInfo</code> should exists.
Skip transaction_fee::burn_fee verification.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_EpilogueGasPayerAbortsIf">EpilogueGasPayerAbortsIf</a> &#123; gas_payer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>) &#125;;<br /></code></pre>



<a id="@Specification_1_epilogue_return_deposit"></a>

### Function `epilogue_return_deposit`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_return_deposit">epilogue_return_deposit</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_epilogue_gas_payer"></a>

### Function `epilogue_gas_payer`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer">epilogue_gas_payer</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)<br /></code></pre>


Abort according to the conditions.
<code>AptosCoinCapabilities</code> and <code>CoinInfo</code> should exist.
Skip transaction_fee::burn_fee verification.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="transaction_validation.md#0x1_transaction_validation_EpilogueGasPayerAbortsIf">EpilogueGasPayerAbortsIf</a>;<br /></code></pre>




<a id="0x1_transaction_validation_EpilogueGasPayerAbortsIf"></a>


<pre><code><b>schema</b> <a href="transaction_validation.md#0x1_transaction_validation_EpilogueGasPayerAbortsIf">EpilogueGasPayerAbortsIf</a> &#123;<br /><a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />gas_payer: <b>address</b>;<br />storage_fee_refunded: u64;<br />txn_gas_price: u64;<br />txn_max_gas_units: u64;<br />gas_units_remaining: u64;<br /><b>aborts_if</b> !(txn_max_gas_units &gt;&#61; gas_units_remaining);<br /><b>let</b> gas_used &#61; txn_max_gas_units &#45; gas_units_remaining;<br /><b>aborts_if</b> !(txn_gas_price &#42; gas_used &lt;&#61; <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>);<br /><b>let</b> transaction_fee_amount &#61; txn_gas_price &#42; gas_used;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> pre_account &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> <b>post</b> <a href="account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer);<br /><b>aborts_if</b> !<b>exists</b>&lt;Account&gt;(addr);<br /><b>aborts_if</b> !(<b>global</b>&lt;Account&gt;(addr).sequence_number &lt; <a href="transaction_validation.md#0x1_transaction_validation_MAX_U64">MAX_U64</a>);<br /><b>ensures</b> <a href="account.md#0x1_account">account</a>.sequence_number &#61;&#61; pre_account.sequence_number &#43; 1;<br /><b>let</b> collect_fee_enabled &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES">features::COLLECT_AND_DISTRIBUTE_GAS_FEES</a>);<br /><b>let</b> collected_fees &#61; <b>global</b>&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount;<br /><b>let</b> aggr &#61; collected_fees.value;<br /><b>let</b> aggr_val &#61; <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr);<br /><b>let</b> aggr_lim &#61; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(aggr);<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
    <b>aborts_if</b> collect_fee_enabled &amp;&amp; !<b>exists</b>&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br /><b>aborts_if</b> collect_fee_enabled &amp;&amp; transaction_fee_amount &gt; 0 &amp;&amp; aggr_val &#43; transaction_fee_amount &gt; aggr_lim;<br /><b>let</b> amount_to_burn&#61; <b>if</b> (collect_fee_enabled) &#123;<br />    0<br />&#125; <b>else</b> &#123;<br />    transaction_fee_amount &#45; storage_fee_refunded<br />&#125;;<br /><b>let</b> apt_addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;().account_address;<br /><b>let</b> maybe_apt_supply &#61; <b>global</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr).supply;<br /><b>let</b> total_supply_enabled &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_apt_supply);<br /><b>let</b> apt_supply &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(maybe_apt_supply);<br /><b>let</b> apt_supply_value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(apt_supply);<br /><b>let</b> <b>post</b> post_maybe_apt_supply &#61; <b>global</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr).supply;<br /><b>let</b> <b>post</b> post_apt_supply &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(post_maybe_apt_supply);<br /><b>let</b> <b>post</b> post_apt_supply_value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(post_apt_supply);<br /><b>aborts_if</b> amount_to_burn &gt; 0 &amp;&amp; !<b>exists</b>&lt;AptosCoinCapabilities&gt;(@aptos_framework);<br /><b>aborts_if</b> amount_to_burn &gt; 0 &amp;&amp; !<b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr);<br /><b>aborts_if</b> amount_to_burn &gt; 0 &amp;&amp; total_supply_enabled &amp;&amp; apt_supply_value &lt; amount_to_burn;<br /><b>ensures</b> total_supply_enabled &#61;&#61;&gt; apt_supply_value &#45; amount_to_burn &#61;&#61; post_apt_supply_value;<br /><b>let</b> amount_to_mint &#61; <b>if</b> (collect_fee_enabled) &#123;<br />    storage_fee_refunded<br />&#125; <b>else</b> &#123;<br />    storage_fee_refunded &#45; transaction_fee_amount<br />&#125;;<br /><b>let</b> total_supply &#61; <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;AptosCoin&gt;;<br /><b>let</b> <b>post</b> post_total_supply &#61; <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;AptosCoin&gt;;<br /><b>aborts_if</b> amount_to_mint &gt; 0 &amp;&amp; !<b>exists</b>&lt;CoinStore&lt;AptosCoin&gt;&gt;(addr);<br /><b>aborts_if</b> amount_to_mint &gt; 0 &amp;&amp; !<b>exists</b>&lt;AptosCoinMintCapability&gt;(@aptos_framework);<br /><b>aborts_if</b> amount_to_mint &gt; 0 &amp;&amp; total_supply &#43; amount_to_mint &gt; MAX_U128;<br /><b>ensures</b> amount_to_mint &gt; 0 &#61;&#61;&gt; post_total_supply &#61;&#61; total_supply &#43; amount_to_mint;<br /><b>let</b> aptos_addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;().account_address;<br /><b>aborts_if</b> (amount_to_mint !&#61; 0) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;AptosCoin&gt;&gt;(aptos_addr);<br /><b>include</b> <a href="coin.md#0x1_coin_CoinAddAbortsIf">coin::CoinAddAbortsIf</a>&lt;AptosCoin&gt; &#123; amount: amount_to_mint &#125;;<br />&#125;<br /></code></pre>



<a id="@Specification_1_epilogue_gas_payer_return_deposit"></a>

### Function `epilogue_gas_payer_return_deposit`


<pre><code><b>fun</b> <a href="transaction_validation.md#0x1_transaction_validation_epilogue_gas_payer_return_deposit">epilogue_gas_payer_return_deposit</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_payer: <b>address</b>, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
