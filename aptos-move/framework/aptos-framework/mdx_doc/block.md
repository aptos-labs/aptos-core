
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="randomness.md#0x1_randomness">0x1::randomness</a>;<br /><b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;<br /><b>use</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg">0x1::reconfiguration_with_dkg</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="state_storage.md#0x1_state_storage">0x1::state_storage</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="transaction_fee.md#0x1_transaction_fee">0x1::transaction_fee</a>;<br /></code></pre>



<a id="0x1_block_BlockResource"></a>

## Resource `BlockResource`

Should be in&#45;sync with BlockResource rust struct in new_block.rs


<pre><code><b>struct</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> <b>has</b> key<br /></code></pre>



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


<pre><code><b>struct</b> <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> <b>has</b> key<br /></code></pre>



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

Should be in&#45;sync with NewBlockEvent rust struct in new_block.rs


<pre><code><b>struct</b> <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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
 On&#45;chain time during the block at the given height
</dd>
</dl>


</details>

<a id="0x1_block_UpdateEpochIntervalEvent"></a>

## Struct `UpdateEpochIntervalEvent`

Event emitted when a proposal is created.


<pre><code><b>struct</b> <a href="block.md#0x1_block_UpdateEpochIntervalEvent">UpdateEpochIntervalEvent</a> <b>has</b> drop, store<br /></code></pre>



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

Should be in&#45;sync with NewBlockEvent rust struct in new_block.rs


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="block.md#0x1_block_NewBlock">NewBlock</a> <b>has</b> drop, store<br /></code></pre>



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
 On&#45;chain time during the block at the given height
</dd>
</dl>


</details>

<a id="0x1_block_UpdateEpochInterval"></a>

## Struct `UpdateEpochInterval`

Event emitted when a proposal is created.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="block.md#0x1_block_UpdateEpochInterval">UpdateEpochInterval</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>const</b> <a href="block.md#0x1_block_MAX_U64">MAX_U64</a>: u64 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_block_EINVALID_PROPOSER"></a>

An invalid proposer was provided. Expected the proposer to be the VM or an active validator.


<pre><code><b>const</b> <a href="block.md#0x1_block_EINVALID_PROPOSER">EINVALID_PROPOSER</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_block_ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT"></a>

The number of new block events does not equal the current block height.


<pre><code><b>const</b> <a href="block.md#0x1_block_ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT">ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_block_EZERO_EPOCH_INTERVAL"></a>

Epoch interval cannot be 0.


<pre><code><b>const</b> <a href="block.md#0x1_block_EZERO_EPOCH_INTERVAL">EZERO_EPOCH_INTERVAL</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_block_EZERO_MAX_CAPACITY"></a>

The maximum capacity of the commit history cannot be 0.


<pre><code><b>const</b> <a href="block.md#0x1_block_EZERO_MAX_CAPACITY">EZERO_MAX_CAPACITY</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_block_initialize"></a>

## Function `initialize`

This can only be called during Genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="block.md#0x1_block_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch_interval_microsecs: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="block.md#0x1_block_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch_interval_microsecs: u64) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>assert</b>!(epoch_interval_microsecs &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="block.md#0x1_block_EZERO_EPOCH_INTERVAL">EZERO_EPOCH_INTERVAL</a>));<br /><br />    <b>move_to</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(aptos_framework, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> &#123;<br />        max_capacity: 2000,<br />        next_idx: 0,<br />        <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),<br />    &#125;);<br /><br />    <b>move_to</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(<br />        aptos_framework,<br />        <a href="block.md#0x1_block_BlockResource">BlockResource</a> &#123;<br />            height: 0,<br />            epoch_interval: epoch_interval_microsecs,<br />            new_block_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a>&gt;(aptos_framework),<br />            update_epoch_interval_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="block.md#0x1_block_UpdateEpochIntervalEvent">UpdateEpochIntervalEvent</a>&gt;(aptos_framework),<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_initialize_commit_history"></a>

## Function `initialize_commit_history`

