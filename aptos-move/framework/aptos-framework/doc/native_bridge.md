
<a id="0x1_native_bridge_store"></a>

# Module `0x1::native_bridge_store`



-  [Resource `SmartTableWrapper`](#0x1_native_bridge_store_SmartTableWrapper)
-  [Struct `OutboundTransfer`](#0x1_native_bridge_store_OutboundTransfer)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_native_bridge_store_initialize)
-  [Function `is_inbound_nonce_set`](#0x1_native_bridge_store_is_inbound_nonce_set)
-  [Function `create_details`](#0x1_native_bridge_store_create_details)
-  [Function `add`](#0x1_native_bridge_store_add)
-  [Function `set_bridge_transfer_id_to_inbound_nonce`](#0x1_native_bridge_store_set_bridge_transfer_id_to_inbound_nonce)
-  [Function `assert_valid_bridge_transfer_id`](#0x1_native_bridge_store_assert_valid_bridge_transfer_id)
-  [Function `bridge_transfer_id`](#0x1_native_bridge_store_bridge_transfer_id)
-  [Function `get_bridge_transfer_details_from_nonce`](#0x1_native_bridge_store_get_bridge_transfer_details_from_nonce)
-  [Function `get_inbound_nonce_from_bridge_transfer_id`](#0x1_native_bridge_store_get_inbound_nonce_from_bridge_transfer_id)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `is_inbound_nonce_set`](#@Specification_1_is_inbound_nonce_set)
    -  [Function `create_details`](#@Specification_1_create_details)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `set_bridge_transfer_id_to_inbound_nonce`](#@Specification_1_set_bridge_transfer_id_to_inbound_nonce)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_ethereum">0x1::ethereum</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_native_bridge_store_SmartTableWrapper"></a>

## Resource `SmartTableWrapper`

A smart table wrapper


<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;K, V&gt; <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_store_OutboundTransfer"></a>

## Struct `OutboundTransfer`

Details on the outbound transfer


<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a> <b>has</b> <b>copy</b>, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>initiator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: <a href="atomic_bridge.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a></code>
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


<a id="0x1_native_bridge_store_MAX_U64"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_store_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_native_bridge_store_ENATIVE_BRIDGE_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_store_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>: u64 = 5;
</code></pre>



<a id="0x1_native_bridge_store_EINVALID_BRIDGE_TRANSFER_ID"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_store_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>: u64 = 4;
</code></pre>



<a id="0x1_native_bridge_store_EZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_store_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 3;
</code></pre>



<a id="0x1_native_bridge_store_EID_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_store_EID_NOT_FOUND">EID_NOT_FOUND</a>: u64 = 7;
</code></pre>



<a id="0x1_native_bridge_store_EINCORRECT_NONCE"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_store_EINCORRECT_NONCE">EINCORRECT_NONCE</a>: u64 = 6;
</code></pre>



<a id="0x1_native_bridge_store_ENONCE_NOT_FOUND"></a>

Error codes


<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_store_ENONCE_NOT_FOUND">ENONCE_NOT_FOUND</a>: u64 = 2;
</code></pre>



<a id="0x1_native_bridge_store_initialize"></a>

## Function `initialize`

Initializes the initiators tables and nonce.

@param aptos_framework The signer for Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> nonces_to_details = <a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt; {
        inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    };

    <b>move_to</b>(aptos_framework, nonces_to_details);

    <b>let</b> ids_to_inbound_nonces = <a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt; {
        inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    };

    <b>move_to</b>(aptos_framework, ids_to_inbound_nonces);
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_is_inbound_nonce_set"></a>

## Function `is_inbound_nonce_set`

Checks if a bridge transfer ID is associated with an inbound nonce.
@param bridge_transfer_id The bridge transfer ID.
@return <code><b>true</b></code> if the ID is associated with an existing inbound nonce, <code><b>false</b></code> otherwise.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_is_inbound_nonce_set">is_inbound_nonce_set</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_is_inbound_nonce_set">is_inbound_nonce_set</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id)
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_create_details"></a>

## Function `create_details`

Creates bridge transfer details with validation.

@param initiator The initiating party of the transfer.
@param recipient The receiving party of the transfer.
@param amount The amount to be transferred.
@param nonce The unique nonce for the transfer.
@return A <code>BridgeTransferDetails</code> object.
@abort If the amount is zero or locks are invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_create_details">create_details</a>(initiator: <b>address</b>, recipient: <a href="atomic_bridge.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, amount: u64, nonce: u64): <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">native_bridge_store::OutboundTransfer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_create_details">create_details</a>(initiator: <b>address</b>, recipient: EthereumAddress, amount: u64, nonce: u64)
    : <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a> {
    <b>assert</b>!(amount &gt; 0, <a href="native_bridge.md#0x1_native_bridge_store_EZERO_AMOUNT">EZERO_AMOUNT</a>);

    // Create a bridge transfer ID algorithmically
    <b>let</b> combined_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&initiator));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&recipient));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amount));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&nonce));
    <b>let</b> bridge_transfer_id = keccak256(combined_bytes);

    <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a> {
        bridge_transfer_id,
        initiator,
        recipient,
        amount,
    }
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_add"></a>

## Function `add`

Record details of an initiated transfer for quick lookup of details, mapping bridge transfer ID to transfer details

@param bridge_transfer_id Bridge transfer ID.
@param details The bridge transfer details


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_add">add</a>(nonce: u64, details: <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">native_bridge_store::OutboundTransfer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_add">add</a>(nonce: u64, details: <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_native_bridge_enabled">features::abort_native_bridge_enabled</a>(), <a href="native_bridge.md#0x1_native_bridge_store_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>);

    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, nonce, details);
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_set_bridge_transfer_id_to_inbound_nonce"></a>

## Function `set_bridge_transfer_id_to_inbound_nonce`

Record details of a completed transfer, mapping bridge transfer ID to inbound nonce

@param bridge_transfer_id Bridge transfer ID.
@param details The bridge transfer details


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_set_bridge_transfer_id_to_inbound_nonce">set_bridge_transfer_id_to_inbound_nonce</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, inbound_nonce: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_set_bridge_transfer_id_to_inbound_nonce">set_bridge_transfer_id_to_inbound_nonce</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, inbound_nonce: u64) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_native_bridge_enabled">features::abort_native_bridge_enabled</a>(), <a href="native_bridge.md#0x1_native_bridge_store_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>);

    <a href="native_bridge.md#0x1_native_bridge_store_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(&bridge_transfer_id);
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id, inbound_nonce);
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_assert_valid_bridge_transfer_id"></a>

