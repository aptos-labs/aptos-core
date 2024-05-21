
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


<pre><code>use 0x1::account;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::option;<br/>use 0x1::randomness;<br/>use 0x1::reconfiguration;<br/>use 0x1::reconfiguration_with_dkg;<br/>use 0x1::stake;<br/>use 0x1::state_storage;<br/>use 0x1::system_addresses;<br/>use 0x1::table_with_length;<br/>use 0x1::timestamp;<br/>use 0x1::transaction_fee;<br/></code></pre>



<a id="0x1_block_BlockResource"></a>

## Resource `BlockResource`

Should be in-sync with BlockResource rust struct in new_block.rs


<pre><code>struct BlockResource has key<br/></code></pre>



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


<pre><code>struct CommitHistory has key<br/></code></pre>



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


<pre><code>struct NewBlockEvent has copy, drop, store<br/></code></pre>



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


<pre><code>struct UpdateEpochIntervalEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct NewBlock has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct UpdateEpochInterval has drop, store<br/></code></pre>



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



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_block_EINVALID_PROPOSER"></a>

An invalid proposer was provided. Expected the proposer to be the VM or an active validator.


<pre><code>const EINVALID_PROPOSER: u64 &#61; 2;<br/></code></pre>



<a id="0x1_block_ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT"></a>

The number of new block events does not equal the current block height.


<pre><code>const ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT: u64 &#61; 1;<br/></code></pre>



<a id="0x1_block_EZERO_EPOCH_INTERVAL"></a>

Epoch interval cannot be 0.


<pre><code>const EZERO_EPOCH_INTERVAL: u64 &#61; 3;<br/></code></pre>



<a id="0x1_block_EZERO_MAX_CAPACITY"></a>

The maximum capacity of the commit history cannot be 0.


<pre><code>const EZERO_MAX_CAPACITY: u64 &#61; 3;<br/></code></pre>



<a id="0x1_block_initialize"></a>

## Function `initialize`

