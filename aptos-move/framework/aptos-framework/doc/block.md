
<a id="0x1_block"></a>

# Module `0x1::block`

This module defines a struct storing the metadata of the block and new block events.


-  [Resource `BlockResource`](#0x1_block_BlockResource)
-  [Resource `CommitHistory`](#0x1_block_CommitHistory)
-  [Struct `NewBlockEvent`](#0x1_block_NewBlockEvent)
-  [Struct `UpdateEpochIntervalEvent`](#0x1_block_UpdateEpochIntervalEvent)
-  [Struct `NewBlock`](#0x1_block_NewBlock)
-  [Struct `UpdateEpochInterval`](#0x1_block_UpdateEpochInterval)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_block_initialize)
-  [Function `initialize_commit_history`](#0x1_block_initialize_commit_history)
-  [Function `update_epoch_interval_microsecs`](#0x1_block_update_epoch_interval_microsecs)
-  [Function `get_epoch_interval_secs`](#0x1_block_get_epoch_interval_secs)
-  [Function `block_prologue_common`](#0x1_block_block_prologue_common)
-  [Function `block_prologue`](#0x1_block_block_prologue)
-  [Function `block_prologue_ext`](#0x1_block_block_prologue_ext)
-  [Function `get_current_block_height`](#0x1_block_get_current_block_height)
-  [Function `emit_new_block_event`](#0x1_block_emit_new_block_event)
-  [Function `emit_genesis_block_event`](#0x1_block_emit_genesis_block_event)
-  [Function `emit_writeset_block_event`](#0x1_block_emit_writeset_block_event)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Resource `BlockResource`](#@Specification_1_BlockResource)
    -  [Resource `CommitHistory`](#@Specification_1_CommitHistory)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `update_epoch_interval_microsecs`](#@Specification_1_update_epoch_interval_microsecs)
    -  [Function `get_epoch_interval_secs`](#@Specification_1_get_epoch_interval_secs)
    -  [Function `block_prologue_common`](#@Specification_1_block_prologue_common)
    -  [Function `block_prologue`](#@Specification_1_block_prologue)
    -  [Function `block_prologue_ext`](#@Specification_1_block_prologue_ext)
    -  [Function `get_current_block_height`](#@Specification_1_get_current_block_height)
    -  [Function `emit_new_block_event`](#@Specification_1_emit_new_block_event)
    -  [Function `emit_genesis_block_event`](#@Specification_1_emit_genesis_block_event)
    -  [Function `emit_writeset_block_event`](#@Specification_1_emit_writeset_block_event)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="randomness.md#0x1_randomness">0x1::randomness</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg">0x1::reconfiguration_with_dkg</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="state_storage.md#0x1_state_storage">0x1::state_storage</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_block_BlockResource"></a>

## Resource `BlockResource`

Should be in-sync with BlockResource rust struct in new_block.rs


<pre><code><b>struct</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>height: u64</code>
</dt>
<dd>
 Height of the current block
</dd>
<dt>
<code>epoch_interval: u64</code>
</dt>
<dd>
 Time period between epochs.
</dd>
<dt>
<code>new_block_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>&gt;</code>
</dt>
<dd>
 Handle where events with the time of new blocks are emitted
</dd>
<dt>
<code>update_epoch_interval_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="block.md#0x1_block_UpdateEpochIntervalEvent">block::UpdateEpochIntervalEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_block_CommitHistory"></a>

## Resource `CommitHistory`

Store new block events as a move resource, internally using a circular buffer.


<pre><code><b>struct</b> <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>max_capacity: u32</code>
</dt>
<dd>

</dd>
<dt>
<code>next_idx: u32</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u32, <a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_block_NewBlockEvent"></a>

## Struct `NewBlockEvent`

Should be in-sync with NewBlockEvent rust struct in new_block.rs


<pre><code><b>struct</b> <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>round: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>height: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_microseconds: u64</code>
</dt>
<dd>
 On-chain time during the block at the given height
</dd>
</dl>


</details>

<a id="0x1_block_UpdateEpochIntervalEvent"></a>

## Struct `UpdateEpochIntervalEvent`

Event emitted when a proposal is created.


<pre><code><b>struct</b> <a href="block.md#0x1_block_UpdateEpochIntervalEvent">UpdateEpochIntervalEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_epoch_interval: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_epoch_interval: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_block_NewBlock"></a>

## Struct `NewBlock`

Should be in-sync with NewBlockEvent rust struct in new_block.rs


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="block.md#0x1_block_NewBlock">NewBlock</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>round: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>height: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_microseconds: u64</code>
</dt>
<dd>
 On-chain time during the block at the given height
</dd>
</dl>


</details>

<a id="0x1_block_UpdateEpochInterval"></a>

## Struct `UpdateEpochInterval`

Event emitted when a proposal is created.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="block.md#0x1_block_UpdateEpochInterval">UpdateEpochInterval</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_epoch_interval: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_epoch_interval: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_block_MAX_U64"></a>



<pre><code><b>const</b> <a href="block.md#0x1_block_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_block_EINVALID_PROPOSER"></a>

An invalid proposer was provided. Expected the proposer to be the VM or an active validator.


<pre><code><b>const</b> <a href="block.md#0x1_block_EINVALID_PROPOSER">EINVALID_PROPOSER</a>: u64 = 2;
</code></pre>



<a id="0x1_block_ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT"></a>

The number of new block events does not equal the current block height.


<pre><code><b>const</b> <a href="block.md#0x1_block_ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT">ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT</a>: u64 = 1;
</code></pre>



<a id="0x1_block_EZERO_EPOCH_INTERVAL"></a>

Epoch interval cannot be 0.


<pre><code><b>const</b> <a href="block.md#0x1_block_EZERO_EPOCH_INTERVAL">EZERO_EPOCH_INTERVAL</a>: u64 = 3;
</code></pre>



<a id="0x1_block_EZERO_MAX_CAPACITY"></a>

The maximum capacity of the commit history cannot be 0.


<pre><code><b>const</b> <a href="block.md#0x1_block_EZERO_MAX_CAPACITY">EZERO_MAX_CAPACITY</a>: u64 = 3;
</code></pre>



<a id="0x1_block_initialize"></a>

## Function `initialize`

This can only be called during Genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="block.md#0x1_block_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch_interval_microsecs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="block.md#0x1_block_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch_interval_microsecs: u64) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(epoch_interval_microsecs &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="block.md#0x1_block_EZERO_EPOCH_INTERVAL">EZERO_EPOCH_INTERVAL</a>));

    <b>move_to</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(aptos_framework, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> {
        max_capacity: 2000,
        next_idx: 0,
        <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),
    });

    <b>move_to</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(
        aptos_framework,
        <a href="block.md#0x1_block_BlockResource">BlockResource</a> {
            height: 0,
            epoch_interval: epoch_interval_microsecs,
            new_block_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a>&gt;(aptos_framework),
            update_epoch_interval_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="block.md#0x1_block_UpdateEpochIntervalEvent">UpdateEpochIntervalEvent</a>&gt;(aptos_framework),
        }
    );
}
</code></pre>



</details>

<a id="0x1_block_initialize_commit_history"></a>

## Function `initialize_commit_history`

Initialize the commit history resource if it's not in genesis.


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_initialize_commit_history">initialize_commit_history</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_capacity: u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_initialize_commit_history">initialize_commit_history</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_capacity: u32) {
    <b>assert</b>!(max_capacity &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="block.md#0x1_block_EZERO_MAX_CAPACITY">EZERO_MAX_CAPACITY</a>));
    <b>move_to</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(fx, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> {
        max_capacity,
        next_idx: 0,
        <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),
    });
}
</code></pre>



</details>

<a id="0x1_block_update_epoch_interval_microsecs"></a>

## Function `update_epoch_interval_microsecs`

Update the epoch interval.
Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_update_epoch_interval_microsecs">update_epoch_interval_microsecs</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_epoch_interval: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_update_epoch_interval_microsecs">update_epoch_interval_microsecs</a>(
    aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_epoch_interval: u64,
) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(new_epoch_interval &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="block.md#0x1_block_EZERO_EPOCH_INTERVAL">EZERO_EPOCH_INTERVAL</a>));

    <b>let</b> block_resource = <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);
    <b>let</b> old_epoch_interval = block_resource.epoch_interval;
    block_resource.epoch_interval = new_epoch_interval;

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="block.md#0x1_block_UpdateEpochInterval">UpdateEpochInterval</a> { old_epoch_interval, new_epoch_interval },
        );
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="block.md#0x1_block_UpdateEpochIntervalEvent">UpdateEpochIntervalEvent</a>&gt;(
            &<b>mut</b> block_resource.update_epoch_interval_events,
            <a href="block.md#0x1_block_UpdateEpochIntervalEvent">UpdateEpochIntervalEvent</a> { old_epoch_interval, new_epoch_interval },
        );
    };
}
</code></pre>



</details>

<a id="0x1_block_get_epoch_interval_secs"></a>

## Function `get_epoch_interval_secs`

Return epoch interval in seconds.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_epoch_interval_secs">get_epoch_interval_secs</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_epoch_interval_secs">get_epoch_interval_secs</a>(): u64 <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> {
    <b>borrow_global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework).epoch_interval / 1000000
}
</code></pre>



</details>

<a id="0x1_block_block_prologue_common"></a>

## Function `block_prologue_common`



<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(
    vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>,
    epoch: u64,
    round: u64,
    proposer: <b>address</b>,
    failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64
): u64 <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> {
    // Operational constraint: can only be invoked by the VM.
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);

    // Blocks can only be produced by a valid proposer or by the VM itself for Nil blocks (no user txs).
    <b>assert</b>!(
        proposer == @vm_reserved || <a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(proposer),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="block.md#0x1_block_EINVALID_PROPOSER">EINVALID_PROPOSER</a>),
    );

    <b>let</b> proposer_index = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    <b>if</b> (proposer != @vm_reserved) {
        proposer_index = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="stake.md#0x1_stake_get_validator_index">stake::get_validator_index</a>(proposer));
    };

    <b>let</b> block_metadata_ref = <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);
    block_metadata_ref.height = <a href="event.md#0x1_event_counter">event::counter</a>(&block_metadata_ref.new_block_events);

    <b>let</b> new_block_event = <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,
        epoch,
        round,
        height: block_metadata_ref.height,
        previous_block_votes_bitvec,
        proposer,
        failed_proposer_indices,
        time_microseconds: <a href="timestamp.md#0x1_timestamp">timestamp</a>,
    };
    <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(vm, &<b>mut</b> block_metadata_ref.new_block_events, new_block_event);

    // Performance scores have <b>to</b> be updated before the epoch transition <b>as</b> the transaction that triggers the
    // transition is the last <a href="block.md#0x1_block">block</a> in the previous epoch.
    <a href="stake.md#0x1_stake_update_performance_statistics">stake::update_performance_statistics</a>(proposer_index, failed_proposer_indices);
    <a href="state_storage.md#0x1_state_storage_on_new_block">state_storage::on_new_block</a>(<a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>());

    //<a href="scheduled_txns.md#0x1_scheduled_txns_remove_txns">scheduled_txns::remove_txns</a>();

    block_metadata_ref.epoch_interval
}
</code></pre>



</details>

<a id="0x1_block_block_prologue"></a>

## Function `block_prologue`

Set the metadata for the current block.
The runtime always runs this before executing the transactions in a block.


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue">block_prologue</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue">block_prologue</a>(
    vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>,
    epoch: u64,
    round: u64,
    proposer: <b>address</b>,
    failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64
) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> {
    <b>let</b> epoch_interval = <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(&vm, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, epoch, round, proposer, failed_proposer_indices, previous_block_votes_bitvec, <a href="timestamp.md#0x1_timestamp">timestamp</a>);
    <a href="randomness.md#0x1_randomness_on_new_block">randomness::on_new_block</a>(&vm, epoch, round, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>());
    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> - <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>() &gt;= epoch_interval) {
        <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();
    };
}
</code></pre>



</details>

<a id="0x1_block_block_prologue_ext"></a>

## Function `block_prologue_ext`

<code><a href="block.md#0x1_block_block_prologue">block_prologue</a>()</code> but trigger reconfiguration with DKG after epoch timed out.


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_ext">block_prologue_ext</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64, randomness_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_ext">block_prologue_ext</a>(
    vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>,
    epoch: u64,
    round: u64,
    proposer: <b>address</b>,
    failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64,
    randomness_seed: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> {
    <b>let</b> epoch_interval = <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(
        &vm,
        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,
        epoch,
        round,
        proposer,
        failed_proposer_indices,
        previous_block_votes_bitvec,
        <a href="timestamp.md#0x1_timestamp">timestamp</a>
    );
    <a href="randomness.md#0x1_randomness_on_new_block">randomness::on_new_block</a>(&vm, epoch, round, randomness_seed);

    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> - <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>() &gt;= epoch_interval) {
        <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">reconfiguration_with_dkg::try_start</a>();
    };
}
</code></pre>



</details>

<a id="0x1_block_get_current_block_height"></a>

## Function `get_current_block_height`

Get the current block height


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_current_block_height">get_current_block_height</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_current_block_height">get_current_block_height</a>(): u64 <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> {
    <b>borrow_global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework).height
}
</code></pre>



</details>

<a id="0x1_block_emit_new_block_event"></a>

## Function `emit_new_block_event`

Emit the event and update height and global timestamp


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, event_handle: &<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>&gt;, new_block_event: <a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(
    vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    event_handle: &<b>mut</b> EventHandle&lt;<a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a>&gt;,
    new_block_event: <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a>,
) <b>acquires</b> <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(@aptos_framework)) {
        <b>let</b> commit_history_ref = <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(@aptos_framework);
        <b>let</b> idx = commit_history_ref.next_idx;
        <b>if</b> (<a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_contains">table_with_length::contains</a>(&commit_history_ref.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, idx)) {
            <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> commit_history_ref.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, idx);
        };
        <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> commit_history_ref.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, idx, <b>copy</b> new_block_event);
        <b>spec</b> {
            <b>assume</b> idx + 1 &lt;= MAX_U32;
        };
        commit_history_ref.next_idx = (idx + 1) % commit_history_ref.max_capacity;
    };
    <a href="timestamp.md#0x1_timestamp_update_global_time">timestamp::update_global_time</a>(vm, new_block_event.proposer, new_block_event.time_microseconds);
    <b>assert</b>!(
        <a href="event.md#0x1_event_counter">event::counter</a>(event_handle) == new_block_event.height,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="block.md#0x1_block_ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT">ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT</a>),
    );
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a>&gt;(event_handle, new_block_event);
}
</code></pre>



</details>

<a id="0x1_block_emit_genesis_block_event"></a>

## Function `emit_genesis_block_event`

Emit a <code><a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a></code> event. This function will be invoked by genesis directly to generate the very first
reconfiguration event.


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_genesis_block_event">emit_genesis_block_event</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_genesis_block_event">emit_genesis_block_event</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> {
    <b>let</b> block_metadata_ref = <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);
    <b>let</b> genesis_id = @0x0;
    <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(
        &vm,
        &<b>mut</b> block_metadata_ref.new_block_events,
        <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a> {
            <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: genesis_id,
            epoch: 0,
            round: 0,
            height: 0,
            previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            proposer: @vm_reserved,
            failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            time_microseconds: 0,
        },
    );
}
</code></pre>



</details>

<a id="0x1_block_emit_writeset_block_event"></a>

## Function `emit_writeset_block_event`

Emit a <code><a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a></code> event. This function will be invoked by write set script directly to generate the
new block event for WriteSetPayload.


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_emit_writeset_block_event">emit_writeset_block_event</a>(vm_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fake_block_hash: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_emit_writeset_block_event">emit_writeset_block_event</a>(vm_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fake_block_hash: <b>address</b>) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm_signer);
    <b>let</b> block_metadata_ref = <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);
    block_metadata_ref.height = <a href="event.md#0x1_event_counter">event::counter</a>(&block_metadata_ref.new_block_events);

    <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(
        vm_signer,
        &<b>mut</b> block_metadata_ref.new_block_events,
        <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a> {
            <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: fake_block_hash,
            epoch: <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>(),
            round: <a href="block.md#0x1_block_MAX_U64">MAX_U64</a>,
            height: block_metadata_ref.height,
            previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            proposer: @vm_reserved,
            failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            time_microseconds: <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>(),
        },
    );
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
<td>During the module's initialization, it guarantees that the BlockResource resource moves under the Aptos framework account with initial values.</td>
<td>High</td>
<td>The initialize function is responsible for setting up the initial state of the module, ensuring that the following conditions are met (1) the BlockResource resource is created, indicating its existence within the module's context, and moved under the Aptos framework account, (2) the block height is set to zero during initialization, and (3) the epoch interval is greater than zero.</td>
<td>Formally Verified via <a href="#high-level-req-1">Initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Only the Aptos framework address may execute the following functionalities: (1) initialize BlockResource, and (2) update the epoch interval.</td>
<td>Critical</td>
<td>The initialize and  update_epoch_interval_microsecs functions ensure that only aptos_framework can call them.</td>
<td>Formally Verified via <a href="#high-level-req-2.1">Initialize</a> and <a href="#high-level-req-2.2">update_epoch_interval_microsecs</a>.</td>
</tr>

