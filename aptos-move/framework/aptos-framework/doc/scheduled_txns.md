
<a id="0x1_scheduled_txns"></a>

# Module `0x1::scheduled_txns`



-  [Struct `ScheduledTransaction`](#0x1_scheduled_txns_ScheduledTransaction)
<<<<<<< HEAD
-  [Struct `ScheduledTransactionWithKey`](#0x1_scheduled_txns_ScheduledTransactionWithKey)
-  [Struct `ScheduleMapKey`](#0x1_scheduled_txns_ScheduleMapKey)
-  [Resource `ScheduleQueue`](#0x1_scheduled_txns_ScheduleQueue)
-  [Resource `AuxiliaryData`](#0x1_scheduled_txns_AuxiliaryData)
-  [Resource `ToRemoveTbl`](#0x1_scheduled_txns_ToRemoveTbl)
-  [Enum `CancelledTxnCode`](#0x1_scheduled_txns_CancelledTxnCode)
-  [Struct `TransactionExpiredEvent`](#0x1_scheduled_txns_TransactionExpiredEvent)
-  [Struct `ShutdownEvent`](#0x1_scheduled_txns_ShutdownEvent)
-  [Struct `KeyAndTxnInfo`](#0x1_scheduled_txns_KeyAndTxnInfo)
=======
-  [Struct `ScheduledTransactionWithId`](#0x1_scheduled_txns_ScheduledTransactionWithId)
-  [Struct `TransactionId`](#0x1_scheduled_txns_TransactionId)
-  [Struct `ScheduleMapKey`](#0x1_scheduled_txns_ScheduleMapKey)
-  [Struct `Empty`](#0x1_scheduled_txns_Empty)
-  [Resource `ScheduleQueue`](#0x1_scheduled_txns_ScheduleQueue)
-  [Resource `GasFeeDepositStoreSignerCap`](#0x1_scheduled_txns_GasFeeDepositStoreSignerCap)
-  [Resource `ToRemoveTbl`](#0x1_scheduled_txns_ToRemoveTbl)
-  [Enum `Result`](#0x1_scheduled_txns_Result)
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
-  [Struct `State`](#0x1_scheduled_txns_State)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_scheduled_txns_initialize)
-  [Function `shutdown`](#0x1_scheduled_txns_shutdown)
<<<<<<< HEAD
-  [Function `new_scheduled_transaction`](#0x1_scheduled_txns_new_scheduled_transaction)
-  [Function `insert`](#0x1_scheduled_txns_insert)
-  [Function `cancel`](#0x1_scheduled_txns_cancel)
-  [Function `cancel_internal`](#0x1_scheduled_txns_cancel_internal)
=======
-  [Function `insert`](#0x1_scheduled_txns_insert)
-  [Function `cancel`](#0x1_scheduled_txns_cancel)
-  [Function `insert_txn_with_id`](#0x1_scheduled_txns_insert_txn_with_id)
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
-  [Function `get_ready_transactions`](#0x1_scheduled_txns_get_ready_transactions)
-  [Function `finish_execution`](#0x1_scheduled_txns_finish_execution)
-  [Function `remove_txns`](#0x1_scheduled_txns_remove_txns)
-  [Function `step`](#0x1_scheduled_txns_step)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<<<<<<< HEAD
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
=======
<b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_scheduled_txns_ScheduledTransaction"></a>

## Struct `ScheduledTransaction`

ScheduledTransaction with permission signer handle, scheduled_time, gas params, and function


<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<<<<<<< HEAD
<code>sender_addr: <b>address</b></code>
</dt>
<dd>
 32 bytes
=======
<code>sender_handle: <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a></code>
</dt>
<dd>
 72 bytes (32 + 32 + 8)
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dd>
<dt>
<code>scheduled_time: u64</code>
</dt>
<dd>
 100ms granularity
</dd>
<dt>
<code>max_gas_amount: u64</code>
</dt>
<dd>
 Maximum gas to spend for this transaction
</dd>
<dt>
<code>max_gas_unit_price: u64</code>
</dt>
<dd>
 Charged @ lesser of {max_gas_unit_price, max_gas_unit_price other than this in the block executed}
</dd>
<dt>
<<<<<<< HEAD
<code>pass_signer: bool</code>
</dt>
<dd>
 Option to pass a signer to the function
</dd>
<dt>
<code>f: |<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;| <b>has</b> <b>copy</b> + drop + store</code>
</dt>
<dd>
 Variables are captured in the closure; optionally a signer is passed; no return
=======
<code>next_schedule_delta_time: u64</code>
</dt>
<dd>
 txn to be rescheduled at scheduled_time + next_schedule_delta_time.
 Note: (1) Once set, the txn will be rescheduled at the same delta interval next time, and so on.
       (2) Can be cancelled, with the same id returned in insert(), to stop the perpetual rescheduling.
       (3) If one rescheduled fails or is expired, the perpetual rescheduling chain will be broken.
       (4) If scheduled_time + next_schedule_delta_time < current_time, the txn reschedule will fail.
</dd>
<dt>
<code>f: |()| <b>with</b> <b>copy</b>+store</code>
</dt>
<dd>
 Variables are captured in the closure; no arguments passed; no return
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dd>
</dl>


</details>

<<<<<<< HEAD
<a id="0x1_scheduled_txns_ScheduledTransactionWithKey"></a>

## Struct `ScheduledTransactionWithKey`
=======
<a id="0x1_scheduled_txns_ScheduledTransactionWithId"></a>

## Struct `ScheduledTransactionWithId`
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)

We pass the id around instead re-computing it


<<<<<<< HEAD
<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithKey">ScheduledTransactionWithKey</a> <b>has</b> drop
=======
<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a> <b>has</b> <b>copy</b>, drop, store
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a></code>
</dt>
<dd>

</dd>
<dt>
<<<<<<< HEAD
<code>key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a></code>
=======
<code>txn_id: <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">scheduled_txns::TransactionId</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_TransactionId"></a>

## Struct `TransactionId`

SHA3-256


<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">TransactionId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_ScheduleMapKey"></a>

## Struct `ScheduleMapKey`

First sorted in ascending order of time, then on gas priority, and finally on txn_id
gas_priority = U64_MAX - gas_unit_price; we want higher gas_unit_price to come before lower gas_unit_price
<<<<<<< HEAD
The goal is to have fixed (less variable) size 'key', 'val' entries in BigOrderedMap, hence we use txn_id
as a key. That is we have "{time, gas_priority, txn_id} -> ScheduledTxn" instead of
"{time, gas_priority} --> List<(txn_id, ScheduledTxn)>".
Note: ScheduledTxn is still variable size though due to its closure.
=======
The goal is to have fixed size key, val entries in BigOrderedMap, hence we use txn_id as a key instead of
having {time, gas_priority} --> List<txn_id>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)


<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_priority: u64</code>
</dt>
<dd>

</dd>
<dt>
<<<<<<< HEAD
<code>txn_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 SHA3-256
=======
<code>txn_id: <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">scheduled_txns::TransactionId</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_Empty"></a>

## Struct `Empty`

Dummy struct to use as a value type in BigOrderedMap


<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_Empty">Empty</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_ScheduleQueue"></a>

## Resource `ScheduleQueue`



<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<<<<<<< HEAD
<code>schedule_map: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>&gt;</code>
</dt>
<dd>
 key_size = 48 bytes; value_size = key_size + AVG_SCHED_TXN_SIZE
=======
<code>schedule_map: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_Empty">scheduled_txns::Empty</a>&gt;</code>
</dt>
<dd>
 key_size = 48 bytes; value_size = 0
</dd>
<dt>
<code>txn_tbl: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">scheduled_txns::TransactionId</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>&gt;</code>
</dt>
<dd>
 lookup 'ScheduledTransaction' by txn_id
 Using a 'table' is costly because it creates a new slot for every <key, val>; hence using BigOrderedMap
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dd>
</dl>


</details>

<<<<<<< HEAD
<a id="0x1_scheduled_txns_AuxiliaryData"></a>

## Resource `AuxiliaryData`
=======
<a id="0x1_scheduled_txns_GasFeeDepositStoreSignerCap"></a>

## Resource `GasFeeDepositStoreSignerCap`
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)

Signer for the store for gas fee deposits


<<<<<<< HEAD
<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> <b>has</b> key
=======
<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a> <b>has</b> key
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<<<<<<< HEAD
<code>gas_fee_deposit_store_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>

</dd>
<dt>
<code>stop_scheduling: bool</code>
=======
<code>cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_ToRemoveTbl"></a>

## Resource `ToRemoveTbl`



<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<<<<<<< HEAD
<code>remove_tbl: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u16, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>&gt;&gt;</code>
=======
<code>remove_tbl: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u16, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">scheduled_txns::TransactionId</a>&gt;&gt;</code>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dt>
<dd>

</dd>
</dl>


</details>

<<<<<<< HEAD
<a id="0x1_scheduled_txns_CancelledTxnCode"></a>

## Enum `CancelledTxnCode`



<pre><code>enum <a href="scheduled_txns.md#0x1_scheduled_txns_CancelledTxnCode">CancelledTxnCode</a> <b>has</b> drop, store
=======
<a id="0x1_scheduled_txns_Result"></a>

## Enum `Result`



<pre><code>enum <a href="scheduled_txns.md#0x1_scheduled_txns_Result">Result</a>&lt;T&gt; <b>has</b> <b>copy</b>, drop
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Variants</summary>


<details>
<<<<<<< HEAD
<summary>Shutdown</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>Expired</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_scheduled_txns_TransactionExpiredEvent"></a>

## Struct `TransactionExpiredEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionExpiredEvent">TransactionExpiredEvent</a> <b>has</b> drop, store
</code></pre>

=======
<summary>Ok</summary>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)


<details>
<summary>Fields</summary>


<dl>
<dt>
<<<<<<< HEAD
<code>key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a></code>
</dt>
<dd>

</dd>
<dt>
<code>sender_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_txn_code: <a href="scheduled_txns.md#0x1_scheduled_txns_CancelledTxnCode">scheduled_txns::CancelledTxnCode</a></code>
=======
<code>0: T</code>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dt>
<dd>

</dd>
</dl>


</details>

<<<<<<< HEAD
<a id="0x1_scheduled_txns_ShutdownEvent"></a>

## Struct `ShutdownEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ShutdownEvent">ShutdownEvent</a> <b>has</b> drop, store
</code></pre>

=======
</details>

<details>
<summary>Err</summary>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)


<details>
<summary>Fields</summary>


<dl>
<dt>
<<<<<<< HEAD
<code>complete: bool</code>
=======
<code>0: u64</code>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</dt>
<dd>

</dd>
</dl>


</details>

<<<<<<< HEAD
<a id="0x1_scheduled_txns_KeyAndTxnInfo"></a>

## Struct `KeyAndTxnInfo`



<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a></code>
</dt>
<dd>

</dd>
<dt>
<code>account_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_amt: u64</code>
</dt>
<dd>

</dd>
</dl>

=======
</details>
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)

</details>

<a id="0x1_scheduled_txns_State"></a>

## Struct `State`



<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_State">State</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>count: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_scheduled_txns_MICRO_CONVERSION_FACTOR"></a>

Conversion factor between our time granularity (100ms) and microseconds


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a>: u64 = 100000;
</code></pre>



<a id="0x1_scheduled_txns_AVG_FUNC_SIZE"></a>

The maximum size of a function in bytes


<<<<<<< HEAD
<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AVG_FUNC_SIZE">AVG_FUNC_SIZE</a>: u16 = 1000;
=======
<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AVG_FUNC_SIZE">AVG_FUNC_SIZE</a>: u16 = 128;
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<a id="0x1_scheduled_txns_AVG_SCHED_TXN_SIZE"></a>



<<<<<<< HEAD
<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AVG_SCHED_TXN_SIZE">AVG_SCHED_TXN_SIZE</a>: u16 = 1056;
=======
<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AVG_SCHED_TXN_SIZE">AVG_SCHED_TXN_SIZE</a>: u16 = 256;
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<a id="0x1_scheduled_txns_BIG_ORDRD_MAP_TGT_ND_SZ"></a>

BigOrderedMap has MAX_NODE_BYTES = 409600 (400KB), MAX_DEGREE = 4096, DEFAULT_TARGET_NODE_SIZE = 4096;


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_BIG_ORDRD_MAP_TGT_ND_SZ">BIG_ORDRD_MAP_TGT_ND_SZ</a>: u16 = 4096;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_SIGNER"></a>

Map key already exists


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SIGNER">EINVALID_SIGNER</a>: u64 = 1;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_TIME"></a>

Scheduled time is in the past


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_TIME">EINVALID_TIME</a>: u64 = 2;
</code></pre>



<<<<<<< HEAD
<a id="0x1_scheduled_txns_EUNAVAILABLE"></a>

Scheduling is stopped


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EUNAVAILABLE">EUNAVAILABLE</a>: u64 = 3;
</code></pre>



=======
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
<a id="0x1_scheduled_txns_EXPIRY_DELTA"></a>

If we cannot schedule in 100 * time granularity (10s, i.e 100 blocks), we will abort the txn


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EXPIRY_DELTA">EXPIRY_DELTA</a>: u64 = 100;
</code></pre>



<<<<<<< HEAD
<a id="0x1_scheduled_txns_GET_READY_TRANSACTIONS_LIMIT"></a>

The maximum number of scheduled transactions that can be run in a block


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_GET_READY_TRANSACTIONS_LIMIT">GET_READY_TRANSACTIONS_LIMIT</a>: u64 = 100;
</code></pre>



=======
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
<a id="0x1_scheduled_txns_MAX_FUNC_SIZE"></a>

The maximum size of a function in bytes


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_MAX_FUNC_SIZE">MAX_FUNC_SIZE</a>: u16 = 1024;
</code></pre>



<a id="0x1_scheduled_txns_MILLI_CONVERSION_FACTOR"></a>

Conversion factor between our time granularity (100ms) and milliseconds


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_MILLI_CONVERSION_FACTOR">MILLI_CONVERSION_FACTOR</a>: u64 = 100;
</code></pre>



<<<<<<< HEAD
<a id="0x1_scheduled_txns_REMOVE_LIMIT"></a>

The maximum number of transactions that are removed from the queue in a block
Even if there is a backlog of things to be removed, this will eventually catch-up.


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_REMOVE_LIMIT">REMOVE_LIMIT</a>: u64 = 200;
</code></pre>



=======
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
<a id="0x1_scheduled_txns_SCHEDULE_MAP_KEY_SIZE"></a>



<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_SCHEDULE_MAP_KEY_SIZE">SCHEDULE_MAP_KEY_SIZE</a>: u16 = 48;
</code></pre>



<<<<<<< HEAD
<a id="0x1_scheduled_txns_SHUTDOWN_CANCEL_LIMIT"></a>

The maximum number of transactions that can be cancelled in a block during shutdown


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_SHUTDOWN_CANCEL_LIMIT">SHUTDOWN_CANCEL_LIMIT</a>: u64 = 200;
=======
<a id="0x1_scheduled_txns_SCHEDULE_MAP_LEAF_DEGREE"></a>



<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_SCHEDULE_MAP_LEAF_DEGREE">SCHEDULE_MAP_LEAF_DEGREE</a>: u16 = 80;
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<a id="0x1_scheduled_txns_TO_REMOVE_PARALLELISM"></a>

We want reduce the contention while scheduled txns are being executed


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>: u64 = 32;
</code></pre>



<a id="0x1_scheduled_txns_TXN_ID_SIZE"></a>

SHA3-256 produces 32 bytes


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_TXN_ID_SIZE">TXN_ID_SIZE</a>: u16 = 32;
</code></pre>



<a id="0x1_scheduled_txns_U64_MAX"></a>



<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_U64_MAX">U64_MAX</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_scheduled_txns_initialize"></a>

## Function `initialize`

Can be called only by the framework


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);

    // Create owner <a href="account.md#0x1_account">account</a> for handling deposits
    <b>let</b> owner_addr = @0xb; // Replace <b>with</b> your desired <b>address</b>
    <b>let</b> (owner_signer, owner_cap) =
        <a href="account.md#0x1_account_create_framework_reserved_account">account::create_framework_reserved_account</a>(owner_addr);

    // Initialize fungible store for the owner
    <b>let</b> metadata = ensure_paired_metadata&lt;AptosCoin&gt;();
    <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">primary_fungible_store::ensure_primary_store_exists</a>(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&owner_signer), metadata
    );

    // Store the <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>
<<<<<<< HEAD
    <b>move_to</b>(framework, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> { gas_fee_deposit_store_signer_cap: owner_cap, stop_scheduling: <b>false</b> });
=======
    <b>move_to</b>(framework, <a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a> { cap: owner_cap });
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)

    // Initialize queue
    <b>let</b> queue = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a> {
        schedule_map: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_config">big_ordered_map::new_with_config</a>(
            <a href="scheduled_txns.md#0x1_scheduled_txns_BIG_ORDRD_MAP_TGT_ND_SZ">BIG_ORDRD_MAP_TGT_ND_SZ</a> / <a href="scheduled_txns.md#0x1_scheduled_txns_SCHEDULE_MAP_KEY_SIZE">SCHEDULE_MAP_KEY_SIZE</a>,
<<<<<<< HEAD
            (<a href="scheduled_txns.md#0x1_scheduled_txns_BIG_ORDRD_MAP_TGT_ND_SZ">BIG_ORDRD_MAP_TGT_ND_SZ</a> / (<a href="scheduled_txns.md#0x1_scheduled_txns_SCHEDULE_MAP_KEY_SIZE">SCHEDULE_MAP_KEY_SIZE</a> + <a href="scheduled_txns.md#0x1_scheduled_txns_AVG_SCHED_TXN_SIZE">AVG_SCHED_TXN_SIZE</a>)),
            <b>true</b>
        ),
=======
            <a href="scheduled_txns.md#0x1_scheduled_txns_SCHEDULE_MAP_LEAF_DEGREE">SCHEDULE_MAP_LEAF_DEGREE</a>,
            <b>true</b>
        ),
        txn_tbl: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_config">big_ordered_map::new_with_config</a>(
            (<a href="scheduled_txns.md#0x1_scheduled_txns_BIG_ORDRD_MAP_TGT_ND_SZ">BIG_ORDRD_MAP_TGT_ND_SZ</a> / <a href="scheduled_txns.md#0x1_scheduled_txns_TXN_ID_SIZE">TXN_ID_SIZE</a>),
            (<a href="scheduled_txns.md#0x1_scheduled_txns_BIG_ORDRD_MAP_TGT_ND_SZ">BIG_ORDRD_MAP_TGT_ND_SZ</a> / (<a href="scheduled_txns.md#0x1_scheduled_txns_TXN_ID_SIZE">TXN_ID_SIZE</a> + <a href="scheduled_txns.md#0x1_scheduled_txns_AVG_SCHED_TXN_SIZE">AVG_SCHED_TXN_SIZE</a>)),
            <b>true</b>
        )
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
    };
    <b>move_to</b>(framework, queue);

    // Parallelizable data structure used <b>to</b> track executed txn_ids.
    <b>move_to</b>(
        framework,
        <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> {
<<<<<<< HEAD
            remove_tbl: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;u16, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>&gt;&gt;()
=======
            remove_tbl: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;u16, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">TransactionId</a>&gt;&gt;()
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
        }
    );
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_shutdown"></a>

## Function `shutdown`

Stop, remove and refund all scheduled txns; can be called only by the framework


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_shutdown">shutdown</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_shutdown">shutdown</a>(
    framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
<<<<<<< HEAD
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);

    // set stop_scheduling flag
    <b>let</b> aux_data = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework));
    aux_data.stop_scheduling = <b>true</b>;

    <b>let</b> txns_to_cancel = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a>&gt;();
    // Make a list of txns <b>to</b> cancel <b>with</b> their keys and signers
    {
        <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework));

        // Iterate through schedule_map <b>to</b> get all transactions
        <b>let</b> iter = queue.schedule_map.new_begin_iter();
        <b>let</b> cancel_count = 0;
        <b>while</b> ((!iter.iter_is_end(&queue.schedule_map)) && (cancel_count &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_SHUTDOWN_CANCEL_LIMIT">SHUTDOWN_CANCEL_LIMIT</a>)) {
            <b>let</b> key = iter.iter_borrow_key();
            <b>let</b> txn = iter.iter_borrow(&queue.schedule_map);
            <b>let</b> deposit_amt = txn.max_gas_amount * txn.max_gas_unit_price;
            txns_to_cancel.push_back(<a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> {
                key: *key,
                account_addr: txn.sender_addr,
                deposit_amt
            });
            cancel_count = cancel_count + 1;
            iter = iter.iter_next(&queue.schedule_map);
        };
    };

    // Cancel transactions
    <b>while</b> (!txns_to_cancel.is_empty()) {
        <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> { key, account_addr, deposit_amt } = txns_to_cancel.pop_back();
        <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(account_addr, key, deposit_amt);
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_TransactionExpiredEvent">TransactionExpiredEvent</a> {
            key,
            sender_addr: account_addr,
            cancelled_txn_code: CancelledTxnCode::Shutdown
        });
    };

    // Remove and destroy schedule_map <b>if</b> empty
    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework));
    <b>if</b> (queue.schedule_map.is_empty()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_ShutdownEvent">ShutdownEvent</a> { complete: <b>true</b> });
    };

    // Clean up <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>
    <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> { remove_tbl } = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework));
=======
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);

    // Make a list of txns <b>to</b> cancel and refund
    <b>let</b> txns_to_cancel = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a>&gt;();
    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework));
    <b>let</b> iter = queue.txn_tbl.new_begin_iter();
    <b>while</b> (!iter.iter_is_end(&queue.txn_tbl)) {
        <b>let</b> txn_id = *iter.iter_borrow_key();
        <b>let</b> txn = iter.iter_borrow(&queue.txn_tbl);
        <b>let</b> scheduled_txn_with_id = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a> { txn: *txn, txn_id };
        txns_to_cancel.push_back(scheduled_txn_with_id);
        iter = iter.iter_next(&queue.txn_tbl);
    };

    // Cancel all transactions
    <b>while</b> (!txns_to_cancel.is_empty()) {
        <b>let</b> txn_with_id = txns_to_cancel.pop_back();
        // Create a new <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from the stored handle
        <b>let</b> schedule_txn_signer =
            <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(
                &txn_with_id.txn.sender_handle
            );
        <a href="scheduled_txns.md#0x1_scheduled_txns_cancel">cancel</a>(&schedule_txn_signer, txn_with_id.txn_id.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>);
    };

    // Remove and destroy resources
    <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a> { schedule_map, txn_tbl } =
        <b>move_from</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework));
    schedule_map.destroy(|_| {});
    txn_tbl.destroy(|_| {});

    // Clean up <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>; we can only empty the <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> but not drop it!
    <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> { remove_tbl } =
        <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework));
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>) {
        <b>if</b> (remove_tbl.contains((i <b>as</b> u16))) {
            remove_tbl.remove((i <b>as</b> u16));
        };
        i = i + 1;
    };
}
</code></pre>



</details>

<<<<<<< HEAD
<a id="0x1_scheduled_txns_new_scheduled_transaction"></a>

## Function `new_scheduled_transaction`

todo: Do we need a function to pause/unpause without issuing refund of deposit ???
Constructor


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_new_scheduled_transaction">new_scheduled_transaction</a>(sender_addr: <b>address</b>, scheduled_time: u64, max_gas_amount: u64, max_gas_unit_price: u64, pass_signer: bool, f: |<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;| <b>has</b> <b>copy</b> + drop + store): <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_new_scheduled_transaction">new_scheduled_transaction</a>(
    sender_addr: <b>address</b>,
    scheduled_time: u64,
    max_gas_amount: u64,
    max_gas_unit_price: u64,
    pass_signer: bool,
    f: |Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;| <b>has</b> <b>copy</b> + store + drop,
): <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a> {
    <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a> {
        sender_addr,
        scheduled_time,
        max_gas_amount,
        max_gas_unit_price,
        pass_signer,
        f,
    }
}
</code></pre>



</details>

=======
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
<a id="0x1_scheduled_txns_insert"></a>

## Function `insert`

<<<<<<< HEAD
Insert a scheduled transaction into the queue. Txn_id is returned to user, which can be used to cancel the txn.


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_insert">insert</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>): <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>
=======
todo: Do we need a function to pause ???
Insert a scheduled transaction into the queue. Txn_id is returned to user, which can be used to cancel the txn.


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_insert">insert</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_insert">insert</a>(
<<<<<<< HEAD
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a>
): <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    // If scheduling is shutdown, we cannot schedule <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> more transactions
    <b>let</b> aux_data = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(!aux_data.stop_scheduling, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unavailable">error::unavailable</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EUNAVAILABLE">EUNAVAILABLE</a>));

    // Generate unique transaction ID
    <b>let</b> txn_id = sha3_256(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&txn));

    // we expect the sender <b>to</b> be a permissioned <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender) == txn.sender_addr,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SIGNER">EINVALID_SIGNER</a>)
    );

    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);

    // Only schedule txns in the future
    <b>let</b> txn_time = txn.scheduled_time / <a href="scheduled_txns.md#0x1_scheduled_txns_MILLI_CONVERSION_FACTOR">MILLI_CONVERSION_FACTOR</a>; // Round down <b>to</b> the nearest 100ms
    <b>let</b> block_time = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() / <a href="scheduled_txns.md#0x1_scheduled_txns_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a>;
    <b>assert</b>!(txn_time &gt; block_time, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_TIME">EINVALID_TIME</a>));

    // Insert the transaction into the schedule_map
    // Create schedule map key
    <b>let</b> key = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> {
        time: txn_time,
        gas_priority: <a href="scheduled_txns.md#0x1_scheduled_txns_U64_MAX">U64_MAX</a> - txn.max_gas_unit_price,
        txn_id
    };
    queue.schedule_map.add(key, txn);

    // Collect deposit
    // Get owner <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>
    <b>let</b> gas_deposit_store_cap =
        <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>let</b> gas_deposit_store_signer =
        <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&gas_deposit_store_cap.gas_fee_deposit_store_signer_cap);
    <b>let</b> gas_deposit_store_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&gas_deposit_store_signer);

    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(
        sender,
        gas_deposit_store_addr,
        txn.max_gas_amount * txn.max_gas_unit_price
    );

    key