## Function `assert_valid_bridge_transfer_id`

Asserts that the bridge transfer ID is valid.

@param bridge_transfer_id The bridge transfer ID to validate.
@abort If the ID is invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bridge_transfer_id) == 32, <a href="native_bridge.md#0x1_native_bridge_store_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>);
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_bridge_transfer_id"></a>

## Function `bridge_transfer_id`

Generates a unique bridge transfer ID based on transfer details and nonce.

@param details The bridge transfer details.
@return The generated bridge transfer ID.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_bridge_transfer_id">bridge_transfer_id</a>(initiator: <b>address</b>, recipient: <a href="atomic_bridge.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, amount: u64, nonce: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_bridge_transfer_id">bridge_transfer_id</a>(initiator: <b>address</b>, recipient: EthereumAddress, amount: u64, nonce: u64) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> combined_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&initiator));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&recipient));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amount));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&nonce));
    keccak256(combined_bytes)
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_get_bridge_transfer_details_from_nonce"></a>

## Function `get_bridge_transfer_details_from_nonce`

Gets the bridge transfer details (<code><a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a></code>) from the given nonce.
@param nonce The nonce of the bridge transfer.
@return The <code><a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a></code> struct containing the transfer details.
@abort If the nonce is not found in the smart table.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_get_bridge_transfer_details_from_nonce">get_bridge_transfer_details_from_nonce</a>(nonce: u64): <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">native_bridge_store::OutboundTransfer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_get_bridge_transfer_details_from_nonce">get_bridge_transfer_details_from_nonce</a>(nonce: u64): <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a> <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework);

    // Check <b>if</b> the nonce <b>exists</b> in the <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, nonce), <a href="native_bridge.md#0x1_native_bridge_store_ENONCE_NOT_FOUND">ENONCE_NOT_FOUND</a>);

    // If it <b>exists</b>, <b>return</b> the associated `<a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>` details
    *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, nonce)
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_get_inbound_nonce_from_bridge_transfer_id"></a>

