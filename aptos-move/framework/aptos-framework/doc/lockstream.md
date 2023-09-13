
<a name="0x1_lockstream"></a>

# Module `0x1::lockstream`



-  [Resource `LockstreamPool`](#0x1_lockstream_LockstreamPool)
-  [Struct `LockerInfo`](#0x1_lockstream_LockerInfo)
-  [Struct `LockstreamPoolID`](#0x1_lockstream_LockstreamPoolID)
-  [Struct `LockstreamCreationEvent`](#0x1_lockstream_LockstreamCreationEvent)
-  [Struct `LockstreamLockEvent`](#0x1_lockstream_LockstreamLockEvent)
-  [Struct `LockstreamNewPremierLockerEvent`](#0x1_lockstream_LockstreamNewPremierLockerEvent)
-  [Struct `LockstreamClaimEvent`](#0x1_lockstream_LockstreamClaimEvent)
-  [Struct `LockstreamSweepEvent`](#0x1_lockstream_LockstreamSweepEvent)
-  [Resource `LockstreamLockerEventHandles`](#0x1_lockstream_LockstreamLockerEventHandles)
-  [Struct `LockstreamPoolMetadataView`](#0x1_lockstream_LockstreamPoolMetadataView)
-  [Struct `LockerInfoView`](#0x1_lockstream_LockerInfoView)
-  [Constants](#@Constants_0)
-  [Function `create`](#0x1_lockstream_create)
-  [Function `lock`](#0x1_lockstream_lock)
-  [Function `claim`](#0x1_lockstream_claim)
-  [Function `sweep`](#0x1_lockstream_sweep)
-  [Function `current_period`](#0x1_lockstream_current_period)
-  [Function `locker`](#0x1_lockstream_locker)
-  [Function `lockers`](#0x1_lockstream_lockers)
-  [Function `lockers_paginated`](#0x1_lockstream_lockers_paginated)
-  [Function `metadata`](#0x1_lockstream_metadata)
-  [Function `pool_id`](#0x1_lockstream_pool_id)
-  [Function `locker_amounts_derived`](#0x1_lockstream_locker_amounts_derived)
-  [Function `period`](#0x1_lockstream_period)
-  [Function `pool_id_and_immutable_reference`](#0x1_lockstream_pool_id_and_immutable_reference)
-  [Function `pool_id_and_mutable_reference`](#0x1_lockstream_pool_id_and_mutable_reference)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/big_vector.md#0x1_big_vector">0x1::big_vector</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_lockstream_LockstreamPool"></a>

## Resource `LockstreamPool`

All times in UNIX seconds.


<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>base_locked: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;BaseType&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>quote_locked: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;QuoteType&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>locker_addresses: <a href="../../aptos-stdlib/doc/big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>lockers: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<b>address</b>, <a href="lockstream.md#0x1_lockstream_LockerInfo">lockstream::LockerInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>initial_base_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>initial_quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>premier_locker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>premier_locker_initial_quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>creation_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>stream_start_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>stream_end_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_last_call_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>premier_sweep_last_call_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>creation_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamCreationEvent">lockstream::LockstreamCreationEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>lock_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamLockEvent">lockstream::LockstreamLockEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_premier_locker_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamNewPremierLockerEvent">lockstream::LockstreamNewPremierLockerEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamClaimEvent">lockstream::LockstreamClaimEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sweep_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamSweepEvent">lockstream::LockstreamSweepEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockerInfo"></a>

## Struct `LockerInfo`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockerInfo">LockerInfo</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>initial_quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>base_claimed: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>quote_claimed: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockstreamPoolID"></a>

## Struct `LockstreamPoolID`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">LockstreamPoolID</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>base_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
<dt>
<code>quote_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockstreamCreationEvent"></a>

## Struct `LockstreamCreationEvent`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamCreationEvent">LockstreamCreationEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_id: <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>initial_base_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>creation_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>stream_start_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>stream_end_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_last_call_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>premier_sweep_last_call_time: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockstreamLockEvent"></a>

## Struct `LockstreamLockEvent`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamLockEvent">LockstreamLockEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_id: <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>lock_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>quote_lock_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_quote_locked_for_locker: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_quote_locked_for_pool: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockstreamNewPremierLockerEvent"></a>

## Struct `LockstreamNewPremierLockerEvent`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamNewPremierLockerEvent">LockstreamNewPremierLockerEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_id: <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>lock_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_premier_locker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_premier_locker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_premier_locker_total_quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>old_premier_locker_total_quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_quote_locked_for_pool: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockstreamClaimEvent"></a>

## Struct `LockstreamClaimEvent`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamClaimEvent">LockstreamClaimEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_id: <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>claim_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>base_claimed: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>quote_claimed: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_base_claimed_for_locker: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_quote_claimed_for_locker: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockstreamSweepEvent"></a>

## Struct `LockstreamSweepEvent`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamSweepEvent">LockstreamSweepEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_id: <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>sweep_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>base_sweep_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>quote_sweep_amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockstreamLockerEventHandles"></a>

## Resource `LockstreamLockerEventHandles`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>lock_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamLockEvent">lockstream::LockstreamLockEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_premier_locker_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamNewPremierLockerEvent">lockstream::LockstreamNewPremierLockerEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamClaimEvent">lockstream::LockstreamClaimEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sweep_event_handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamSweepEvent">lockstream::LockstreamSweepEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockstreamPoolMetadataView"></a>

## Struct `LockstreamPoolMetadataView`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockstreamPoolMetadataView">LockstreamPoolMetadataView</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_id: <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>base_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>n_lockers: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>initial_base_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>initial_quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>premier_locker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>premier_locker_initial_quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>creation_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>stream_start_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>stream_end_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_last_call_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>premier_sweep_last_call_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>current_period: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_lockstream_LockerInfoView"></a>

## Struct `LockerInfoView`



<pre><code><b>struct</b> <a href="lockstream.md#0x1_lockstream_LockerInfoView">LockerInfoView</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_id: <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>locker: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pro_rata_base_share: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>initial_quote_locked: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>base_claimed: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>quote_claimed: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>claimable_base: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>claimable_quote: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_lockstream_E_LOCKSTREAM_POOL_EXISTS"></a>

Lockstream pool for base tye, quote type, and creator exists.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_LOCKSTREAM_POOL_EXISTS">E_LOCKSTREAM_POOL_EXISTS</a>: u64 = 3;
</code></pre>



<a name="0x1_lockstream_E_NOTHING_TO_SWEEP"></a>

No coins in lockstream pool left to sweep.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_NOTHING_TO_SWEEP">E_NOTHING_TO_SWEEP</a>: u64 = 12;
</code></pre>



<a name="0x1_lockstream_E_NOT_A_LOCKER"></a>

Signer is not a locker in the lockstream.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_NOT_A_LOCKER">E_NOT_A_LOCKER</a>: u64 = 6;
</code></pre>



<a name="0x1_lockstream_E_NO_LOCKSTREAM_POOL"></a>

No lockstream pool for base type, quote type, and creator.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_NO_LOCKSTREAM_POOL">E_NO_LOCKSTREAM_POOL</a>: u64 = 2;
</code></pre>



<a name="0x1_lockstream_E_NO_QUOTE_LOCK_AMOUNT"></a>

No quote lock amount specified.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_NO_QUOTE_LOCK_AMOUNT">E_NO_QUOTE_LOCK_AMOUNT</a>: u64 = 5;
</code></pre>



<a name="0x1_lockstream_E_QUOTE_NOT_COIN"></a>

Quote type provided by creator is not a coin type.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_QUOTE_NOT_COIN">E_QUOTE_NOT_COIN</a>: u64 = 1;
</code></pre>



<a name="0x1_lockstream_E_TIME_WINDOWS_INVALID"></a>

Time window bounds provided by creator are invalid.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_TIME_WINDOWS_INVALID">E_TIME_WINDOWS_INVALID</a>: u64 = 0;
</code></pre>



<a name="0x1_lockstream_E_TOO_EARLY_FOR_MERCENARY_SWEEP"></a>

Too early for mercenary locker to sweep lockstream pool.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_TOO_EARLY_FOR_MERCENARY_SWEEP">E_TOO_EARLY_FOR_MERCENARY_SWEEP</a>: u64 = 11;
</code></pre>



<a name="0x1_lockstream_E_TOO_EARLY_FOR_PREMIER_SWEEP"></a>

Too early for premier locker to sweep lockstream pool.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_TOO_EARLY_FOR_PREMIER_SWEEP">E_TOO_EARLY_FOR_PREMIER_SWEEP</a>: u64 = 9;
</code></pre>



<a name="0x1_lockstream_E_TOO_EARLY_TO_CLAIM"></a>

Too early to claim from lockstream.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_TOO_EARLY_TO_CLAIM">E_TOO_EARLY_TO_CLAIM</a>: u64 = 7;
</code></pre>



<a name="0x1_lockstream_E_TOO_LATE_FOR_PREMIER_SWEEP"></a>

Too late for premier locker to sweep lockstream pool.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_FOR_PREMIER_SWEEP">E_TOO_LATE_FOR_PREMIER_SWEEP</a>: u64 = 10;
</code></pre>



<a name="0x1_lockstream_E_TOO_LATE_TO_CLAIM"></a>

Too late to claim from lockstream.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_TO_CLAIM">E_TOO_LATE_TO_CLAIM</a>: u64 = 8;
</code></pre>



<a name="0x1_lockstream_E_TOO_LATE_TO_LOCK"></a>

Too late to lock more quote into lockstream pool.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_TO_LOCK">E_TOO_LATE_TO_LOCK</a>: u64 = 4;
</code></pre>



<a name="0x1_lockstream_FREE_WRITE_BYTES_QUOTA"></a>

Free number of bytes for a global storage write.


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_FREE_WRITE_BYTES_QUOTA">FREE_WRITE_BYTES_QUOTA</a>: u64 = 1024;
</code></pre>



<a name="0x1_lockstream_MIN_BYTES_BCS_SEQUENCE_LENGTH"></a>

Minimum number of bytes required to encode the number of
elements in a vector (for a vector with less than 128 elements).


<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_MIN_BYTES_BCS_SEQUENCE_LENGTH">MIN_BYTES_BCS_SEQUENCE_LENGTH</a>: u64 = 1;
</code></pre>



<a name="0x1_lockstream_PERIOD_CLAIMING_GRACE_PERIOD"></a>



<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_PERIOD_CLAIMING_GRACE_PERIOD">PERIOD_CLAIMING_GRACE_PERIOD</a>: u8 = 3;
</code></pre>



<a name="0x1_lockstream_PERIOD_LOCKING"></a>



<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_PERIOD_LOCKING">PERIOD_LOCKING</a>: u8 = 1;
</code></pre>



<a name="0x1_lockstream_PERIOD_MERCENARY_SWEEP"></a>



<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_PERIOD_MERCENARY_SWEEP">PERIOD_MERCENARY_SWEEP</a>: u8 = 5;
</code></pre>



<a name="0x1_lockstream_PERIOD_PREMIER_SWEEP"></a>



<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_PERIOD_PREMIER_SWEEP">PERIOD_PREMIER_SWEEP</a>: u8 = 4;
</code></pre>



<a name="0x1_lockstream_PERIOD_STREAMING"></a>



<pre><code><b>const</b> <a href="lockstream.md#0x1_lockstream_PERIOD_STREAMING">PERIOD_STREAMING</a>: u8 = 2;
</code></pre>



<a name="0x1_lockstream_create"></a>

## Function `create`

All times in UNIX seconds.


<pre><code><b>public</b> entry <b>fun</b> <a href="lockstream.md#0x1_lockstream_create">create</a>&lt;BaseType, QuoteType&gt;(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_base_locked: u64, stream_start_time: u64, stream_end_time: u64, claim_last_call_time: u64, premier_sweep_last_call_time: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="lockstream.md#0x1_lockstream_create">create</a>&lt;
    BaseType,
    QuoteType
&gt;(
    creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    initial_base_locked: u64,
    stream_start_time: u64,
    stream_end_time: u64,
    claim_last_call_time: u64,
    premier_sweep_last_call_time: u64,
) {
    <b>let</b> creator_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator_addr),
        <a href="lockstream.md#0x1_lockstream_E_LOCKSTREAM_POOL_EXISTS">E_LOCKSTREAM_POOL_EXISTS</a>
    );
    <b>let</b> creation_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>assert</b>!(
        creation_time        &lt; stream_start_time &&
        stream_start_time    &lt; stream_end_time &&
        stream_end_time      &lt; claim_last_call_time &&
        claim_last_call_time &lt; premier_sweep_last_call_time,
        <a href="lockstream.md#0x1_lockstream_E_TIME_WINDOWS_INVALID">E_TIME_WINDOWS_INVALID</a>
    );
    <b>assert</b>!(<a href="coin.md#0x1_coin_is_coin_initialized">coin::is_coin_initialized</a>&lt;QuoteType&gt;(), <a href="lockstream.md#0x1_lockstream_E_QUOTE_NOT_COIN">E_QUOTE_NOT_COIN</a>);
    <b>let</b> creation_event_handle = <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(creator);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> creation_event_handle, <a href="lockstream.md#0x1_lockstream_LockstreamCreationEvent">LockstreamCreationEvent</a> {
        pool_id: <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">LockstreamPoolID</a> {
            creator: creator_addr,
            base_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;BaseType&gt;(),
            quote_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;QuoteType&gt;(),
        },
        initial_base_locked,
        creation_time,
        stream_start_time,
        stream_end_time,
        claim_last_call_time,
        premier_sweep_last_call_time,
    });
    <b>let</b> big_vector_bucket_size =
        (<a href="lockstream.md#0x1_lockstream_FREE_WRITE_BYTES_QUOTA">FREE_WRITE_BYTES_QUOTA</a> - <a href="lockstream.md#0x1_lockstream_MIN_BYTES_BCS_SEQUENCE_LENGTH">MIN_BYTES_BCS_SEQUENCE_LENGTH</a>) /
        <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_size_of_val">type_info::size_of_val</a>(&@0x0);
    <b>move_to</b>(creator, <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt; {
        base_locked: <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>(creator, initial_base_locked),
        quote_locked: <a href="coin.md#0x1_coin_zero">coin::zero</a>(),
        locker_addresses: <a href="../../aptos-stdlib/doc/big_vector.md#0x1_big_vector_empty">big_vector::empty</a>(big_vector_bucket_size),
        lockers: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        initial_base_locked,
        initial_quote_locked: 0,
        premier_locker: @0x0,
        premier_locker_initial_quote_locked: 0,
        creation_time,
        stream_start_time,
        stream_end_time,
        claim_last_call_time,
        premier_sweep_last_call_time,
        creation_event_handle,
        lock_event_handle: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(creator),
        new_premier_locker_event_handle:
            <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(creator),
        claim_event_handle: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(creator),
        sweep_event_handle: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(creator),
    });
}
</code></pre>



</details>

<a name="0x1_lockstream_lock"></a>

## Function `lock`



<pre><code><b>public</b> entry <b>fun</b> <a href="lockstream.md#0x1_lockstream_lock">lock</a>&lt;BaseType, QuoteType&gt;(locker: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creator: <b>address</b>, quote_lock_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="lockstream.md#0x1_lockstream_lock">lock</a>&lt;
    BaseType,
    QuoteType
&gt;(
    locker: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    creator: <b>address</b>,
    quote_lock_amount: u64,
) <b>acquires</b>
    <a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a>,
    <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>
{
    <b>let</b> (pool_id, pool_ref_mut) =
        <a href="lockstream.md#0x1_lockstream_pool_id_and_mutable_reference">pool_id_and_mutable_reference</a>&lt;BaseType, QuoteType&gt;(creator);
    <b>assert</b>!(quote_lock_amount &gt; 0, <a href="lockstream.md#0x1_lockstream_E_NO_QUOTE_LOCK_AMOUNT">E_NO_QUOTE_LOCK_AMOUNT</a>);
    <b>let</b> lock_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> period = <a href="lockstream.md#0x1_lockstream_period">period</a>(pool_ref_mut, lock_time);
    <b>assert</b>!(period == <a href="lockstream.md#0x1_lockstream_PERIOD_LOCKING">PERIOD_LOCKING</a>, <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_TO_LOCK">E_TOO_LATE_TO_LOCK</a>);
    <a href="coin.md#0x1_coin_merge">coin::merge</a>(
        &<b>mut</b> pool_ref_mut.quote_locked,
        <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>(locker, quote_lock_amount)
    );
    <b>let</b> total_quote_locked_for_pool =
        <a href="coin.md#0x1_coin_value">coin::value</a>(&pool_ref_mut.quote_locked);
    <b>let</b> lockers_ref_mut = &<b>mut</b> pool_ref_mut.lockers;
    <b>let</b> locker_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(locker);
    <b>let</b> locking_more = <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(lockers_ref_mut, locker_addr);
    <b>let</b> total_quote_locked_for_locker = <b>if</b> (locking_more) {
        <b>let</b> locker_info_ref_mut =
            <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(lockers_ref_mut, locker_addr);
        <b>let</b> already_locked = locker_info_ref_mut.initial_quote_locked;
        <b>let</b> total_locked = already_locked + quote_lock_amount;
        locker_info_ref_mut.initial_quote_locked = total_locked;
        total_locked
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(lockers_ref_mut, locker_addr, <a href="lockstream.md#0x1_lockstream_LockerInfo">LockerInfo</a> {
            initial_quote_locked: quote_lock_amount,
            base_claimed: 0,
            quote_claimed: 0,
        });
        <a href="../../aptos-stdlib/doc/big_vector.md#0x1_big_vector_push_back">big_vector::push_back</a>(
            &<b>mut</b> pool_ref_mut.locker_addresses,
            locker_addr
        );
        quote_lock_amount
    };
    <b>let</b> lock_event = <a href="lockstream.md#0x1_lockstream_LockstreamLockEvent">LockstreamLockEvent</a> {
        pool_id,
        lock_time,
        locker: locker_addr,
        quote_lock_amount,
        total_quote_locked_for_locker,
        total_quote_locked_for_pool
    };
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> pool_ref_mut.lock_event_handle, lock_event);
    <b>if</b> (!<b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a>&gt;(locker_addr))
        <b>move_to</b>(locker, <a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a> {
            lock_event_handle: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(locker),
            new_premier_locker_event_handle:
                <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(locker),
            claim_event_handle: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(locker),
            sweep_event_handle: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>(locker),
        });
    <b>let</b> locker_handles_ref_mut =
        <b>borrow_global_mut</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a>&gt;(locker_addr);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> locker_handles_ref_mut.lock_event_handle,
        lock_event
    );
    <b>let</b> new_premier_locker =
        total_quote_locked_for_locker &gt;
        pool_ref_mut.premier_locker_initial_quote_locked;
    <b>if</b> (new_premier_locker) {
        <b>let</b> premier_locker_event = <a href="lockstream.md#0x1_lockstream_LockstreamNewPremierLockerEvent">LockstreamNewPremierLockerEvent</a> {
            pool_id,
            lock_time,
            new_premier_locker: locker_addr,
            old_premier_locker: pool_ref_mut.premier_locker,
            new_premier_locker_total_quote_locked:
                total_quote_locked_for_locker,
            old_premier_locker_total_quote_locked:
                pool_ref_mut.premier_locker_initial_quote_locked,
            total_quote_locked_for_pool,
        };
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> pool_ref_mut.new_premier_locker_event_handle,
            premier_locker_event
        );
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> locker_handles_ref_mut.new_premier_locker_event_handle,
            premier_locker_event
        );
        pool_ref_mut.premier_locker = locker_addr;
        pool_ref_mut.premier_locker_initial_quote_locked =
            total_quote_locked_for_locker;
    }
}
</code></pre>



</details>

<a name="0x1_lockstream_claim"></a>

## Function `claim`



<pre><code><b>public</b> entry <b>fun</b> <a href="lockstream.md#0x1_lockstream_claim">claim</a>&lt;BaseType, QuoteType&gt;(locker: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="lockstream.md#0x1_lockstream_claim">claim</a>&lt;
    BaseType,
    QuoteType
&gt;(
    locker: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    creator: <b>address</b>,
) <b>acquires</b>
    <a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a>,
    <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>
{
    <b>let</b> (pool_id, pool_ref_mut) =
        <a href="lockstream.md#0x1_lockstream_pool_id_and_mutable_reference">pool_id_and_mutable_reference</a>&lt;BaseType, QuoteType&gt;(creator);
    <b>let</b> claim_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> period = <a href="lockstream.md#0x1_lockstream_period">period</a>(pool_ref_mut, claim_time);
    <b>assert</b>!(!(<a href="lockstream.md#0x1_lockstream_period">period</a> &lt; <a href="lockstream.md#0x1_lockstream_PERIOD_STREAMING">PERIOD_STREAMING</a>), <a href="lockstream.md#0x1_lockstream_E_TOO_EARLY_TO_CLAIM">E_TOO_EARLY_TO_CLAIM</a>);
    <b>assert</b>!(!(period &gt; <a href="lockstream.md#0x1_lockstream_PERIOD_CLAIMING_GRACE_PERIOD">PERIOD_CLAIMING_GRACE_PERIOD</a>), <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_TO_CLAIM">E_TOO_LATE_TO_CLAIM</a>);
    <b>let</b> locker_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(locker);
    <b>let</b> (_, base_claimed, quote_claimed) = <a href="lockstream.md#0x1_lockstream_locker_amounts_derived">locker_amounts_derived</a>(
        pool_ref_mut,
        locker_addr,
        claim_time
    );
    <b>let</b> locker_info_ref_mut =
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> pool_ref_mut.lockers, locker_addr);
    <b>if</b> (base_claimed &gt; 0) {
        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;BaseType&gt;(locker);
        <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(
            locker_addr,
            <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> pool_ref_mut.base_locked, base_claimed)
        );
    };
    <b>if</b> (quote_claimed &gt; 0) {
        <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(
            locker_addr,
            <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> pool_ref_mut.quote_locked, quote_claimed)
        );
    };
    <b>if</b> (base_claimed &gt; 0 || quote_claimed &gt; 0) {
        <b>let</b> total_base_claimed =
            base_claimed + locker_info_ref_mut.base_claimed;
        <b>let</b> total_quote_claimed =
            quote_claimed + locker_info_ref_mut.quote_claimed;
        <b>let</b> claim_event = <a href="lockstream.md#0x1_lockstream_LockstreamClaimEvent">LockstreamClaimEvent</a> {
            pool_id,
            claim_time,
            locker: locker_addr,
            base_claimed,
            quote_claimed,
            total_base_claimed_for_locker: total_base_claimed,
            total_quote_claimed_for_locker: total_quote_claimed,
        };
        locker_info_ref_mut.base_claimed = total_base_claimed;
        locker_info_ref_mut.quote_claimed = total_quote_claimed;
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> pool_ref_mut.claim_event_handle,
            claim_event
        );
        <b>let</b> locker_handles_ref_mut =
            <b>borrow_global_mut</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a>&gt;(locker_addr);
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> locker_handles_ref_mut.claim_event_handle,
            claim_event
        );
    }
}
</code></pre>



</details>

<a name="0x1_lockstream_sweep"></a>

## Function `sweep`



<pre><code><b>public</b> entry <b>fun</b> <a href="lockstream.md#0x1_lockstream_sweep">sweep</a>&lt;BaseType, QuoteType&gt;(locker: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="lockstream.md#0x1_lockstream_sweep">sweep</a>&lt;
    BaseType,
    QuoteType
&gt;(
    locker: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    creator: <b>address</b>,
) <b>acquires</b>
    <a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a>,
    <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>
{
    <b>let</b> (pool_id, pool_ref_mut) =
        <a href="lockstream.md#0x1_lockstream_pool_id_and_mutable_reference">pool_id_and_mutable_reference</a>&lt;BaseType, QuoteType&gt;(creator);
    <b>let</b> lockers_ref_mut = &<b>mut</b> pool_ref_mut.lockers;
    <b>let</b> locker_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(locker);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(lockers_ref_mut, locker_addr),
        <a href="lockstream.md#0x1_lockstream_E_NOT_A_LOCKER">E_NOT_A_LOCKER</a>
    );
    <b>let</b> sweep_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> period = <a href="lockstream.md#0x1_lockstream_period">period</a>(pool_ref_mut, sweep_time);
    <b>if</b> (locker_addr == pool_ref_mut.premier_locker) {
        <b>assert</b>!(
            !(<a href="lockstream.md#0x1_lockstream_period">period</a> &lt; <a href="lockstream.md#0x1_lockstream_PERIOD_PREMIER_SWEEP">PERIOD_PREMIER_SWEEP</a>),
            <a href="lockstream.md#0x1_lockstream_E_TOO_EARLY_FOR_PREMIER_SWEEP">E_TOO_EARLY_FOR_PREMIER_SWEEP</a>
        );
        <b>assert</b>!(
            !(period &gt; <a href="lockstream.md#0x1_lockstream_PERIOD_MERCENARY_SWEEP">PERIOD_MERCENARY_SWEEP</a>),
            <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_FOR_PREMIER_SWEEP">E_TOO_LATE_FOR_PREMIER_SWEEP</a>
        );
    } <b>else</b> {
        <b>assert</b>!(
            period == <a href="lockstream.md#0x1_lockstream_PERIOD_MERCENARY_SWEEP">PERIOD_MERCENARY_SWEEP</a>,
            <a href="lockstream.md#0x1_lockstream_E_TOO_EARLY_FOR_MERCENARY_SWEEP">E_TOO_EARLY_FOR_MERCENARY_SWEEP</a>
        );
    };
    <b>let</b> base_to_sweep = <a href="coin.md#0x1_coin_value">coin::value</a>(&pool_ref_mut.base_locked);
    <b>let</b> quote_to_sweep = <a href="coin.md#0x1_coin_value">coin::value</a>(&pool_ref_mut.quote_locked);
    <b>assert</b>!(
        base_to_sweep &gt; 0 || quote_to_sweep &gt; 0,
        <a href="lockstream.md#0x1_lockstream_E_NOTHING_TO_SWEEP">E_NOTHING_TO_SWEEP</a>
    );
    <b>if</b> (base_to_sweep &gt; 0) {
        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;BaseType&gt;(locker);
        <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(
            locker_addr,
            <a href="coin.md#0x1_coin_extract_all">coin::extract_all</a>(&<b>mut</b> pool_ref_mut.base_locked)
        );
    };
    <b>if</b> (quote_to_sweep &gt; 0) {
        <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(
            locker_addr,
            <a href="coin.md#0x1_coin_extract_all">coin::extract_all</a>(&<b>mut</b> pool_ref_mut.quote_locked)
        );
    };
    <b>let</b> sweep_event = <a href="lockstream.md#0x1_lockstream_LockstreamSweepEvent">LockstreamSweepEvent</a> {
        pool_id,
        sweep_time,
        locker: locker_addr,
        base_sweep_amount: base_to_sweep,
        quote_sweep_amount: quote_to_sweep,
    };
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> pool_ref_mut.sweep_event_handle, sweep_event);
    <b>let</b> locker_handles_ref_mut =
        <b>borrow_global_mut</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamLockerEventHandles">LockstreamLockerEventHandles</a>&gt;(locker_addr);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> locker_handles_ref_mut.sweep_event_handle,
        sweep_event
    );
}
</code></pre>



</details>

<a name="0x1_lockstream_current_period"></a>

## Function `current_period`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_current_period">current_period</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_current_period">current_period</a>&lt;
    BaseType,
    QuoteType
&gt;(creator: <b>address</b>):
Option&lt;u8&gt;
<b>acquires</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator)) {
        <b>let</b> pool_ref = <b>borrow_global</b>&lt;
            <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator);
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="lockstream.md#0x1_lockstream_period">period</a>(pool_ref, <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()))
    } <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a name="0x1_lockstream_locker"></a>

## Function `locker`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_locker">locker</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>, locker: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="lockstream.md#0x1_lockstream_LockerInfoView">lockstream::LockerInfoView</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_locker">locker</a>&lt;
    BaseType,
    QuoteType
&gt;(
    creator: <b>address</b>,
    locker: <b>address</b>,
):
Option&lt;<a href="lockstream.md#0x1_lockstream_LockerInfoView">LockerInfoView</a>&gt;
<b>acquires</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator)) {
        <b>let</b> (pool_id, pool_ref) =
            <a href="lockstream.md#0x1_lockstream_pool_id_and_immutable_reference">pool_id_and_immutable_reference</a>&lt;BaseType, QuoteType&gt;(creator);
        <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&pool_ref.lockers, locker))
            <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
        <b>let</b> time_seconds = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
        <b>let</b> (pro_rata_base_share, claimable_base, claimable_quote) =
            <a href="lockstream.md#0x1_lockstream_locker_amounts_derived">locker_amounts_derived</a>(pool_ref, locker, time_seconds);
        <b>let</b> locker_info_ref = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&pool_ref.lockers, locker);
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="lockstream.md#0x1_lockstream_LockerInfoView">LockerInfoView</a> {
            pool_id,
            locker,
            pro_rata_base_share,
            initial_quote_locked: locker_info_ref.initial_quote_locked,
            base_claimed: locker_info_ref.base_claimed,
            quote_claimed: locker_info_ref.quote_claimed,
            claimable_base,
            claimable_quote,
        })
    } <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a name="0x1_lockstream_lockers"></a>

## Function `lockers`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_lockers">lockers</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="lockstream.md#0x1_lockstream_LockerInfoView">lockstream::LockerInfoView</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_lockers">lockers</a>&lt;
    BaseType,
    QuoteType
&gt;(creator: <b>address</b>):
Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="lockstream.md#0x1_lockstream_LockerInfoView">LockerInfoView</a>&gt;&gt;
<b>acquires</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator)) {
        <b>let</b> pool_ref = <b>borrow_global</b>&lt;
            <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator);
        <b>let</b> lockers = <a href="../../aptos-stdlib/doc/big_vector.md#0x1_big_vector_to_vector">big_vector::to_vector</a>(&pool_ref.locker_addresses);
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map">vector::map</a>(lockers, |e| {
            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(<a href="lockstream.md#0x1_lockstream_locker">locker</a>&lt;BaseType, QuoteType&gt;(creator, e))
        }))
    } <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a name="0x1_lockstream_lockers_paginated"></a>

