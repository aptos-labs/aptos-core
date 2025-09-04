
<a id="0x1_transaction_context"></a>

# Module `0x1::transaction_context`



-  [Struct `AUID`](#0x1_transaction_context_AUID)
-  [Struct `EntryFunctionPayload`](#0x1_transaction_context_EntryFunctionPayload)
-  [Struct `MultisigPayload`](#0x1_transaction_context_MultisigPayload)
-  [Constants](#@Constants_0)
-  [Function `get_txn_hash`](#0x1_transaction_context_get_txn_hash)
-  [Function `get_transaction_hash`](#0x1_transaction_context_get_transaction_hash)
-  [Function `generate_unique_address`](#0x1_transaction_context_generate_unique_address)
-  [Function `generate_auid_address`](#0x1_transaction_context_generate_auid_address)
-  [Function `get_script_hash`](#0x1_transaction_context_get_script_hash)
-  [Function `generate_auid`](#0x1_transaction_context_generate_auid)
-  [Function `auid_address`](#0x1_transaction_context_auid_address)
-  [Function `sender`](#0x1_transaction_context_sender)
-  [Function `sender_internal`](#0x1_transaction_context_sender_internal)
-  [Function `secondary_signers`](#0x1_transaction_context_secondary_signers)
-  [Function `secondary_signers_internal`](#0x1_transaction_context_secondary_signers_internal)
-  [Function `gas_payer`](#0x1_transaction_context_gas_payer)
-  [Function `gas_payer_internal`](#0x1_transaction_context_gas_payer_internal)
-  [Function `max_gas_amount`](#0x1_transaction_context_max_gas_amount)
-  [Function `max_gas_amount_internal`](#0x1_transaction_context_max_gas_amount_internal)
-  [Function `gas_unit_price`](#0x1_transaction_context_gas_unit_price)
-  [Function `gas_unit_price_internal`](#0x1_transaction_context_gas_unit_price_internal)
-  [Function `chain_id`](#0x1_transaction_context_chain_id)
-  [Function `chain_id_internal`](#0x1_transaction_context_chain_id_internal)
-  [Function `entry_function_payload`](#0x1_transaction_context_entry_function_payload)
-  [Function `entry_function_payload_internal`](#0x1_transaction_context_entry_function_payload_internal)
-  [Function `account_address`](#0x1_transaction_context_account_address)
-  [Function `module_name`](#0x1_transaction_context_module_name)
-  [Function `function_name`](#0x1_transaction_context_function_name)
-  [Function `type_arg_names`](#0x1_transaction_context_type_arg_names)
-  [Function `args`](#0x1_transaction_context_args)
-  [Function `multisig_payload`](#0x1_transaction_context_multisig_payload)
-  [Function `multisig_payload_internal`](#0x1_transaction_context_multisig_payload_internal)
-  [Function `multisig_address`](#0x1_transaction_context_multisig_address)
-  [Function `inner_entry_function_payload`](#0x1_transaction_context_inner_entry_function_payload)
-  [Function `monotonically_increasing_counter`](#0x1_transaction_context_monotonically_increasing_counter)
-  [Function `monotonically_increasing_counter_internal`](#0x1_transaction_context_monotonically_increasing_counter_internal)
-  [Specification](#@Specification_1)
    -  [Function `get_txn_hash`](#@Specification_1_get_txn_hash)
    -  [Function `get_transaction_hash`](#@Specification_1_get_transaction_hash)
    -  [Function `generate_unique_address`](#@Specification_1_generate_unique_address)
    -  [Function `generate_auid_address`](#@Specification_1_generate_auid_address)
    -  [Function `get_script_hash`](#@Specification_1_get_script_hash)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `auid_address`](#@Specification_1_auid_address)
    -  [Function `sender_internal`](#@Specification_1_sender_internal)
    -  [Function `secondary_signers_internal`](#@Specification_1_secondary_signers_internal)
    -  [Function `gas_payer_internal`](#@Specification_1_gas_payer_internal)
    -  [Function `max_gas_amount_internal`](#@Specification_1_max_gas_amount_internal)
    -  [Function `gas_unit_price_internal`](#@Specification_1_gas_unit_price_internal)
    -  [Function `chain_id_internal`](#@Specification_1_chain_id_internal)
    -  [Function `entry_function_payload_internal`](#@Specification_1_entry_function_payload_internal)
    -  [Function `multisig_payload_internal`](#@Specification_1_multisig_payload_internal)
    -  [Function `monotonically_increasing_counter_internal`](#@Specification_1_monotonically_increasing_counter_internal)


<pre><code><b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_transaction_context_AUID"></a>

## Struct `AUID`

A wrapper denoting velor unique identifer (AUID)
for storing an address


<pre><code><b>struct</b> <a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>unique_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_context_EntryFunctionPayload"></a>

## Struct `EntryFunctionPayload`

Represents the entry function payload.


<pre><code><b>struct</b> <a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>module_name: <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>function_name: <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>ty_args_names: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>args: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_context_MultisigPayload"></a>

## Struct `MultisigPayload`

Represents the multisig payload.


<pre><code><b>struct</b> <a href="transaction_context.md#0x1_transaction_context_MultisigPayload">MultisigPayload</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>entry_function_payload: <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_transaction_context_EMONOTONICALLY_INCREASING_COUNTER_NOT_ENABLED"></a>

The monotonically increasing counter is not enabled.


<pre><code><b>const</b> <a href="transaction_context.md#0x1_transaction_context_EMONOTONICALLY_INCREASING_COUNTER_NOT_ENABLED">EMONOTONICALLY_INCREASING_COUNTER_NOT_ENABLED</a>: u64 = 3;
</code></pre>



<a id="0x1_transaction_context_EMONOTONICALLY_INCREASING_COUNTER_OVERFLOW"></a>

The monotonically increasing counter has overflowed (too many calls in a single session).


<pre><code><b>const</b> <a href="transaction_context.md#0x1_transaction_context_EMONOTONICALLY_INCREASING_COUNTER_OVERFLOW">EMONOTONICALLY_INCREASING_COUNTER_OVERFLOW</a>: u64 = 4;
</code></pre>



<a id="0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED"></a>

The transaction context extension feature is not enabled.


<pre><code><b>const</b> <a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>: u64 = 2;
</code></pre>



<a id="0x1_transaction_context_ETRANSACTION_CONTEXT_NOT_AVAILABLE"></a>

Transaction context is only available in the user transaction prologue, execution, or epilogue phases.


<pre><code><b>const</b> <a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_NOT_AVAILABLE">ETRANSACTION_CONTEXT_NOT_AVAILABLE</a>: u64 = 1;
</code></pre>



<a id="0x1_transaction_context_get_txn_hash"></a>

## Function `get_txn_hash`

Returns the transaction hash of the current transaction.


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_transaction_context_get_transaction_hash"></a>

## Function `get_transaction_hash`

Returns the transaction hash of the current transaction.
Internally calls the private function <code>get_txn_hash</code>.
This function is created for to feature gate the <code>get_txn_hash</code> function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">get_transaction_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">get_transaction_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_generate_unique_address"></a>

## Function `generate_unique_address`

Returns a universally unique identifier (of type address) generated
by hashing the transaction hash of this transaction and a sequence number
specific to this transaction. This function can be called any
number of times inside a single transaction. Each such call increments
the sequence number and generates a new unique address.
Uses Scheme in types/src/transaction/authenticator.rs for domain separation
from other ways of generating unique addresses.


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>(): <b>address</b>;
</code></pre>



</details>

<a id="0x1_transaction_context_generate_auid_address"></a>

## Function `generate_auid_address`

Returns a velor unique identifier. Internally calls
the private function <code>generate_unique_address</code>. This function is
created for to feature gate the <code>generate_unique_address</code> function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">generate_auid_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">generate_auid_address</a>(): <b>address</b> {
    <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_get_script_hash"></a>

## Function `get_script_hash`

Returns the script hash of the current entry function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_transaction_context_generate_auid"></a>

## Function `generate_auid`

This method runs <code>generate_unique_address</code> native function and returns
the generated unique address wrapped in the AUID class.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid">generate_auid</a>(): <a href="transaction_context.md#0x1_transaction_context_AUID">transaction_context::AUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid">generate_auid</a>(): <a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a> {
    <b>return</b> <a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a> {
        unique_address: <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>()
    }
}
</code></pre>



</details>

<a id="0x1_transaction_context_auid_address"></a>

## Function `auid_address`

Returns the unique address wrapped in the given AUID struct.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_auid_address">auid_address</a>(auid: &<a href="transaction_context.md#0x1_transaction_context_AUID">transaction_context::AUID</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_auid_address">auid_address</a>(auid: &<a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a>): <b>address</b> {
    auid.unique_address
}
</code></pre>



</details>

<a id="0x1_transaction_context_sender"></a>

## Function `sender`

Returns the sender's address for the current transaction.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_sender">sender</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_sender">sender</a>(): <b>address</b> {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <a href="transaction_context.md#0x1_transaction_context_sender_internal">sender_internal</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_sender_internal"></a>

## Function `sender_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_sender_internal">sender_internal</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_sender_internal">sender_internal</a>(): <b>address</b>;
</code></pre>



</details>

<a id="0x1_transaction_context_secondary_signers"></a>

## Function `secondary_signers`

Returns the list of the secondary signers for the current transaction.
If the current transaction has no secondary signers, this function returns an empty vector.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_secondary_signers">secondary_signers</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_secondary_signers">secondary_signers</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <a href="transaction_context.md#0x1_transaction_context_secondary_signers_internal">secondary_signers_internal</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_secondary_signers_internal"></a>

## Function `secondary_signers_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_secondary_signers_internal">secondary_signers_internal</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_secondary_signers_internal">secondary_signers_internal</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;;
</code></pre>



</details>

<a id="0x1_transaction_context_gas_payer"></a>

## Function `gas_payer`

Returns the gas payer address for the current transaction.
It is either the sender's address if no separate gas fee payer is specified for the current transaction,
or the address of the separate gas fee payer if one is specified.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_payer">gas_payer</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_payer">gas_payer</a>(): <b>address</b> {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <a href="transaction_context.md#0x1_transaction_context_gas_payer_internal">gas_payer_internal</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_gas_payer_internal"></a>

## Function `gas_payer_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_payer_internal">gas_payer_internal</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_payer_internal">gas_payer_internal</a>(): <b>address</b>;
</code></pre>



</details>

<a id="0x1_transaction_context_max_gas_amount"></a>

## Function `max_gas_amount`

Returns the max gas amount in units which is specified for the current transaction.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_max_gas_amount">max_gas_amount</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_max_gas_amount">max_gas_amount</a>(): u64 {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <a href="transaction_context.md#0x1_transaction_context_max_gas_amount_internal">max_gas_amount_internal</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_max_gas_amount_internal"></a>

## Function `max_gas_amount_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_max_gas_amount_internal">max_gas_amount_internal</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_max_gas_amount_internal">max_gas_amount_internal</a>(): u64;
</code></pre>



</details>

<a id="0x1_transaction_context_gas_unit_price"></a>

## Function `gas_unit_price`

Returns the gas unit price in Octas which is specified for the current transaction.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_unit_price">gas_unit_price</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_unit_price">gas_unit_price</a>(): u64 {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <a href="transaction_context.md#0x1_transaction_context_gas_unit_price_internal">gas_unit_price_internal</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_gas_unit_price_internal"></a>

## Function `gas_unit_price_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_unit_price_internal">gas_unit_price_internal</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_unit_price_internal">gas_unit_price_internal</a>(): u64;
</code></pre>



</details>

<a id="0x1_transaction_context_chain_id"></a>

## Function `chain_id`

Returns the chain ID specified for the current transaction.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code><b>public</b> <b>fun</b> <a href="chain_id.md#0x1_chain_id">chain_id</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_id.md#0x1_chain_id">chain_id</a>(): u8 {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <a href="transaction_context.md#0x1_transaction_context_chain_id_internal">chain_id_internal</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_chain_id_internal"></a>

## Function `chain_id_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_chain_id_internal">chain_id_internal</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_chain_id_internal">chain_id_internal</a>(): u8;
</code></pre>



</details>

<a id="0x1_transaction_context_entry_function_payload"></a>

## Function `entry_function_payload`

Returns the entry function payload if the current transaction has such a payload. Otherwise, return <code>None</code>.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_entry_function_payload">entry_function_payload</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_entry_function_payload">entry_function_payload</a>(): Option&lt;<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a>&gt; {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <a href="transaction_context.md#0x1_transaction_context_entry_function_payload_internal">entry_function_payload_internal</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_entry_function_payload_internal"></a>

## Function `entry_function_payload_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_entry_function_payload_internal">entry_function_payload_internal</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_entry_function_payload_internal">entry_function_payload_internal</a>(): Option&lt;<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a>&gt;;
</code></pre>



</details>

<a id="0x1_transaction_context_account_address"></a>

## Function `account_address`

Returns the account address of the entry function payload.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_account_address">account_address</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_account_address">account_address</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a>): <b>address</b> {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    payload.account_address
}
</code></pre>



</details>

<a id="0x1_transaction_context_module_name"></a>

## Function `module_name`

Returns the module name of the entry function payload.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_module_name">module_name</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_module_name">module_name</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a>): String {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    payload.module_name
}
</code></pre>



</details>

<a id="0x1_transaction_context_function_name"></a>

## Function `function_name`

Returns the function name of the entry function payload.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_function_name">function_name</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_function_name">function_name</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a>): String {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    payload.function_name
}
</code></pre>



</details>

<a id="0x1_transaction_context_type_arg_names"></a>

## Function `type_arg_names`

Returns the type arguments names of the entry function payload.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_type_arg_names">type_arg_names</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_type_arg_names">type_arg_names</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a>): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt; {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    payload.ty_args_names
}
</code></pre>



</details>

<a id="0x1_transaction_context_args"></a>

## Function `args`

Returns the arguments of the entry function payload.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_args">args</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_args">args</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a>): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    payload.args
}
</code></pre>



</details>

<a id="0x1_transaction_context_multisig_payload"></a>

## Function `multisig_payload`

Returns the multisig payload if the current transaction has such a payload. Otherwise, return <code>None</code>.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_multisig_payload">multisig_payload</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">transaction_context::MultisigPayload</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_multisig_payload">multisig_payload</a>(): Option&lt;<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">MultisigPayload</a>&gt; {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <a href="transaction_context.md#0x1_transaction_context_multisig_payload_internal">multisig_payload_internal</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_multisig_payload_internal"></a>

## Function `multisig_payload_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_multisig_payload_internal">multisig_payload_internal</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">transaction_context::MultisigPayload</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_multisig_payload_internal">multisig_payload_internal</a>(): Option&lt;<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">MultisigPayload</a>&gt;;
</code></pre>



</details>

<a id="0x1_transaction_context_multisig_address"></a>

## Function `multisig_address`

Returns the multisig account address of the multisig payload.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_multisig_address">multisig_address</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">transaction_context::MultisigPayload</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_multisig_address">multisig_address</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">MultisigPayload</a>): <b>address</b> {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    payload.multisig_address
}
</code></pre>



</details>

<a id="0x1_transaction_context_inner_entry_function_payload"></a>

## Function `inner_entry_function_payload`

Returns the inner entry function payload of the multisig payload.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_inner_entry_function_payload">inner_entry_function_payload</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">transaction_context::MultisigPayload</a>): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_inner_entry_function_payload">inner_entry_function_payload</a>(payload: &<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">MultisigPayload</a>): Option&lt;<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">EntryFunctionPayload</a>&gt; {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    payload.entry_function_payload
}
</code></pre>



</details>

<a id="0x1_transaction_context_monotonically_increasing_counter"></a>

## Function `monotonically_increasing_counter`

Returns a monotonically increasing counter value that combines timestamp, transaction index,
session counter, and local counter into a 128-bit value.
Format: <code>&lt;reserved_byte (8 bits)&gt; || timestamp_us (64 bits) || transaction_index (32 bits) || session_counter (8 bits) || local_counter (16 bits)</code>
The function aborts if the local counter overflows (after 65535 calls in a single session).


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_monotonically_increasing_counter">monotonically_increasing_counter</a>(): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_monotonically_increasing_counter">monotonically_increasing_counter</a>(): u128 {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_transaction_context_extension_enabled">features::transaction_context_extension_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED">ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED</a>));
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_is_monotonically_increasing_counter_enabled">features::is_monotonically_increasing_counter_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="transaction_context.md#0x1_transaction_context_EMONOTONICALLY_INCREASING_COUNTER_NOT_ENABLED">EMONOTONICALLY_INCREASING_COUNTER_NOT_ENABLED</a>));
    <b>let</b> timestamp_us = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <a href="transaction_context.md#0x1_transaction_context_monotonically_increasing_counter_internal">monotonically_increasing_counter_internal</a>(timestamp_us)
}
</code></pre>



</details>

<a id="0x1_transaction_context_monotonically_increasing_counter_internal"></a>

## Function `monotonically_increasing_counter_internal`



<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_monotonically_increasing_counter_internal">monotonically_increasing_counter_internal</a>(timestamp_us: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_monotonically_increasing_counter_internal">monotonically_increasing_counter_internal</a>(timestamp_us: u64): u128;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_get_txn_hash"></a>

### Function `get_txn_hash`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="transaction_context.md#0x1_transaction_context_spec_get_txn_hash">spec_get_txn_hash</a>();
</code></pre>




<a id="0x1_transaction_context_spec_get_txn_hash"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_get_txn_hash">spec_get_txn_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



<a id="@Specification_1_get_transaction_hash"></a>

### Function `get_transaction_hash`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">get_transaction_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="transaction_context.md#0x1_transaction_context_spec_get_txn_hash">spec_get_txn_hash</a>();
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> [abstract] len(result) == 32;
</code></pre>



<a id="@Specification_1_generate_unique_address"></a>

### Function `generate_unique_address`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>(): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">spec_generate_unique_address</a>();
</code></pre>




<a id="0x1_transaction_context_spec_generate_unique_address"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">spec_generate_unique_address</a>(): <b>address</b>;
</code></pre>



<a id="@Specification_1_generate_auid_address"></a>

### Function `generate_auid_address`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">generate_auid_address</a>(): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>ensures</b> [abstract] result == <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">spec_generate_unique_address</a>();
</code></pre>



<a id="@Specification_1_get_script_hash"></a>

### Function `get_script_hash`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>





<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Fetching the transaction hash should return a vector with 32 bytes.</td>
<td>Medium</td>
<td>The get_transaction_hash function calls the native function get_txn_hash, which fetches the NativeTransactionContext struct and returns the txn_hash field.</td>
<td>Audited that the native function returns the txn hash, whose size is 32 bytes. This has been modeled as the abstract postcondition that the returned vector is of length 32. Formally verified via <a href="#high-level-req-1">get_txn_hash</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Fetching the unique address should never abort.</td>
<td>Low</td>
<td>The function auid_address returns the unique address from a supplied AUID resource.</td>
<td>Formally verified via <a href="#high-level-req-2">auid_address</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Generating the unique address should return a vector with 32 bytes.</td>
<td>Medium</td>
<td>The generate_auid_address function checks calls the native function generate_unique_address which fetches the NativeTransactionContext struct, increments the auid_counter by one, and then creates a new authentication key from a preimage, which is then returned.</td>
<td>Audited that the native function returns an address, and the length of an address is 32 bytes. This has been modeled as the abstract postcondition that the returned vector is of length 32. Formally verified via <a href="#high-level-req-3">generate_auid_address</a>.</td>
</tr>

<tr>
<td>4</td>
<td>Fetching the script hash of the current entry function should never fail and should return a vector with 32 bytes if the transaction payload is a script, otherwise an empty vector.</td>
<td>Low</td>
<td>The native function get_script_hash returns the NativeTransactionContext.script_hash field.</td>
<td>Audited that the native function holds the required property. This has been modeled as the abstract spec. Formally verified via <a href="#high-level-req-4">get_script_hash</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> opaque;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="transaction_context.md#0x1_transaction_context_spec_get_script_hash">spec_get_script_hash</a>();
<b>ensures</b> [abstract] len(result) == 32;
</code></pre>




<a id="0x1_transaction_context_spec_get_script_hash"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_get_script_hash">spec_get_script_hash</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



<a id="@Specification_1_auid_address"></a>

### Function `auid_address`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_auid_address">auid_address</a>(auid: &<a href="transaction_context.md#0x1_transaction_context_AUID">transaction_context::AUID</a>): <b>address</b>
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_sender_internal"></a>

### Function `sender_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_sender_internal">sender_internal</a>(): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_secondary_signers_internal"></a>

### Function `secondary_signers_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_secondary_signers_internal">secondary_signers_internal</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_gas_payer_internal"></a>

### Function `gas_payer_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_payer_internal">gas_payer_internal</a>(): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_max_gas_amount_internal"></a>

### Function `max_gas_amount_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_max_gas_amount_internal">max_gas_amount_internal</a>(): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_gas_unit_price_internal"></a>

### Function `gas_unit_price_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_gas_unit_price_internal">gas_unit_price_internal</a>(): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_chain_id_internal"></a>

### Function `chain_id_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_chain_id_internal">chain_id_internal</a>(): u8
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_entry_function_payload_internal"></a>

### Function `entry_function_payload_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_entry_function_payload_internal">entry_function_payload_internal</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_multisig_payload_internal"></a>

### Function `multisig_payload_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_multisig_payload_internal">multisig_payload_internal</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="transaction_context.md#0x1_transaction_context_MultisigPayload">transaction_context::MultisigPayload</a>&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_monotonically_increasing_counter_internal"></a>

### Function `monotonically_increasing_counter_internal`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_monotonically_increasing_counter_internal">monotonically_increasing_counter_internal</a>(timestamp_us: u64): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