Initialize the commit history resource if it&apos;s not in genesis.


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_initialize_commit_history">initialize_commit_history</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_capacity: u32)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_initialize_commit_history">initialize_commit_history</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_capacity: u32) &#123;<br />    <b>assert</b>!(max_capacity &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="block.md#0x1_block_EZERO_MAX_CAPACITY">EZERO_MAX_CAPACITY</a>));<br />    <b>move_to</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(fx, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> &#123;<br />        max_capacity,<br />        next_idx: 0,<br />        <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_update_epoch_interval_microsecs"></a>

## Function `update_epoch_interval_microsecs`

Update the epoch interval.
Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_update_epoch_interval_microsecs">update_epoch_interval_microsecs</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_epoch_interval: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_update_epoch_interval_microsecs">update_epoch_interval_microsecs</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    new_epoch_interval: u64,<br />) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>assert</b>!(new_epoch_interval &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="block.md#0x1_block_EZERO_EPOCH_INTERVAL">EZERO_EPOCH_INTERVAL</a>));<br /><br />    <b>let</b> block_resource &#61; <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);<br />    <b>let</b> old_epoch_interval &#61; block_resource.epoch_interval;<br />    block_resource.epoch_interval &#61; new_epoch_interval;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="block.md#0x1_block_UpdateEpochInterval">UpdateEpochInterval</a> &#123; old_epoch_interval, new_epoch_interval &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="block.md#0x1_block_UpdateEpochIntervalEvent">UpdateEpochIntervalEvent</a>&gt;(<br />        &amp;<b>mut</b> block_resource.update_epoch_interval_events,<br />        <a href="block.md#0x1_block_UpdateEpochIntervalEvent">UpdateEpochIntervalEvent</a> &#123; old_epoch_interval, new_epoch_interval &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_get_epoch_interval_secs"></a>

## Function `get_epoch_interval_secs`

Return epoch interval in seconds.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_epoch_interval_secs">get_epoch_interval_secs</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_epoch_interval_secs">get_epoch_interval_secs</a>(): u64 <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework).epoch_interval / 1000000<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_block_prologue_common"></a>

## Function `block_prologue_common`