## Function `lockers_paginated`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_lockers_paginated">lockers_paginated</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>, start_index: u64, end_index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="lockstream.md#0x1_lockstream_LockerInfoView">lockstream::LockerInfoView</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_lockers_paginated">lockers_paginated</a>&lt;
    BaseType,
    QuoteType
&gt;(
    creator: <b>address</b>,
    start_index: u64,
    end_index: u64,
): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="lockstream.md#0x1_lockstream_LockerInfoView">LockerInfoView</a>&gt;&gt;
<b>acquires</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator)) {
        <b>let</b> pool_ref = <b>borrow_global</b>&lt;
            <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator);
        <b>let</b> n_lockers = <a href="../../aptos-stdlib/doc/big_vector.md#0x1_big_vector_length">big_vector::length</a>(&pool_ref.locker_addresses);
        <b>if</b> ((end_index &lt; start_index) ||
            (start_index &gt;= n_lockers) ||
            (end_index &gt;= n_lockers)) <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
        <b>let</b> i = start_index;
        <b>let</b> lockers = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
        <b>while</b> (i &lt;= end_index) {
            <b>let</b> pool_ref = <b>borrow_global</b>&lt;
                <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator);
            <b>let</b> locker_address =
                *<a href="../../aptos-stdlib/doc/big_vector.md#0x1_big_vector_borrow">big_vector::borrow</a>(&pool_ref.locker_addresses, i);
            <b>let</b> locker = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(
                <a href="lockstream.md#0x1_lockstream_locker">locker</a>&lt;BaseType, QuoteType&gt;(creator, locker_address)
            );
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> lockers, locker);
            i = i + 1;
        };
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(lockers)
    } <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a name="0x1_lockstream_metadata"></a>

