
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


<pre><code>use 0x1::account;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::option;
use 0x1::randomness;
use 0x1::reconfiguration;
use 0x1::reconfiguration_with_dkg;
use 0x1::stake;
use 0x1::state_storage;
use 0x1::system_addresses;
use 0x1::table_with_length;
use 0x1::timestamp;
use 0x1::transaction_fee;
</code></pre>



<a id="0x1_block_BlockResource"></a>

## Resource `BlockResource`

Should be in-sync with BlockResource rust struct in new_block.rs


<pre><code>struct BlockResource has key
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
<code>new_block_events: event::EventHandle&lt;block::NewBlockEvent&gt;</code>
</dt>
<dd>
 Handle where events with the time of new blocks are emitted
</dd>
<dt>
<code>update_epoch_interval_events: event::EventHandle&lt;block::UpdateEpochIntervalEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_block_CommitHistory"></a>

## Resource `CommitHistory`

Store new block events as a move resource, internally using a circular buffer.


<pre><code>struct CommitHistory has key
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
<code>table: table_with_length::TableWithLength&lt;u32, block::NewBlockEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_block_NewBlockEvent"></a>

## Struct `NewBlockEvent`

Should be in-sync with NewBlockEvent rust struct in new_block.rs


<pre><code>struct NewBlockEvent has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hash: address</code>
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
<code>previous_block_votes_bitvec: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: address</code>
</dt>
<dd>

</dd>
<dt>
<code>failed_proposer_indices: vector&lt;u64&gt;</code>
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


<pre><code>struct UpdateEpochIntervalEvent has drop, store
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


<pre><code>&#35;[event]
struct NewBlock has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hash: address</code>
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
<code>previous_block_votes_bitvec: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: address</code>
</dt>
<dd>

</dd>
<dt>
<code>failed_proposer_indices: vector&lt;u64&gt;</code>
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


<pre><code>&#35;[event]
struct UpdateEpochInterval has drop, store
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



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_block_EINVALID_PROPOSER"></a>

An invalid proposer was provided. Expected the proposer to be the VM or an active validator.


<pre><code>const EINVALID_PROPOSER: u64 &#61; 2;
</code></pre>



<a id="0x1_block_ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT"></a>

The number of new block events does not equal the current block height.


<pre><code>const ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT: u64 &#61; 1;
</code></pre>



<a id="0x1_block_EZERO_EPOCH_INTERVAL"></a>

Epoch interval cannot be 0.


<pre><code>const EZERO_EPOCH_INTERVAL: u64 &#61; 3;
</code></pre>



<a id="0x1_block_EZERO_MAX_CAPACITY"></a>

The maximum capacity of the commit history cannot be 0.


<pre><code>const EZERO_MAX_CAPACITY: u64 &#61; 3;
</code></pre>



<a id="0x1_block_initialize"></a>

## Function `initialize`