## Function `get_inbound_nonce_from_bridge_transfer_id`

Gets inbound <code>nonce</code> from <code>bridge_transfer_id</code>
@param bridge_transfer_id The ID bridge transfer.
@return the nonce
@abort If the nonce is not found in the smart table.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_get_inbound_nonce_from_bridge_transfer_id">get_inbound_nonce_from_bridge_transfer_id</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_get_inbound_nonce_from_bridge_transfer_id">get_inbound_nonce_from_bridge_transfer_id</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework);

     // Check <b>if</b> the nonce <b>exists</b> in the <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id), <a href="native_bridge.md#0x1_native_bridge_store_ENONCE_NOT_FOUND">ENONCE_NOT_FOUND</a>);

    // If it <b>exists</b>, <b>return</b> the associated nonce
    *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_is_inbound_nonce_set"></a>

### Function `is_inbound_nonce_set`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_is_inbound_nonce_set">is_inbound_nonce_set</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>




<pre><code><b>ensures</b> result == <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework)
    && <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(
        <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework).inner,
        bridge_transfer_id
    );
</code></pre>



<a id="@Specification_1_create_details"></a>

### Function `create_details`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_create_details">create_details</a>(initiator: <b>address</b>, recipient: <a href="atomic_bridge.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, amount: u64, nonce: u64): <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">native_bridge_store::OutboundTransfer</a>
</code></pre>




<pre><code><b>aborts_if</b> amount == 0;
<b>ensures</b> result.bridge_transfer_id == <a href="native_bridge.md#0x1_native_bridge_store_bridge_transfer_id">bridge_transfer_id</a>(
    initiator,
    recipient,
    amount,
    nonce
);
<b>ensures</b> result.initiator == initiator;
<b>ensures</b> result.recipient == recipient;
<b>ensures</b> result.amount == amount;
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_add">add</a>(nonce: u64, details: <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">native_bridge_store::OutboundTransfer</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework);
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework).inner,
    nonce
);
<b>ensures</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework).inner,
    nonce
);
<b>ensures</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_len">smart_table::spec_len</a>(
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework).inner
) == <b>old</b>(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_len">smart_table::spec_len</a>(
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_store_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework).inner
)) + 1;
</code></pre>



<a id="@Specification_1_set_bridge_transfer_id_to_inbound_nonce"></a>

### Function `set_bridge_transfer_id_to_inbound_nonce`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_set_bridge_transfer_id_to_inbound_nonce">set_bridge_transfer_id_to_inbound_nonce</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, inbound_nonce: u64)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework);
<b>ensures</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework).inner,
    bridge_transfer_id
);
</code></pre>



<a id="0x1_native_bridge_configuration"></a>

# Module `0x1::native_bridge_configuration`