<tr>
<td>3</td>
<td>When updating the epoch interval, its value must be greater than zero and BlockResource must exist.</td>
<td>High</td>
<td>The update_epoch_interval_microsecs function asserts that new_epoch_interval is greater than zero and updates BlockResource's state.</td>
<td>Formally verified via <a href="#high-level-req-3.1">UpdateEpochIntervalMicrosecs</a> and <a href="#high-level-req-3.2">epoch_interval</a>.</td>
</tr>

<tr>
<td>4</td>
<td>Only a valid proposer or the virtual machine is authorized to produce blocks.</td>
<td>Critical</td>
<td>During the execution of the block_prologue function, the validity of the proposer address is verified when setting the metadata for the current block.</td>
<td>Formally Verified via <a href="#high-level-req-4">block_prologue</a>.</td>
</tr>

<tr>
<td>5</td>
<td>While emitting a new block event, the number of them is equal to the current block height.</td>
<td>Medium</td>
<td>The emit_new_block_event function asserts that the number of new block events equals the current block height.</td>
<td>Formally Verified via <a href="#high-level-req-5">emit_new_block_event</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_BlockResource"></a>

### Resource `BlockResource`


<pre><code><b>struct</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> <b>has</b> key
</code></pre>



<dl>
<dt>
<code>height: u64</code>
</dt>
<dd>
 Height of the current block