## Function `metadata`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_metadata">metadata</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPoolMetadataView">lockstream::LockstreamPoolMetadataView</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_metadata">metadata</a>&lt;
    BaseType,
    QuoteType
&gt;(creator: <b>address</b>):
Option&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPoolMetadataView">LockstreamPoolMetadataView</a>&gt;
<b>acquires</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator)) {
        <b>let</b> (pool_id, pool_ref) =
            <a href="lockstream.md#0x1_lockstream_pool_id_and_immutable_reference">pool_id_and_immutable_reference</a>&lt;BaseType, QuoteType&gt;(creator);
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="lockstream.md#0x1_lockstream_LockstreamPoolMetadataView">LockstreamPoolMetadataView</a>{
            pool_id,
            base_locked: <a href="coin.md#0x1_coin_value">coin::value</a>(&pool_ref.base_locked),
            quote_locked: <a href="coin.md#0x1_coin_value">coin::value</a>(&pool_ref.quote_locked),
            n_lockers: <a href="../../aptos-stdlib/doc/big_vector.md#0x1_big_vector_length">big_vector::length</a>(&pool_ref.locker_addresses),
            initial_base_locked: pool_ref.initial_base_locked,
            initial_quote_locked: pool_ref.initial_quote_locked,
            premier_locker: pool_ref.premier_locker,
            premier_locker_initial_quote_locked:
                pool_ref.premier_locker_initial_quote_locked,
            creation_time: pool_ref.creation_time,
            stream_start_time: pool_ref.stream_start_time,
            stream_end_time: pool_ref.stream_end_time,
            claim_last_call_time: pool_ref.stream_end_time,
            premier_sweep_last_call_time:
                pool_ref.premier_sweep_last_call_time,
            current_period: <a href="lockstream.md#0x1_lockstream_period">period</a>(pool_ref, <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()),
        })
    } <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a name="0x1_lockstream_pool_id"></a>