<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(vm: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(<br />    vm: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>,<br />    epoch: u64,<br />    round: u64,<br />    proposer: <b>address</b>,<br />    failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,<br />    previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64<br />): u64 <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> &#123;<br />    // Operational constraint: can only be invoked by the VM.<br />    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);<br /><br />    // Blocks can only be produced by a valid proposer or by the VM itself for Nil blocks (no user txs).<br />    <b>assert</b>!(<br />        proposer &#61;&#61; @vm_reserved &#124;&#124; <a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(proposer),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="block.md#0x1_block_EINVALID_PROPOSER">EINVALID_PROPOSER</a>),<br />    );<br /><br />    <b>let</b> proposer_index &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();<br />    <b>if</b> (proposer !&#61; @vm_reserved) &#123;<br />        proposer_index &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="stake.md#0x1_stake_get_validator_index">stake::get_validator_index</a>(proposer));<br />    &#125;;<br /><br />    <b>let</b> block_metadata_ref &#61; <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);<br />    block_metadata_ref.height &#61; <a href="event.md#0x1_event_counter">event::counter</a>(&amp;block_metadata_ref.new_block_events);<br /><br />    // Emit both <a href="event.md#0x1_event">event</a> v1 and v2 for compatibility. Eventually only <b>module</b> events will be kept.<br />    <b>let</b> new_block_event &#61; <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,<br />        epoch,<br />        round,<br />        height: block_metadata_ref.height,<br />        previous_block_votes_bitvec,<br />        proposer,<br />        failed_proposer_indices,<br />        time_microseconds: <a href="timestamp.md#0x1_timestamp">timestamp</a>,<br />    &#125;;<br />    <b>let</b> new_block_event_v2 &#61; <a href="block.md#0x1_block_NewBlock">NewBlock</a> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,<br />        epoch,<br />        round,<br />        height: block_metadata_ref.height,<br />        previous_block_votes_bitvec,<br />        proposer,<br />        failed_proposer_indices,<br />        time_microseconds: <a href="timestamp.md#0x1_timestamp">timestamp</a>,<br />    &#125;;<br />    <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(vm, &amp;<b>mut</b> block_metadata_ref.new_block_events, new_block_event, new_block_event_v2);<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_collect_and_distribute_gas_fees">features::collect_and_distribute_gas_fees</a>()) &#123;<br />        // Assign the fees collected from the previous <a href="block.md#0x1_block">block</a> <b>to</b> the previous <a href="block.md#0x1_block">block</a> proposer.<br />        // If for <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> reason the fees cannot be assigned, this function burns the collected coins.<br />        <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">transaction_fee::process_collected_fees</a>();<br />        // Set the proposer of this <a href="block.md#0x1_block">block</a> <b>as</b> the receiver of the fees, so that the fees for this<br />        // <a href="block.md#0x1_block">block</a> are assigned <b>to</b> the right <a href="account.md#0x1_account">account</a>.<br />        <a href="transaction_fee.md#0x1_transaction_fee_register_proposer_for_fee_collection">transaction_fee::register_proposer_for_fee_collection</a>(proposer);<br />    &#125;;<br /><br />    // Performance scores have <b>to</b> be updated before the epoch transition <b>as</b> the transaction that triggers the<br />    // transition is the last <a href="block.md#0x1_block">block</a> in the previous epoch.<br />    <a href="stake.md#0x1_stake_update_performance_statistics">stake::update_performance_statistics</a>(proposer_index, failed_proposer_indices);<br />    <a href="state_storage.md#0x1_state_storage_on_new_block">state_storage::on_new_block</a>(<a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>());<br /><br />    block_metadata_ref.epoch_interval<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_block_prologue"></a>

## Function `block_prologue`

Set the metadata for the current block.
The runtime always runs this before executing the transactions in a block.


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue">block_prologue</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue">block_prologue</a>(<br />    vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>,<br />    epoch: u64,<br />    round: u64,<br />    proposer: <b>address</b>,<br />    failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,<br />    previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64<br />) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> &#123;<br />    <b>let</b> epoch_interval &#61; <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(&amp;vm, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, epoch, round, proposer, failed_proposer_indices, previous_block_votes_bitvec, <a href="timestamp.md#0x1_timestamp">timestamp</a>);<br />    <a href="randomness.md#0x1_randomness_on_new_block">randomness::on_new_block</a>(&amp;vm, epoch, round, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>());<br />    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> &#45; <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>() &gt;&#61; epoch_interval) &#123;<br />        <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_block_prologue_ext"></a>

## Function `block_prologue_ext`

<code><a href="block.md#0x1_block_block_prologue">block_prologue</a>()</code> but trigger reconfiguration with DKG after epoch timed out.


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_ext">block_prologue_ext</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64, randomness_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_ext">block_prologue_ext</a>(<br />    vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>,<br />    epoch: u64,<br />    round: u64,<br />    proposer: <b>address</b>,<br />    failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,<br />    previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64,<br />    randomness_seed: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> &#123;<br />    <b>let</b> epoch_interval &#61; <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(<br />        &amp;vm,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,<br />        epoch,<br />        round,<br />        proposer,<br />        failed_proposer_indices,<br />        previous_block_votes_bitvec,<br />        <a href="timestamp.md#0x1_timestamp">timestamp</a><br />    );<br />    <a href="randomness.md#0x1_randomness_on_new_block">randomness::on_new_block</a>(&amp;vm, epoch, round, randomness_seed);<br /><br />    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> &#45; <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>() &gt;&#61; epoch_interval) &#123;<br />        <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">reconfiguration_with_dkg::try_start</a>();<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_get_current_block_height"></a>

## Function `get_current_block_height`

Get the current block height


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_current_block_height">get_current_block_height</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_current_block_height">get_current_block_height</a>(): u64 <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework).height<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_emit_new_block_event"></a>

## Function `emit_new_block_event`

Emit the event and update height and global timestamp


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(vm: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, event_handle: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>&gt;, new_block_event: <a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>, new_block_event_v2: <a href="block.md#0x1_block_NewBlock">block::NewBlock</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(<br />    vm: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    event_handle: &amp;<b>mut</b> EventHandle&lt;<a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a>&gt;,<br />    new_block_event: <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a>,<br />    new_block_event_v2: <a href="block.md#0x1_block_NewBlock">NewBlock</a><br />) <b>acquires</b> <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(@aptos_framework)) &#123;<br />        <b>let</b> commit_history_ref &#61; <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(@aptos_framework);<br />        <b>let</b> idx &#61; commit_history_ref.next_idx;<br />        <b>if</b> (<a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_contains">table_with_length::contains</a>(&amp;commit_history_ref.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, idx)) &#123;<br />            <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&amp;<b>mut</b> commit_history_ref.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, idx);<br />        &#125;;<br />        <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&amp;<b>mut</b> commit_history_ref.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, idx, <b>copy</b> new_block_event);<br />        <b>spec</b> &#123;<br />            <b>assume</b> idx &#43; 1 &lt;&#61; MAX_U32;<br />        &#125;;<br />        commit_history_ref.next_idx &#61; (idx &#43; 1) % commit_history_ref.max_capacity;<br />    &#125;;<br />    <a href="timestamp.md#0x1_timestamp_update_global_time">timestamp::update_global_time</a>(vm, new_block_event.proposer, new_block_event.time_microseconds);<br />    <b>assert</b>!(<br />        <a href="event.md#0x1_event_counter">event::counter</a>(event_handle) &#61;&#61; new_block_event.height,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="block.md#0x1_block_ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT">ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT</a>),<br />    );<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(new_block_event_v2);<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a>&gt;(event_handle, new_block_event);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_emit_genesis_block_event"></a>

## Function `emit_genesis_block_event`

Emit a <code><a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a></code> event. This function will be invoked by genesis directly to generate the very first
reconfiguration event.


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_genesis_block_event">emit_genesis_block_event</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_genesis_block_event">emit_genesis_block_event</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> &#123;<br />    <b>let</b> block_metadata_ref &#61; <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);<br />    <b>let</b> genesis_id &#61; @0x0;<br />    <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(<br />        &amp;vm,<br />        &amp;<b>mut</b> block_metadata_ref.new_block_events,<br />        <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a> &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: genesis_id,<br />            epoch: 0,<br />            round: 0,<br />            height: 0,<br />            previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />            proposer: @vm_reserved,<br />            failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />            time_microseconds: 0,<br />        &#125;,<br />        <a href="block.md#0x1_block_NewBlock">NewBlock</a> &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: genesis_id,<br />            epoch: 0,<br />            round: 0,<br />            height: 0,<br />            previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />            proposer: @vm_reserved,<br />            failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />            time_microseconds: 0,<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_block_emit_writeset_block_event"></a>

## Function `emit_writeset_block_event`

Emit a <code><a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a></code> event. This function will be invoked by write set script directly to generate the
new block event for WriteSetPayload.


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_emit_writeset_block_event">emit_writeset_block_event</a>(vm_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fake_block_hash: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_emit_writeset_block_event">emit_writeset_block_event</a>(vm_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fake_block_hash: <b>address</b>) <b>acquires</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a>, <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm_signer);<br />    <b>let</b> block_metadata_ref &#61; <b>borrow_global_mut</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);<br />    block_metadata_ref.height &#61; <a href="event.md#0x1_event_counter">event::counter</a>(&amp;block_metadata_ref.new_block_events);<br /><br />    <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(<br />        vm_signer,<br />        &amp;<b>mut</b> block_metadata_ref.new_block_events,<br />        <a href="block.md#0x1_block_NewBlockEvent">NewBlockEvent</a> &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: fake_block_hash,<br />            epoch: <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>(),<br />            round: <a href="block.md#0x1_block_MAX_U64">MAX_U64</a>,<br />            height: block_metadata_ref.height,<br />            previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />            proposer: @vm_reserved,<br />            failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />            time_microseconds: <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>(),<br />        &#125;,<br />        <a href="block.md#0x1_block_NewBlock">NewBlock</a> &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: fake_block_hash,<br />            epoch: <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>(),<br />            round: <a href="block.md#0x1_block_MAX_U64">MAX_U64</a>,<br />            height: block_metadata_ref.height,<br />            previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />            proposer: @vm_reserved,<br />            failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />            time_microseconds: <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>(),<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



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
<td>During the module&apos;s initialization, it guarantees that the BlockResource resource moves under the Aptos framework account with initial values.</td>
<td>High</td>
<td>The initialize function is responsible for setting up the initial state of the module, ensuring that the following conditions are met (1) the BlockResource resource is created, indicating its existence within the module&apos;s context, and moved under the Aptos framework account, (2) the block height is set to zero during initialization, and (3) the epoch interval is greater than zero.</td>
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
<td>The update_epoch_interval_microsecs function asserts that new_epoch_interval is greater than zero and updates BlockResource&apos;s state.</td>
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