-  [Resource `BridgeConfig`](#0x1_native_bridge_configuration_BridgeConfig)
-  [Struct `BridgeConfigRelayerUpdated`](#0x1_native_bridge_configuration_BridgeConfigRelayerUpdated)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_native_bridge_configuration_initialize)
-  [Function `update_bridge_relayer`](#0x1_native_bridge_configuration_update_bridge_relayer)
-  [Function `bridge_relayer`](#0x1_native_bridge_configuration_bridge_relayer)
-  [Function `assert_is_caller_relayer`](#0x1_native_bridge_configuration_assert_is_caller_relayer)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `update_bridge_relayer`](#@Specification_1_update_bridge_relayer)
    -  [Function `bridge_relayer`](#@Specification_1_bridge_relayer)
    -  [Function `assert_is_caller_relayer`](#@Specification_1_assert_is_caller_relayer)


<pre><code><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_native_bridge_configuration_BridgeConfig"></a>

## Resource `BridgeConfig`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_relayer: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_configuration_BridgeConfigRelayerUpdated"></a>

## Struct `BridgeConfigRelayerUpdated`

Event emitted when the bridge relayer is updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfigRelayerUpdated">BridgeConfigRelayerUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_relayer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_relayer: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_native_bridge_configuration_EINVALID_BRIDGE_RELAYER"></a>

Error code for invalid bridge relayer


<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_configuration_EINVALID_BRIDGE_RELAYER">EINVALID_BRIDGE_RELAYER</a>: u64 = 1;
</code></pre>



<a id="0x1_native_bridge_configuration_initialize"></a>

## Function `initialize`

Initializes the bridge configuration with Aptos framework as the bridge relayer.

@param aptos_framework The signer representing the Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> bridge_config = <a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a> {
        bridge_relayer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework),
    };
    <b>move_to</b>(aptos_framework, bridge_config);
}
</code></pre>



</details>

<a id="0x1_native_bridge_configuration_update_bridge_relayer"></a>

## Function `update_bridge_relayer`

Updates the bridge relayer, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_relayer The new address to be set as the bridge relayer.
@abort If the current relayer is the same as the new relayer.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_update_bridge_relayer">update_bridge_relayer</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_relayer: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_update_bridge_relayer">update_bridge_relayer</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_relayer: <b>address</b>
)   <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> bridge_config = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework);
    <b>let</b> old_relayer = bridge_config.bridge_relayer;
    <b>assert</b>!(old_relayer != new_relayer, <a href="native_bridge.md#0x1_native_bridge_configuration_EINVALID_BRIDGE_RELAYER">EINVALID_BRIDGE_RELAYER</a>);

    bridge_config.bridge_relayer = new_relayer;

    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfigRelayerUpdated">BridgeConfigRelayerUpdated</a> {
            old_relayer,
            new_relayer,
        },
    );
}
</code></pre>



</details>

<a id="0x1_native_bridge_configuration_bridge_relayer"></a>

## Function `bridge_relayer`

Retrieves the address of the current bridge relayer.

@return The address of the current bridge relayer.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_bridge_relayer">bridge_relayer</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_bridge_relayer">bridge_relayer</a>(): <b>address</b> <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_relayer
}
</code></pre>



</details>

<a id="0x1_native_bridge_configuration_assert_is_caller_relayer"></a>

## Function `assert_is_caller_relayer`

Asserts that the caller is the current bridge relayer.

@param caller The signer whose authority is being checked.
@abort If the caller is not the current bridge relayer.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_assert_is_caller_relayer">assert_is_caller_relayer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_assert_is_caller_relayer">assert_is_caller_relayer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <b>assert</b>!(<b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_relayer == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(caller), <a href="native_bridge.md#0x1_native_bridge_configuration_EINVALID_BRIDGE_RELAYER">EINVALID_BRIDGE_RELAYER</a>);
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>ensures</b> <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)).bridge_relayer == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
</code></pre>



<a id="@Specification_1_update_bridge_relayer"></a>

### Function `update_bridge_relayer`


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_update_bridge_relayer">update_bridge_relayer</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_relayer: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)).bridge_relayer == new_relayer;
<b>ensures</b> <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)).bridge_relayer == new_relayer;
</code></pre>



<a id="@Specification_1_bridge_relayer"></a>

### Function `bridge_relayer`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_bridge_relayer">bridge_relayer</a>(): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework);
<b>ensures</b> result == <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_relayer;
</code></pre>



<a id="@Specification_1_assert_is_caller_relayer"></a>

### Function `assert_is_caller_relayer`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_configuration_assert_is_caller_relayer">assert_is_caller_relayer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework);
<b>aborts_if</b> <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_relayer != <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(caller);
</code></pre>



<a id="0x1_native_bridge_core"></a>

# Module `0x1::native_bridge_core`



