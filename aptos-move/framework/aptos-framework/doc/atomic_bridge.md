
<a id="0x1_atomic_bridge_store"></a>

# Module `0x1::atomic_bridge_store`



-  [Struct `AddressPair`](#0x1_atomic_bridge_store_AddressPair)
-  [Resource `SmartTableWrapper`](#0x1_atomic_bridge_store_SmartTableWrapper)
-  [Struct `BridgeTransferDetails`](#0x1_atomic_bridge_store_BridgeTransferDetails)
-  [Resource `Nonce`](#0x1_atomic_bridge_store_Nonce)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_atomic_bridge_store_initialize)
-  [Function `now`](#0x1_atomic_bridge_store_now)
-  [Function `create_time_lock`](#0x1_atomic_bridge_store_create_time_lock)
-  [Function `create_details`](#0x1_atomic_bridge_store_create_details)
-  [Function `add`](#0x1_atomic_bridge_store_add)
-  [Function `assert_min_time_lock`](#0x1_atomic_bridge_store_assert_min_time_lock)
-  [Function `assert_pending`](#0x1_atomic_bridge_store_assert_pending)
-  [Function `assert_valid_hash_lock`](#0x1_atomic_bridge_store_assert_valid_hash_lock)
-  [Function `assert_valid_bridge_transfer_id`](#0x1_atomic_bridge_store_assert_valid_bridge_transfer_id)
-  [Function `create_hashlock`](#0x1_atomic_bridge_store_create_hashlock)
-  [Function `assert_correct_hash_lock`](#0x1_atomic_bridge_store_assert_correct_hash_lock)
-  [Function `assert_timed_out_lock`](#0x1_atomic_bridge_store_assert_timed_out_lock)
-  [Function `assert_within_timelock`](#0x1_atomic_bridge_store_assert_within_timelock)
-  [Function `complete`](#0x1_atomic_bridge_store_complete)
-  [Function `cancel`](#0x1_atomic_bridge_store_cancel)
-  [Function `complete_details`](#0x1_atomic_bridge_store_complete_details)
-  [Function `complete_transfer`](#0x1_atomic_bridge_store_complete_transfer)
-  [Function `cancel_details`](#0x1_atomic_bridge_store_cancel_details)
-  [Function `cancel_transfer`](#0x1_atomic_bridge_store_cancel_transfer)
-  [Function `bridge_transfer_id`](#0x1_atomic_bridge_store_bridge_transfer_id)
-  [Function `get_bridge_transfer_details_initiator`](#0x1_atomic_bridge_store_get_bridge_transfer_details_initiator)
-  [Function `get_bridge_transfer_details_counterparty`](#0x1_atomic_bridge_store_get_bridge_transfer_details_counterparty)
-  [Function `get_bridge_transfer_details`](#0x1_atomic_bridge_store_get_bridge_transfer_details)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `create_time_lock`](#@Specification_1_create_time_lock)
    -  [Function `create_details`](#@Specification_1_create_details)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `create_hashlock`](#@Specification_1_create_hashlock)
    -  [Function `complete`](#@Specification_1_complete)
    -  [Function `cancel`](#@Specification_1_cancel)
    -  [Function `complete_details`](#@Specification_1_complete_details)
    -  [Function `complete_transfer`](#@Specification_1_complete_transfer)
    -  [Function `cancel_details`](#@Specification_1_cancel_details)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="ethereum.md#0x1_ethereum">0x1::ethereum</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_atomic_bridge_store_AddressPair"></a>

## Struct `AddressPair`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_AddressPair">AddressPair</a>&lt;Initiator: store, Recipient: store&gt; <b>has</b> <b>copy</b>, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>initiator: Initiator</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: Recipient</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_store_SmartTableWrapper"></a>

## Resource `SmartTableWrapper`

A smart table wrapper


<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;K, V&gt; <b>has</b> store, key
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

<a id="0x1_atomic_bridge_store_BridgeTransferDetails"></a>

## Struct `BridgeTransferDetails`

Details on the transfer


<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator: store, Recipient: store&gt; <b>has</b> <b>copy</b>, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addresses: <a href="atomic_bridge.md#0x1_atomic_bridge_store_AddressPair">atomic_bridge_store::AddressPair</a>&lt;Initiator, Recipient&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>state: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_store_Nonce"></a>

## Resource `Nonce`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_Nonce">Nonce</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_atomic_bridge_store_MAX_U64"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_atomic_bridge_store_EATOMIC_BRIDGE_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>: u64 = 9;
</code></pre>



<a id="0x1_atomic_bridge_store_CANCELLED_TRANSACTION"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_CANCELLED_TRANSACTION">CANCELLED_TRANSACTION</a>: u8 = 3;
</code></pre>



<a id="0x1_atomic_bridge_store_COMPLETED_TRANSACTION"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_COMPLETED_TRANSACTION">COMPLETED_TRANSACTION</a>: u8 = 2;
</code></pre>



<a id="0x1_atomic_bridge_store_EEXPIRED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EEXPIRED">EEXPIRED</a>: u64 = 3;
</code></pre>



<a id="0x1_atomic_bridge_store_EINVALID_BRIDGE_TRANSFER_ID"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>: u64 = 8;
</code></pre>



<a id="0x1_atomic_bridge_store_EINVALID_HASH_LOCK"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_HASH_LOCK">EINVALID_HASH_LOCK</a>: u64 = 5;
</code></pre>



<a id="0x1_atomic_bridge_store_EINVALID_PRE_IMAGE"></a>

Error codes


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_PRE_IMAGE">EINVALID_PRE_IMAGE</a>: u64 = 1;
</code></pre>



<a id="0x1_atomic_bridge_store_EINVALID_TIME_LOCK"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_TIME_LOCK">EINVALID_TIME_LOCK</a>: u64 = 6;
</code></pre>



<a id="0x1_atomic_bridge_store_ENOT_EXPIRED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_ENOT_EXPIRED">ENOT_EXPIRED</a>: u64 = 4;
</code></pre>



<a id="0x1_atomic_bridge_store_ENOT_PENDING_TRANSACTION"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_ENOT_PENDING_TRANSACTION">ENOT_PENDING_TRANSACTION</a>: u64 = 2;
</code></pre>



<a id="0x1_atomic_bridge_store_EZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 7;
</code></pre>



<a id="0x1_atomic_bridge_store_MIN_TIME_LOCK"></a>

Minimum time lock of 1 second


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_MIN_TIME_LOCK">MIN_TIME_LOCK</a>: u64 = 1;
</code></pre>



<a id="0x1_atomic_bridge_store_PENDING_TRANSACTION"></a>

Transaction states


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>: u8 = 1;
</code></pre>



<a id="0x1_atomic_bridge_store_initialize"></a>

## Function `initialize`

Initializes the initiators and counterparties tables and nonce.

@param aptos_framework The signer for Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>move_to</b>(aptos_framework, <a href="atomic_bridge.md#0x1_atomic_bridge_store_Nonce">Nonce</a> {
        inner: 0,
    });

    <b>let</b> initiators = <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;<b>address</b>, EthereumAddress&gt;&gt; {
        inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    };

    <b>move_to</b>(aptos_framework, initiators);

    <b>let</b> counterparties = <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;EthereumAddress, <b>address</b>&gt;&gt; {
        inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    };

    <b>move_to</b>(aptos_framework, counterparties);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_now"></a>

## Function `now`

Returns the current time in seconds.

@return Current timestamp in seconds.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>() : u64 {
    <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_create_time_lock"></a>

## Function `create_time_lock`

Creates a time lock by adding a duration to the current time.

@param lock The duration to lock.
@return The calculated time lock.
@abort If lock is not above MIN_TIME_LOCK


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_time_lock">create_time_lock</a>(time_lock: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_time_lock">create_time_lock</a>(time_lock: u64) : u64 {
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_min_time_lock">assert_min_time_lock</a>(time_lock);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>() + time_lock
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_create_details"></a>

## Function `create_details`

Creates bridge transfer details with validation.

@param initiator The initiating party of the transfer.
@param recipient The receiving party of the transfer.
@param amount The amount to be transferred.
@param hash_lock The hash lock for the transfer.
@param time_lock The time lock for the transfer.
@return A <code><a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a></code> object.
@abort If the amount is zero or locks are invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_details">create_details</a>&lt;Initiator: store, Recipient: store&gt;(initiator: Initiator, recipient: Recipient, amount: u64, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, time_lock: u64): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_details">create_details</a>&lt;Initiator: store, Recipient: store&gt;(initiator: Initiator, recipient: Recipient, amount: u64, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, time_lock: u64)
    : <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt; {
    <b>assert</b>!(amount &gt; 0, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EZERO_AMOUNT">EZERO_AMOUNT</a>);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_hash_lock">assert_valid_hash_lock</a>(&hash_lock);
    time_lock = <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_time_lock">create_time_lock</a>(time_lock);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a> {
        addresses: <a href="atomic_bridge.md#0x1_atomic_bridge_store_AddressPair">AddressPair</a> {
            initiator,
            recipient
        },
        amount,
        hash_lock,
        time_lock,
        state: <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>,
    }
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_add"></a>

## Function `add`

Record details of a transfer

@param bridge_transfer_id Bridge transfer ID.
@param details The bridge transfer details


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_add">add</a>&lt;Initiator: store, Recipient: store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, details: <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_add">add</a>&lt;Initiator: store, Recipient: store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, details: <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_atomic_bridge_enabled">features::abort_atomic_bridge_enabled</a>(), <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(&bridge_transfer_id);
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id, details);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_min_time_lock"></a>

## Function `assert_min_time_lock`

Asserts that the time lock is valid.

@param time_lock
@abort If the time lock is invalid.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_min_time_lock">assert_min_time_lock</a>(time_lock: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_min_time_lock">assert_min_time_lock</a>(time_lock: u64) {
    <b>assert</b>!(time_lock &gt;= <a href="atomic_bridge.md#0x1_atomic_bridge_store_MIN_TIME_LOCK">MIN_TIME_LOCK</a>, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_TIME_LOCK">EINVALID_TIME_LOCK</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_pending"></a>

## Function `assert_pending`

Asserts that the details state is pending.

@param details The bridge transfer details to check.
@abort If the state is not pending.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_pending">assert_pending</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_pending">assert_pending</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    <b>assert</b>!(details.state == <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>, <a href="atomic_bridge.md#0x1_atomic_bridge_store_ENOT_PENDING_TRANSACTION">ENOT_PENDING_TRANSACTION</a>)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_valid_hash_lock"></a>

## Function `assert_valid_hash_lock`

Asserts that the hash lock is valid.

@param hash_lock The hash lock to validate.
@abort If the hash lock is invalid.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_hash_lock">assert_valid_hash_lock</a>(hash_lock: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_hash_lock">assert_valid_hash_lock</a>(hash_lock: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(hash_lock) == 32, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_HASH_LOCK">EINVALID_HASH_LOCK</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_valid_bridge_transfer_id"></a>

## Function `assert_valid_bridge_transfer_id`

Asserts that the bridge transfer ID is valid.

@param bridge_transfer_id The bridge transfer ID to validate.
@abort If the ID is invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bridge_transfer_id) == 32, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_create_hashlock"></a>

## Function `create_hashlock`

Creates a hash lock from a pre-image.

@param pre_image The pre-image to hash.
@return The generated hash lock.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_hashlock">create_hashlock</a>(pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_hashlock">create_hashlock</a>(pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&pre_image) &gt; 0, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_PRE_IMAGE">EINVALID_PRE_IMAGE</a>);
    keccak256(pre_image)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_correct_hash_lock"></a>

## Function `assert_correct_hash_lock`

Asserts that the hash lock matches the expected value.

@param details The bridge transfer details.
@param hash_lock The hash lock to compare.
@abort If the hash lock is incorrect.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_correct_hash_lock">assert_correct_hash_lock</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_correct_hash_lock">assert_correct_hash_lock</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>assert</b>!(&hash_lock == &details.hash_lock, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_PRE_IMAGE">EINVALID_PRE_IMAGE</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_timed_out_lock"></a>

## Function `assert_timed_out_lock`

Asserts that the time lock has expired.

@param details The bridge transfer details.
@abort If the time lock has not expired.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_timed_out_lock">assert_timed_out_lock</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_timed_out_lock">assert_timed_out_lock</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    <b>assert</b>!(<a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>() &gt; details.time_lock, <a href="atomic_bridge.md#0x1_atomic_bridge_store_ENOT_EXPIRED">ENOT_EXPIRED</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_within_timelock"></a>

## Function `assert_within_timelock`

Asserts we are still within the timelock.

@param details The bridge transfer details.
@abort If the time lock has expired.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_within_timelock">assert_within_timelock</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_within_timelock">assert_within_timelock</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    <b>assert</b>!(!(<a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>() &gt; details.time_lock), <a href="atomic_bridge.md#0x1_atomic_bridge_store_EEXPIRED">EEXPIRED</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_complete"></a>

## Function `complete`

Completes the bridge transfer.

@param details The bridge transfer details to complete.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete">complete</a>&lt;Initiator: store, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete">complete</a>&lt;Initiator: store, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    details.state = <a href="atomic_bridge.md#0x1_atomic_bridge_store_COMPLETED_TRANSACTION">COMPLETED_TRANSACTION</a>;
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_cancel"></a>

## Function `cancel`

Cancels the bridge transfer.

@param details The bridge transfer details to cancel.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel">cancel</a>&lt;Initiator: store, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel">cancel</a>&lt;Initiator: store, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    details.state = <a href="atomic_bridge.md#0x1_atomic_bridge_store_CANCELLED_TRANSACTION">CANCELLED_TRANSACTION</a>;
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_complete_details"></a>

## Function `complete_details`

Validates and completes a bridge transfer by confirming the hash lock and state.

@param hash_lock The hash lock used to validate the transfer.
@param details The mutable reference to the bridge transfer details to be completed.
@return A tuple containing the recipient and the amount of the transfer.
@abort If the hash lock is invalid, the transfer is not pending, or the hash lock does not match.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_details">complete_details</a>&lt;Initiator: store, Recipient: <b>copy</b>, store&gt;(hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;): (Recipient, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_details">complete_details</a>&lt;Initiator: store, Recipient: store + <b>copy</b>&gt;(hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) : (Recipient, u64) {
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_hash_lock">assert_valid_hash_lock</a>(&hash_lock);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_pending">assert_pending</a>(details);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_correct_hash_lock">assert_correct_hash_lock</a>(details, hash_lock);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_within_timelock">assert_within_timelock</a>(details);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete">complete</a>(details);

    (details.addresses.recipient, details.amount)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_complete_transfer"></a>

## Function `complete_transfer`

Completes a bridge transfer by validating the hash lock and updating the transfer state.

@param bridge_transfer_id The ID of the bridge transfer to complete.
@param hash_lock The hash lock used to validate the transfer.
@return A tuple containing the recipient of the transfer and the amount transferred.
@abort If the bridge transfer details are not found or if the completion checks in <code>complete_details</code> fail.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_transfer">complete_transfer</a>&lt;Initiator: store, Recipient: <b>copy</b>, store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (Recipient, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_transfer">complete_transfer</a>&lt;Initiator: store, Recipient: <b>copy</b> + store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) : (Recipient, u64) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_atomic_bridge_enabled">features::abort_atomic_bridge_enabled</a>(), <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>);

    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework);

    <b>let</b> details = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(
        &<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner,
        bridge_transfer_id);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_details">complete_details</a>&lt;Initiator, Recipient&gt;(hash_lock, details)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_cancel_details"></a>

## Function `cancel_details`

Cancels a pending bridge transfer if the time lock has expired.

@param details A mutable reference to the bridge transfer details to be canceled.
@return A tuple containing the initiator of the transfer and the amount to be refunded.
@abort If the transfer is not in a pending state or the time lock has not expired.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_details">cancel_details</a>&lt;Initiator: <b>copy</b>, store, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;): (Initiator, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_details">cancel_details</a>&lt;Initiator: store + <b>copy</b>, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) : (Initiator, u64) {
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_pending">assert_pending</a>(details);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_timed_out_lock">assert_timed_out_lock</a>(details);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel">cancel</a>(details);

    (details.addresses.initiator, details.amount)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_cancel_transfer"></a>

## Function `cancel_transfer`

Cancels a bridge transfer if it is pending and the time lock has expired.

@param bridge_transfer_id The ID of the bridge transfer to cancel.
@return A tuple containing the initiator of the transfer and the amount to be refunded.
@abort If the bridge transfer details are not found or if the cancellation conditions in <code>cancel_details</code> fail.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_transfer">cancel_transfer</a>&lt;Initiator: <b>copy</b>, store, Recipient: store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (Initiator, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_transfer">cancel_transfer</a>&lt;Initiator: store + <b>copy</b>, Recipient: store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) : (Initiator, u64) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_atomic_bridge_enabled">features::abort_atomic_bridge_enabled</a>(), <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>);

    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework);

    <b>let</b> details = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(
        &<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner,
        bridge_transfer_id);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_details">cancel_details</a>&lt;Initiator, Recipient&gt;(details)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_bridge_transfer_id"></a>

## Function `bridge_transfer_id`

Generates a unique bridge transfer ID based on transfer details and nonce.

@param details The bridge transfer details.
@return The generated bridge transfer ID.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_bridge_transfer_id">bridge_transfer_id</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_bridge_transfer_id">bridge_transfer_id</a>&lt;Initiator: store, Recipient: store&gt;(details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_Nonce">Nonce</a> {
    <b>let</b> nonce = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_Nonce">Nonce</a>&gt;(@aptos_framework);
    <b>let</b> combined_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&details.addresses.initiator));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&details.addresses.recipient));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, details.hash_lock);
    <b>if</b> (nonce.inner == <a href="atomic_bridge.md#0x1_atomic_bridge_store_MAX_U64">MAX_U64</a>) {
        nonce.inner = 0;  // Wrap around <b>to</b> 0 <b>if</b> at maximum value
    } <b>else</b> {
        nonce.inner = nonce.inner + 1;  // Safe <b>to</b> increment without overflow
    };
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&nonce.inner));

    keccak256(combined_bytes)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_get_bridge_transfer_details_initiator"></a>

## Function `get_bridge_transfer_details_initiator`

Gets initiator bridge transfer details given a bridge transfer ID

@param bridge_transfer_id A 32-byte vector of unsigned 8-bit integers.
@return A <code><a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a></code> struct.
@abort If there is no transfer in the atomic bridge store.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details_initiator">get_bridge_transfer_details_initiator</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;<b>address</b>, <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details_initiator">get_bridge_transfer_details_initiator</a>(
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;<b>address</b>, EthereumAddress&gt; <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details">get_bridge_transfer_details</a>(bridge_transfer_id)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_get_bridge_transfer_details_counterparty"></a>

## Function `get_bridge_transfer_details_counterparty`

Gets counterparty bridge transfer details given a bridge transfer ID

@param bridge_transfer_id A 32-byte vector of unsigned 8-bit integers.
@return A <code><a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a></code> struct.
@abort If there is no transfer in the atomic bridge store.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details_counterparty">get_bridge_transfer_details_counterparty</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;<a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, <b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details_counterparty">get_bridge_transfer_details_counterparty</a>(
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;EthereumAddress, <b>address</b>&gt; <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details">get_bridge_transfer_details</a>(bridge_transfer_id)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_get_bridge_transfer_details"></a>

## Function `get_bridge_transfer_details`



<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details">get_bridge_transfer_details</a>&lt;Initiator: <b>copy</b>, store, Recipient: <b>copy</b>, store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details">get_bridge_transfer_details</a>&lt;Initiator: store + <b>copy</b>, Recipient: store + <b>copy</b>&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt; <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a> {
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework);

    <b>let</b> details_ref = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(
        &<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner,
        bridge_transfer_id
    );

    *details_ref
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_Nonce">Nonce</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;<b>address</b>, EthereumAddress&gt;&gt;&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;EthereumAddress, <b>address</b>&gt;&gt;&gt;(addr);
</code></pre>




<a id="0x1_atomic_bridge_store_TimeLockAbortsIf"></a>


<pre><code><b>schema</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_TimeLockAbortsIf">TimeLockAbortsIf</a> {
    time_lock: u64;
    <b>aborts_if</b> time_lock &lt; <a href="atomic_bridge.md#0x1_atomic_bridge_store_MIN_TIME_LOCK">MIN_TIME_LOCK</a>;
    <b>aborts_if</b> !<b>exists</b>&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);
    <b>aborts_if</b> time_lock &gt; <a href="atomic_bridge.md#0x1_atomic_bridge_store_MAX_U64">MAX_U64</a> - <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();
}
</code></pre>



<a id="@Specification_1_create_time_lock"></a>

### Function `create_time_lock`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_time_lock">create_time_lock</a>(time_lock: u64): u64
</code></pre>




<pre><code><b>include</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_TimeLockAbortsIf">TimeLockAbortsIf</a>;
<b>ensures</b> result == <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() + time_lock;
</code></pre>


If the sum of <code><a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>()</code> and <code>lock</code> does not overflow, the result is the sum of <code><a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>()</code> and <code>lock</code>.


<pre><code><b>ensures</b> (<a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() + time_lock &lt;= 0xFFFFFFFFFFFFFFFF) ==&gt; result == <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() + time_lock;
</code></pre>



<a id="@Specification_1_create_details"></a>

### Function `create_details`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_details">create_details</a>&lt;Initiator: store, Recipient: store&gt;(initiator: Initiator, recipient: Recipient, amount: u64, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, time_lock: u64): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;
</code></pre>




<pre><code><b>include</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_TimeLockAbortsIf">TimeLockAbortsIf</a>;
<b>aborts_if</b> amount == 0;
<b>aborts_if</b> len(hash_lock) != 32;
<b>ensures</b> result == <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt; {
        addresses: <a href="atomic_bridge.md#0x1_atomic_bridge_store_AddressPair">AddressPair</a>&lt;Initiator, Recipient&gt; {
        initiator,
        recipient
    },
    amount,
    hash_lock,
    time_lock: <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() + time_lock,
    state: <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>,
};
</code></pre>




<a id="0x1_atomic_bridge_store_AddAbortsIf"></a>


<pre><code><b>schema</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_AddAbortsIf">AddAbortsIf</a>&lt;T&gt; {
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: SmartTable&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, T&gt;;
    <b>aborts_if</b> len(bridge_transfer_id) != 32;
    <b>aborts_if</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, bridge_transfer_id);
    <b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_ATOMIC_BRIDGE">features::ATOMIC_BRIDGE</a>);
}
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_add">add</a>&lt;Initiator: store, Recipient: store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, details: <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>




<pre><code><b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework).inner;
<b>include</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_AddAbortsIf">AddAbortsIf</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework);
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, bridge_transfer_id);
<b>ensures</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(<b>global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework).inner, bridge_transfer_id);
<b>ensures</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_len">smart_table::spec_len</a>(<b>global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework).inner) ==
    <b>old</b>(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_len">smart_table::spec_len</a>(<b>global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework).inner)) + 1;
</code></pre>




<a id="0x1_atomic_bridge_store_HashLockAbortsIf"></a>


<pre><code><b>schema</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_HashLockAbortsIf">HashLockAbortsIf</a> {
    hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    <b>aborts_if</b> len(hash_lock) != 32;
}
</code></pre>




<a id="0x1_atomic_bridge_store_BridgetTransferDetailsAbortsIf"></a>


<pre><code><b>schema</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgetTransferDetailsAbortsIf">BridgetTransferDetailsAbortsIf</a>&lt;Initiator, Recipient&gt; {
    hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    details: <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;;
    <b>include</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_HashLockAbortsIf">HashLockAbortsIf</a>;
    <b>aborts_if</b> <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt; details.time_lock;
    <b>aborts_if</b> !<b>exists</b>&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);
    <b>aborts_if</b> details.state != <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>;
    <b>aborts_if</b> details.hash_lock != hash_lock;
}
</code></pre>



<a id="@Specification_1_create_hashlock"></a>

### Function `create_hashlock`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_hashlock">create_hashlock</a>(pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>aborts_if</b> len(pre_image) == 0;
</code></pre>



<a id="@Specification_1_complete"></a>

### Function `complete`


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete">complete</a>&lt;Initiator: store, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>




<pre><code><b>requires</b> details.state == <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>;
<b>ensures</b> details.state == <a href="atomic_bridge.md#0x1_atomic_bridge_store_COMPLETED_TRANSACTION">COMPLETED_TRANSACTION</a>;
</code></pre>



<a id="@Specification_1_cancel"></a>

### Function `cancel`


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel">cancel</a>&lt;Initiator: store, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>




<pre><code><b>requires</b> details.state == <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>;
<b>ensures</b> details.state == <a href="atomic_bridge.md#0x1_atomic_bridge_store_CANCELLED_TRANSACTION">CANCELLED_TRANSACTION</a>;
</code></pre>



<a id="@Specification_1_complete_details"></a>

### Function `complete_details`


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_details">complete_details</a>&lt;Initiator: store, Recipient: <b>copy</b>, store&gt;(hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;): (Recipient, u64)
</code></pre>




<pre><code><b>include</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgetTransferDetailsAbortsIf">BridgetTransferDetailsAbortsIf</a>&lt;Initiator, Recipient&gt;;
</code></pre>



<a id="@Specification_1_complete_transfer"></a>

### Function `complete_transfer`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_transfer">complete_transfer</a>&lt;Initiator: store, Recipient: <b>copy</b>, store&gt;(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (Recipient, u64)
</code></pre>




<pre><code><b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework).inner;
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_ATOMIC_BRIDGE">features::ATOMIC_BRIDGE</a>);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;&gt;&gt;(@aptos_framework);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, bridge_transfer_id);
<b>let</b> details = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_get">smart_table::spec_get</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, bridge_transfer_id);
<b>include</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgetTransferDetailsAbortsIf">BridgetTransferDetailsAbortsIf</a>&lt;Initiator, Recipient&gt;;
</code></pre>




<a id="0x1_atomic_bridge_store_AbortBridgetTransferDetailsAbortsIf"></a>


<pre><code><b>schema</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_AbortBridgetTransferDetailsAbortsIf">AbortBridgetTransferDetailsAbortsIf</a>&lt;Initiator, Recipient&gt; {
    details: <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;;
    <b>aborts_if</b> details.state != <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>;
    <b>aborts_if</b> !(<a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt; details.time_lock);
    <b>aborts_if</b> !<b>exists</b>&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);
    <b>ensures</b> details.state == <a href="atomic_bridge.md#0x1_atomic_bridge_store_CANCELLED_TRANSACTION">CANCELLED_TRANSACTION</a>;
}
</code></pre>



<a id="@Specification_1_cancel_details"></a>

### Function `cancel_details`


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_details">cancel_details</a>&lt;Initiator: <b>copy</b>, store, Recipient: store&gt;(details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;): (Initiator, u64)
</code></pre>




<pre><code><b>include</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_AbortBridgetTransferDetailsAbortsIf">AbortBridgetTransferDetailsAbortsIf</a>&lt;Initiator, Recipient&gt;;
</code></pre>



<a id="0x1_atomic_bridge_configuration"></a>

# Module `0x1::atomic_bridge_configuration`



-  [Resource `BridgeConfig`](#0x1_atomic_bridge_configuration_BridgeConfig)
-  [Struct `BridgeConfigOperatorUpdated`](#0x1_atomic_bridge_configuration_BridgeConfigOperatorUpdated)
-  [Struct `InitiatorTimeLockUpdated`](#0x1_atomic_bridge_configuration_InitiatorTimeLockUpdated)
-  [Struct `CounterpartyTimeLockUpdated`](#0x1_atomic_bridge_configuration_CounterpartyTimeLockUpdated)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_atomic_bridge_configuration_initialize)
-  [Function `update_bridge_operator`](#0x1_atomic_bridge_configuration_update_bridge_operator)
-  [Function `set_initiator_time_lock_duration`](#0x1_atomic_bridge_configuration_set_initiator_time_lock_duration)
-  [Function `set_counterparty_time_lock_duration`](#0x1_atomic_bridge_configuration_set_counterparty_time_lock_duration)
-  [Function `initiator_timelock_duration`](#0x1_atomic_bridge_configuration_initiator_timelock_duration)
-  [Function `counterparty_timelock_duration`](#0x1_atomic_bridge_configuration_counterparty_timelock_duration)
-  [Function `bridge_operator`](#0x1_atomic_bridge_configuration_bridge_operator)
-  [Function `assert_is_caller_operator`](#0x1_atomic_bridge_configuration_assert_is_caller_operator)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `update_bridge_operator`](#@Specification_1_update_bridge_operator)


<pre><code><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_atomic_bridge_configuration_BridgeConfig"></a>

## Resource `BridgeConfig`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>initiator_time_lock: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>counterparty_time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_configuration_BridgeConfigOperatorUpdated"></a>

## Struct `BridgeConfigOperatorUpdated`

Event emitted when the bridge operator is updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfigOperatorUpdated">BridgeConfigOperatorUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
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
</dl>


</details>

<a id="0x1_atomic_bridge_configuration_InitiatorTimeLockUpdated"></a>

## Struct `InitiatorTimeLockUpdated`

Event emitted when the initiator time lock has been updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_InitiatorTimeLockUpdated">InitiatorTimeLockUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_configuration_CounterpartyTimeLockUpdated"></a>

## Struct `CounterpartyTimeLockUpdated`

Event emitted when the initiator time lock has been updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_CounterpartyTimeLockUpdated">CounterpartyTimeLockUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_atomic_bridge_configuration_COUNTERPARTY_TIME_LOCK_DUARTION"></a>

Counterparty time lock duration is 24 hours in seconds


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_COUNTERPARTY_TIME_LOCK_DUARTION">COUNTERPARTY_TIME_LOCK_DUARTION</a>: u64 = 86400;
</code></pre>



<a id="0x1_atomic_bridge_configuration_EINVALID_BRIDGE_OPERATOR"></a>

Error code for invalid bridge operator


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EINVALID_BRIDGE_OPERATOR">EINVALID_BRIDGE_OPERATOR</a>: u64 = 1;
</code></pre>



<a id="0x1_atomic_bridge_configuration_INITIATOR_TIME_LOCK_DUARTION"></a>

Initiator time lock duration is 48 hours in seconds


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_INITIATOR_TIME_LOCK_DUARTION">INITIATOR_TIME_LOCK_DUARTION</a>: u64 = 172800;
</code></pre>



<a id="0x1_atomic_bridge_configuration_initialize"></a>

## Function `initialize`

Initializes the bridge configuration with Aptos framework as the bridge operator.

@param aptos_framework The signer representing the Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> bridge_config = <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> {
        bridge_operator: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework),
        initiator_time_lock: <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_INITIATOR_TIME_LOCK_DUARTION">INITIATOR_TIME_LOCK_DUARTION</a>,
        counterparty_time_lock: <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_COUNTERPARTY_TIME_LOCK_DUARTION">COUNTERPARTY_TIME_LOCK_DUARTION</a>,
    };
    <b>move_to</b>(aptos_framework, bridge_config);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_update_bridge_operator"></a>

## Function `update_bridge_operator`

Updates the bridge operator, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_operator The new address to be set as the bridge operator.
@abort If the current operator is the same as the new operator.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_update_bridge_operator">update_bridge_operator</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_update_bridge_operator">update_bridge_operator</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>
)   <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> bridge_config = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework);
    <b>let</b> old_operator = bridge_config.bridge_operator;
    <b>assert</b>!(old_operator != new_operator, <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EINVALID_BRIDGE_OPERATOR">EINVALID_BRIDGE_OPERATOR</a>);

    bridge_config.bridge_operator = new_operator;

    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfigOperatorUpdated">BridgeConfigOperatorUpdated</a> {
            old_operator,
            new_operator,
        },
    );
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_set_initiator_time_lock_duration"></a>

## Function `set_initiator_time_lock_duration`



<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_set_initiator_time_lock_duration">set_initiator_time_lock_duration</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, time_lock: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_set_initiator_time_lock_duration">set_initiator_time_lock_duration</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, time_lock: u64
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).initiator_time_lock = time_lock;

    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_InitiatorTimeLockUpdated">InitiatorTimeLockUpdated</a> {
            time_lock
        },
    );
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_set_counterparty_time_lock_duration"></a>

## Function `set_counterparty_time_lock_duration`



<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_set_counterparty_time_lock_duration">set_counterparty_time_lock_duration</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, time_lock: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_set_counterparty_time_lock_duration">set_counterparty_time_lock_duration</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, time_lock: u64
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).counterparty_time_lock = time_lock;

    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_CounterpartyTimeLockUpdated">CounterpartyTimeLockUpdated</a> {
            time_lock
        },
    );
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_initiator_timelock_duration"></a>

## Function `initiator_timelock_duration`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initiator_timelock_duration">initiator_timelock_duration</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initiator_timelock_duration">initiator_timelock_duration</a>() : u64 <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <b>borrow_global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).initiator_time_lock
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_counterparty_timelock_duration"></a>

## Function `counterparty_timelock_duration`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_counterparty_timelock_duration">counterparty_timelock_duration</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_counterparty_timelock_duration">counterparty_timelock_duration</a>() : u64 <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <b>borrow_global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).counterparty_time_lock
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_bridge_operator"></a>

## Function `bridge_operator`

Retrieves the address of the current bridge operator.

@return The address of the current bridge operator.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_bridge_operator">bridge_operator</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_bridge_operator">bridge_operator</a>(): <b>address</b> <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_operator
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_assert_is_caller_operator"></a>

## Function `assert_is_caller_operator`

Asserts that the caller is the current bridge operator.

@param caller The signer whose authority is being checked.
@abort If the caller is not the current bridge operator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_assert_is_caller_operator">assert_is_caller_operator</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_assert_is_caller_operator">assert_is_caller_operator</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> {
    <b>assert</b>!(<b>borrow_global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_operator == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(caller), <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EINVALID_BRIDGE_OPERATOR">EINVALID_BRIDGE_OPERATOR</a>);
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>exists</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>ensures</b> <b>global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)).bridge_operator == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
</code></pre>



<a id="@Specification_1_update_bridge_operator"></a>

### Function `update_bridge_operator`


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_update_bridge_operator">update_bridge_operator</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> !<b>exists</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>aborts_if</b> <b>global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)).bridge_operator == new_operator;
<b>ensures</b> <b>global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)).bridge_operator == new_operator;
</code></pre>



<a id="0x1_atomic_bridge"></a>

# Module `0x1::atomic_bridge`



-  [Resource `AptosCoinBurnCapability`](#0x1_atomic_bridge_AptosCoinBurnCapability)
-  [Resource `AptosCoinMintCapability`](#0x1_atomic_bridge_AptosCoinMintCapability)
-  [Resource `AptosFABurnCapabilities`](#0x1_atomic_bridge_AptosFABurnCapabilities)
-  [Resource `AptosFAMintCapabilities`](#0x1_atomic_bridge_AptosFAMintCapabilities)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_atomic_bridge_initialize)
-  [Function `store_aptos_coin_burn_cap`](#0x1_atomic_bridge_store_aptos_coin_burn_cap)
-  [Function `store_aptos_coin_mint_cap`](#0x1_atomic_bridge_store_aptos_coin_mint_cap)
-  [Function `mint`](#0x1_atomic_bridge_mint)
-  [Function `burn`](#0x1_atomic_bridge_burn)


<pre><code><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration">0x1::atomic_bridge_configuration</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store">0x1::atomic_bridge_store</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_atomic_bridge_AptosCoinBurnCapability"></a>

## Resource `AptosCoinBurnCapability`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a> <b>has</b> key
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

<a id="0x1_atomic_bridge_AptosCoinMintCapability"></a>

## Resource `AptosCoinMintCapability`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a> <b>has</b> key
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

<a id="0x1_atomic_bridge_AptosFABurnCapabilities"></a>

## Resource `AptosFABurnCapabilities`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosFABurnCapabilities">AptosFABurnCapabilities</a> <b>has</b> key
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

<a id="0x1_atomic_bridge_AptosFAMintCapabilities"></a>

## Resource `AptosFAMintCapabilities`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosFAMintCapabilities">AptosFAMintCapabilities</a> <b>has</b> key
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


<a id="0x1_atomic_bridge_EATOMIC_BRIDGE_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>: u64 = 1;
</code></pre>



<a id="0x1_atomic_bridge_initialize"></a>

## Function `initialize`

Initializes the atomic bridge by setting up necessary configurations.

@param aptos_framework The signer representing the Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initialize">atomic_bridge_configuration::initialize</a>(aptos_framework);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_initialize">atomic_bridge_store::initialize</a>(aptos_framework);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_aptos_coin_burn_cap"></a>

## Function `store_aptos_coin_burn_cap`

Stores the burn capability for AptosCoin, converting to a fungible asset reference if the feature is enabled.

@param aptos_framework The signer representing the Aptos framework.
@param burn_cap The burn capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: BurnCapability&lt;AptosCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
        <b>let</b> burn_ref = <a href="coin.md#0x1_coin_convert_and_take_paired_burn_ref">coin::convert_and_take_paired_burn_ref</a>(burn_cap);
        <b>move_to</b>(aptos_framework, <a href="atomic_bridge.md#0x1_atomic_bridge_AptosFABurnCapabilities">AptosFABurnCapabilities</a> { burn_ref });
    } <b>else</b> {
        <b>move_to</b>(aptos_framework, <a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a> { burn_cap })
    }
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

Stores the mint capability for AptosCoin.

@param aptos_framework The signer representing the Aptos framework.
@param mint_cap The mint capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: MintCapability&lt;AptosCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>move_to</b>(aptos_framework, <a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a> { mint_cap })
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_mint"></a>

## Function `mint`

Mints a specified amount of AptosCoin to a recipient's address.

@param recipient The address of the recipient to mint coins to.
@param amount The amount of AptosCoin to mint.
@abort If the mint capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_mint">mint</a>(recipient: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_mint">mint</a>(recipient: <b>address</b>, amount: u64) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_atomic_bridge_enabled">features::abort_atomic_bridge_enabled</a>(), <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>);

    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(recipient, <a href="coin.md#0x1_coin_mint">coin::mint</a>(
        amount,
        &<b>borrow_global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a>&gt;(@aptos_framework).mint_cap
    ));
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_burn"></a>

## Function `burn`

Burns a specified amount of AptosCoin from an address.

@param from The address from which to burn AptosCoin.
@param amount The amount of AptosCoin to burn.
@abort If the burn capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_burn">burn</a>(from: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_burn">burn</a>(from: <b>address</b>, amount: u64) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_atomic_bridge_enabled">features::abort_atomic_bridge_enabled</a>(), <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>);

    <a href="coin.md#0x1_coin_burn_from">coin::burn_from</a>(
        from,
        amount,
        &<b>borrow_global</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a>&gt;(@aptos_framework).burn_cap,
    );
}
</code></pre>



</details>



<a id="0x1_atomic_bridge_counterparty"></a>

# Module `0x1::atomic_bridge_counterparty`



-  [Struct `BridgeTransferLockedEvent`](#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent)
-  [Struct `BridgeTransferCompletedEvent`](#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent)
-  [Struct `BridgeTransferCancelledEvent`](#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent)
-  [Resource `BridgeCounterpartyEvents`](#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents)
-  [Function `initialize`](#0x1_atomic_bridge_counterparty_initialize)
-  [Function `lock_bridge_transfer_assets`](#0x1_atomic_bridge_counterparty_lock_bridge_transfer_assets)
-  [Function `complete_bridge_transfer`](#0x1_atomic_bridge_counterparty_complete_bridge_transfer)
-  [Function `abort_bridge_transfer`](#0x1_atomic_bridge_counterparty_abort_bridge_transfer)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_atomic_bridge">0x1::atomic_bridge</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration">0x1::atomic_bridge_configuration</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store">0x1::atomic_bridge_store</a>;
<b>use</b> <a href="ethereum.md#0x1_ethereum">0x1::ethereum</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
</code></pre>



<a id="0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent"></a>

## Struct `BridgeTransferLockedEvent`

An event triggered upon locking assets for a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent">BridgeTransferLockedEvent</a> <b>has</b> drop, store
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
<code>hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent"></a>

## Struct `BridgeTransferCompletedEvent`

An event triggered upon completing a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> <b>has</b> drop, store
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
<code>pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent"></a>

## Struct `BridgeTransferCancelledEvent`

An event triggered upon cancelling a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent">BridgeTransferCancelledEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents"></a>

## Resource `BridgeCounterpartyEvents`

This struct will store the event handles for bridge events.


<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_locked_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent">atomic_bridge_counterparty::BridgeTransferLockedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_completed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent">atomic_bridge_counterparty::BridgeTransferCompletedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_cancelled_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent">atomic_bridge_counterparty::BridgeTransferCancelledEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_counterparty_initialize"></a>

## Function `initialize`

Initializes the module and stores the <code>EventHandle</code>s in the resource.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(aptos_framework, <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a> {
        bridge_transfer_locked_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent">BridgeTransferLockedEvent</a>&gt;(aptos_framework),
        bridge_transfer_completed_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a>&gt;(aptos_framework),
        bridge_transfer_cancelled_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent">BridgeTransferCancelledEvent</a>&gt;(aptos_framework),
    });
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_counterparty_lock_bridge_transfer_assets"></a>

## Function `lock_bridge_transfer_assets`

Locks assets for a bridge transfer by the initiator.

@param caller The signer representing the bridge operator.
@param initiator The initiator's Ethereum address as a vector of bytes.
@param bridge_transfer_id The unique identifier for the bridge transfer.
@param hash_lock The hash lock for securing the transfer.
@param time_lock The time lock duration for the transfer.
@param recipient The address of the recipient on the Aptos blockchain.
@param amount The amount of assets to be locked.
@abort If the caller is not the bridge operator.


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_lock_bridge_transfer_assets">lock_bridge_transfer_assets</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_lock_bridge_transfer_assets">lock_bridge_transfer_assets</a> (
    caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    recipient: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a> {
    <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_assert_is_caller_operator">atomic_bridge_configuration::assert_is_caller_operator</a>(caller);
    <b>let</b> ethereum_address = <a href="ethereum.md#0x1_ethereum_ethereum_address_no_eip55">ethereum::ethereum_address_no_eip55</a>(initiator);
    <b>let</b> time_lock = <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_counterparty_timelock_duration">atomic_bridge_configuration::counterparty_timelock_duration</a>();
    <b>let</b> details = <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_details">atomic_bridge_store::create_details</a>(
        ethereum_address,
        recipient,
        amount,
        hash_lock,
        time_lock
    );

    // bridge_store::add_counterparty(bridge_transfer_id, details);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_add">atomic_bridge_store::add</a>(bridge_transfer_id, details);

    <b>let</b> bridge_events = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a>&gt;(@aptos_framework);

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> bridge_events.bridge_transfer_locked_events,
        <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent">BridgeTransferLockedEvent</a> {
            bridge_transfer_id,
            initiator,
            recipient,
            amount,
            hash_lock,
            time_lock,
        },
    );
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_counterparty_complete_bridge_transfer"></a>

## Function `complete_bridge_transfer`

Completes a bridge transfer by revealing the pre-image.

@param bridge_transfer_id The unique identifier for the bridge transfer.
@param pre_image The pre-image that matches the hash lock to complete the transfer.
@abort If the caller is not the bridge operator or the hash lock validation fails.


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_complete_bridge_transfer">complete_bridge_transfer</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_complete_bridge_transfer">complete_bridge_transfer</a> (
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a> {
    <b>let</b> (recipient, amount) = <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_transfer">atomic_bridge_store::complete_transfer</a>&lt;EthereumAddress, <b>address</b>&gt;(
        bridge_transfer_id,
        create_hashlock(pre_image)
    );

    // Mint, fails silently
    <a href="atomic_bridge.md#0x1_atomic_bridge_mint">atomic_bridge::mint</a>(recipient, amount);

    <b>let</b> bridge_counterparty_events = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a>&gt;(@aptos_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> bridge_counterparty_events.bridge_transfer_completed_events,
        <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> {
            bridge_transfer_id,
            pre_image,
        },
    );
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_counterparty_abort_bridge_transfer"></a>

## Function `abort_bridge_transfer`

Aborts a bridge transfer if the time lock has expired.

@param caller The signer representing the bridge operator.
@param bridge_transfer_id The unique identifier for the bridge transfer.
@abort If the caller is not the bridge operator or if the time lock has not expired.


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_abort_bridge_transfer">abort_bridge_transfer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_abort_bridge_transfer">abort_bridge_transfer</a> (
    caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a> {
    <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_assert_is_caller_operator">atomic_bridge_configuration::assert_is_caller_operator</a>(caller);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_transfer">atomic_bridge_store::cancel_transfer</a>&lt;EthereumAddress, <b>address</b>&gt;(bridge_transfer_id);

    <b>let</b> bridge_counterparty_events = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a>&gt;(@aptos_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> bridge_counterparty_events.bridge_transfer_cancelled_events,
        <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent">BridgeTransferCancelledEvent</a> {
            bridge_transfer_id,
        },
    );
}
</code></pre>



</details>



<a id="0x1_atomic_bridge_initiator"></a>

# Module `0x1::atomic_bridge_initiator`



-  [Struct `BridgeTransferInitiatedEvent`](#0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent)
-  [Struct `BridgeTransferCompletedEvent`](#0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent)
-  [Struct `BridgeTransferRefundedEvent`](#0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent)
-  [Resource `BridgeInitiatorEvents`](#0x1_atomic_bridge_initiator_BridgeInitiatorEvents)
-  [Function `initialize`](#0x1_atomic_bridge_initiator_initialize)
-  [Function `initiate_bridge_transfer`](#0x1_atomic_bridge_initiator_initiate_bridge_transfer)
-  [Function `complete_bridge_transfer`](#0x1_atomic_bridge_initiator_complete_bridge_transfer)
-  [Function `refund_bridge_transfer`](#0x1_atomic_bridge_initiator_refund_bridge_transfer)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_atomic_bridge">0x1::atomic_bridge</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration">0x1::atomic_bridge_configuration</a>;
<b>use</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store">0x1::atomic_bridge_store</a>;
<b>use</b> <a href="ethereum.md#0x1_ethereum">0x1::ethereum</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent"></a>

## Struct `BridgeTransferInitiatedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a> <b>has</b> drop, store
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
<code>hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent"></a>

## Struct `BridgeTransferCompletedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> <b>has</b> drop, store
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
<code>pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent"></a>

## Struct `BridgeTransferRefundedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent">BridgeTransferRefundedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_initiator_BridgeInitiatorEvents"></a>

## Resource `BridgeInitiatorEvents`

This struct will store the event handles for bridge events.


<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_initiated_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent">atomic_bridge_initiator::BridgeTransferInitiatedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_completed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent">atomic_bridge_initiator::BridgeTransferCompletedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_refunded_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent">atomic_bridge_initiator::BridgeTransferRefundedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_initiator_initialize"></a>

## Function `initialize`

Initializes the module and stores the <code>EventHandle</code>s in the resource.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(aptos_framework, <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a> {
        bridge_transfer_initiated_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a>&gt;(aptos_framework),
        bridge_transfer_completed_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a>&gt;(aptos_framework),
        bridge_transfer_refunded_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent">BridgeTransferRefundedEvent</a>&gt;(aptos_framework),
    });
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_initiator_initiate_bridge_transfer"></a>

## Function `initiate_bridge_transfer`

Initiate a bridge transfer of ETH from Movement to the base layer
Anyone can initiate a bridge transfer from the source chain
The amount is burnt from the initiator


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_initiate_bridge_transfer">initiate_bridge_transfer</a>(initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_initiate_bridge_transfer">initiate_bridge_transfer</a>(
    initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    amount: u64
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a> {
    <b>let</b> ethereum_address = <a href="ethereum.md#0x1_ethereum_ethereum_address_no_eip55">ethereum::ethereum_address_no_eip55</a>(recipient);
    <b>let</b> initiator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(initiator);
    <b>let</b> time_lock = <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initiator_timelock_duration">atomic_bridge_configuration::initiator_timelock_duration</a>();

    <b>let</b> details =
        <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_details">atomic_bridge_store::create_details</a>(
            initiator_address,
            ethereum_address, amount,
            hash_lock,
            time_lock
        );

    <b>let</b> bridge_transfer_id = bridge_transfer_id(&details);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_add">atomic_bridge_store::add</a>(bridge_transfer_id, details);
    <a href="atomic_bridge.md#0x1_atomic_bridge_burn">atomic_bridge::burn</a>(initiator_address, amount);

    <b>let</b> bridge_initiator_events = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a>&gt;(@aptos_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> bridge_initiator_events.bridge_transfer_initiated_events,
        <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a> {
            bridge_transfer_id,
            initiator: initiator_address,
            recipient,
            amount,
            hash_lock,
            time_lock
        },
    );
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_initiator_complete_bridge_transfer"></a>

## Function `complete_bridge_transfer`

Bridge operator can complete the transfer


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_complete_bridge_transfer">complete_bridge_transfer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_complete_bridge_transfer">complete_bridge_transfer</a> (
    caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a> {
    assert_is_caller_operator(caller);
    <b>let</b> (_, _) = <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_transfer">atomic_bridge_store::complete_transfer</a>&lt;<b>address</b>, EthereumAddress&gt;(bridge_transfer_id, create_hashlock(pre_image));

    <b>let</b> bridge_initiator_events = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a>&gt;(@aptos_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> bridge_initiator_events.bridge_transfer_completed_events,
        <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> {
            bridge_transfer_id,
            pre_image,
        },
    );
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_initiator_refund_bridge_transfer"></a>

## Function `refund_bridge_transfer`

Anyone can refund the transfer on the source chain once time lock has passed


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_refund_bridge_transfer">refund_bridge_transfer</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_refund_bridge_transfer">refund_bridge_transfer</a> (
    _caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a> {
    <b>let</b> (receiver, amount) = <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_transfer">atomic_bridge_store::cancel_transfer</a>&lt;<b>address</b>, EthereumAddress&gt;(bridge_transfer_id);
    <a href="atomic_bridge.md#0x1_atomic_bridge_mint">atomic_bridge::mint</a>(receiver, amount);

    <b>let</b> bridge_initiator_events = <b>borrow_global_mut</b>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a>&gt;(@aptos_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> bridge_initiator_events.bridge_transfer_refunded_events,
        <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent">BridgeTransferRefundedEvent</a> {
            bridge_transfer_id,
        },
    );
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