<pre><code><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);<br /><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="block.md#0x1_block_CommitHistory">CommitHistory</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_BlockResource"></a>

### Resource `BlockResource`


<pre><code><b>struct</b> <a href="block.md#0x1_block_BlockResource">BlockResource</a> <b>has</b> key<br /></code></pre>



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



<pre><code>// This enforces <a id="high-level-req-3.2" href="#high-level-req">high&#45;level requirement 3</a>:
<b>invariant</b> epoch_interval &gt; 0;<br /></code></pre>



<a id="@Specification_1_CommitHistory"></a>

### Resource `CommitHistory`


<pre><code><b>struct</b> <a href="block.md#0x1_block_CommitHistory">CommitHistory</a> <b>has</b> key<br /></code></pre>



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



<pre><code><b>invariant</b> max_capacity &gt; 0;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="block.md#0x1_block_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch_interval_microsecs: u64)<br /></code></pre>


The caller is aptos_framework.
The new_epoch_interval must be greater than 0.
The BlockResource is not under the caller before initializing.
The Account is not under the caller until the BlockResource is created for the caller.
Make sure The BlockResource under the caller existed after initializing.
The number of new events created does not exceed MAX_U64.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>include</b> <a href="block.md#0x1_block_Initialize">Initialize</a>;<br /><b>include</b> <a href="block.md#0x1_block_NewEventHandle">NewEventHandle</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num &#43; 2 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /></code></pre>