</dd>
<dt>
<code>epoch_interval: u64</code>
</dt>
<dd>
 Time period between epochs.
</dd>
<dt>
<code>new_block_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>&gt;</code>
</dt>
<dd>
 Handle where events with the time of new blocks are emitted
</dd>
<dt>
<code>update_epoch_interval_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="block.md#0x1_block_UpdateEpochIntervalEvent">block::UpdateEpochIntervalEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>// This enforces <a id="high-level-req-3.2" href="#high-level-req">high-level requirement 3</a>:
<b>invariant</b> epoch_interval &gt; 0;
</code></pre>



<a id="@Specification_1_CommitHistory"></a>

### Resource `CommitHistory`


<pre><code><b>struct</b> <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> <b>has</b> key
</code></pre>



<dl>
<dt>
<code>max_capacity: u32</code>
</dt>
<dd>

</dd>
<dt>
<code>next_idx: u32</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u32, <a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> max_capacity &gt; 0;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="block.md#0x1_block_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch_interval_microsecs: u64)
</code></pre>


The caller is aptos_framework.
The new_epoch_interval must be greater than 0.
The BlockResource is not under the caller before initializing.
The Account is not under the caller until the BlockResource is created for the caller.
Make sure The BlockResource under the caller existed after initializing.
The number of new events created does not exceed MAX_U64.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>include</b> <a href="block.md#0x1_block_Initialize">Initialize</a>;
<b>include</b> <a href="block.md#0x1_block_NewEventHandle">NewEventHandle</a>;
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
<b>let</b> <a href="account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num + 2 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
</code></pre>