## Function `pool_id`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;
    BaseType,
    QuoteType
&gt;(creator: <b>address</b>):
Option&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPoolID">LockstreamPoolID</a>&gt; {
    <b>if</b> (<b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="lockstream.md#0x1_lockstream_LockstreamPoolID">LockstreamPoolID</a> {
            creator,
            base_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;BaseType&gt;(),
            quote_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;QuoteType&gt;(),
        })
    } <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a name="0x1_lockstream_locker_amounts_derived"></a>

## Function `locker_amounts_derived`



<pre><code><b>fun</b> <a href="lockstream.md#0x1_lockstream_locker_amounts_derived">locker_amounts_derived</a>&lt;BaseType, QuoteType&gt;(pool_ref: &<a href="lockstream.md#0x1_lockstream_LockstreamPool">lockstream::LockstreamPool</a>&lt;BaseType, QuoteType&gt;, locker: <b>address</b>, time_seconds: u64): (u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="lockstream.md#0x1_lockstream_locker_amounts_derived">locker_amounts_derived</a>&lt;
    BaseType,
    QuoteType
&gt;(
    pool_ref: &<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;,
    locker: <b>address</b>,
    time_seconds: u64,
): (
   u64,
   u64,
   u64,
) {
    <b>let</b> period = <a href="lockstream.md#0x1_lockstream_period">period</a>(pool_ref, time_seconds);
    <b>let</b> lockers_ref = &pool_ref.lockers;
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(lockers_ref, locker), <a href="lockstream.md#0x1_lockstream_E_NOT_A_LOCKER">E_NOT_A_LOCKER</a>);
    <b>let</b> locker_info_ref = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(lockers_ref, locker);
    <b>let</b> initial_quote_locked = locker_info_ref.initial_quote_locked;
    <b>let</b> pro_rata_base_share = <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(
        pool_ref.initial_base_locked,
        initial_quote_locked,
        pool_ref.initial_quote_locked,
    );
    <b>let</b> claimable_period =
        period == <a href="lockstream.md#0x1_lockstream_PERIOD_STREAMING">PERIOD_STREAMING</a> ||
        period == <a href="lockstream.md#0x1_lockstream_PERIOD_CLAIMING_GRACE_PERIOD">PERIOD_CLAIMING_GRACE_PERIOD</a>;
    <b>let</b> (claimable_base, claimable_quote) = <b>if</b> (claimable_period) {
        <b>let</b> (claimable_base_ceiling, claimable_quote_ceiling) =
            <b>if</b> (period == <a href="lockstream.md#0x1_lockstream_PERIOD_CLAIMING_GRACE_PERIOD">PERIOD_CLAIMING_GRACE_PERIOD</a>)
                (pro_rata_base_share, initial_quote_locked) <b>else</b>
        {
            <b>let</b> stream_start = pool_ref.stream_start_time;
            <b>let</b> elapsed = time_seconds - stream_start;
            <b>let</b> duration = pool_ref.stream_end_time - stream_start;
            (
                <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(pro_rata_base_share, elapsed, duration),
                <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(initial_quote_locked, elapsed, duration),
            )
        };
        (
            claimable_base_ceiling - locker_info_ref.base_claimed,
            claimable_quote_ceiling - locker_info_ref.quote_claimed,
        )
    } <b>else</b> {
        (0, 0)
    };
    (
        pro_rata_base_share,
        claimable_base,
        claimable_quote,
    )
}
</code></pre>