=======
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a>
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a> {
    // todo: we should limit the size of the scheduled txn; NOTE that f is of variable size ???
    // Generate a unique transaction ID only once and only here. Because all periodically rescheduled txns (<b>if</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a>)
    // will have the same txn_id generated here (which is different from the sha3_256 of the rescheduled txn)
    <b>let</b> txn_id = <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">TransactionId</a> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: sha3_256(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&txn))
    };
    <b>let</b> result = <a href="scheduled_txns.md#0x1_scheduled_txns_insert_txn_with_id">insert_txn_with_id</a>(sender, txn, txn_id);

    // Pattern match on the <a href="scheduled_txns.md#0x1_scheduled_txns_Result">Result</a>
    match(result) {
        Result::Ok(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>) =&gt; <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,
        Result::Err(error_code) =&gt; { <b>abort</b> error_code }
    }
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_cancel"></a>

## Function `cancel`

Cancel a scheduled transaction, must be called by the signer who originally scheduled the transaction.


<<<<<<< HEAD
<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel">cancel</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>)
=======
<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel">cancel</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel">cancel</a>(
<<<<<<< HEAD
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    // If scheduling is shutdown, we cannot schedule <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> more transactions
    <b>let</b> aux_data = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(!aux_data.stop_scheduling, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unavailable">error::unavailable</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EUNAVAILABLE">EUNAVAILABLE</a>));

    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>if</b> (!queue.schedule_map.contains(&key)) {
        <b>return</b>
    };

    <b>let</b> txn = *queue.schedule_map.borrow(&key);
    <b>let</b> deposit_amt = txn.max_gas_amount * txn.max_gas_unit_price;

    // verify sender
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender) == txn.sender_addr,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SIGNER">EINVALID_SIGNER</a>)
    );
    <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), key, deposit_amt);
