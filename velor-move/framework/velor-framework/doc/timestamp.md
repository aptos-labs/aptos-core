
<a id="0x1_timestamp"></a>

# Module `0x1::timestamp`

This module keeps a global wall clock that stores the current Unix time in microseconds.
It interacts with the other modules in the following ways:
* genesis: to initialize the timestamp
* block: to reach consensus on the global wall clock time


-  [Resource `CurrentTimeMicroseconds`](#0x1_timestamp_CurrentTimeMicroseconds)
-  [Constants](#@Constants_0)
-  [Function `set_time_has_started`](#0x1_timestamp_set_time_has_started)
-  [Function `update_global_time`](#0x1_timestamp_update_global_time)
-  [Function `now_microseconds`](#0x1_timestamp_now_microseconds)
-  [Function `now_seconds`](#0x1_timestamp_now_seconds)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `update_global_time`](#@Specification_1_update_global_time)


<pre><code><b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_timestamp_CurrentTimeMicroseconds"></a>

## Resource `CurrentTimeMicroseconds`

A singleton resource holding the current Unix time in microseconds


<pre><code><b>struct</b> <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>microseconds: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_timestamp_ENOT_OPERATING"></a>

The blockchain is not in an operating state yet


<pre><code><b>const</b> <a href="timestamp.md#0x1_timestamp_ENOT_OPERATING">ENOT_OPERATING</a>: u64 = 1;
</code></pre>



<a id="0x1_timestamp_EINVALID_TIMESTAMP"></a>

An invalid timestamp was provided


<pre><code><b>const</b> <a href="timestamp.md#0x1_timestamp_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>: u64 = 2;
</code></pre>



<a id="0x1_timestamp_MICRO_CONVERSION_FACTOR"></a>

Conversion factor between seconds and microseconds


<pre><code><b>const</b> <a href="timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a>: u64 = 1000000;
</code></pre>



<a id="0x1_timestamp_set_time_has_started"></a>

## Function `set_time_has_started`

Marks that time has started. This can only be called from genesis and with the velor framework account.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timestamp.md#0x1_timestamp_set_time_has_started">set_time_has_started</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timestamp.md#0x1_timestamp_set_time_has_started">set_time_has_started</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <b>let</b> timer = <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> { microseconds: 0 };
    <b>move_to</b>(velor_framework, timer);
}
</code></pre>



</details>

<a id="0x1_timestamp_update_global_time"></a>

## Function `update_global_time`

Updates the wall clock time by consensus. Requires VM privilege and will be invoked during block prologue.


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_update_global_time">update_global_time</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, proposer: <b>address</b>, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_update_global_time">update_global_time</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    proposer: <b>address</b>,
    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64
) <b>acquires</b> <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> {
    // Can only be invoked by VelorVM <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>.
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(<a href="account.md#0x1_account">account</a>);

    <b>let</b> global_timer = <b>borrow_global_mut</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a>&gt;(@velor_framework);
    <b>let</b> now = global_timer.microseconds;
    <b>if</b> (proposer == @vm_reserved) {
        // NIL <a href="block.md#0x1_block">block</a> <b>with</b> null <b>address</b> <b>as</b> proposer. Timestamp must be equal.
        <b>assert</b>!(now == <a href="timestamp.md#0x1_timestamp">timestamp</a>, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="timestamp.md#0x1_timestamp_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>));
    } <b>else</b> {
        // Normal <a href="block.md#0x1_block">block</a>. Time must advance
        <b>assert</b>!(now &lt; <a href="timestamp.md#0x1_timestamp">timestamp</a>, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="timestamp.md#0x1_timestamp_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>));
        global_timer.microseconds = <a href="timestamp.md#0x1_timestamp">timestamp</a>;
    };
}
</code></pre>



</details>

<a id="0x1_timestamp_now_microseconds"></a>

## Function `now_microseconds`

Gets the current time in microseconds.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_now_microseconds">now_microseconds</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_now_microseconds">now_microseconds</a>(): u64 <b>acquires</b> <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> {
    <b>borrow_global</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a>&gt;(@velor_framework).microseconds
}
</code></pre>



</details>

<a id="0x1_timestamp_now_seconds"></a>

## Function `now_seconds`

Gets the current time in seconds.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_now_seconds">now_seconds</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_now_seconds">now_seconds</a>(): u64 <b>acquires</b> <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> {
    <a href="timestamp.md#0x1_timestamp_now_microseconds">now_microseconds</a>() / <a href="timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a>
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
<td>There should only exist one global wall clock and it should be created during genesis.</td>
<td>High</td>
<td>The function set_time_has_started is only called by genesis::initialize and ensures that no other resources of this type exist by only assigning it to a predefined account.</td>
<td>Formally verified via <a href="#high-level-req-1">module</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The global wall clock resource should only be owned by the Velor framework.</td>
<td>High</td>
<td>The function set_time_has_started ensures that only the velor_framework account can possess the CurrentTimeMicroseconds resource using the assert_velor_framework function.</td>
<td>Formally verified via <a href="#high-level-req-2">module</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The clock time should only be updated by the VM account.</td>
<td>High</td>
<td>The update_global_time function asserts that the transaction signer is the vm_reserved account.</td>
<td>Formally verified via <a href="#high-level-req-3">UpdateGlobalTimeAbortsIf</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The clock time should increase with every update as agreed through consensus and proposed by the current epoch's validator.</td>
<td>High</td>
<td>The update_global_time function asserts that the new timestamp is greater than the current timestamp.</td>
<td>Formally verified via <a href="#high-level-req-4">UpdateGlobalTimeAbortsIf</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a> and <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a>&gt;(@velor_framework);
</code></pre>



<a id="@Specification_1_update_global_time"></a>

### Function `update_global_time`


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_update_global_time">update_global_time</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, proposer: <b>address</b>, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64)
</code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="timestamp.md#0x1_timestamp_UpdateGlobalTimeAbortsIf">UpdateGlobalTimeAbortsIf</a>;
<b>ensures</b> (proposer != @vm_reserved) ==&gt; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>() == <a href="timestamp.md#0x1_timestamp">timestamp</a>);
</code></pre>




<a id="0x1_timestamp_UpdateGlobalTimeAbortsIf"></a>


<pre><code><b>schema</b> <a href="timestamp.md#0x1_timestamp_UpdateGlobalTimeAbortsIf">UpdateGlobalTimeAbortsIf</a> {
    <a href="account.md#0x1_account">account</a>: <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    proposer: <b>address</b>;
    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64;
    // This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    <b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_vm">system_addresses::is_vm</a>(<a href="account.md#0x1_account">account</a>);
    // This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
    <b>aborts_if</b> (proposer == @vm_reserved) && (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>() != <a href="timestamp.md#0x1_timestamp">timestamp</a>);
    <b>aborts_if</b> (proposer != @vm_reserved) && (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>() &gt;= <a href="timestamp.md#0x1_timestamp">timestamp</a>);
}
</code></pre>




<a id="0x1_timestamp_spec_now_microseconds"></a>


<pre><code><b>fun</b> <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>(): u64 {
   <b>global</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a>&gt;(@velor_framework).microseconds
}
</code></pre>




<a id="0x1_timestamp_spec_now_seconds"></a>


<pre><code><b>fun</b> <a href="timestamp.md#0x1_timestamp_spec_now_seconds">spec_now_seconds</a>(): u64 {
   <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>() / <a href="timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a>
}
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
