
<a id="0x1_state_storage"></a>

# Module `0x1::state_storage`



-  [Struct `Usage`](#0x1_state_storage_Usage)
-  [Resource `StateStorageUsage`](#0x1_state_storage_StateStorageUsage)
-  [Resource `GasParameter`](#0x1_state_storage_GasParameter)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_state_storage_initialize)
-  [Function `on_new_block`](#0x1_state_storage_on_new_block)
-  [Function `current_items_and_bytes`](#0x1_state_storage_current_items_and_bytes)
-  [Function `get_state_storage_usage_only_at_epoch_beginning`](#0x1_state_storage_get_state_storage_usage_only_at_epoch_beginning)
-  [Function `on_reconfig`](#0x1_state_storage_on_reconfig)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `on_new_block`](#@Specification_1_on_new_block)
    -  [Function `current_items_and_bytes`](#@Specification_1_current_items_and_bytes)
    -  [Function `get_state_storage_usage_only_at_epoch_beginning`](#@Specification_1_get_state_storage_usage_only_at_epoch_beginning)
    -  [Function `on_reconfig`](#@Specification_1_on_reconfig)


<pre><code><b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_state_storage_Usage"></a>

## Struct `Usage`



<pre><code><b>struct</b> <a href="state_storage.md#0x1_state_storage_Usage">Usage</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>items: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bytes: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_state_storage_StateStorageUsage"></a>

## Resource `StateStorageUsage`

This is updated at the beginning of each epoch, reflecting the storage
usage after the last txn of the previous epoch is committed.


<pre><code><b>struct</b> <a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>usage: <a href="state_storage.md#0x1_state_storage_Usage">state_storage::Usage</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_state_storage_GasParameter"></a>

## Resource `GasParameter`



<pre><code><b>struct</b> <a href="state_storage.md#0x1_state_storage_GasParameter">GasParameter</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>usage: <a href="state_storage.md#0x1_state_storage_Usage">state_storage::Usage</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_state_storage_ESTATE_STORAGE_USAGE"></a>



<pre><code><b>const</b> <a href="state_storage.md#0x1_state_storage_ESTATE_STORAGE_USAGE">ESTATE_STORAGE_USAGE</a>: u64 = 0;
</code></pre>



<a id="0x1_state_storage_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_initialize">initialize</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_initialize">initialize</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="state_storage.md#0x1_state_storage_ESTATE_STORAGE_USAGE">ESTATE_STORAGE_USAGE</a>)
    );
    <b>move_to</b>(velor_framework, <a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a> {
        epoch: 0,
        usage: <a href="state_storage.md#0x1_state_storage_Usage">Usage</a> {
            items: 0,
            bytes: 0,
        }
    });
}
</code></pre>



</details>

<a id="0x1_state_storage_on_new_block"></a>

## Function `on_new_block`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_on_new_block">on_new_block</a>(epoch: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_on_new_block">on_new_block</a>(epoch: u64) <b>acquires</b> <a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="state_storage.md#0x1_state_storage_ESTATE_STORAGE_USAGE">ESTATE_STORAGE_USAGE</a>)
    );
    <b>let</b> usage = <b>borrow_global_mut</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework);
    <b>if</b> (epoch != usage.epoch) {
        usage.epoch = epoch;
        usage.usage = <a href="state_storage.md#0x1_state_storage_get_state_storage_usage_only_at_epoch_beginning">get_state_storage_usage_only_at_epoch_beginning</a>();
    }
}
</code></pre>



</details>

<a id="0x1_state_storage_current_items_and_bytes"></a>

## Function `current_items_and_bytes`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_current_items_and_bytes">current_items_and_bytes</a>(): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_current_items_and_bytes">current_items_and_bytes</a>(): (u64, u64) <b>acquires</b> <a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="state_storage.md#0x1_state_storage_ESTATE_STORAGE_USAGE">ESTATE_STORAGE_USAGE</a>)
    );
    <b>let</b> usage = <b>borrow_global</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework);
    (usage.usage.items, usage.usage.bytes)
}
</code></pre>



</details>

<a id="0x1_state_storage_get_state_storage_usage_only_at_epoch_beginning"></a>

## Function `get_state_storage_usage_only_at_epoch_beginning`

Warning: the result returned is based on the base state view held by the
VM for the entire block or chunk of transactions, it's only deterministic
if called from the first transaction of the block because the execution layer
guarantees a fresh state view then.


<pre><code><b>fun</b> <a href="state_storage.md#0x1_state_storage_get_state_storage_usage_only_at_epoch_beginning">get_state_storage_usage_only_at_epoch_beginning</a>(): <a href="state_storage.md#0x1_state_storage_Usage">state_storage::Usage</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="state_storage.md#0x1_state_storage_get_state_storage_usage_only_at_epoch_beginning">get_state_storage_usage_only_at_epoch_beginning</a>(): <a href="state_storage.md#0x1_state_storage_Usage">Usage</a>;
</code></pre>



</details>

<a id="0x1_state_storage_on_reconfig"></a>

## Function `on_reconfig`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_on_reconfig">on_reconfig</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_on_reconfig">on_reconfig</a>() {
    <b>abort</b> 0
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
<td>Given the blockchain is in an operating state, the resources for tracking state storage usage and gas parameters must exist for the Velor framework address.</td>
<td>Critical</td>
<td>The initialize function ensures only the Velor framework address can call it.</td>
<td>Formally verified via <a href="#high-level-req-1">module</a>.</td>
</tr>

<tr>
<td>2</td>
<td>During the initialization of the module, it is guaranteed that the resource for tracking state storage usage will be moved under the Velor framework account with default initial values.</td>
<td>Medium</td>
<td>The resource for tracking state storage usage may only be initialized with specific values and published under the velor_framework account.</td>
<td>Formally verified via <a href="#high-level-req-2">initialize</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The initialization function is only called once, during genesis.</td>
<td>Medium</td>
<td>The initialize function ensures StateStorageUsage does not already exist.</td>
<td>Formally verified via <a href="#high-level-req-3">initialize</a>.</td>
</tr>

<tr>
<td>4</td>
<td>During the initialization of the module, it is guaranteed that the resource for tracking state storage usage will be moved under the Velor framework account with default initial values.</td>
<td>Medium</td>
<td>The resource for tracking state storage usage may only be initialized with specific values and published under the velor_framework account.</td>
<td>Formally verified via <a href="#high-level-req-4">initialize</a>.</td>
</tr>

<tr>
<td>5</td>
<td>The structure for tracking state storage usage should exist for it to be updated at the beginning of each new block and for retrieving the values of structure members.</td>
<td>Medium</td>
<td>The functions on_new_block and current_items_and_bytes verify that the StateStorageUsage structure exists before performing any further operations.</td>
<td>Formally Verified via <a href="#high-level-req-5.1">current_items_and_bytes</a>, <a href="#high-level-req-5.2">on_new_block</a>, and the <a href="#high-level-req-5.3">global invariant</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a> and <a id="high-level-req-5.3" href="#high-level-req">high-level requirement 5</a>:
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework);
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_GasParameter">GasParameter</a>&gt;(@velor_framework);
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_initialize">initialize</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>


ensure caller is admin.
aborts if StateStorageUsage already exists.


<pre><code><b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_velor_framework_address">system_addresses::is_velor_framework_address</a>(addr);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework);
<b>let</b> <b>post</b> state_usage = <b>global</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework);
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>ensures</b> state_usage.epoch == 0 && state_usage.usage.bytes == 0 && state_usage.usage.items == 0;
</code></pre>



<a id="@Specification_1_on_new_block"></a>

### Function `on_new_block`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_on_new_block">on_new_block</a>(epoch: u64)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-5.2" href="#high-level-req">high-level requirement 5</a>:
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> epoch == <b>global</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework).epoch;
</code></pre>



<a id="@Specification_1_current_items_and_bytes"></a>

### Function `current_items_and_bytes`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_current_items_and_bytes">current_items_and_bytes</a>(): (u64, u64)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-5.1" href="#high-level-req">high-level requirement 5</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">StateStorageUsage</a>&gt;(@velor_framework);
</code></pre>



<a id="@Specification_1_get_state_storage_usage_only_at_epoch_beginning"></a>

### Function `get_state_storage_usage_only_at_epoch_beginning`


<pre><code><b>fun</b> <a href="state_storage.md#0x1_state_storage_get_state_storage_usage_only_at_epoch_beginning">get_state_storage_usage_only_at_epoch_beginning</a>(): <a href="state_storage.md#0x1_state_storage_Usage">state_storage::Usage</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_on_reconfig"></a>

### Function `on_reconfig`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="state_storage.md#0x1_state_storage_on_reconfig">on_reconfig</a>()
</code></pre>




<pre><code><b>aborts_if</b> <b>true</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