<a id="0x1_block_BlockRequirement"></a>


<pre><code><b>schema</b> <a href="block.md#0x1_block_BlockRequirement">BlockRequirement</a> {
    vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>;
    epoch: u64;
    round: u64;
    proposer: <b>address</b>;
    failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;;
    previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64;
    <b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
    <b>requires</b> <a href="system_addresses.md#0x1_system_addresses_is_vm">system_addresses::is_vm</a>(vm);
    // This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
    <b>requires</b> proposer == @vm_reserved || <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">stake::spec_is_current_epoch_validator</a>(proposer);
    <b>requires</b> (proposer == @vm_reserved) ==&gt; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() == <a href="timestamp.md#0x1_timestamp">timestamp</a>);
    <b>requires</b> (proposer != @vm_reserved) ==&gt; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &lt; <a href="timestamp.md#0x1_timestamp">timestamp</a>);
    <b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
    <b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;
}
</code></pre>




<a id="0x1_block_Initialize"></a>


<pre><code><b>schema</b> <a href="block.md#0x1_block_Initialize">Initialize</a> {
    aptos_framework: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    epoch_interval_microsecs: u64;
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
    // This enforces <a id="high-level-req-2.1" href="#high-level-req">high-level requirement 2</a>:
    <b>aborts_if</b> addr != @aptos_framework;
    <b>aborts_if</b> epoch_interval_microsecs == 0;
    <b>aborts_if</b> <b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(addr);
    <b>aborts_if</b> <b>exists</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(addr);
    <b>ensures</b> <b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(addr);
    <b>ensures</b> <b>exists</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(addr);
    <b>ensures</b> <b>global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(addr).height == 0;
}
</code></pre>