-  [Resource `AptosCoinBurnCapability`](#0x1_native_bridge_core_AptosCoinBurnCapability)
-  [Resource `AptosCoinMintCapability`](#0x1_native_bridge_core_AptosCoinMintCapability)
-  [Resource `AptosFABurnCapabilities`](#0x1_native_bridge_core_AptosFABurnCapabilities)
-  [Resource `AptosFAMintCapabilities`](#0x1_native_bridge_core_AptosFAMintCapabilities)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_native_bridge_core_initialize)
-  [Function `store_aptos_coin_burn_cap`](#0x1_native_bridge_core_store_aptos_coin_burn_cap)
-  [Function `store_aptos_coin_mint_cap`](#0x1_native_bridge_core_store_aptos_coin_mint_cap)
-  [Function `mint`](#0x1_native_bridge_core_mint)
-  [Function `burn`](#0x1_native_bridge_core_burn)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `store_aptos_coin_burn_cap`](#@Specification_1_store_aptos_coin_burn_cap)
    -  [Function `store_aptos_coin_mint_cap`](#@Specification_1_store_aptos_coin_mint_cap)
    -  [Function `mint`](#@Specification_1_mint)
    -  [Function `burn`](#@Specification_1_burn)


<pre><code><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="native_bridge.md#0x1_native_bridge_configuration">0x1::native_bridge_configuration</a>;
<b>use</b> <a href="native_bridge.md#0x1_native_bridge_store">0x1::native_bridge_store</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_native_bridge_core_AptosCoinBurnCapability"></a>

## Resource `AptosCoinBurnCapability`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_core_AptosCoinMintCapability"></a>

## Resource `AptosCoinMintCapability`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_core_AptosFABurnCapabilities"></a>

## Resource `AptosFABurnCapabilities`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_core_AptosFABurnCapabilities">AptosFABurnCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_ref: <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_core_AptosFAMintCapabilities"></a>

## Resource `AptosFAMintCapabilities`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_core_AptosFAMintCapabilities">AptosFAMintCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_ref: <a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_native_bridge_core_ENATIVE_BRIDGE_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_core_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>: u64 = 1;
</code></pre>



<a id="0x1_native_bridge_core_initialize"></a>

## Function `initialize`

Initializes the atomic bridge by setting up necessary configurations.

@param aptos_framework The signer representing the Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="native_bridge.md#0x1_native_bridge_configuration_initialize">native_bridge_configuration::initialize</a>(aptos_framework);
    <a href="native_bridge.md#0x1_native_bridge_store_initialize">native_bridge_store::initialize</a>(aptos_framework);
}
</code></pre>



</details>

<a id="0x1_native_bridge_core_store_aptos_coin_burn_cap"></a>

## Function `store_aptos_coin_burn_cap`

Stores the burn capability for AptosCoin, converting to a fungible asset reference if the feature is enabled.

@param aptos_framework The signer representing the Aptos framework.
@param burn_cap The burn capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: BurnCapability&lt;AptosCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
        <b>let</b> burn_ref = <a href="coin.md#0x1_coin_convert_and_take_paired_burn_ref">coin::convert_and_take_paired_burn_ref</a>(burn_cap);
        <b>move_to</b>(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_core_AptosFABurnCapabilities">AptosFABurnCapabilities</a> { burn_ref });
    } <b>else</b> {
        <b>move_to</b>(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a> { burn_cap })
    }
}
</code></pre>



</details>

<a id="0x1_native_bridge_core_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

Stores the mint capability for AptosCoin.

@param aptos_framework The signer representing the Aptos framework.
@param mint_cap The mint capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: MintCapability&lt;AptosCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>move_to</b>(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a> { mint_cap })
}
</code></pre>



</details>

<a id="0x1_native_bridge_core_mint"></a>

## Function `mint`

Mints a specified amount of AptosCoin to a recipient's address.

@param recipient The address of the recipient to mint coins to.
@param amount The amount of AptosCoin to mint.
@abort If the mint capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_mint">mint</a>(recipient: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_mint">mint</a>(recipient: <b>address</b>, amount: u64) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_native_bridge_enabled">features::abort_native_bridge_enabled</a>(), <a href="native_bridge.md#0x1_native_bridge_core_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>);

    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(recipient, <a href="coin.md#0x1_coin_mint">coin::mint</a>(
        amount,
        &<b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a>&gt;(@aptos_framework).mint_cap
    ));
}
</code></pre>



</details>

<a id="0x1_native_bridge_core_burn"></a>

## Function `burn`

Burns a specified amount of AptosCoin from an address.

@param from The address from which to burn AptosCoin.
@param amount The amount of AptosCoin to burn.
@abort If the burn capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_burn">burn</a>(from: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_burn">burn</a>(from: <b>address</b>, amount: u64) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_native_bridge_enabled">features::abort_native_bridge_enabled</a>(), <a href="native_bridge.md#0x1_native_bridge_core_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>);

    <a href="coin.md#0x1_coin_burn_from">coin::burn_from</a>(
        from,
        amount,
        &<b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a>&gt;(@aptos_framework).burn_cap,
    );
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a>&gt;(@aptos_framework);
<b>aborts_if</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a>&gt;(@aptos_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a>&gt;(@aptos_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_store_aptos_coin_burn_cap"></a>

### Function `store_aptos_coin_burn_cap`


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a>&gt;(@aptos_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_store_aptos_coin_mint_cap"></a>

### Function `store_aptos_coin_mint_cap`


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a>&gt;(@aptos_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_mint">mint</a>(recipient: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinMintCapability">AptosCoinMintCapability</a>&gt;(@aptos_framework);
<b>aborts_if</b> amount == 0;
<b>ensures</b> <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(recipient) == <b>old</b>(<a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(recipient)) + amount;
</code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_core_burn">burn</a>(from: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_core_AptosCoinBurnCapability">AptosCoinBurnCapability</a>&gt;(@aptos_framework);
<b>aborts_if</b> <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(from) &lt; amount;
<b>ensures</b> <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(from) == <b>old</b>(<a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(from)) - amount;
</code></pre>



<a id="0x1_native_bridge"></a>

# Module `0x1::native_bridge`



-  [Struct `BridgeTransferInitiatedEvent`](#0x1_native_bridge_BridgeTransferInitiatedEvent)
-  [Struct `BridgeTransferCompletedEvent`](#0x1_native_bridge_BridgeTransferCompletedEvent)
-  [Resource `BridgeEvents`](#0x1_native_bridge_BridgeEvents)
-  [Resource `Nonce`](#0x1_native_bridge_Nonce)
-  [Constants](#@Constants_0)
-  [Function `increment_and_get_nonce`](#0x1_native_bridge_increment_and_get_nonce)
-  [Function `initialize`](#0x1_native_bridge_initialize)
-  [Function `initiate_bridge_transfer`](#0x1_native_bridge_initiate_bridge_transfer)
-  [Function `complete_bridge_transfer`](#0x1_native_bridge_complete_bridge_transfer)
-  [Specification](#@Specification_1)
    -  [Function `increment_and_get_nonce`](#@Specification_1_increment_and_get_nonce)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `initiate_bridge_transfer`](#@Specification_1_initiate_bridge_transfer)
    -  [Function `complete_bridge_transfer`](#@Specification_1_complete_bridge_transfer)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_ethereum">0x1::ethereum</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="native_bridge.md#0x1_native_bridge_configuration">0x1::native_bridge_configuration</a>;
<b>use</b> <a href="native_bridge.md#0x1_native_bridge_core">0x1::native_bridge_core</a>;
<b>use</b> <a href="native_bridge.md#0x1_native_bridge_store">0x1::native_bridge_store</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_native_bridge_BridgeTransferInitiatedEvent"></a>

## Struct `BridgeTransferInitiatedEvent`

An event triggered upon initiating a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>initiator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>nonce: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_BridgeTransferCompletedEvent"></a>

## Struct `BridgeTransferCompletedEvent`

An event triggered upon completing a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>nonce: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_BridgeEvents"></a>

## Resource `BridgeEvents`

This struct will store the event handles for bridge events.


<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_initiated_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeTransferInitiatedEvent">native_bridge::BridgeTransferInitiatedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_completed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeTransferCompletedEvent">native_bridge::BridgeTransferCompletedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_Nonce"></a>

## Resource `Nonce`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>: u64 = 2;
</code></pre>



<a id="0x1_native_bridge_EEVENT_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EEVENT_NOT_FOUND">EEVENT_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x1_native_bridge_EINVALID_NONCE"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_NONCE">EINVALID_NONCE</a>: u64 = 4;
</code></pre>



<a id="0x1_native_bridge_ETRANSFER_ALREADY_PROCESSED"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ETRANSFER_ALREADY_PROCESSED">ETRANSFER_ALREADY_PROCESSED</a>: u64 = 1;
</code></pre>



<a id="0x1_native_bridge_increment_and_get_nonce"></a>

## Function `increment_and_get_nonce`

Increment and get the current nonce


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_increment_and_get_nonce">increment_and_get_nonce</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_increment_and_get_nonce">increment_and_get_nonce</a>(): u64 <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> {
    <b>let</b> nonce_ref = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework);
    nonce_ref.value = nonce_ref.value + 1;
    nonce_ref.value
}
</code></pre>



</details>

<a id="0x1_native_bridge_initialize"></a>

## Function `initialize`

Initializes the module and stores the <code>EventHandle</code>s in the resource.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    // Ensure the nonce is not already initialized
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)),
        2
    );

    // Create the <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> resource <b>with</b> an initial value of 0
    <b>move_to</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> {
        value: 0
    });

    <b>move_to</b>(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a> {
        bridge_transfer_initiated_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a>&gt;(aptos_framework),
        bridge_transfer_completed_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a>&gt;(aptos_framework),
    });
}
</code></pre>



</details>

<a id="0x1_native_bridge_initiate_bridge_transfer"></a>

## Function `initiate_bridge_transfer`

Initiate a bridge transfer of MOVE from Movement to Ethereum
Anyone can initiate a bridge transfer from the source chain
The amount is burnt from the initiator and the module-level nonce is incremented
@param initiator The initiator's Ethereum address as a vector of bytes.
@param recipient The address of the recipient on the Aptos blockchain.
@param amount The amount of assets to be locked.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initiate_bridge_transfer">initiate_bridge_transfer</a>(initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initiate_bridge_transfer">initiate_bridge_transfer</a>(
    initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    amount: u64
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>, <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> {
    <b>let</b> initiator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(initiator);
    <b>let</b> recipient_address = <a href="atomic_bridge.md#0x1_ethereum_ethereum_address_no_eip55">ethereum::ethereum_address_no_eip55</a>(recipient);

    // Increment and retrieve the nonce
    <b>let</b> nonce = <a href="native_bridge.md#0x1_native_bridge_increment_and_get_nonce">increment_and_get_nonce</a>();

    // Create bridge transfer details
    <b>let</b> details = <a href="native_bridge.md#0x1_native_bridge_store_create_details">native_bridge_store::create_details</a>(
        initiator_address,
        recipient_address,
        amount,
        nonce
    );

    <b>let</b> bridge_transfer_id = <a href="native_bridge.md#0x1_native_bridge_store_bridge_transfer_id">native_bridge_store::bridge_transfer_id</a>(
        initiator_address,
        recipient_address,
        amount,
        nonce
    );

    // Add the transfer details <b>to</b> storage
    <a href="native_bridge.md#0x1_native_bridge_store_add">native_bridge_store::add</a>(nonce, details);

    // Burn the amount from the initiator
    <a href="native_bridge.md#0x1_native_bridge_core_burn">native_bridge_core::burn</a>(initiator_address, amount);

    <b>let</b> bridge_events = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework);

    // Emit an <a href="event.md#0x1_event">event</a> <b>with</b> nonce
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
         &<b>mut</b> bridge_events.bridge_transfer_initiated_events,
        <a href="native_bridge.md#0x1_native_bridge_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a> {
            bridge_transfer_id,
            initiator: initiator_address,
            recipient,
            amount,
            nonce,
        }
    );
}
</code></pre>



</details>

<a id="0x1_native_bridge_complete_bridge_transfer"></a>

## Function `complete_bridge_transfer`

Completes a bridge transfer by the initiator.

@param caller The signer representing the bridge relayer.
@param initiator The initiator's Ethereum address as a vector of bytes.
@param bridge_transfer_id The unique identifier for the bridge transfer.
@param recipient The address of the recipient on the Aptos blockchain.
@param amount The amount of assets to be locked.
@param nonce The unique nonce for the transfer.
@abort If the caller is not the bridge relayer or the transfer has already been processed.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_complete_bridge_transfer">complete_bridge_transfer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient: <b>address</b>, amount: u64, nonce: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_complete_bridge_transfer">complete_bridge_transfer</a>(
    caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    recipient: <b>address</b>,
    amount: u64,
    nonce: u64
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a> {
    // Ensure the caller is the bridge relayer
    <a href="native_bridge.md#0x1_native_bridge_configuration_assert_is_caller_relayer">native_bridge_configuration::assert_is_caller_relayer</a>(caller);

    // Check <b>if</b> the bridge transfer ID is already associated <b>with</b> an inbound nonce
    <b>let</b> inbound_nonce_exists = <a href="native_bridge.md#0x1_native_bridge_store_is_inbound_nonce_set">native_bridge_store::is_inbound_nonce_set</a>(bridge_transfer_id);
    <b>assert</b>!(!inbound_nonce_exists, <a href="native_bridge.md#0x1_native_bridge_ETRANSFER_ALREADY_PROCESSED">ETRANSFER_ALREADY_PROCESSED</a>); // Abort <b>if</b> the transfer is already processed
    <b>assert</b>!(nonce &gt; 0, <a href="native_bridge.md#0x1_native_bridge_EINVALID_NONCE">EINVALID_NONCE</a>);

    // Validate the bridge_transfer_id by reconstructing the <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
    <b>let</b> combined_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&initiator));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&recipient));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amount));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&nonce));
    <b>assert</b>!(keccak256(combined_bytes) == bridge_transfer_id, <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>);

    // Record the transfer <b>as</b> completed by associating the bridge_transfer_id <b>with</b> the inbound nonce
    <a href="native_bridge.md#0x1_native_bridge_store_set_bridge_transfer_id_to_inbound_nonce">native_bridge_store::set_bridge_transfer_id_to_inbound_nonce</a>(bridge_transfer_id, nonce);

    // Mint <b>to</b> the recipient
    <a href="native_bridge.md#0x1_native_bridge_core_mint">native_bridge_core::mint</a>(recipient, amount);

    // Emit the <a href="event.md#0x1_event">event</a>
    <b>let</b> bridge_events = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> bridge_events.bridge_transfer_completed_events,
        <a href="native_bridge.md#0x1_native_bridge_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> {
            bridge_transfer_id,
            initiator,
            recipient,
            amount,
            nonce,
        },
    );
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_increment_and_get_nonce"></a>

### Function `increment_and_get_nonce`


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_increment_and_get_nonce">increment_and_get_nonce</a>(): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework);
<b>ensures</b> <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework).value == <b>old</b>(<b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework).value) + 1;
<b>ensures</b> result == <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework).value;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>ensures</b> <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)).value == 1;
<b>ensures</b> <b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>ensures</b>
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework))
        .bridge_transfer_initiated_events.counter == 0;