=======
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a> {
    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>let</b> txn_id = <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">TransactionId</a> { <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: txn_id };
    <b>if</b> (!queue.txn_tbl.contains(&txn_id)) {
        <b>return</b>;
    };

    <b>let</b> txn = queue.txn_tbl.borrow(&txn_id);
    <b>let</b> deposit_amt = txn.max_gas_amount * txn.max_gas_unit_price;

    // we expect the sender <b>to</b> be a permissioned <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
    <b>let</b> schedule_txn_signer =
        <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(
            &txn.sender_handle
        );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender) == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&schedule_txn_signer),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SIGNER">EINVALID_SIGNER</a>)
    );

    // Remove the transaction from the schedule_map & txn_tbl
    <b>let</b> key = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> {
        time: txn.scheduled_time / <a href="scheduled_txns.md#0x1_scheduled_txns_MILLI_CONVERSION_FACTOR">MILLI_CONVERSION_FACTOR</a>,
        gas_priority: <a href="scheduled_txns.md#0x1_scheduled_txns_U64_MAX">U64_MAX</a> - txn.max_gas_unit_price,
        txn_id
    };
    queue.schedule_map.remove(&key);
    queue.txn_tbl.remove(&txn_id);

    // Refund the deposit
    // Get owner <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>
    <b>let</b> gas_deposit_store_cap =
        <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a>&gt;(@aptos_framework);
    <b>let</b> gas_deposit_store_signer =
        <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&gas_deposit_store_cap.cap);

    // Refund deposit from owner's store <b>to</b> sender
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(
        &gas_deposit_store_signer,
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        deposit_amt
    );
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
}
</code></pre>