<a id="0x1_block_NewEventHandle"></a>


<pre><code><b>schema</b> <a href="block.md#0x1_block_NewEventHandle">NewEventHandle</a> {
    aptos_framework: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
    <b>let</b> <a href="account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);
    <b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num + 2 &gt; <a href="block.md#0x1_block_MAX_U64">MAX_U64</a>;
}
</code></pre>



<a id="@Specification_1_update_epoch_interval_microsecs"></a>

### Function `update_epoch_interval_microsecs`


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_update_epoch_interval_microsecs">update_epoch_interval_microsecs</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_epoch_interval: u64)
</code></pre>


The caller is @aptos_framework.
The new_epoch_interval must be greater than 0.
The BlockResource existed under the @aptos_framework.


<pre><code>// This enforces <a id="high-level-req-3.1" href="#high-level-req">high-level requirement 3</a>:
<b>include</b> <a href="block.md#0x1_block_UpdateEpochIntervalMicrosecs">UpdateEpochIntervalMicrosecs</a>;
</code></pre>




<a id="0x1_block_UpdateEpochIntervalMicrosecs"></a>


<pre><code><b>schema</b> <a href="block.md#0x1_block_UpdateEpochIntervalMicrosecs">UpdateEpochIntervalMicrosecs</a> {
    aptos_framework: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    new_epoch_interval: u64;
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
    // This enforces <a id="high-level-req-2.2" href="#high-level-req">high-level requirement 2</a>:
    <b>aborts_if</b> addr != @aptos_framework;
    <b>aborts_if</b> new_epoch_interval == 0;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(addr);
    <b>let</b> <b>post</b> block_resource = <b>global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(addr);
    <b>ensures</b> block_resource.epoch_interval == new_epoch_interval;
}
</code></pre>



