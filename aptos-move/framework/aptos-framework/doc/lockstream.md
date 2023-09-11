
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
-  [Constants](#@Constants_0)
-  [Function `create`](#0x1_lockstream_create)
-  [Function `lock`](#0x1_lockstream_lock)
-  [Function `claim`](#0x1_lockstream_claim)
-  [Function `sweep`](#0x1_lockstream_sweep)
-  [Function `pool_id`](#0x1_lockstream_pool_id)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
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
    <b>move_to</b>(creator, <a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt; {
        base_locked: <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>(creator, initial_base_locked),
        quote_locked: <a href="coin.md#0x1_coin_zero">coin::zero</a>(),
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
    <b>assert</b>!(quote_lock_amount &gt; 0, <a href="lockstream.md#0x1_lockstream_E_NO_QUOTE_LOCK_AMOUNT">E_NO_QUOTE_LOCK_AMOUNT</a>);
    <b>let</b> pool_id = <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;BaseType, QuoteType&gt;(creator);
    <b>let</b> pool_ref_mut =
        <b>borrow_global_mut</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator);
    <b>let</b> lock_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>assert</b>!(
        lock_time &lt; pool_ref_mut.stream_start_time,
        <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_TO_LOCK">E_TOO_LATE_TO_LOCK</a>
    );
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
    <b>let</b> pool_id = <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;BaseType, QuoteType&gt;(creator);
    <b>let</b> pool_ref_mut =
        <b>borrow_global_mut</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator);
    <b>let</b> lockers_ref_mut = &<b>mut</b> pool_ref_mut.lockers;
    <b>let</b> locker_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(locker);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(lockers_ref_mut, locker_addr),
        <a href="lockstream.md#0x1_lockstream_E_NOT_A_LOCKER">E_NOT_A_LOCKER</a>
    );
    <b>let</b> claim_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>assert</b>!(
        claim_time &gt; pool_ref_mut.stream_start_time,
        <a href="lockstream.md#0x1_lockstream_E_TOO_EARLY_TO_CLAIM">E_TOO_EARLY_TO_CLAIM</a>
    );
    <b>assert</b>!(
        claim_time &lt;= pool_ref_mut.claim_last_call_time,
        <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_TO_CLAIM">E_TOO_LATE_TO_CLAIM</a>
    );
    <b>let</b> locker_info_ref_mut =
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(lockers_ref_mut, locker_addr);
    <b>let</b> locker_initial_quote_locked =
        locker_info_ref_mut.initial_quote_locked;
    <b>let</b> pro_rata_base = <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(
        pool_ref_mut.initial_base_locked,
        locker_initial_quote_locked,
        pool_ref_mut.initial_quote_locked
    );
    <b>let</b> stream_done = claim_time &gt;= pool_ref_mut.stream_end_time;
    <b>let</b> (base_claimed_ceiling, quote_claimed_ceiling) = <b>if</b> (stream_done) {
        (pro_rata_base, locker_initial_quote_locked)
    } <b>else</b> {
        <b>let</b> stream_start = pool_ref_mut.stream_start_time;
        <b>let</b> elapsed = claim_time - stream_start;
        <b>let</b> duration = pool_ref_mut.stream_end_time - stream_start;
        (
            <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(pro_rata_base, elapsed, duration),
            <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_mul_div">math64::mul_div</a>(locker_initial_quote_locked, elapsed, duration)
        )
    };
    <b>let</b> base_claimed =
        base_claimed_ceiling - locker_info_ref_mut.base_claimed;
    <b>let</b> quote_claimed =
        quote_claimed_ceiling - locker_info_ref_mut.quote_claimed;
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
    <b>let</b> pool_id = <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;BaseType, QuoteType&gt;(creator);
    <b>let</b> pool_ref_mut =
        <b>borrow_global_mut</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator);
    <b>let</b> lockers_ref_mut = &<b>mut</b> pool_ref_mut.lockers;
    <b>let</b> locker_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(locker);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(lockers_ref_mut, locker_addr),
        <a href="lockstream.md#0x1_lockstream_E_NOT_A_LOCKER">E_NOT_A_LOCKER</a>
    );
    <b>let</b> sweep_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>if</b> (locker_addr == pool_ref_mut.premier_locker) {
        <b>assert</b>!(
            sweep_time &gt; pool_ref_mut.claim_last_call_time,
            <a href="lockstream.md#0x1_lockstream_E_TOO_EARLY_FOR_PREMIER_SWEEP">E_TOO_EARLY_FOR_PREMIER_SWEEP</a>
        );
        <b>assert</b>!(
            sweep_time &lt;= pool_ref_mut.premier_sweep_last_call_time, <a href="lockstream.md#0x1_lockstream_E_TOO_LATE_FOR_PREMIER_SWEEP">E_TOO_LATE_FOR_PREMIER_SWEEP</a>
        );
    } <b>else</b> {
        <b>assert</b>!(
            sweep_time &gt; pool_ref_mut.premier_sweep_last_call_time,
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

<a name="0x1_lockstream_pool_id"></a>

## Function `pool_id`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;BaseType, QuoteType&gt;(creator: <b>address</b>): <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">lockstream::LockstreamPoolID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lockstream.md#0x1_lockstream_pool_id">pool_id</a>&lt;
    BaseType,
    QuoteType
&gt;(creator: <b>address</b>):
<a href="lockstream.md#0x1_lockstream_LockstreamPoolID">LockstreamPoolID</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="lockstream.md#0x1_lockstream_LockstreamPool">LockstreamPool</a>&lt;BaseType, QuoteType&gt;&gt;(creator),
        <a href="lockstream.md#0x1_lockstream_E_NO_LOCKSTREAM_POOL">E_NO_LOCKSTREAM_POOL</a>
    );
    <a href="lockstream.md#0x1_lockstream_LockstreamPoolID">LockstreamPoolID</a> {
        creator,
        base_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;BaseType&gt;(),
        quote_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;QuoteType&gt;(),
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
