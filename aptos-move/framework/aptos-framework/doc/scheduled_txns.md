
<a id="0x1_scheduled_txns"></a>

# Module `0x1::scheduled_txns`



-  [Enum `ScheduledFunction`](#0x1_scheduled_txns_ScheduledFunction)
-  [Struct `ScheduledTransaction`](#0x1_scheduled_txns_ScheduledTransaction)
-  [Struct `ScheduledTransactionInfoWithKey`](#0x1_scheduled_txns_ScheduledTransactionInfoWithKey)
-  [Struct `ScheduleMapKey`](#0x1_scheduled_txns_ScheduleMapKey)
-  [Struct `Empty`](#0x1_scheduled_txns_Empty)
-  [Resource `ScheduleQueue`](#0x1_scheduled_txns_ScheduleQueue)
-  [Enum `ScheduledTxnsModuleStatus`](#0x1_scheduled_txns_ScheduledTxnsModuleStatus)
-  [Resource `AuxiliaryData`](#0x1_scheduled_txns_AuxiliaryData)
-  [Resource `ToRemoveTbl`](#0x1_scheduled_txns_ToRemoveTbl)
-  [Enum `CancelledTxnCode`](#0x1_scheduled_txns_CancelledTxnCode)
-  [Struct `TransactionScheduledEvent`](#0x1_scheduled_txns_TransactionScheduledEvent)
-  [Struct `TransactionFailedEvent`](#0x1_scheduled_txns_TransactionFailedEvent)
-  [Struct `ShutdownEvent`](#0x1_scheduled_txns_ShutdownEvent)
-  [Struct `KeyAndTxnInfo`](#0x1_scheduled_txns_KeyAndTxnInfo)
-  [Struct `State`](#0x1_scheduled_txns_State)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_scheduled_txns_initialize)
-  [Function `start_shutdown`](#0x1_scheduled_txns_start_shutdown)
-  [Function `continue_shutdown`](#0x1_scheduled_txns_continue_shutdown)
-  [Function `re_initialize`](#0x1_scheduled_txns_re_initialize)
-  [Function `process_shutdown_batch`](#0x1_scheduled_txns_process_shutdown_batch)
-  [Function `complete_shutdown`](#0x1_scheduled_txns_complete_shutdown)
-  [Function `pause_scheduled_txns`](#0x1_scheduled_txns_pause_scheduled_txns)
-  [Function `unpause_scheduled_txns`](#0x1_scheduled_txns_unpause_scheduled_txns)
-  [Function `set_expiry_delta`](#0x1_scheduled_txns_set_expiry_delta)
-  [Function `new_scheduled_transaction`](#0x1_scheduled_txns_new_scheduled_transaction)
-  [Function `insert`](#0x1_scheduled_txns_insert)
-  [Function `cancel`](#0x1_scheduled_txns_cancel)
-  [Function `truncate_to_u64`](#0x1_scheduled_txns_truncate_to_u64)
-  [Function `hash_to_u256`](#0x1_scheduled_txns_hash_to_u256)
-  [Function `cancel_internal`](#0x1_scheduled_txns_cancel_internal)
-  [Function `get_ready_transactions`](#0x1_scheduled_txns_get_ready_transactions)
-  [Function `get_ready_transactions_with_limit`](#0x1_scheduled_txns_get_ready_transactions_with_limit)
-  [Function `mark_txn_to_remove`](#0x1_scheduled_txns_mark_txn_to_remove)
-  [Function `cancel_and_remove_expired_txns`](#0x1_scheduled_txns_cancel_and_remove_expired_txns)
-  [Function `remove_txns`](#0x1_scheduled_txns_remove_txns)
-  [Function `execute_user_function_wrapper`](#0x1_scheduled_txns_execute_user_function_wrapper)
-  [Function `emit_transaction_failed_event`](#0x1_scheduled_txns_emit_transaction_failed_event)
-  [Function `step`](#0x1_scheduled_txns_step)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_scheduled_txns_ScheduledFunction"></a>

## Enum `ScheduledFunction`



<pre><code>enum <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledFunction">ScheduledFunction</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: |<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;| <b>has</b> <b>copy</b> + drop + store</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_scheduled_txns_ScheduledTransaction"></a>

## Struct `ScheduledTransaction`

ScheduledTransaction with scheduled_time, gas params, and function


<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sender_addr: <b>address</b></code>
</dt>
<dd>
 32 bytes
</dd>
<dt>
<code>scheduled_time_ms: u64</code>
</dt>
<dd>
 UTC timestamp in milliseconds
</dd>
<dt>
<code>max_gas_amount: u64</code>
</dt>
<dd>
 Maximum gas to spend for this transaction
</dd>
<dt>
<code>gas_unit_price: u64</code>
</dt>
<dd>
 Gas unit price that the user is willing to pay for this txn when it is scheduled
</dd>
<dt>
<code>pass_signer: bool</code>
</dt>
<dd>
 Option to pass a signer to the function
</dd>
<dt>
<code>f: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledFunction">scheduled_txns::ScheduledFunction</a></code>
</dt>
<dd>
 Variables are captured in the closure; optionally a signer is passed; no return
</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_ScheduledTransactionInfoWithKey"></a>

## Struct `ScheduledTransactionInfoWithKey`

We pass around only needed info


<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionInfoWithKey">ScheduledTransactionInfoWithKey</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sender_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>max_gas_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_unit_price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_ScheduleMapKey"></a>

## Struct `ScheduleMapKey`

First sorted in ascending order of time, then on gas priority, and finally on txn_id
The goal is to have fixed (less variable) size 'key', 'val' entries in BigOrderedMap, hence we use txn_id
as a key. That is we have "{time, gas_priority, txn_id} -> ScheduledTxn" instead of
"{time, gas_priority} --> List<(txn_id, ScheduledTxn)>".
Note: ScheduledTxn is still variable size though due to its closure.


<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>time: u64</code>
</dt>
<dd>
 UTC timestamp ms
</dd>
<dt>
<code>gas_priority: u64</code>
</dt>
<dd>
 gas_priority = U64_MAX - gas_unit_price; we want higher gas_unit_price to come before lower gas_unit_price
</dd>
<dt>
<code>txn_id: u256</code>
</dt>
<dd>
 SHA3-256
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
<code>schedule_map: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_Empty">scheduled_txns::Empty</a>&gt;</code>
</dt>
<dd>
 key_size = 48 bytes
</dd>
<dt>
<code>txn_table: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u256, <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>&gt;</code>
</dt>
<dd>
 key: txn_id; value: ScheduledTransaction (metadata, function and capture)
</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_ScheduledTxnsModuleStatus"></a>

## Enum `ScheduledTxnsModuleStatus`



<pre><code>enum <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTxnsModuleStatus">ScheduledTxnsModuleStatus</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Active</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>Paused</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>ShutdownInProgress</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>ShutdownComplete</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_scheduled_txns_AuxiliaryData"></a>

## Resource `AuxiliaryData`

Stores module level auxiliary data


<pre><code><b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>gas_fee_deposit_store_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>
 Capability for managing the gas fee deposit store
</dd>
<dt>
<code>module_status: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTxnsModuleStatus">scheduled_txns::ScheduledTxnsModuleStatus</a></code>
</dt>
<dd>

</dd>
<dt>
<code>expiry_delta: u64</code>
</dt>
<dd>
 Expiry delta used to determine when scheduled transactions become invalid (and subsequently aborted)
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
<code>remove_tbl: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u16, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>&gt;&gt;</code>
</dt>
<dd>
 After a transaction is executed, it is marked for removal from the ScheduleQueue using this table.
 Direct removal from the ScheduleQueue is avoided to prevent serialization on access of ScheduleQueue.
 The remove table has as many slots as the number of transactions run in a block, minimizing the chances of
 serialization
</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_CancelledTxnCode"></a>

## Enum `CancelledTxnCode`



<pre><code>enum <a href="scheduled_txns.md#0x1_scheduled_txns_CancelledTxnCode">CancelledTxnCode</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
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

<details>
<summary>Failed</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_scheduled_txns_TransactionScheduledEvent"></a>

## Struct `TransactionScheduledEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionScheduledEvent">TransactionScheduledEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>block_time_ms: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>scheduled_txn_hash: u256</code>
</dt>
<dd>

</dd>
<dt>
<code>sender_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>scheduled_time_ms: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>max_gas_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_unit_price: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_TransactionFailedEvent"></a>

## Struct `TransactionFailedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionFailedEvent">TransactionFailedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>scheduled_txn_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>scheduled_txn_hash: u256</code>
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
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_scheduled_txns_ShutdownEvent"></a>

## Struct `ShutdownEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ShutdownEvent">ShutdownEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>complete: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

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


<a id="0x1_scheduled_txns_AVG_SCHED_TXN_SIZE"></a>

The average size of a scheduled transaction to provide an estimate of leaf nodes of BigOrderedMap


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AVG_SCHED_TXN_SIZE">AVG_SCHED_TXN_SIZE</a>: u16 = 1024;
</code></pre>



<a id="0x1_scheduled_txns_BIG_ORDRD_MAP_TGT_ND_SZ"></a>

BigOrderedMap has MAX_NODE_BYTES = 409600 (400KB), MAX_DEGREE = 4096, DEFAULT_TARGET_NODE_SIZE = 4096;


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_BIG_ORDRD_MAP_TGT_ND_SZ">BIG_ORDRD_MAP_TGT_ND_SZ</a>: u16 = 4096;
</code></pre>



<a id="0x1_scheduled_txns_CANCEL_DELTA_DEFAULT"></a>

Can't cancel a transaction that is going to be run in next 10 seconds


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_CANCEL_DELTA_DEFAULT">CANCEL_DELTA_DEFAULT</a>: u64 = 10000;
</code></pre>



<a id="0x1_scheduled_txns_DEPOSIT_STORE_OWNER_ADDR"></a>

Framework owned address that stores the deposits for all scheduled txns


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_DEPOSIT_STORE_OWNER_ADDR">DEPOSIT_STORE_OWNER_ADDR</a>: <b>address</b> = 0xb;
</code></pre>



<a id="0x1_scheduled_txns_ECANCEL_TOO_LATE"></a>

Cannot cancel a transaction that is about to be run or has already been run


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ECANCEL_TOO_LATE">ECANCEL_TOO_LATE</a>: u64 = 13;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_HASH_SIZE"></a>

Indicates error in SHA3-256 generation


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_HASH_SIZE">EINVALID_HASH_SIZE</a>: u64 = 7;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_PAUSE_ATTEMPT"></a>

Can be paused only when module is in Active state


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_PAUSE_ATTEMPT">EINVALID_PAUSE_ATTEMPT</a>: u64 = 11;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_SHUTDOWN_ATTEMPT"></a>

Shutdown attempted without starting it


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SHUTDOWN_ATTEMPT">EINVALID_SHUTDOWN_ATTEMPT</a>: u64 = 9;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_SHUTDOWN_START"></a>

Trying to start shutdown when module is not in Active state


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SHUTDOWN_START">EINVALID_SHUTDOWN_START</a>: u64 = 8;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_SIGNER"></a>

Map key already exists


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SIGNER">EINVALID_SIGNER</a>: u64 = 1;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_TIME"></a>

Scheduled time is in the past


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_TIME">EINVALID_TIME</a>: u64 = 2;
</code></pre>



<a id="0x1_scheduled_txns_EINVALID_UNPAUSE_ATTEMPT"></a>

Can be paused only when module is in Paused state


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_UNPAUSE_ATTEMPT">EINVALID_UNPAUSE_ATTEMPT</a>: u64 = 12;
</code></pre>



<a id="0x1_scheduled_txns_ELOW_GAS_UNIT_PRICE"></a>

Gas unit price is too low


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ELOW_GAS_UNIT_PRICE">ELOW_GAS_UNIT_PRICE</a>: u64 = 4;
</code></pre>



<a id="0x1_scheduled_txns_ESHUTDOWN_IN_PROGRESS"></a>

Shutdown is already in progress


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ESHUTDOWN_IN_PROGRESS">ESHUTDOWN_IN_PROGRESS</a>: u64 = 10;
</code></pre>



<a id="0x1_scheduled_txns_ETOO_LOW_GAS_AMOUNT"></a>

Gas amout too low, not enough to cover fixed costs while running the scheduled transaction


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ETOO_LOW_GAS_AMOUNT">ETOO_LOW_GAS_AMOUNT</a>: u64 = 5;
</code></pre>



<a id="0x1_scheduled_txns_ETXN_TOO_LARGE"></a>

Txn size is too large; beyond 10KB


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ETXN_TOO_LARGE">ETXN_TOO_LARGE</a>: u64 = 6;
</code></pre>



<a id="0x1_scheduled_txns_EUNAVAILABLE"></a>

Scheduling is stopped


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EUNAVAILABLE">EUNAVAILABLE</a>: u64 = 3;
</code></pre>



<a id="0x1_scheduled_txns_EXPIRE_TRANSACTIONS_LIMIT"></a>

Maximum number of transactions that can be expired during block prologue


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EXPIRE_TRANSACTIONS_LIMIT">EXPIRE_TRANSACTIONS_LIMIT</a>: u64 = 200;
</code></pre>



<a id="0x1_scheduled_txns_EXPIRY_DELTA_DEFAULT"></a>

If we cannot schedule in 10s, we will abort the txn


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_EXPIRY_DELTA_DEFAULT">EXPIRY_DELTA_DEFAULT</a>: u64 = 10000;
</code></pre>



<a id="0x1_scheduled_txns_GET_READY_TRANSACTIONS_LIMIT"></a>

Maximum number of scheduled transactions that can be run in a block


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_GET_READY_TRANSACTIONS_LIMIT">GET_READY_TRANSACTIONS_LIMIT</a>: u64 = 100;
</code></pre>



<a id="0x1_scheduled_txns_MASK_64"></a>



<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_MASK_64">MASK_64</a>: u256 = 18446744073709551615;
</code></pre>



<a id="0x1_scheduled_txns_MAX_SCHED_TXN_SIZE"></a>

Max size of a scheduled transaction; 1MB for now as we are bounded by the slot size


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_MAX_SCHED_TXN_SIZE">MAX_SCHED_TXN_SIZE</a>: u64 = 1048576;
</code></pre>



<a id="0x1_scheduled_txns_MIN_GAS_AMOUNT"></a>

Min gas amount


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_MIN_GAS_AMOUNT">MIN_GAS_AMOUNT</a>: u64 = 100;
</code></pre>



<a id="0x1_scheduled_txns_MIN_GAS_UNIT_PRICE"></a>

Min gas unit price


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_MIN_GAS_UNIT_PRICE">MIN_GAS_UNIT_PRICE</a>: u64 = 100;
</code></pre>



<a id="0x1_scheduled_txns_SCHEDULE_MAP_KEY_SIZE"></a>



<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_SCHEDULE_MAP_KEY_SIZE">SCHEDULE_MAP_KEY_SIZE</a>: u16 = 48;
</code></pre>



<a id="0x1_scheduled_txns_SHUTDOWN_CANCEL_BATCH_SIZE_DEFAULT"></a>

Maximum number of transactions that can be cancelled in a block during shutdown


<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_SHUTDOWN_CANCEL_BATCH_SIZE_DEFAULT">SHUTDOWN_CANCEL_BATCH_SIZE_DEFAULT</a>: u64 = 200;
</code></pre>



<a id="0x1_scheduled_txns_TO_REMOVE_PARALLELISM"></a>



<pre><code><b>const</b> <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>: u64 = 100;
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


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);

    // Create owner <a href="account.md#0x1_account">account</a> for handling deposits
    <b>let</b> owner_addr = <a href="scheduled_txns.md#0x1_scheduled_txns_DEPOSIT_STORE_OWNER_ADDR">DEPOSIT_STORE_OWNER_ADDR</a>;
    <b>let</b> (owner_signer, owner_cap) =
        <a href="account.md#0x1_account_create_framework_reserved_account">account::create_framework_reserved_account</a>(owner_addr);

    // Initialize fungible store for the owner
    <b>let</b> metadata = ensure_paired_metadata&lt;AptosCoin&gt;();
    <b>let</b> deposit_store =
        <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">primary_fungible_store::ensure_primary_store_exists</a>(
            <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&owner_signer), metadata
        );
    upgrade_store_to_concurrent(&owner_signer, deposit_store);

    // Store the <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>
    <b>move_to</b>(
        framework,
        <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
            gas_fee_deposit_store_signer_cap: owner_cap,
            module_status: ScheduledTxnsModuleStatus::Active,
            expiry_delta: <a href="scheduled_txns.md#0x1_scheduled_txns_EXPIRY_DELTA_DEFAULT">EXPIRY_DELTA_DEFAULT</a>
        }
    );

    // Initialize queue
    <b>let</b> queue = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a> {
        schedule_map: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_reusable">big_ordered_map::new_with_reusable</a>(),
        txn_table: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;u256, <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a>&gt;()
    };
    <b>move_to</b>(framework, queue);

    // Initialize remove_tbl <b>with</b> empty vectors for all slots
    <b>let</b> remove_tbl = <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;u16, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>&gt;&gt;();
    <b>let</b> i: u16 = 0;
    <b>while</b> ((i <b>as</b> u64) &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>) {
        remove_tbl.add(i, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>&gt;());
        i = i + 1;
    };

    // Parallelizable data structure used <b>to</b> track executed txn_ids.
    <b>move_to</b>(framework, <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> { remove_tbl });
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_start_shutdown"></a>

## Function `start_shutdown`

Starts the shutdown process. Can only be called when module status is Active.
We need a governance proposal to shutdown the module. Possible reasons to shutdown are:
(a) the stakeholders decide the feature is no longer needed
(b) there is an invariant violation detected, and the only way out is to shutdown and cancel all txns


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_start_shutdown">start_shutdown</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_start_shutdown">start_shutdown</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);

    <b>let</b> aux_data = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SHUTDOWN_START">EINVALID_SHUTDOWN_START</a>)
    );
    aux_data.module_status = ScheduledTxnsModuleStatus::ShutdownInProgress;

    // we don't <a href="scheduled_txns.md#0x1_scheduled_txns_process_shutdown_batch">process_shutdown_batch</a>() immediately here <b>to</b> avoid race conditions <b>with</b> the scheduled transactions
    // that are being run in the same <a href="block.md#0x1_block">block</a>
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_continue_shutdown"></a>

## Function `continue_shutdown`

Continues shutdown process. Can only be called when module status is ShutdownInProgress.


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_continue_shutdown">continue_shutdown</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, cancel_batch_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_continue_shutdown">continue_shutdown</a>(
    framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, cancel_batch_size: u64
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <a href="scheduled_txns.md#0x1_scheduled_txns_process_shutdown_batch">process_shutdown_batch</a>(cancel_batch_size);
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_re_initialize"></a>

## Function `re_initialize`

Re-initialize the module after the shutdown is complete
We need a governance proposal to re-initialize the module.


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_re_initialize">re_initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_re_initialize">re_initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>let</b> aux_data = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        (aux_data.module_status == ScheduledTxnsModuleStatus::ShutdownComplete),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_ESHUTDOWN_IN_PROGRESS">ESHUTDOWN_IN_PROGRESS</a>)
    );
    aux_data.module_status = ScheduledTxnsModuleStatus::Active;
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_process_shutdown_batch"></a>

## Function `process_shutdown_batch`

Stop, remove and refund all scheduled txns


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_process_shutdown_batch">process_shutdown_batch</a>(cancel_batch_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_process_shutdown_batch">process_shutdown_batch</a>(
    cancel_batch_size: u64
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <b>let</b> aux_data = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        (aux_data.module_status == ScheduledTxnsModuleStatus::ShutdownInProgress),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SHUTDOWN_ATTEMPT">EINVALID_SHUTDOWN_ATTEMPT</a>)
    );

    <b>let</b> txns_to_cancel = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a>&gt;();
    // Make a list of txns <b>to</b> cancel <b>with</b> their keys and signers
    {
        <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);

        // Iterate through schedule_map <b>to</b> get all transactions
        <b>let</b> iter = queue.schedule_map.new_begin_iter();
        <b>let</b> cancel_count = 0;
        <b>while</b> ((!iter.iter_is_end(&queue.schedule_map))
            && (cancel_count &lt; cancel_batch_size)) {
            <b>let</b> key = iter.iter_borrow_key();
            <b>if</b> (!queue.txn_table.contains(key.txn_id)) {
                // the scheduled txn is run in the same <a href="block.md#0x1_block">block</a>, but before this 'shutdown txn'
                <b>continue</b>;
            };
            <b>let</b> txn = queue.txn_table.borrow(key.txn_id);
            <b>let</b> deposit_amt = txn.max_gas_amount * txn.gas_unit_price;
            txns_to_cancel.push_back(
                <a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> { key: *key, account_addr: txn.sender_addr, deposit_amt }
            );
            cancel_count = cancel_count + 1;
            iter = iter.iter_next(&queue.schedule_map);
        };
    };

    // Cancel transactions
    <b>while</b> (!txns_to_cancel.is_empty()) {
        <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> { key, account_addr, deposit_amt } =
            txns_to_cancel.pop_back();
        <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(account_addr, key, deposit_amt);
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionFailedEvent">TransactionFailedEvent</a> {
                scheduled_txn_time: key.time,
                scheduled_txn_hash: key.txn_id,
                sender_addr: account_addr,
                cancelled_txn_code: CancelledTxnCode::Shutdown
            }
        );
    };

    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>if</b> (queue.schedule_map.is_empty()) {
        <a href="scheduled_txns.md#0x1_scheduled_txns_complete_shutdown">complete_shutdown</a>();
    };
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_complete_shutdown"></a>

## Function `complete_shutdown`



<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_complete_shutdown">complete_shutdown</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_complete_shutdown">complete_shutdown</a>() <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> {
    <b>let</b> aux_data = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        (aux_data.module_status == ScheduledTxnsModuleStatus::ShutdownInProgress),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SHUTDOWN_ATTEMPT">EINVALID_SHUTDOWN_ATTEMPT</a>)
    );
    aux_data.module_status = ScheduledTxnsModuleStatus::ShutdownComplete;

    // Clean up <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>
    <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> { remove_tbl } = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>&gt;(@aptos_framework);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>) {
        <b>if</b> (remove_tbl.contains((i <b>as</b> u16))) {
            remove_tbl.remove((i <b>as</b> u16));
        };
        i = i + 1;
    };

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_ShutdownEvent">ShutdownEvent</a> { complete: <b>true</b> });
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_pause_scheduled_txns"></a>

## Function `pause_scheduled_txns`

Pause the scheduled transactions module
Internally called by the system if any system level invariant of scheduled txns is violated.
Next steps is to have a governance proposal to:
(a) unpause the module or
(b) start the shutdown process


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_pause_scheduled_txns">pause_scheduled_txns</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_pause_scheduled_txns">pause_scheduled_txns</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>let</b> aux_data = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_PAUSE_ATTEMPT">EINVALID_PAUSE_ATTEMPT</a>)
    );
    aux_data.module_status = ScheduledTxnsModuleStatus::Paused;
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_unpause_scheduled_txns"></a>

## Function `unpause_scheduled_txns`

Unpause the scheduled transactions module.
This can be called by a governace proposal. It is advised that this be called only after ensuring that the
system invariants won't be violated again.


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_unpause_scheduled_txns">unpause_scheduled_txns</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_unpause_scheduled_txns">unpause_scheduled_txns</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>let</b> aux_data = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        (aux_data.module_status == ScheduledTxnsModuleStatus::Paused),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_UNPAUSE_ATTEMPT">EINVALID_UNPAUSE_ATTEMPT</a>)
    );
    aux_data.module_status = ScheduledTxnsModuleStatus::Active;
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_set_expiry_delta"></a>

## Function `set_expiry_delta`

Change the expiry delta for scheduled transactions; can be called only by the framework


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_set_expiry_delta">set_expiry_delta</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_expiry_delta: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_set_expiry_delta">set_expiry_delta</a>(
    framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_expiry_delta: u64
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>let</b> aux_data = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework));
    aux_data.expiry_delta = new_expiry_delta;
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_new_scheduled_transaction"></a>

## Function `new_scheduled_transaction`

Constructor


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_new_scheduled_transaction">new_scheduled_transaction</a>(sender_addr: <b>address</b>, scheduled_time_ms: u64, max_gas_amount: u64, gas_unit_price: u64, pass_signer: bool, f: |<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;| <b>has</b> <b>copy</b> + drop + store): <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_new_scheduled_transaction">new_scheduled_transaction</a>(
    sender_addr: <b>address</b>,
    scheduled_time_ms: u64,
    max_gas_amount: u64,
    gas_unit_price: u64,
    pass_signer: bool,
    f: |Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;| <b>has</b> <b>copy</b> + store + drop
): <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a> {
    <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a> {
        sender_addr,
        scheduled_time_ms,
        max_gas_amount,
        gas_unit_price,
        pass_signer,
        f: ScheduledFunction::V1(f)
    }
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_insert"></a>

## Function `insert`

Insert a scheduled transaction into the queue. ScheduleMapKey is returned to user, which can be used to cancel the txn.


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_insert">insert</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">scheduled_txns::ScheduledTransaction</a>): <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_insert">insert</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransaction">ScheduledTransaction</a>
): <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    // If scheduling is shutdown, we cannot schedule <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> more transactions
    <b>let</b> aux_data = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unavailable">error::unavailable</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EUNAVAILABLE">EUNAVAILABLE</a>)
    );

    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender) == txn.sender_addr,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SIGNER">EINVALID_SIGNER</a>)
    );

    // Only schedule txns in the future
    <b>let</b> txn_time = txn.scheduled_time_ms;
    <b>let</b> block_time_ms = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() / 1000;
    <b>assert</b>!(txn_time &gt; block_time_ms, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_TIME">EINVALID_TIME</a>));

    <b>assert</b>!(
        txn.gas_unit_price &gt;= <a href="scheduled_txns.md#0x1_scheduled_txns_MIN_GAS_UNIT_PRICE">MIN_GAS_UNIT_PRICE</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_ELOW_GAS_UNIT_PRICE">ELOW_GAS_UNIT_PRICE</a>)
    );

    <b>assert</b>!(
        txn.max_gas_amount &gt;= <a href="scheduled_txns.md#0x1_scheduled_txns_MIN_GAS_AMOUNT">MIN_GAS_AMOUNT</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_ETOO_LOW_GAS_AMOUNT">ETOO_LOW_GAS_AMOUNT</a>)
    );

    <b>let</b> txn_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&txn);
    <b>assert</b>!(
        txn_bytes.length() &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_MAX_SCHED_TXN_SIZE">MAX_SCHED_TXN_SIZE</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_ETXN_TOO_LARGE">ETXN_TOO_LARGE</a>)
    );

    // Generate unique transaction ID
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = sha3_256(txn_bytes);
    <b>let</b> txn_id = <a href="scheduled_txns.md#0x1_scheduled_txns_hash_to_u256">hash_to_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>);

    // Insert the transaction into the schedule_map
    // Create schedule map key
    <b>let</b> key = <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a> {
        time: txn_time,
        gas_priority: <a href="scheduled_txns.md#0x1_scheduled_txns_U64_MAX">U64_MAX</a> - txn.gas_unit_price,
        txn_id
    };

    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    queue.schedule_map.add(key, <a href="scheduled_txns.md#0x1_scheduled_txns_Empty">Empty</a> {});
    queue.txn_table.add(key.txn_id, txn);

    // Collect deposit
    // Get owner <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>
    <b>let</b> gas_deposit_store_cap = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>let</b> gas_deposit_store_addr =
        <a href="account.md#0x1_account_get_signer_capability_address">account::get_signer_capability_address</a>(
            &gas_deposit_store_cap.gas_fee_deposit_store_signer_cap
        );

    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(
        sender,
        gas_deposit_store_addr,
        txn.max_gas_amount * txn.gas_unit_price
    );

    // Emit <a href="event.md#0x1_event">event</a> that txn <b>has</b> been scheduled; for now indexer wants <b>to</b> consume this
    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionScheduledEvent">TransactionScheduledEvent</a> {
            block_time_ms,
            scheduled_txn_hash: txn_id,
            sender_addr: txn.sender_addr,
            scheduled_time_ms: txn.scheduled_time_ms,
            max_gas_amount: txn.max_gas_amount,
            gas_unit_price: txn.gas_unit_price
        }
    );

    key
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_cancel"></a>

## Function `cancel`

Cancel a scheduled transaction, must be called by the signer who originally scheduled the transaction.


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel">cancel</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel">cancel</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    // If scheduling is shutdown, we cannot schedule <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> more transactions
    <b>let</b> aux_data = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unavailable">error::unavailable</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EUNAVAILABLE">EUNAVAILABLE</a>)
    );

    <b>let</b> curr_time_ms = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() / 1000;
    <b>assert</b>!(
        (curr_time_ms &lt; key.time) && ((key.time - curr_time_ms)
            &gt; <a href="scheduled_txns.md#0x1_scheduled_txns_CANCEL_DELTA_DEFAULT">CANCEL_DELTA_DEFAULT</a>),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_ECANCEL_TOO_LATE">ECANCEL_TOO_LATE</a>)
    );

    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>if</b> (!queue.schedule_map.contains(&key)
        || !queue.txn_table.contains(key.txn_id)) {
        // this is more of a paranoid check, we should never get here, rather throw <a href="scheduled_txns.md#0x1_scheduled_txns_ECANCEL_TOO_LATE">ECANCEL_TOO_LATE</a> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a>
        // Second check <b>if</b> for the case: the scheduled txn is run in the same <a href="block.md#0x1_block">block</a>, but before this 'cancel txn'
        <b>return</b>
    };

    <b>let</b> txn = queue.txn_table.borrow(key.txn_id);
    <b>let</b> deposit_amt = txn.max_gas_amount * txn.gas_unit_price;

    // verify sender
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender) == txn.sender_addr,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_SIGNER">EINVALID_SIGNER</a>)
    );
    <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), key, deposit_amt);
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_truncate_to_u64"></a>

## Function `truncate_to_u64`



<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_truncate_to_u64">truncate_to_u64</a>(val: u256): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_truncate_to_u64">truncate_to_u64</a>(val: u256): u64 {
    <b>let</b> masked = val & <a href="scheduled_txns.md#0x1_scheduled_txns_MASK_64">MASK_64</a>; // Truncate high bits
    (masked <b>as</b> u64) // Now safe: always &lt;= u64::MAX
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_hash_to_u256"></a>

## Function `hash_to_u256`



<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_hash_to_u256">hash_to_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_hash_to_u256">hash_to_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u256 {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>.length() == 32, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="scheduled_txns.md#0x1_scheduled_txns_EINVALID_HASH_SIZE">EINVALID_HASH_SIZE</a>));
    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u256">from_bcs::to_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>)
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_cancel_internal"></a>

## Function `cancel_internal`

Internal cancel function that takes an address instead of signer. No signer verification, assumes key is present
in the schedule_map.


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(account_addr: <b>address</b>, key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>, deposit_amt: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(
    account_addr: <b>address</b>, key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>, deposit_amt: u64
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);

    // Remove the transaction from schedule_map and txn_table
    queue.schedule_map.remove(&key);
    queue.txn_table.remove(key.txn_id);

    // Refund the deposit
    // Get owner <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> from <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>
    <b>let</b> gas_deposit_store_cap = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>let</b> gas_deposit_store_signer =
        <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
            &gas_deposit_store_cap.gas_fee_deposit_store_signer_cap
        );

    // Refund deposit from owner's store <b>to</b> sender
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(
        &gas_deposit_store_signer,
        account_addr,
        deposit_amt
    );
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_get_ready_transactions"></a>

## Function `get_ready_transactions`

Gets txns due to be run; also expire txns that could not be run for a while (mostly due to low gas priority)


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_get_ready_transactions">get_ready_transactions</a>(block_timestamp_ms: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionInfoWithKey">scheduled_txns::ScheduledTransactionInfoWithKey</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_get_ready_transactions">get_ready_transactions</a>(
    block_timestamp_ms: u64
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionInfoWithKey">ScheduledTransactionInfoWithKey</a>&gt; <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> {
    <a href="scheduled_txns.md#0x1_scheduled_txns_get_ready_transactions_with_limit">get_ready_transactions_with_limit</a>(
        block_timestamp_ms, <a href="scheduled_txns.md#0x1_scheduled_txns_GET_READY_TRANSACTIONS_LIMIT">GET_READY_TRANSACTIONS_LIMIT</a>
    )
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_get_ready_transactions_with_limit"></a>

## Function `get_ready_transactions_with_limit`



<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_get_ready_transactions_with_limit">get_ready_transactions_with_limit</a>(block_timestamp_ms: u64, limit: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionInfoWithKey">scheduled_txns::ScheduledTransactionInfoWithKey</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_get_ready_transactions_with_limit">get_ready_transactions_with_limit</a>(
    block_timestamp_ms: u64, limit: u64
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionInfoWithKey">ScheduledTransactionInfoWithKey</a>&gt; <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> {
    <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txns">remove_txns</a>(block_timestamp_ms);
    // If scheduling is shutdown, we cannot schedule <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> more transactions
    <b>let</b> aux_data = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>if</b> (aux_data.module_status != ScheduledTxnsModuleStatus::Active) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionInfoWithKey">ScheduledTransactionInfoWithKey</a>&gt;();
    };

    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionInfoWithKey">ScheduledTransactionInfoWithKey</a>&gt;();
    <b>let</b> count = 0;

    <b>let</b> iter = queue.schedule_map.new_begin_iter();
    <b>while</b> ((count &lt; limit) && !iter.iter_is_end(&queue.schedule_map)) {
        <b>let</b> key = iter.iter_borrow_key();
        <b>if</b> (key.time &gt; block_timestamp_ms) {
            <b>break</b>;
        };
        <b>let</b> txn = queue.txn_table.borrow(key.txn_id);

        <b>let</b> scheduled_txn_info_with_key =
            <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduledTransactionInfoWithKey">ScheduledTransactionInfoWithKey</a> {
                sender_addr: txn.sender_addr,
                max_gas_amount: txn.max_gas_amount,
                gas_unit_price: txn.gas_unit_price,
                key: *key
            };

        <b>if</b> ((block_timestamp_ms &gt; key.time)
            && ((block_timestamp_ms - key.time) &gt; aux_data.expiry_delta)) {
            <b>continue</b>;
        } <b>else</b> {
            <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a>.push_back(scheduled_txn_info_with_key);
        };
        // we do not want an unbounded size of ready or expirable txns; hence we increment either way
        count = count + 1;
        iter = iter.iter_next(&queue.schedule_map);
    };

    <a href="scheduled_txns.md#0x1_scheduled_txns">scheduled_txns</a>
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_mark_txn_to_remove"></a>

## Function `mark_txn_to_remove`

Increment after every scheduled transaction is run
IMP: Make sure this does not affect parallel execution of txns


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_mark_txn_to_remove">mark_txn_to_remove</a>(key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_mark_txn_to_remove">mark_txn_to_remove</a>(key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a> {
    // Calculate <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> index using <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
    <b>let</b> tbl_idx = ((<a href="scheduled_txns.md#0x1_scheduled_txns_truncate_to_u64">truncate_to_u64</a>(key.txn_id) % <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>) <b>as</b> u16);
    <b>let</b> to_remove = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>&gt;(@aptos_framework);
    <b>let</b> keys = to_remove.remove_tbl.borrow_mut(tbl_idx);
    keys.push_back(key);
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_cancel_and_remove_expired_txns"></a>

## Function `cancel_and_remove_expired_txns`



<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_and_remove_expired_txns">cancel_and_remove_expired_txns</a>(block_timestamp_ms: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_and_remove_expired_txns">cancel_and_remove_expired_txns</a>(
    block_timestamp_ms: u64
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    <b>let</b> aux_data = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a>&gt;(@aptos_framework);
    <b>let</b> queue = <b>borrow_global</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
    <b>let</b> txns_to_expire = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a>&gt;();

    // collect expired transactions
    <b>let</b> iter = queue.schedule_map.new_begin_iter();
    <b>let</b> expire_count = 0;
    <b>while</b> (!iter.iter_is_end(&queue.schedule_map)
        && expire_count &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_EXPIRE_TRANSACTIONS_LIMIT">EXPIRE_TRANSACTIONS_LIMIT</a>) {
        <b>let</b> key = iter.iter_borrow_key();
        <b>if</b> ((block_timestamp_ms &lt; key.time)
            || ((block_timestamp_ms - key.time) &lt;= aux_data.expiry_delta)) {
            <b>break</b>;
        };

        // Get transaction info before cancelling
        <b>let</b> txn = queue.txn_table.borrow(key.txn_id);
        <b>let</b> deposit_amt = txn.max_gas_amount * txn.gas_unit_price;

        txns_to_expire.push_back(
            <a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> { key: *key, account_addr: txn.sender_addr, deposit_amt }
        );
        expire_count = expire_count + 1;
        iter = iter.iter_next(&queue.schedule_map);
    };

    // cancel expired transactions
    <b>while</b> (!txns_to_expire.is_empty()) {
        <b>let</b> <a href="scheduled_txns.md#0x1_scheduled_txns_KeyAndTxnInfo">KeyAndTxnInfo</a> { key, account_addr, deposit_amt } =
            txns_to_expire.pop_back();
        <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_internal">cancel_internal</a>(account_addr, key, deposit_amt);
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionFailedEvent">TransactionFailedEvent</a> {
                scheduled_txn_time: key.time,
                scheduled_txn_hash: key.txn_id,
                sender_addr: account_addr,
                cancelled_txn_code: CancelledTxnCode::Expired
            }
        );
    };
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_remove_txns"></a>

## Function `remove_txns`

Remove the txns that are run


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txns">remove_txns</a>(block_timestamp_ms: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txns">remove_txns</a>(
    block_timestamp_ms: u64
) <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>, <a href="scheduled_txns.md#0x1_scheduled_txns_AuxiliaryData">AuxiliaryData</a> {
    {
        <b>let</b> to_remove = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ToRemoveTbl">ToRemoveTbl</a>&gt;(@aptos_framework);
        <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);
        <b>let</b> tbl_idx: u16 = 0;

        <b>while</b> ((tbl_idx <b>as</b> u64) &lt; <a href="scheduled_txns.md#0x1_scheduled_txns_TO_REMOVE_PARALLELISM">TO_REMOVE_PARALLELISM</a>) {
            <b>if</b> (to_remove.remove_tbl.contains(tbl_idx)) {
                <b>let</b> keys = to_remove.remove_tbl.borrow_mut(tbl_idx);

                <b>while</b> (!keys.is_empty()) {
                    <b>let</b> key = keys.pop_back();
                    <b>if</b> (queue.schedule_map.contains(&key)) {
                        // Remove transaction from schedule_map and txn_table
                        <b>if</b> (queue.txn_table.contains(key.txn_id)) {
                            queue.txn_table.remove(key.txn_id);
                        };
                        queue.schedule_map.remove(&key);
                    };
                };
            };
            tbl_idx = tbl_idx + 1;
        };
    };
    <a href="scheduled_txns.md#0x1_scheduled_txns_cancel_and_remove_expired_txns">cancel_and_remove_expired_txns</a>(block_timestamp_ms);
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_execute_user_function_wrapper"></a>

## Function `execute_user_function_wrapper`

Called by the executor when the scheduled transaction is run


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_execute_user_function_wrapper">execute_user_function_wrapper</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_execute_user_function_wrapper">execute_user_function_wrapper</a>(
    <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>
): bool <b>acquires</b> <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a> {
    <b>let</b> queue = <b>borrow_global_mut</b>&lt;<a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleQueue">ScheduleQueue</a>&gt;(@aptos_framework);

    <b>if</b> (!queue.schedule_map.contains(&txn_key)) {
        // It is possible that the scheduled transaction was cancelled before in the same <a href="block.md#0x1_block">block</a>
        <b>return</b> <b>false</b>;
    };
    <b>let</b> txn = queue.txn_table.borrow(txn_key.txn_id);
    <b>let</b> pass_signer = txn.pass_signer;

    match(txn.f) {
        ScheduledFunction::V1(f) =&gt; {
            <b>if</b> (pass_signer) {
                f(some(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>));
            } <b>else</b> {
                f(std::option::none());
            };
        }
    };

    // The scheduled transaction is removed from two data structures at different times:
    // 1. From schedule_map (BigOrderedMap): Removed in next <a href="block.md#0x1_block">block</a>'s prologue <b>to</b> allow parallel execution
    //    of all scheduled transactions in the current <a href="block.md#0x1_block">block</a>
    // 2. From txn_table: Removed immediately after transaction execution in this function <b>to</b> enable
    //    proper refunding of storage gas fees <b>to</b> the user
    queue.txn_table.remove(txn_key.txn_id);
    <b>true</b>
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_emit_transaction_failed_event"></a>

## Function `emit_transaction_failed_event`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_emit_transaction_failed_event">emit_transaction_failed_event</a>(key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>, sender_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_emit_transaction_failed_event">emit_transaction_failed_event</a>(
    key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">ScheduleMapKey</a>, sender_addr: <b>address</b>
) {
    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="scheduled_txns.md#0x1_scheduled_txns_TransactionFailedEvent">TransactionFailedEvent</a> {
            scheduled_txn_time: key.time,
            scheduled_txn_hash: key.txn_id,
            sender_addr,
            cancelled_txn_code: CancelledTxnCode::Failed
        }
    );
}
</code></pre>



</details>

<a id="0x1_scheduled_txns_step"></a>

## Function `step`



<pre><code>#[persistent]
<b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_step">step</a>(state: <a href="scheduled_txns.md#0x1_scheduled_txns_State">scheduled_txns::State</a>, _s: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="scheduled_txns.md#0x1_scheduled_txns_step">step</a>(state: <a href="scheduled_txns.md#0x1_scheduled_txns_State">State</a>, _s: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;) {
    <b>if</b> (state.count &lt; 10) {
        state.count = state.count + 1;
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