<a id="@Specification_1_get_epoch_interval_secs"></a>

### Function `get_epoch_interval_secs`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_epoch_interval_secs">get_epoch_interval_secs</a>(): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_block_prologue_common"></a>

### Function `block_prologue_common`


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64): u64
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 1000;
<b>include</b> <a href="block.md#0x1_block_BlockRequirement">BlockRequirement</a>;
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_block_prologue"></a>

### Function `block_prologue`


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue">block_prologue</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 1000;
<b>requires</b> <a href="timestamp.md#0x1_timestamp">timestamp</a> &gt;= <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>();
<b>include</b> <a href="block.md#0x1_block_BlockRequirement">BlockRequirement</a>;
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_block_prologue_ext"></a>

### Function `block_prologue_ext`


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_ext">block_prologue_ext</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64, randomness_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 1000;
<b>requires</b> <a href="timestamp.md#0x1_timestamp">timestamp</a> &gt;= <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>();
<b>include</b> <a href="block.md#0x1_block_BlockRequirement">BlockRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_get_current_block_height"></a>

### Function `get_current_block_height`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_current_block_height">get_current_block_height</a>(): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_emit_new_block_event"></a>

### Function `emit_new_block_event`


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, event_handle: &<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>&gt;, new_block_event: <a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>)
</code></pre>