</details>

<<<<<<< HEAD
<a id="0x1_scheduled_txns_cancel_internal"></a>

## Function `cancel_internal`

Internal cancel function that takes an address instead of signer. No signer verification, assumes key is present
in the schedule_map.


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(account_addr: <b>address</b>, key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>, deposit_amt: u64)
=======
<a id="0x1_scheduled_txns_insert_txn_with_id"></a>

## Function `insert_txn_with_id`

Common function called for both insert and reschedule


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_insert_txn_with_id">insert_txn_with_id</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>, txn_id: <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">scheduled_txns::TransactionId</a>): <a href="scheduled_txns.md#0x1_scheduled_txns_Result">scheduled_txns::Result</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Implementation</summary>


<<<<<<< HEAD
<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(
    account_addr: <b>address</b>,
    key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>,
    deposit_amt: u64,
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);

    // Remove the transaction from schedule_map
    queue.schedule_map.remove(&key);

    // Refund the deposit
    // Get owner <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>
    <b>let</b> gas_deposit_store_cap =
        <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>let</b> gas_deposit_store_signer =
        <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&gas_deposit_store_cap.gas_fee_deposit_store_signer_cap);

    // Refund deposit from owner's store <b>to</b> sender
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(
        &gas_deposit_store_signer,
        account_addr,
        deposit_amt
    );