</details>

<a name="0x1_lockstream_period"></a>

## Function `period`



<pre><code><b>fun</b> <a href="lockstream.md#0x1_lockstream_period">period</a>&lt;BaseType, QuoteType&gt;(pool_ref: &<a href="lockstream.md#0x1_lockstream_LockstreamPool">lockstream::LockstreamPool</a>&lt;BaseType, QuoteType&gt;, time_seconds: u64): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="lockstream.md#0x1_lockstream_period">period</a>&lt;
    BaseType,
    QuoteType
&gt;(
    pool_ref: &<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;,
    time_seconds: u64
): u8 {
    <b>if</b> (time_seconds &lt; pool_ref.stream_start_time) <a href="lockstream.md#0x1_lockstream_PERIOD_LOCKING">PERIOD_LOCKING</a> <b>else</b>
    <b>if</b> (time_seconds &lt;= pool_ref.stream_end_time) <a href="lockstream.md#0x1_lockstream_PERIOD_STREAMING">PERIOD_STREAMING</a> <b>else</b>
    <b>if</b> (time_seconds &lt;= pool_ref.claim_last_call_time)
        <a href="lockstream.md#0x1_lockstream_PERIOD_CLAIMING_GRACE_PERIOD">PERIOD_CLAIMING_GRACE_PERIOD</a> <b>else</b>
    <b>if</b> (time_seconds &lt;= pool_ref.premier_sweep_last_call_time)
        <a href="lockstream.md#0x1_lockstream_PERIOD_PREMIER_SWEEP">PERIOD_PREMIER_SWEEP</a> <b>else</b>
    <a href="lockstream.md#0x1_lockstream_PERIOD_MERCENARY_SWEEP">PERIOD_MERCENARY_SWEEP</a>
}
</code></pre>