<pre><code><b>let</b> proposer = new_block_event.proposer;
<b>let</b> <a href="timestamp.md#0x1_timestamp">timestamp</a> = new_block_event.time_microseconds;
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>requires</b> <a href="system_addresses.md#0x1_system_addresses_is_vm">system_addresses::is_vm</a>(vm);
<b>requires</b> (proposer == @vm_reserved) ==&gt; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() == <a href="timestamp.md#0x1_timestamp">timestamp</a>);
<b>requires</b> (proposer != @vm_reserved) ==&gt; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &lt; <a href="timestamp.md#0x1_timestamp">timestamp</a>);
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
<b>requires</b> <a href="event.md#0x1_event_counter">event::counter</a>(event_handle) == new_block_event.height;
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_emit_genesis_block_event"></a>

### Function `emit_genesis_block_event`


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_genesis_block_event">emit_genesis_block_event</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>requires</b> <a href="system_addresses.md#0x1_system_addresses_is_vm">system_addresses::is_vm</a>(vm);
<b>requires</b> <a href="event.md#0x1_event_counter">event::counter</a>(<b>global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework).new_block_events) == 0;
<b>requires</b> (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() == 0);
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_emit_writeset_block_event"></a>

### Function `emit_writeset_block_event`


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_emit_writeset_block_event">emit_writeset_block_event</a>(vm_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fake_block_hash: <b>address</b>)
</code></pre>


The caller is @vm_reserved.
The BlockResource existed under the @aptos_framework.
The Configuration existed under the @aptos_framework.
The CurrentTimeMicroseconds existed under the @aptos_framework.


<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="block.md#0x1_block_EmitWritesetBlockEvent">EmitWritesetBlockEvent</a>;
</code></pre>




<a id="0x1_block_EmitWritesetBlockEvent"></a>


<pre><code><b>schema</b> <a href="block.md#0x1_block_EmitWritesetBlockEvent">EmitWritesetBlockEvent</a> {
    vm_signer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(vm_signer);
    <b>aborts_if</b> addr != @vm_reserved;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">reconfiguration::Configuration</a>&gt;(@aptos_framework);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