<a id="@Specification_1_update_epoch_interval_microsecs"></a>

### Function `update_epoch_interval_microsecs`


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_update_epoch_interval_microsecs">update_epoch_interval_microsecs</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_epoch_interval: u64)<br /></code></pre>


The caller is @aptos_framework.
The new_epoch_interval must be greater than 0.
The BlockResource existed under the @aptos_framework.


<pre><code>// This enforces <a id="high-level-req-3.1" href="#high-level-req">high&#45;level requirement 3</a>:
<b>include</b> <a href="block.md#0x1_block_UpdateEpochIntervalMicrosecs">UpdateEpochIntervalMicrosecs</a>;<br /></code></pre>




<a id="0x1_block_UpdateEpochIntervalMicrosecs"></a>


<pre><code><b>schema</b> <a href="block.md#0x1_block_UpdateEpochIntervalMicrosecs">UpdateEpochIntervalMicrosecs</a> &#123;<br />aptos_framework: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />new_epoch_interval: u64;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br />// This enforces <a id="high-level-req-2.2" href="#high-level-req">high&#45;level requirement 2</a>:
    <b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> new_epoch_interval &#61;&#61; 0;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(addr);<br /><b>let</b> <b>post</b> block_resource &#61; <b>global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(addr);<br /><b>ensures</b> block_resource.epoch_interval &#61;&#61; new_epoch_interval;<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_epoch_interval_secs"></a>

### Function `get_epoch_interval_secs`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_epoch_interval_secs">get_epoch_interval_secs</a>(): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_block_prologue_common"></a>