<b>ensures</b>
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework))
        .bridge_transfer_completed_events.counter == 0;
</code></pre>



<a id="@Specification_1_initiate_bridge_transfer"></a>

### Function `initiate_bridge_transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initiate_bridge_transfer">initiate_bridge_transfer</a>(initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, amount: u64)
</code></pre>




<pre><code><b>aborts_if</b> amount == 0;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework);
<b>ensures</b> <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework).value == <b>old</b>(<b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework).value) + 1;
<b>ensures</b>
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework).bridge_transfer_initiated_events.counter ==
    <b>old</b>(
        <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework).bridge_transfer_initiated_events.counter
    ) + 1;
</code></pre>



<a id="@Specification_1_complete_bridge_transfer"></a>

### Function `complete_bridge_transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_complete_bridge_transfer">complete_bridge_transfer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient: <b>address</b>, amount: u64, nonce: u64)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">native_bridge_configuration::BridgeConfig</a>&gt;(@aptos_framework);
<b>aborts_if</b> <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_configuration_BridgeConfig">native_bridge_configuration::BridgeConfig</a>&gt;(@aptos_framework).bridge_relayer != <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(caller);
<b>aborts_if</b> <a href="native_bridge.md#0x1_native_bridge_store_is_inbound_nonce_set">native_bridge_store::is_inbound_nonce_set</a>(bridge_transfer_id);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework);
<b>ensures</b> <a href="native_bridge.md#0x1_native_bridge_store_is_inbound_nonce_set">native_bridge_store::is_inbound_nonce_set</a>(bridge_transfer_id);
<b>ensures</b>
    <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework).bridge_transfer_completed_events.counter ==
    <b>old</b>(
        <b>global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework).bridge_transfer_completed_events.counter
    ) + 1;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
