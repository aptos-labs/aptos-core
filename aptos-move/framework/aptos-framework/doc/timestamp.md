
<a id="0x1_timestamp"></a>

# Module `0x1::timestamp`

This module keeps a global wall clock that stores the current Unix time in microseconds.
It interacts with the other modules in the following ways:
&#42; genesis: to initialize the timestamp
&#42; block: to reach consensus on the global wall clock time


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


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_timestamp_CurrentTimeMicroseconds"></a>

## Resource `CurrentTimeMicroseconds`

A singleton resource holding the current Unix time in microseconds


<pre><code><b>struct</b> <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> <b>has</b> key<br /></code></pre>



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


<pre><code><b>const</b> <a href="timestamp.md#0x1_timestamp_ENOT_OPERATING">ENOT_OPERATING</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_timestamp_EINVALID_TIMESTAMP"></a>

An invalid timestamp was provided


<pre><code><b>const</b> <a href="timestamp.md#0x1_timestamp_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_timestamp_MICRO_CONVERSION_FACTOR"></a>

Conversion factor between seconds and microseconds


<pre><code><b>const</b> <a href="timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a>: u64 &#61; 1000000;<br /></code></pre>



<a id="0x1_timestamp_set_time_has_started"></a>

## Function `set_time_has_started`

Marks that time has started. This can only be called from genesis and with the aptos framework account.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timestamp.md#0x1_timestamp_set_time_has_started">set_time_has_started</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timestamp.md#0x1_timestamp_set_time_has_started">set_time_has_started</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>let</b> timer &#61; <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> &#123; microseconds: 0 &#125;;<br />    <b>move_to</b>(aptos_framework, timer);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_timestamp_update_global_time"></a>

## Function `update_global_time`

Updates the wall clock time by consensus. Requires VM privilege and will be invoked during block prologue.


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_update_global_time">update_global_time</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, proposer: <b>address</b>, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_update_global_time">update_global_time</a>(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    proposer: <b>address</b>,<br />    <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64<br />) <b>acquires</b> <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> &#123;<br />    // Can only be invoked by AptosVM <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>.<br />    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(<a href="account.md#0x1_account">account</a>);<br /><br />    <b>let</b> global_timer &#61; <b>borrow_global_mut</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br />    <b>let</b> now &#61; global_timer.microseconds;<br />    <b>if</b> (proposer &#61;&#61; @vm_reserved) &#123;<br />        // NIL <a href="block.md#0x1_block">block</a> <b>with</b> null <b>address</b> <b>as</b> proposer. Timestamp must be equal.<br />        <b>assert</b>!(now &#61;&#61; <a href="timestamp.md#0x1_timestamp">timestamp</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="timestamp.md#0x1_timestamp_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>));<br />    &#125; <b>else</b> &#123;<br />        // Normal <a href="block.md#0x1_block">block</a>. Time must advance<br />        <b>assert</b>!(now &lt; <a href="timestamp.md#0x1_timestamp">timestamp</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="timestamp.md#0x1_timestamp_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>));<br />        global_timer.microseconds &#61; <a href="timestamp.md#0x1_timestamp">timestamp</a>;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_timestamp_now_microseconds"></a>

## Function `now_microseconds`

Gets the current time in microseconds.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_now_microseconds">now_microseconds</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_now_microseconds">now_microseconds</a>(): u64 <b>acquires</b> <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a>&gt;(@aptos_framework).microseconds<br />&#125;<br /></code></pre>



</details>

<a id="0x1_timestamp_now_seconds"></a>

## Function `now_seconds`

Gets the current time in seconds.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_now_seconds">now_seconds</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_now_seconds">now_seconds</a>(): u64 <b>acquires</b> <a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a> &#123;<br />    <a href="timestamp.md#0x1_timestamp_now_microseconds">now_microseconds</a>() / <a href="timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a><br />&#125;<br /></code></pre>



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
<td>The global wall clock resource should only be owned by the Aptos framework.</td>
<td>High</td>
<td>The function set_time_has_started ensures that only the aptos_framework account can possess the CurrentTimeMicroseconds resource using the assert_aptos_framework function.</td>
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
<td>The clock time should increase with every update as agreed through consensus and proposed by the current epoch&apos;s validator.</td>
<td>High</td>
<td>The update_global_time function asserts that the new timestamp is greater than the current timestamp.</td>
<td>Formally verified via <a href="#high-level-req-4">UpdateGlobalTimeAbortsIf</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a> and <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_update_global_time"></a>

### Function `update_global_time`


<pre><code><b>public</b> <b>fun</b> <a href="timestamp.md#0x1_timestamp_update_global_time">update_global_time</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, proposer: <b>address</b>, <a href="timestamp.md#0x1_timestamp">timestamp</a>: u64)<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="timestamp.md#0x1_timestamp_UpdateGlobalTimeAbortsIf">UpdateGlobalTimeAbortsIf</a>;<br /><b>ensures</b> (proposer !&#61; @vm_reserved) &#61;&#61;&gt; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>() &#61;&#61; <a href="timestamp.md#0x1_timestamp">timestamp</a>);<br /></code></pre>




<a id="0x1_timestamp_UpdateGlobalTimeAbortsIf"></a>


<pre><code><b>schema</b> <a href="timestamp.md#0x1_timestamp_UpdateGlobalTimeAbortsIf">UpdateGlobalTimeAbortsIf</a> &#123;<br /><a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />proposer: <b>address</b>;<br /><a href="timestamp.md#0x1_timestamp">timestamp</a>: u64;<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
    <b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_vm">system_addresses::is_vm</a>(<a href="account.md#0x1_account">account</a>);<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
    <b>aborts_if</b> (proposer &#61;&#61; @vm_reserved) &amp;&amp; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>() !&#61; <a href="timestamp.md#0x1_timestamp">timestamp</a>);<br /><b>aborts_if</b> (proposer !&#61; @vm_reserved) &amp;&amp; (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>() &gt;&#61; <a href="timestamp.md#0x1_timestamp">timestamp</a>);<br />&#125;<br /></code></pre>




<a id="0x1_timestamp_spec_now_microseconds"></a>


<pre><code><b>fun</b> <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>(): u64 &#123;<br />   <b>global</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">CurrentTimeMicroseconds</a>&gt;(@aptos_framework).microseconds<br />&#125;<br /></code></pre>




<a id="0x1_timestamp_spec_now_seconds"></a>


<pre><code><b>fun</b> <a href="timestamp.md#0x1_timestamp_spec_now_seconds">spec_now_seconds</a>(): u64 &#123;<br />   <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">spec_now_microseconds</a>() / <a href="timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">MICRO_CONVERSION_FACTOR</a><br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