### Function `block_prologue_common`


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_common">block_prologue_common</a>(vm: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 1000;<br /><b>include</b> <a href="block.md#0x1_block_BlockRequirement">BlockRequirement</a>;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_block_prologue"></a>

### Function `block_prologue`


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue">block_prologue</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 1000;<br /><b>requires</b> <a href="timestamp.md#0x1_timestamp">timestamp</a> &gt;&#61; <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>();<br /><b>include</b> <a href="block.md#0x1_block_BlockRequirement">BlockRequirement</a>;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_block_prologue_ext"></a>

### Function `block_prologue_ext`


<pre><code><b>fun</b> <a href="block.md#0x1_block_block_prologue_ext">block_prologue_ext</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, epoch: u64, round: u64, proposer: <b>address</b>, failed_proposer_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_block_votes_bitvec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64, randomness_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 1000;<br /><b>requires</b> <a href="timestamp.md#0x1_timestamp">timestamp</a> &gt;&#61; <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>();<br /><b>include</b> <a href="block.md#0x1_block_BlockRequirement">BlockRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_get_current_block_height"></a>

### Function `get_current_block_height`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="block.md#0x1_block_get_current_block_height">get_current_block_height</a>(): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_emit_new_block_event"></a>

### Function `emit_new_block_event`


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_new_block_event">emit_new_block_event</a>(vm: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, event_handle: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>&gt;, new_block_event: <a href="block.md#0x1_block_NewBlockEvent">block::NewBlockEvent</a>, new_block_event_v2: <a href="block.md#0x1_block_NewBlock">block::NewBlock</a>)<br /></code></pre>




<pre><code><b>let</b> proposer &#61; new_block_event.proposer;<br /><b>let</b> <a href="timestamp.md#0x1_timestamp">timestamp</a> &#61; new_block_event.time_microseconds;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>requires</b> <a href="system_addresses.md#0x1_system_addresses_is_vm">system_addresses::is_vm</a>(vm);<br /><b>requires</b> (proposer &#61;&#61; @vm_reserved) &#61;&#61;&gt; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &#61;&#61; <a href="timestamp.md#0x1_timestamp">timestamp</a>);<br /><b>requires</b> (proposer !&#61; @vm_reserved) &#61;&#61;&gt; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &lt; <a href="timestamp.md#0x1_timestamp">timestamp</a>);<br />// This enforces <a id="high-level-req-5" href="#high-level-req">high&#45;level requirement 5</a>:
<b>requires</b> <a href="event.md#0x1_event_counter">event::counter</a>(event_handle) &#61;&#61; new_block_event.height;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_emit_genesis_block_event"></a>

### Function `emit_genesis_block_event`


<pre><code><b>fun</b> <a href="block.md#0x1_block_emit_genesis_block_event">emit_genesis_block_event</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>requires</b> <a href="system_addresses.md#0x1_system_addresses_is_vm">system_addresses::is_vm</a>(vm);<br /><b>requires</b> <a href="event.md#0x1_event_counter">event::counter</a>(<b>global</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework).new_block_events) &#61;&#61; 0;<br /><b>requires</b> (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &#61;&#61; 0);<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_emit_writeset_block_event"></a>

### Function `emit_writeset_block_event`


<pre><code><b>public</b> <b>fun</b> <a href="block.md#0x1_block_emit_writeset_block_event">emit_writeset_block_event</a>(vm_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fake_block_hash: <b>address</b>)<br /></code></pre>


The caller is @vm_reserved.
The BlockResource existed under the @aptos_framework.
The Configuration existed under the @aptos_framework.
The CurrentTimeMicroseconds existed under the @aptos_framework.


<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="block.md#0x1_block_EmitWritesetBlockEvent">EmitWritesetBlockEvent</a>;<br /></code></pre>




<a id="0x1_block_EmitWritesetBlockEvent"></a>


<pre><code><b>schema</b> <a href="block.md#0x1_block_EmitWritesetBlockEvent">EmitWritesetBlockEvent</a> &#123;<br />vm_signer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(vm_signer);<br /><b>aborts_if</b> addr !&#61; @vm_reserved;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">BlockResource</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">reconfiguration::Configuration</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