=======
<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_insert_txn_with_id">insert_txn_with_id</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a>, txn_id: <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">TransactionId</a>
): <a href="scheduled_txns.md#0x1_scheduled_txns_Result">Result</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a> {
    // we expect the sender <b>to</b> be a permissioned <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
    <b>let</b> schedule_txn_signer =
        <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(
            &txn.sender_handle
        );
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender) != <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&schedule_txn_signer)) {
        <b>return</b> Result::Err(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SIGNER">EINVALID_SIGNER</a>));
    };

    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>if</b> (queue.txn_tbl.contains(&txn_id)) {
        <b>return</b> Result::Ok(txn_id.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>);
    };

    // Only schedule txns in the future
    <b>let</b> txn_time = txn.scheduled_time / <a href="scheduled_txns.md#0x1_scheduled_txns_MILLI_CONVERSION_FACTOR">MILLI_CONVERSION_FACTOR</a>; // Round down <b>to</b> the nearest 100ms
    <b>let</b> block_time = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() / <a href="scheduled_txns.md#0x1_scheduled_txns_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a>;
    <b>if</b> (txn_time &lt; block_time) {
        <b>return</b> Result::Err(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_TIME">EINVALID_TIME</a>));
    };

    // We need inverse of gas_unit_price for ordering because <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> is sorted in ascending order time
    // first and then on gas_priority
    <b>let</b> gas_priority = <a href="scheduled_txns.md#0x1_scheduled_txns_U64_MAX">U64_MAX</a> - txn.max_gas_unit_price;
    <b>let</b> key = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> { time: txn_time, gas_priority, txn_id };

    // Insert the transaction into the schedule_map & txn_tbl
    queue.schedule_map.add(key, <a href="scheduled_txns.md#0x1_scheduled_txns_Empty">Empty</a> {});
    queue.txn_tbl.add(txn_id, txn);

    // Collect deposit
    // Get owner <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>
    <b>let</b> gas_deposit_store_cap =
        <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a>&gt;(@aptos_framework);
    <b>let</b> gas_deposit_store_signer =
        <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(&gas_deposit_store_cap.cap);
    <b>let</b> gas_deposit_store_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&gas_deposit_store_signer);

    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(
        sender,
        gas_deposit_store_addr,
        txn.max_gas_amount * txn.max_gas_unit_price
    );
    Result::Ok(txn_id.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>)
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_get_ready_transactions"></a>

## Function `get_ready_transactions`

Gets txns due to be run; also expire txns that could not be run for a while (mostly due to low gas priority)


<<<<<<< HEAD
<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_get_ready_transactions">get_ready_transactions</a>(<a href="timestamp.md#0x1_timestamp">timestamp</a>: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithKey">scheduled_txns::ScheduledTransactionWithKey</a>&gt;
=======
<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_get_ready_transactions">get_ready_transactions</a>(<a href="timestamp.md#0x1_timestamp">timestamp</a>: u64, limit: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">scheduled_txns::ScheduledTransactionWithId</a>&gt;
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_get_ready_transactions">get_ready_transactions</a>(
<<<<<<< HEAD
    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithKey">ScheduledTransactionWithKey</a>&gt; <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    // If scheduling is shutdown, we cannot schedule <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> more transactions
    <b>let</b> aux_data = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>if</b> (aux_data.stop_scheduling) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithKey">ScheduledTransactionWithKey</a>&gt;();
    };

    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>let</b> block_time = <a href="timestamp.md#0x1_timestamp">timestamp</a> / <a href="scheduled_txns.md#0x1_scheduled_txns_MILLI_CONVERSION_FACTOR">MILLI_CONVERSION_FACTOR</a>;
    <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithKey">ScheduledTransactionWithKey</a>&gt;();
    <b>let</b> count = 0;
    <b>let</b> txns_to_expire = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a>&gt;();

    <b>let</b> iter = queue.schedule_map.new_begin_iter();
    <b>while</b> (!iter.iter_is_end(&queue.schedule_map) && count &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_GET_READY_TRANSACTIONS_LIMIT">GET_READY_TRANSACTIONS_LIMIT</a>) {
        <b>let</b> key = iter.iter_borrow_key();
        <b>if</b> (key.time &gt; block_time) {
            <b>break</b>;
        };
        <b>let</b> txn = *iter.iter_borrow(&queue.schedule_map);
        <b>let</b> scheduled_txn_with_id = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithKey">ScheduledTransactionWithKey</a> {
            txn,
            key: *key,
        };

        <b>if</b> (key.time + <a href="scheduled_txns.md#0x1_scheduled_txns_EXPIRY_DELTA">EXPIRY_DELTA</a> &lt; block_time) {
            // Transaction <b>has</b> expired
            <b>let</b> deposit_amt = txn.max_gas_amount * txn.max_gas_unit_price;
            txns_to_expire.push_back(<a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> {
                key: *key,
                account_addr: txn.sender_addr,
                deposit_amt
            });
        } <b>else</b> {
            <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a>.push_back(scheduled_txn_with_id);
        };
        // we do not want an unbounded size of ready or expirable txns; hence we increment either way
        count = count + 1;
        iter = iter.iter_next(&queue.schedule_map);
    };

    // Cancel expired transactions
    <b>while</b> (!txns_to_expire.is_empty()) {
        <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> { key, account_addr, deposit_amt } = txns_to_expire.pop_back();
        <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(
            account_addr,
            key,
            deposit_amt
        );
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_TransactionExpiredEvent">TransactionExpiredEvent</a> {
            key,
            sender_addr: account_addr,
            cancelled_txn_code: CancelledTxnCode::Expired
        });