</details>

<a name="0x1_lockstream_pool_id_and_immutable_reference"></a>

## Function `pool_id_and_immutable_reference`



<pre><code><b>fun</b> <a href="lockstream.md#0x1_lockstream_pool_id_and_immutable_reference">pool_id_and_immutable_reference</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>): (<a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a>, &<a href="lockstream.md#0x1_lockstream_LockstreamPool">lockstream::LockstreamPool</a>&lt;BaseType, QuoteType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="lockstream.md#0x1_lockstream_pool_id_and_immutable_reference">pool_id_and_immutable_reference</a>&lt;
    BaseType,
    QuoteType
&gt;(creator: <b>address</b>): (
    <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">LockstreamPoolID</a>,
    &<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;
) <b>acquires</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a> {
    <b>let</b> pool_id_option = <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;BaseType, QuoteType&gt;(creator);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&pool_id_option), <a href="lockstream.md#0x1_lockstream_E_NO_LOCKSTREAM_POOL">E_NO_LOCKSTREAM_POOL</a>);
    (
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(pool_id_option),
        <b>borrow_global</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator),
    )
}
</code></pre>



</details>

<a name="0x1_lockstream_pool_id_and_mutable_reference"></a>

## Function `pool_id_and_mutable_reference`



<pre><code><b>fun</b> <a href="lockstream.md#0x1_lockstream_pool_id_and_mutable_reference">pool_id_and_mutable_reference</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>): (<a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a>, &<b>mut</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">lockstream::LockstreamPool</a>&lt;BaseType, QuoteType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="lockstream.md#0x1_lockstream_pool_id_and_mutable_reference">pool_id_and_mutable_reference</a>&lt;
    BaseType,
    QuoteType
&gt;(creator: <b>address</b>): (
    <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">LockstreamPoolID</a>,
    &<b>mut</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;
) <b>acquires</b> <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a> {
    <b>let</b> pool_id_option = <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;BaseType, QuoteType&gt;(creator);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&pool_id_option), <a href="lockstream.md#0x1_lockstream_E_NO_LOCKSTREAM_POOL">E_NO_LOCKSTREAM_POOL</a>);
    (
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(pool_id_option),
        <b>borrow_global_mut</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator),
    )
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