This can only be called during Genesis.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, epoch_interval_microsecs: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, epoch_interval_microsecs: u64) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(epoch_interval_microsecs &gt; 0, error::invalid_argument(EZERO_EPOCH_INTERVAL));<br/><br/>    move_to&lt;CommitHistory&gt;(aptos_framework, CommitHistory &#123;<br/>        max_capacity: 2000,<br/>        next_idx: 0,<br/>        table: table_with_length::new(),<br/>    &#125;);<br/><br/>    move_to&lt;BlockResource&gt;(<br/>        aptos_framework,<br/>        BlockResource &#123;<br/>            height: 0,<br/>            epoch_interval: epoch_interval_microsecs,<br/>            new_block_events: account::new_event_handle&lt;NewBlockEvent&gt;(aptos_framework),<br/>            update_epoch_interval_events: account::new_event_handle&lt;UpdateEpochIntervalEvent&gt;(aptos_framework),<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_initialize_commit_history"></a>

## Function `initialize_commit_history`

Initialize the commit history resource if it's not in genesis.


<pre><code>public fun initialize_commit_history(fx: &amp;signer, max_capacity: u32)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_commit_history(fx: &amp;signer, max_capacity: u32) &#123;<br/>    assert!(max_capacity &gt; 0, error::invalid_argument(EZERO_MAX_CAPACITY));<br/>    move_to&lt;CommitHistory&gt;(fx, CommitHistory &#123;<br/>        max_capacity,<br/>        next_idx: 0,<br/>        table: table_with_length::new(),<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_update_epoch_interval_microsecs"></a>

## Function `update_epoch_interval_microsecs`

Update the epoch interval.
Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code>public fun update_epoch_interval_microsecs(aptos_framework: &amp;signer, new_epoch_interval: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_epoch_interval_microsecs(<br/>    aptos_framework: &amp;signer,<br/>    new_epoch_interval: u64,<br/>) acquires BlockResource &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(new_epoch_interval &gt; 0, error::invalid_argument(EZERO_EPOCH_INTERVAL));<br/><br/>    let block_resource &#61; borrow_global_mut&lt;BlockResource&gt;(@aptos_framework);<br/>    let old_epoch_interval &#61; block_resource.epoch_interval;<br/>    block_resource.epoch_interval &#61; new_epoch_interval;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            UpdateEpochInterval &#123; old_epoch_interval, new_epoch_interval &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;UpdateEpochIntervalEvent&gt;(<br/>        &amp;mut block_resource.update_epoch_interval_events,<br/>        UpdateEpochIntervalEvent &#123; old_epoch_interval, new_epoch_interval &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_get_epoch_interval_secs"></a>

## Function `get_epoch_interval_secs`

Return epoch interval in seconds.


<pre><code>&#35;[view]<br/>public fun get_epoch_interval_secs(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_epoch_interval_secs(): u64 acquires BlockResource &#123;<br/>    borrow_global&lt;BlockResource&gt;(@aptos_framework).epoch_interval / 1000000<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_block_prologue_common"></a>

## Function `block_prologue_common`



<pre><code>fun block_prologue_common(vm: &amp;signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun block_prologue_common(<br/>    vm: &amp;signer,<br/>    hash: address,<br/>    epoch: u64,<br/>    round: u64,<br/>    proposer: address,<br/>    failed_proposer_indices: vector&lt;u64&gt;,<br/>    previous_block_votes_bitvec: vector&lt;u8&gt;,<br/>    timestamp: u64<br/>): u64 acquires BlockResource, CommitHistory &#123;<br/>    // Operational constraint: can only be invoked by the VM.<br/>    system_addresses::assert_vm(vm);<br/><br/>    // Blocks can only be produced by a valid proposer or by the VM itself for Nil blocks (no user txs).<br/>    assert!(<br/>        proposer &#61;&#61; @vm_reserved &#124;&#124; stake::is_current_epoch_validator(proposer),<br/>        error::permission_denied(EINVALID_PROPOSER),<br/>    );<br/><br/>    let proposer_index &#61; option::none();<br/>    if (proposer !&#61; @vm_reserved) &#123;<br/>        proposer_index &#61; option::some(stake::get_validator_index(proposer));<br/>    &#125;;<br/><br/>    let block_metadata_ref &#61; borrow_global_mut&lt;BlockResource&gt;(@aptos_framework);<br/>    block_metadata_ref.height &#61; event::counter(&amp;block_metadata_ref.new_block_events);<br/><br/>    // Emit both event v1 and v2 for compatibility. Eventually only module events will be kept.<br/>    let new_block_event &#61; NewBlockEvent &#123;<br/>        hash,<br/>        epoch,<br/>        round,<br/>        height: block_metadata_ref.height,<br/>        previous_block_votes_bitvec,<br/>        proposer,<br/>        failed_proposer_indices,<br/>        time_microseconds: timestamp,<br/>    &#125;;<br/>    let new_block_event_v2 &#61; NewBlock &#123;<br/>        hash,<br/>        epoch,<br/>        round,<br/>        height: block_metadata_ref.height,<br/>        previous_block_votes_bitvec,<br/>        proposer,<br/>        failed_proposer_indices,<br/>        time_microseconds: timestamp,<br/>    &#125;;<br/>    emit_new_block_event(vm, &amp;mut block_metadata_ref.new_block_events, new_block_event, new_block_event_v2);<br/><br/>    if (features::collect_and_distribute_gas_fees()) &#123;<br/>        // Assign the fees collected from the previous block to the previous block proposer.<br/>        // If for any reason the fees cannot be assigned, this function burns the collected coins.<br/>        transaction_fee::process_collected_fees();<br/>        // Set the proposer of this block as the receiver of the fees, so that the fees for this<br/>        // block are assigned to the right account.<br/>        transaction_fee::register_proposer_for_fee_collection(proposer);<br/>    &#125;;<br/><br/>    // Performance scores have to be updated before the epoch transition as the transaction that triggers the<br/>    // transition is the last block in the previous epoch.<br/>    stake::update_performance_statistics(proposer_index, failed_proposer_indices);<br/>    state_storage::on_new_block(reconfiguration::current_epoch());<br/><br/>    block_metadata_ref.epoch_interval<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_block_prologue"></a>

## Function `block_prologue`

Set the metadata for the current block.
The runtime always runs this before executing the transactions in a block.


<pre><code>fun block_prologue(vm: signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun block_prologue(<br/>    vm: signer,<br/>    hash: address,<br/>    epoch: u64,<br/>    round: u64,<br/>    proposer: address,<br/>    failed_proposer_indices: vector&lt;u64&gt;,<br/>    previous_block_votes_bitvec: vector&lt;u8&gt;,<br/>    timestamp: u64<br/>) acquires BlockResource, CommitHistory &#123;<br/>    let epoch_interval &#61; block_prologue_common(&amp;vm, hash, epoch, round, proposer, failed_proposer_indices, previous_block_votes_bitvec, timestamp);<br/>    randomness::on_new_block(&amp;vm, epoch, round, option::none());<br/>    if (timestamp &#45; reconfiguration::last_reconfiguration_time() &gt;&#61; epoch_interval) &#123;<br/>        reconfiguration::reconfigure();<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_block_prologue_ext"></a>

## Function `block_prologue_ext`

<code>block_prologue()</code> but trigger reconfiguration with DKG after epoch timed out.


<pre><code>fun block_prologue_ext(vm: signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64, randomness_seed: option::Option&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun block_prologue_ext(<br/>    vm: signer,<br/>    hash: address,<br/>    epoch: u64,<br/>    round: u64,<br/>    proposer: address,<br/>    failed_proposer_indices: vector&lt;u64&gt;,<br/>    previous_block_votes_bitvec: vector&lt;u8&gt;,<br/>    timestamp: u64,<br/>    randomness_seed: Option&lt;vector&lt;u8&gt;&gt;,<br/>) acquires BlockResource, CommitHistory &#123;<br/>    let epoch_interval &#61; block_prologue_common(<br/>        &amp;vm,<br/>        hash,<br/>        epoch,<br/>        round,<br/>        proposer,<br/>        failed_proposer_indices,<br/>        previous_block_votes_bitvec,<br/>        timestamp<br/>    );<br/>    randomness::on_new_block(&amp;vm, epoch, round, randomness_seed);<br/><br/>    if (timestamp &#45; reconfiguration::last_reconfiguration_time() &gt;&#61; epoch_interval) &#123;<br/>        reconfiguration_with_dkg::try_start();<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_get_current_block_height"></a>

## Function `get_current_block_height`

Get the current block height


<pre><code>&#35;[view]<br/>public fun get_current_block_height(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_current_block_height(): u64 acquires BlockResource &#123;<br/>    borrow_global&lt;BlockResource&gt;(@aptos_framework).height<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_emit_new_block_event"></a>

## Function `emit_new_block_event`

Emit the event and update height and global timestamp


<pre><code>fun emit_new_block_event(vm: &amp;signer, event_handle: &amp;mut event::EventHandle&lt;block::NewBlockEvent&gt;, new_block_event: block::NewBlockEvent, new_block_event_v2: block::NewBlock)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun emit_new_block_event(<br/>    vm: &amp;signer,<br/>    event_handle: &amp;mut EventHandle&lt;NewBlockEvent&gt;,<br/>    new_block_event: NewBlockEvent,<br/>    new_block_event_v2: NewBlock<br/>) acquires CommitHistory &#123;<br/>    if (exists&lt;CommitHistory&gt;(@aptos_framework)) &#123;<br/>        let commit_history_ref &#61; borrow_global_mut&lt;CommitHistory&gt;(@aptos_framework);<br/>        let idx &#61; commit_history_ref.next_idx;<br/>        if (table_with_length::contains(&amp;commit_history_ref.table, idx)) &#123;<br/>            table_with_length::remove(&amp;mut commit_history_ref.table, idx);<br/>        &#125;;<br/>        table_with_length::add(&amp;mut commit_history_ref.table, idx, copy new_block_event);<br/>        spec &#123;<br/>            assume idx &#43; 1 &lt;&#61; MAX_U32;<br/>        &#125;;<br/>        commit_history_ref.next_idx &#61; (idx &#43; 1) % commit_history_ref.max_capacity;<br/>    &#125;;<br/>    timestamp::update_global_time(vm, new_block_event.proposer, new_block_event.time_microseconds);<br/>    assert!(<br/>        event::counter(event_handle) &#61;&#61; new_block_event.height,<br/>        error::invalid_argument(ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT),<br/>    );<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(new_block_event_v2);<br/>    &#125;;<br/>    event::emit_event&lt;NewBlockEvent&gt;(event_handle, new_block_event);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_emit_genesis_block_event"></a>

## Function `emit_genesis_block_event`

Emit a <code>NewBlockEvent</code> event. This function will be invoked by genesis directly to generate the very first
reconfiguration event.


<pre><code>fun emit_genesis_block_event(vm: signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun emit_genesis_block_event(vm: signer) acquires BlockResource, CommitHistory &#123;<br/>    let block_metadata_ref &#61; borrow_global_mut&lt;BlockResource&gt;(@aptos_framework);<br/>    let genesis_id &#61; @0x0;<br/>    emit_new_block_event(<br/>        &amp;vm,<br/>        &amp;mut block_metadata_ref.new_block_events,<br/>        NewBlockEvent &#123;<br/>            hash: genesis_id,<br/>            epoch: 0,<br/>            round: 0,<br/>            height: 0,<br/>            previous_block_votes_bitvec: vector::empty(),<br/>            proposer: @vm_reserved,<br/>            failed_proposer_indices: vector::empty(),<br/>            time_microseconds: 0,<br/>        &#125;,<br/>        NewBlock &#123;<br/>            hash: genesis_id,<br/>            epoch: 0,<br/>            round: 0,<br/>            height: 0,<br/>            previous_block_votes_bitvec: vector::empty(),<br/>            proposer: @vm_reserved,<br/>            failed_proposer_indices: vector::empty(),<br/>            time_microseconds: 0,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_block_emit_writeset_block_event"></a>

## Function `emit_writeset_block_event`

Emit a <code>NewBlockEvent</code> event. This function will be invoked by write set script directly to generate the
new block event for WriteSetPayload.


<pre><code>public fun emit_writeset_block_event(vm_signer: &amp;signer, fake_block_hash: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun emit_writeset_block_event(vm_signer: &amp;signer, fake_block_hash: address) acquires BlockResource, CommitHistory &#123;<br/>    system_addresses::assert_vm(vm_signer);<br/>    let block_metadata_ref &#61; borrow_global_mut&lt;BlockResource&gt;(@aptos_framework);<br/>    block_metadata_ref.height &#61; event::counter(&amp;block_metadata_ref.new_block_events);<br/><br/>    emit_new_block_event(<br/>        vm_signer,<br/>        &amp;mut block_metadata_ref.new_block_events,<br/>        NewBlockEvent &#123;<br/>            hash: fake_block_hash,<br/>            epoch: reconfiguration::current_epoch(),<br/>            round: MAX_U64,<br/>            height: block_metadata_ref.height,<br/>            previous_block_votes_bitvec: vector::empty(),<br/>            proposer: @vm_reserved,<br/>            failed_proposer_indices: vector::empty(),<br/>            time_microseconds: timestamp::now_microseconds(),<br/>        &#125;,<br/>        NewBlock &#123;<br/>            hash: fake_block_hash,<br/>            epoch: reconfiguration::current_epoch(),<br/>            round: MAX_U64,<br/>            height: block_metadata_ref.height,<br/>            previous_block_votes_bitvec: vector::empty(),<br/>            proposer: @vm_reserved,<br/>            failed_proposer_indices: vector::empty(),<br/>            time_microseconds: timestamp::now_microseconds(),<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



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


<pre><code>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;BlockResource&gt;(@aptos_framework);<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;CommitHistory&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_BlockResource"></a>

### Resource `BlockResource`


<pre><code>struct BlockResource has key<br/></code></pre>



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
invariant epoch_interval &gt; 0;<br/></code></pre>



<a id="@Specification_1_CommitHistory"></a>

### Resource `CommitHistory`


<pre><code>struct CommitHistory has key<br/></code></pre>



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



<pre><code>invariant max_capacity &gt; 0;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, epoch_interval_microsecs: u64)<br/></code></pre>


The caller is aptos_framework.
The new_epoch_interval must be greater than 0.
The BlockResource is not under the caller before initializing.
The Account is not under the caller until the BlockResource is created for the caller.
Make sure The BlockResource under the caller existed after initializing.
The number of new events created does not exceed MAX_U64.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
include Initialize;<br/>include NewEventHandle;<br/>let addr &#61; signer::address_of(aptos_framework);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>aborts_if account.guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/></code></pre>



<a id="@Specification_1_update_epoch_interval_microsecs"></a>

### Function `update_epoch_interval_microsecs`


<pre><code>public fun update_epoch_interval_microsecs(aptos_framework: &amp;signer, new_epoch_interval: u64)<br/></code></pre>


The caller is @aptos_framework.
The new_epoch_interval must be greater than 0.
The BlockResource existed under the @aptos_framework.


<pre><code>// This enforces <a id="high-level-req-3.1" href="#high-level-req">high-level requirement 3</a>:
include UpdateEpochIntervalMicrosecs;<br/></code></pre>




<a id="0x1_block_UpdateEpochIntervalMicrosecs"></a>


<pre><code>schema UpdateEpochIntervalMicrosecs &#123;<br/>aptos_framework: signer;<br/>new_epoch_interval: u64;<br/>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces <a id="high-level-req-2.2" href="#high-level-req">high-level requirement 2</a>:
    aborts_if addr !&#61; @aptos_framework;<br/>aborts_if new_epoch_interval &#61;&#61; 0;<br/>aborts_if !exists&lt;BlockResource&gt;(addr);<br/>let post block_resource &#61; global&lt;BlockResource&gt;(addr);<br/>ensures block_resource.epoch_interval &#61;&#61; new_epoch_interval;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_epoch_interval_secs"></a>

### Function `get_epoch_interval_secs`


<pre><code>&#35;[view]<br/>public fun get_epoch_interval_secs(): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;BlockResource&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_block_prologue_common"></a>

### Function `block_prologue_common`


<pre><code>fun block_prologue_common(vm: &amp;signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64): u64<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;<br/>include BlockRequirement;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_block_prologue"></a>

### Function `block_prologue`


<pre><code>fun block_prologue(vm: signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;<br/>requires timestamp &gt;&#61; reconfiguration::last_reconfiguration_time();<br/>include BlockRequirement;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_block_prologue_ext"></a>

### Function `block_prologue_ext`


<pre><code>fun block_prologue_ext(vm: signer, hash: address, epoch: u64, round: u64, proposer: address, failed_proposer_indices: vector&lt;u64&gt;, previous_block_votes_bitvec: vector&lt;u8&gt;, timestamp: u64, randomness_seed: option::Option&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;<br/>requires timestamp &gt;&#61; reconfiguration::last_reconfiguration_time();<br/>include BlockRequirement;<br/>include stake::ResourceRequirement;<br/>include stake::GetReconfigStartTimeRequirement;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_get_current_block_height"></a>

### Function `get_current_block_height`


<pre><code>&#35;[view]<br/>public fun get_current_block_height(): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;BlockResource&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_emit_new_block_event"></a>

### Function `emit_new_block_event`


<pre><code>fun emit_new_block_event(vm: &amp;signer, event_handle: &amp;mut event::EventHandle&lt;block::NewBlockEvent&gt;, new_block_event: block::NewBlockEvent, new_block_event_v2: block::NewBlock)<br/></code></pre>




<pre><code>let proposer &#61; new_block_event.proposer;<br/>let timestamp &#61; new_block_event.time_microseconds;<br/>requires chain_status::is_operating();<br/>requires system_addresses::is_vm(vm);<br/>requires (proposer &#61;&#61; @vm_reserved) &#61;&#61;&gt; (timestamp::spec_now_microseconds() &#61;&#61; timestamp);<br/>requires (proposer !&#61; @vm_reserved) &#61;&#61;&gt; (timestamp::spec_now_microseconds() &lt; timestamp);<br/>// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
requires event::counter(event_handle) &#61;&#61; new_block_event.height;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_emit_genesis_block_event"></a>

### Function `emit_genesis_block_event`


<pre><code>fun emit_genesis_block_event(vm: signer)<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>requires system_addresses::is_vm(vm);<br/>requires event::counter(global&lt;BlockResource&gt;(@aptos_framework).new_block_events) &#61;&#61; 0;<br/>requires (timestamp::spec_now_microseconds() &#61;&#61; 0);<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_emit_writeset_block_event"></a>

### Function `emit_writeset_block_event`


<pre><code>public fun emit_writeset_block_event(vm_signer: &amp;signer, fake_block_hash: address)<br/></code></pre>


The caller is @vm_reserved.
The BlockResource existed under the @aptos_framework.
The Configuration existed under the @aptos_framework.
The CurrentTimeMicroseconds existed under the @aptos_framework.


<pre><code>requires chain_status::is_operating();<br/>include EmitWritesetBlockEvent;<br/></code></pre>




<a id="0x1_block_EmitWritesetBlockEvent"></a>


<pre><code>schema EmitWritesetBlockEvent &#123;<br/>vm_signer: signer;<br/>let addr &#61; signer::address_of(vm_signer);<br/>aborts_if addr !&#61; @vm_reserved;<br/>aborts_if !exists&lt;BlockResource&gt;(@aptos_framework);<br/>aborts_if !exists&lt;reconfiguration::Configuration&gt;(@aptos_framework);<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