=======
    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64, limit: u64
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a>&gt; <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a> {
    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>let</b> block_time = <a href="timestamp.md#0x1_timestamp">timestamp</a> / <a href="scheduled_txns.md#0x1_scheduled_txns_MILLI_CONVERSION_FACTOR">MILLI_CONVERSION_FACTOR</a>;
    <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a>&gt;();
    <b>let</b> count = 0;
    <b>let</b> txns_to_expire = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a>&gt;();

    <b>let</b> iter = queue.schedule_map.new_begin_iter();
    <b>while</b> (!iter.iter_is_end(&queue.schedule_map) && count &lt; limit) {
        <b>let</b> scheduled_key = iter.iter_borrow_key();
        <b>if</b> (scheduled_key.time &gt; block_time) {
            <b>return</b> <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a>;
        };
        <b>let</b> txn_id = scheduled_key.txn_id;
        <b>let</b> txn = *queue.txn_tbl.borrow(&txn_id);
        <b>let</b> scheduled_txn_with_id = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a> { txn, txn_id };
        <b>if</b> (scheduled_key.time + <a href="scheduled_txns.md#0x1_scheduled_txns_EXPIRY_DELTA">EXPIRY_DELTA</a> &lt; block_time) {
            // Transaction <b>has</b> expired
            txns_to_expire.push_back(scheduled_txn_with_id);
        } <b>else</b> {
            <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a>.push_back(scheduled_txn_with_id);
            count = count + 1;
        };
        iter = iter.iter_next(&queue.schedule_map);
    };

    <b>while</b> (!txns_to_expire.is_empty()) {
        <b>let</b> txn_with_id = txns_to_expire.pop_back();
        // Create a new <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from the stored handle
        <b>let</b> schedule_txn_signer =
            <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(
                &txn_with_id.txn.sender_handle
            );
        <a href="scheduled_txns.md#0x1_scheduled_txns_cancel">cancel</a>(&schedule_txn_signer, txn_with_id.txn_id.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>);
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
    };

    <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a>
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_finish_execution"></a>

## Function `finish_execution`

Increment after every scheduled transaction is run
IMP: Make sure this does not affect parallel execution of txns


<<<<<<< HEAD
<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_finish_execution">finish_execution</a>(key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>)
=======
<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_finish_execution">finish_execution</a>(txn_id: <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">scheduled_txns::TransactionId</a>)
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Implementation</summary>