This can only be called during Genesis.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, epoch_interval_microsecs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, epoch_interval_microsecs: u64) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(epoch_interval_microsecs &gt; 0, error::invalid_argument(EZERO_EPOCH_INTERVAL));

    move_to&lt;CommitHistory&gt;(aptos_framework, CommitHistory &#123;
        max_capacity: 2000,
        next_idx: 0,
        table: table_with_length::new(),
    &#125;);

    move_to&lt;BlockResource&gt;(
        aptos_framework,
        BlockResource &#123;
            height: 0,
            epoch_interval: epoch_interval_microsecs,
            new_block_events: account::new_event_handle&lt;NewBlockEvent&gt;(aptos_framework),
            update_epoch_interval_events: account::new_event_handle&lt;UpdateEpochIntervalEvent&gt;(aptos_framework),
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_block_initialize_commit_history"></a>

## Function `initialize_commit_history`

Initialize the commit history resource if it's not in genesis.


<pre><code>public fun initialize_commit_history(fx: &amp;signer, max_capacity: u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_commit_history(fx: &amp;signer, max_capacity: u32) &#123;
    assert!(max_capacity &gt; 0, error::invalid_argument(EZERO_MAX_CAPACITY));
    move_to&lt;CommitHistory&gt;(fx, CommitHistory &#123;
        max_capacity,
        next_idx: 0,
        table: table_with_length::new(),
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_block_update_epoch_interval_microsecs"></a>

## Function `update_epoch_interval_microsecs`

Update the epoch interval.
Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code>public fun update_epoch_interval_microsecs(aptos_framework: &amp;signer, new_epoch_interval: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_epoch_interval_microsecs(
    aptos_framework: &amp;signer,
    new_epoch_interval: u64,
) acquires BlockResource &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(new_epoch_interval &gt; 0, error::invalid_argument(EZERO_EPOCH_INTERVAL));

    let block_resource &#61; borrow_global_mut&lt;BlockResource&gt;(@aptos_framework);
    let old_epoch_interval &#61; block_resource.epoch_interval;
    block_resource.epoch_interval &#61; new_epoch_interval;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            UpdateEpochInterval &#123; old_epoch_interval, new_epoch_interval &#125;,
        );
    &#125;;
    event::emit_event&lt;UpdateEpochIntervalEvent&gt;(
        &amp;mut block_resource.update_epoch_interval_events,
        UpdateEpochIntervalEvent &#123; old_epoch_interval, new_epoch_interval &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_block_get_epoch_interval_secs"></a>

## Function `get_epoch_interval_secs`

Return epoch interval in seconds.


<pre><code>&#35;[view]
public fun get_epoch_interval_secs(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_epoch_interval_secs(): u64 acquires BlockResource &#123;
    borrow_global&lt;BlockResource&gt;(@aptos_framework).epoch_interval / 1000000
&#125;
</code></pre>



</details>

<a id="0x1_block_block_prologue_common"></a>

## Function `block_prologue_common`



<pre><code>fun block_prologue_common(vm: &amp;signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun block_prologue_common(
    vm: &amp;signer,
    hash: address,
    epoch: u64,
    round: u64,
    proposer: address,
    failed_proposer_indices: vector&lt;u64&gt;,
    previous_block_votes_bitvec: vector&lt;u8&gt;,
    timestamp: u64
): u64 acquires BlockResource, CommitHistory &#123;
    // Operational constraint: can only be invoked by the VM.
    system_addresses::assert_vm(vm);

    // Blocks can only be produced by a valid proposer or by the VM itself for Nil blocks (no user txs).
    assert!(
        proposer &#61;&#61; @vm_reserved &#124;&#124; stake::is_current_epoch_validator(proposer),
        error::permission_denied(EINVALID_PROPOSER),
    );

    let proposer_index &#61; option::none();
    if (proposer !&#61; @vm_reserved) &#123;
        proposer_index &#61; option::some(stake::get_validator_index(proposer));
    &#125;;

    let block_metadata_ref &#61; borrow_global_mut&lt;BlockResource&gt;(@aptos_framework);
    block_metadata_ref.height &#61; event::counter(&amp;block_metadata_ref.new_block_events);

    // Emit both event v1 and v2 for compatibility. Eventually only module events will be kept.
    let new_block_event &#61; NewBlockEvent &#123;
        hash,
        epoch,
        round,
        height: block_metadata_ref.height,
        previous_block_votes_bitvec,
        proposer,
        failed_proposer_indices,
        time_microseconds: timestamp,
    &#125;;
    let new_block_event_v2 &#61; NewBlock &#123;
        hash,
        epoch,
        round,
        height: block_metadata_ref.height,
        previous_block_votes_bitvec,
        proposer,
        failed_proposer_indices,
        time_microseconds: timestamp,
    &#125;;
    emit_new_block_event(vm, &amp;mut block_metadata_ref.new_block_events, new_block_event, new_block_event_v2);

    if (features::collect_and_distribute_gas_fees()) &#123;
        // Assign the fees collected from the previous block to the previous block proposer.
        // If for any reason the fees cannot be assigned, this function burns the collected coins.
        transaction_fee::process_collected_fees();
        // Set the proposer of this block as the receiver of the fees, so that the fees for this
        // block are assigned to the right account.
        transaction_fee::register_proposer_for_fee_collection(proposer);
    &#125;;

    // Performance scores have to be updated before the epoch transition as the transaction that triggers the
    // transition is the last block in the previous epoch.
    stake::update_performance_statistics(proposer_index, failed_proposer_indices);
    state_storage::on_new_block(reconfiguration::current_epoch());

    block_metadata_ref.epoch_interval
&#125;
</code></pre>



</details>

<a id="0x1_block_block_prologue"></a>

## Function `block_prologue`

Set the metadata for the current block.
The runtime always runs this before executing the transactions in a block.


<pre><code>fun block_prologue(vm: signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun block_prologue(
    vm: signer,
    hash: address,
    epoch: u64,
    round: u64,
    proposer: address,
    failed_proposer_indices: vector&lt;u64&gt;,
    previous_block_votes_bitvec: vector&lt;u8&gt;,
    timestamp: u64
) acquires BlockResource, CommitHistory &#123;
    let epoch_interval &#61; block_prologue_common(&amp;vm, hash, epoch, round, proposer, failed_proposer_indices, previous_block_votes_bitvec, timestamp);
    randomness::on_new_block(&amp;vm, epoch, round, option::none());
    if (timestamp &#45; reconfiguration::last_reconfiguration_time() &gt;&#61; epoch_interval) &#123;
        reconfiguration::reconfigure();
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_block_block_prologue_ext"></a>

## Function `block_prologue_ext`

<code>block_prologue()</code> but trigger reconfiguration with DKG after epoch timed out.


<pre><code>fun block_prologue_ext(vm: signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64, randomness_seed: option::Option&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun block_prologue_ext(
    vm: signer,
    hash: address,
    epoch: u64,
    round: u64,
    proposer: address,
    failed_proposer_indices: vector&lt;u64&gt;,
    previous_block_votes_bitvec: vector&lt;u8&gt;,
    timestamp: u64,
    randomness_seed: Option&lt;vector&lt;u8&gt;&gt;,
) acquires BlockResource, CommitHistory &#123;
    let epoch_interval &#61; block_prologue_common(
        &amp;vm,
        hash,
        epoch,
        round,
        proposer,
        failed_proposer_indices,
        previous_block_votes_bitvec,
        timestamp
    );
    randomness::on_new_block(&amp;vm, epoch, round, randomness_seed);

    if (timestamp &#45; reconfiguration::last_reconfiguration_time() &gt;&#61; epoch_interval) &#123;
        reconfiguration_with_dkg::try_start();
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_block_get_current_block_height"></a>

## Function `get_current_block_height`

Get the current block height


<pre><code>&#35;[view]
public fun get_current_block_height(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_current_block_height(): u64 acquires BlockResource &#123;
    borrow_global&lt;BlockResource&gt;(@aptos_framework).height
&#125;
</code></pre>



</details>

<a id="0x1_block_emit_new_block_event"></a>

## Function `emit_new_block_event`

Emit the event and update height and global timestamp


<pre><code>fun emit_new_block_event(vm: &amp;signer, event_handle: &amp;mut event::EventHandle&lt;block::NewBlockEvent&gt;, new_block_event: block::NewBlockEvent, new_block_event_v2: block::NewBlock)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun emit_new_block_event(
    vm: &amp;signer,
    event_handle: &amp;mut EventHandle&lt;NewBlockEvent&gt;,
    new_block_event: NewBlockEvent,
    new_block_event_v2: NewBlock
) acquires CommitHistory &#123;
    if (exists&lt;CommitHistory&gt;(@aptos_framework)) &#123;
        let commit_history_ref &#61; borrow_global_mut&lt;CommitHistory&gt;(@aptos_framework);
        let idx &#61; commit_history_ref.next_idx;
        if (table_with_length::contains(&amp;commit_history_ref.table, idx)) &#123;
            table_with_length::remove(&amp;mut commit_history_ref.table, idx);
        &#125;;
        table_with_length::add(&amp;mut commit_history_ref.table, idx, copy new_block_event);
        spec &#123;
            assume idx &#43; 1 &lt;&#61; MAX_U32;
        &#125;;
        commit_history_ref.next_idx &#61; (idx &#43; 1) % commit_history_ref.max_capacity;
    &#125;;
    timestamp::update_global_time(vm, new_block_event.proposer, new_block_event.time_microseconds);
    assert!(
        event::counter(event_handle) &#61;&#61; new_block_event.height,
        error::invalid_argument(ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT),
    );
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(new_block_event_v2);
    &#125;;
    event::emit_event&lt;NewBlockEvent&gt;(event_handle, new_block_event);
&#125;
</code></pre>



</details>

<a id="0x1_block_emit_genesis_block_event"></a>

## Function `emit_genesis_block_event`

Emit a <code>NewBlockEvent</code> event. This function will be invoked by genesis directly to generate the very first
reconfiguration event.


<pre><code>fun emit_genesis_block_event(vm: signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun emit_genesis_block_event(vm: signer) acquires BlockResource, CommitHistory &#123;
    let block_metadata_ref &#61; borrow_global_mut&lt;BlockResource&gt;(@aptos_framework);
    let genesis_id &#61; @0x0;
    emit_new_block_event(
        &amp;vm,
        &amp;mut block_metadata_ref.new_block_events,
        NewBlockEvent &#123;
            hash: genesis_id,
            epoch: 0,
            round: 0,
            height: 0,
            previous_block_votes_bitvec: vector::empty(),
            proposer: @vm_reserved,
            failed_proposer_indices: vector::empty(),
            time_microseconds: 0,
        &#125;,
        NewBlock &#123;
            hash: genesis_id,
            epoch: 0,
            round: 0,
            height: 0,
            previous_block_votes_bitvec: vector::empty(),
            proposer: @vm_reserved,
            failed_proposer_indices: vector::empty(),
            time_microseconds: 0,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_block_emit_writeset_block_event"></a>

## Function `emit_writeset_block_event`

Emit a <code>NewBlockEvent</code> event. This function will be invoked by write set script directly to generate the
new block event for WriteSetPayload.


<pre><code>public fun emit_writeset_block_event(vm_signer: &amp;signer, fake_block_hash: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun emit_writeset_block_event(vm_signer: &amp;signer, fake_block_hash: address) acquires BlockResource, CommitHistory &#123;
    system_addresses::assert_vm(vm_signer);
    let block_metadata_ref &#61; borrow_global_mut&lt;BlockResource&gt;(@aptos_framework);
    block_metadata_ref.height &#61; event::counter(&amp;block_metadata_ref.new_block_events);

    emit_new_block_event(
        vm_signer,
        &amp;mut block_metadata_ref.new_block_events,
        NewBlockEvent &#123;
            hash: fake_block_hash,
            epoch: reconfiguration::current_epoch(),
            round: MAX_U64,
            height: block_metadata_ref.height,
            previous_block_votes_bitvec: vector::empty(),
            proposer: @vm_reserved,
            failed_proposer_indices: vector::empty(),
            time_microseconds: timestamp::now_microseconds(),
        &#125;,
        NewBlock &#123;
            hash: fake_block_hash,
            epoch: reconfiguration::current_epoch(),
            round: MAX_U64,
            height: block_metadata_ref.height,
            previous_block_votes_bitvec: vector::empty(),
            proposer: @vm_reserved,
            failed_proposer_indices: vector::empty(),
            time_microseconds: timestamp::now_microseconds(),
        &#125;
    );
&#125;
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


<pre><code>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;BlockResource&gt;(@aptos_framework);
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;CommitHistory&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_BlockResource"></a>

### Resource `BlockResource`


<pre><code>struct BlockResource has key
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
<code>new_block_events: event::EventHandle&lt;block::NewBlockEvent&gt;</code>
</dt>
<dd>
 Handle where events with the time of new blocks are emitted
</dd>
<dt>
<code>update_epoch_interval_events: event::EventHandle&lt;block::UpdateEpochIntervalEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>// This enforces <a id="high-level-req-3.2" href="#high-level-req">high-level requirement 3</a>:
invariant epoch_interval &gt; 0;
</code></pre>



<a id="@Specification_1_CommitHistory"></a>

### Resource `CommitHistory`


<pre><code>struct CommitHistory has key
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
<code>table: table_with_length::TableWithLength&lt;u32, block::NewBlockEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant max_capacity &gt; 0;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, epoch_interval_microsecs: u64)
</code></pre>


The caller is aptos_framework.
The new_epoch_interval must be greater than 0.
The BlockResource is not under the caller before initializing.
The Account is not under the caller until the BlockResource is created for the caller.
Make sure The BlockResource under the caller existed after initializing.
The number of new events created does not exceed MAX_U64.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
include Initialize;
include NewEventHandle;
let addr &#61; signer::address_of(aptos_framework);
let account &#61; global&lt;account::Account&gt;(addr);
aborts_if account.guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;
</code></pre>



<a id="@Specification_1_update_epoch_interval_microsecs"></a>

### Function `update_epoch_interval_microsecs`


<pre><code>public fun update_epoch_interval_microsecs(aptos_framework: &amp;signer, new_epoch_interval: u64)
</code></pre>


The caller is @aptos_framework.
The new_epoch_interval must be greater than 0.
The BlockResource existed under the @aptos_framework.


<pre><code>// This enforces <a id="high-level-req-3.1" href="#high-level-req">high-level requirement 3</a>:
include UpdateEpochIntervalMicrosecs;
</code></pre>




<a id="0x1_block_UpdateEpochIntervalMicrosecs"></a>


<pre><code>schema UpdateEpochIntervalMicrosecs &#123;
    aptos_framework: signer;
    new_epoch_interval: u64;
    let addr &#61; signer::address_of(aptos_framework);
    // This enforces <a id="high-level-req-2.2" href="#high-level-req">high-level requirement 2</a>:
    aborts_if addr !&#61; @aptos_framework;
    aborts_if new_epoch_interval &#61;&#61; 0;
    aborts_if !exists&lt;BlockResource&gt;(addr);
    let post block_resource &#61; global&lt;BlockResource&gt;(addr);
    ensures block_resource.epoch_interval &#61;&#61; new_epoch_interval;
&#125;
</code></pre>



<a id="@Specification_1_get_epoch_interval_secs"></a>

### Function `get_epoch_interval_secs`


<pre><code>&#35;[view]
public fun get_epoch_interval_secs(): u64
</code></pre>




<pre><code>aborts_if !exists&lt;BlockResource&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_block_prologue_common"></a>

### Function `block_prologue_common`


<pre><code>fun block_prologue_common(vm: &amp;signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64): u64
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;
include BlockRequirement;
aborts_if false;
</code></pre>



<a id="@Specification_1_block_prologue"></a>

### Function `block_prologue`


<pre><code>fun block_prologue(vm: signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;
requires timestamp &gt;&#61; reconfiguration::last_reconfiguration_time();
include BlockRequirement;
aborts_if false;
</code></pre>



<a id="@Specification_1_block_prologue_ext"></a>

### Function `block_prologue_ext`


<pre><code>fun block_prologue_ext(vm: signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64, randomness_seed: option::Option&lt;vector&lt;u8&gt;&gt;)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;
requires timestamp &gt;&#61; reconfiguration::last_reconfiguration_time();
include BlockRequirement;
include stake::ResourceRequirement;
include stake::GetReconfigStartTimeRequirement;
aborts_if false;
</code></pre>



<a id="@Specification_1_get_current_block_height"></a>

### Function `get_current_block_height`


<pre><code>&#35;[view]
public fun get_current_block_height(): u64
</code></pre>




<pre><code>aborts_if !exists&lt;BlockResource&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_emit_new_block_event"></a>

### Function `emit_new_block_event`


<pre><code>fun emit_new_block_event(vm: &amp;signer, event_handle: &amp;mut event::EventHandle&lt;block::NewBlockEvent&gt;, new_block_event: block::NewBlockEvent, new_block_event_v2: block::NewBlock)
</code></pre>




<pre><code>let proposer &#61; new_block_event.proposer;
let timestamp &#61; new_block_event.time_microseconds;
requires chain_status::is_operating();
requires system_addresses::is_vm(vm);
requires (proposer &#61;&#61; @vm_reserved) &#61;&#61;&gt; (timestamp::spec_now_microseconds() &#61;&#61; timestamp);
requires (proposer !&#61; @vm_reserved) &#61;&#61;&gt; (timestamp::spec_now_microseconds() &lt; timestamp);
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
requires event::counter(event_handle) &#61;&#61; new_block_event.height;
aborts_if false;
</code></pre>



<a id="@Specification_1_emit_genesis_block_event"></a>

### Function `emit_genesis_block_event`


<pre><code>fun emit_genesis_block_event(vm: signer)
</code></pre>




<pre><code>requires chain_status::is_operating();
requires system_addresses::is_vm(vm);
requires event::counter(global&lt;BlockResource&gt;(@aptos_framework).new_block_events) &#61;&#61; 0;
requires (timestamp::spec_now_microseconds() &#61;&#61; 0);
aborts_if false;
</code></pre>



<a id="@Specification_1_emit_writeset_block_event"></a>

### Function `emit_writeset_block_event`


<pre><code>public fun emit_writeset_block_event(vm_signer: &amp;signer, fake_block_hash: address)
</code></pre>


The caller is @vm_reserved.
The BlockResource existed under the @aptos_framework.
The Configuration existed under the @aptos_framework.
The CurrentTimeMicroseconds existed under the @aptos_framework.


<pre><code>requires chain_status::is_operating();
include EmitWritesetBlockEvent;
</code></pre>




<a id="0x1_block_EmitWritesetBlockEvent"></a>


<pre><code>schema EmitWritesetBlockEvent &#123;
    vm_signer: signer;
    let addr &#61; signer::address_of(vm_signer);
    aborts_if addr !&#61; @vm_reserved;
    aborts_if !exists&lt;BlockResource&gt;(@aptos_framework);
    aborts_if !exists&lt;reconfiguration::Configuration&gt;(@aptos_framework);
    aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