<<<<<<< HEAD
<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_finish_execution">finish_execution</a>(key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> {
    // Get first 8 bytes of the <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> <b>as</b> u64 and then mod
    <b>let</b> hash_bytes = key.txn_id;
    <b>assert</b>!(hash_bytes.length() == 32, hash_bytes.length()); // SHA3-256 produces 32 bytes

    // Take first 8 bytes and convert <b>to</b> u64
    <b>let</b> hash_first_8_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> idx = 0;
    <b>while</b> (idx &lt; 8) {
        hash_first_8_bytes.push_back(hash_bytes[idx]);
        idx = idx + 1;
    };
    <b>let</b> value = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(hash_first_8_bytes);

    // Calculate <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> index using <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
=======
<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_finish_execution">finish_execution</a>(txn_id: <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">TransactionId</a>) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> {
    // Get first 8 bytes of the <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> <b>as</b> u64 and then mod
    <b>let</b> hash_bytes = txn_id.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>;
    <b>assert</b>!(hash_bytes.length() == 32, hash_bytes.length()); // SHA3-256 produces 32 bytes

    // Take first 8 bytes and convert <b>to</b> u64
    <b>let</b> value =
        ((hash_bytes[0] <b>as</b> u64) &lt;&lt; 56) | ((hash_bytes[1] <b>as</b> u64) &lt;&lt; 48)
            | ((hash_bytes[2] <b>as</b> u64) &lt;&lt; 40) | ((hash_bytes[3] <b>as</b> u64) &lt;&lt; 32)
            | ((hash_bytes[4] <b>as</b> u64) &lt;&lt; 24) | ((hash_bytes[5] <b>as</b> u64) &lt;&lt; 16)
            | ((hash_bytes[6] <b>as</b> u64) &lt;&lt; 8) | (hash_bytes[7] <b>as</b> u64);

    // todo: check <b>if</b> it is efficient <b>to</b> compute tbl_idx in rust instead
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
    <b>let</b> tbl_idx = ((value % <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>) <b>as</b> u16);
    <b>let</b> to_remove = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>&gt;(@aptos_framework);

    <b>if</b> (!to_remove.remove_tbl.contains(tbl_idx)) {
<<<<<<< HEAD
        <b>let</b> keys = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>&gt;();
        keys.push_back(key);
        to_remove.remove_tbl.add(tbl_idx, keys);
    } <b>else</b> {
        <b>let</b> keys = to_remove.remove_tbl.borrow_mut(tbl_idx);
        keys.push_back(key);
=======
        <b>let</b> txn_ids = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_TransactionId">TransactionId</a>&gt;();
        txn_ids.push_back(txn_id);
        to_remove.remove_tbl.add(tbl_idx, txn_ids);
    } <b>else</b> {
        <b>let</b> txn_ids = to_remove.remove_tbl.borrow_mut(tbl_idx);
        txn_ids.push_back(txn_id);
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
    };
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_remove_txns"></a>

## Function `remove_txns`

Remove the txns that are run


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txns">remove_txns</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<<<<<<< HEAD
<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txns">remove_txns</a>() <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a> {
    <b>let</b> to_remove = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>&gt;(@aptos_framework);
    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>let</b> tbl_idx: u16 = 0;

    <b>let</b> remove_count = 0;
    <b>while</b> (((tbl_idx <b>as</b> u64) &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>) && (remove_count &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_REMOVE_LIMIT">REMOVE_LIMIT</a>)) {
        <b>if</b> (to_remove.remove_tbl.contains(tbl_idx)) {
            <b>let</b> keys = to_remove.remove_tbl.borrow_mut(tbl_idx);

            <b>while</b> (!keys.is_empty()) {
                <b>let</b> key = keys.pop_back();
                <b>if</b> (queue.schedule_map.contains(&key)) {
                    // Remove transaction from schedule_map
                    remove_count = remove_count + 1;
                    queue.schedule_map.remove(&key);
                    <b>if</b> (remove_count &gt;= <a href="scheduled_txns.md#0x1_scheduled_txns_REMOVE_LIMIT">REMOVE_LIMIT</a>) {
                        <b>break</b>;
                    };
                };
            };
        };
        tbl_idx = tbl_idx + 1;
    };
=======
<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txns">remove_txns</a>() <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_GasFeeDepositStoreSignerCap">GasFeeDepositStoreSignerCap</a> {
    <b>let</b> to_remove = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>&gt;(@aptos_framework);
    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>let</b> idx: u16 = 0;
    <b>let</b> txns_to_reschedule = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a>&gt;();

    <b>while</b> ((idx <b>as</b> u64) &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>) {
        <b>if</b> (to_remove.remove_tbl.contains(idx)) {
            <b>let</b> txn_ids = to_remove.remove_tbl.remove(idx);
            <b>let</b> txn_ids_len = txn_ids.length();
            <b>let</b> txn_idx = 0;

            <b>while</b> (txn_idx &lt; txn_ids_len) {
                <b>let</b> txn_id = *txn_ids.borrow(txn_idx);
                // Remove transaction from txn_tbl
                <b>let</b> txn = queue.txn_tbl.remove(&txn_id);
                <b>let</b> key =
                    <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> {
                        time: txn.scheduled_time / <a href="scheduled_txns.md#0x1_scheduled_txns_MILLI_CONVERSION_FACTOR">MILLI_CONVERSION_FACTOR</a>,
                        gas_priority: <a href="scheduled_txns.md#0x1_scheduled_txns_U64_MAX">U64_MAX</a> - txn.max_gas_unit_price,
                        txn_id
                    };

                <b>if</b> (txn.next_schedule_delta_time &gt; 0) {
                    // Reschedule the transaction
                    txn.scheduled_time =
                        txn.scheduled_time + txn.next_schedule_delta_time;
                    txns_to_reschedule.push_back(
                        <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionWithId">ScheduledTransactionWithId</a> { txn, txn_id }
                    );
                };
                // Remove transaction from schedule_map
                queue.schedule_map.remove(&key);
                txn_idx = txn_idx + 1;
            };
        };
        idx = idx + 1;
    };

    // Reinsert the transactions that need <b>to</b> be rescheduled
    <b>while</b> (!txns_to_reschedule.is_empty()) {
        <b>let</b> txn_with_id = txns_to_reschedule.pop_back();
        // Create a new <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from the stored handle
        <b>let</b> schedule_txn_signer =
            <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(
                &txn_with_id.txn.sender_handle
            );
        <b>let</b> res =
            <a href="scheduled_txns.md#0x1_scheduled_txns_insert_txn_with_id">insert_txn_with_id</a>(
                &schedule_txn_signer, txn_with_id.txn, txn_with_id.txn_id
            );
        match(res) {
            Result::Ok(_) =&gt; {},
            Result::Err(error_code) =&gt; {
                <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(
                    &std::string::utf8(
                        b"Error in rescheduling transaction; perhaps too small reschedule delta time; <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> <a href="code.md#0x1_code">code</a>: "
                    )
                );
                <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&error_code); // Print the <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> <a href="code.md#0x1_code">code</a>
            }
        }
    }
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_step"></a>

## Function `step`



<pre><code>#[persistent]
<<<<<<< HEAD
<b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_step">step</a>(state: <a href="scheduled_txns.md#0x1_scheduled_txns_State">scheduled_txns::State</a>, _s: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;)
=======
<b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_step">step</a>(state: <a href="scheduled_txns.md#0x1_scheduled_txns_State">scheduled_txns::State</a>, _val: u64)
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
</code></pre>



<details>
<summary>Implementation</summary>


<<<<<<< HEAD
<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_step">step</a>(state: <a href="scheduled_txns.md#0x1_scheduled_txns_State">State</a>, _s: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;) {
=======
<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_step">step</a>(state: <a href="scheduled_txns.md#0x1_scheduled_txns_State">State</a>, _val: u64) {
>>>>>>> fbdc411893 ([Scheduled Txns] Introduce scheduled txns move module)
    <b>if</b> (state.count &lt; 10) {
        state.count = state.count + 1;
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
