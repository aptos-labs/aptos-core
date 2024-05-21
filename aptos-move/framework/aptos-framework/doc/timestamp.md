
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


<pre><code>use 0x1::error;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_timestamp_CurrentTimeMicroseconds"></a>

## Resource `CurrentTimeMicroseconds`

A singleton resource holding the current Unix time in microseconds


<pre><code>struct CurrentTimeMicroseconds has key<br/></code></pre>



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


<pre><code>const ENOT_OPERATING: u64 &#61; 1;<br/></code></pre>



<a id="0x1_timestamp_EINVALID_TIMESTAMP"></a>

An invalid timestamp was provided


<pre><code>const EINVALID_TIMESTAMP: u64 &#61; 2;<br/></code></pre>



<a id="0x1_timestamp_MICRO_CONVERSION_FACTOR"></a>

Conversion factor between seconds and microseconds


<pre><code>const MICRO_CONVERSION_FACTOR: u64 &#61; 1000000;<br/></code></pre>



<a id="0x1_timestamp_set_time_has_started"></a>

## Function `set_time_has_started`

Marks that time has started. This can only be called from genesis and with the aptos framework account.


<pre><code>public(friend) fun set_time_has_started(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun set_time_has_started(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    let timer &#61; CurrentTimeMicroseconds &#123; microseconds: 0 &#125;;<br/>    move_to(aptos_framework, timer);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_timestamp_update_global_time"></a>

## Function `update_global_time`

Updates the wall clock time by consensus. Requires VM privilege and will be invoked during block prologue.


<pre><code>public fun update_global_time(account: &amp;signer, proposer: address, timestamp: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_global_time(<br/>    account: &amp;signer,<br/>    proposer: address,<br/>    timestamp: u64<br/>) acquires CurrentTimeMicroseconds &#123;<br/>    // Can only be invoked by AptosVM signer.<br/>    system_addresses::assert_vm(account);<br/><br/>    let global_timer &#61; borrow_global_mut&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>    let now &#61; global_timer.microseconds;<br/>    if (proposer &#61;&#61; @vm_reserved) &#123;<br/>        // NIL block with null address as proposer. Timestamp must be equal.<br/>        assert!(now &#61;&#61; timestamp, error::invalid_argument(EINVALID_TIMESTAMP));<br/>    &#125; else &#123;<br/>        // Normal block. Time must advance<br/>        assert!(now &lt; timestamp, error::invalid_argument(EINVALID_TIMESTAMP));<br/>        global_timer.microseconds &#61; timestamp;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_timestamp_now_microseconds"></a>

## Function `now_microseconds`

Gets the current time in microseconds.


<pre><code>&#35;[view]<br/>public fun now_microseconds(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun now_microseconds(): u64 acquires CurrentTimeMicroseconds &#123;<br/>    borrow_global&lt;CurrentTimeMicroseconds&gt;(@aptos_framework).microseconds<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_timestamp_now_seconds"></a>

## Function `now_seconds`

Gets the current time in seconds.


<pre><code>&#35;[view]<br/>public fun now_seconds(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun now_seconds(): u64 acquires CurrentTimeMicroseconds &#123;<br/>    now_microseconds() / MICRO_CONVERSION_FACTOR<br/>&#125;<br/></code></pre>



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
<td>The clock time should increase with every update as agreed through consensus and proposed by the current epoch's validator.</td>
<td>High</td>
<td>The update_global_time function asserts that the new timestamp is greater than the current timestamp.</td>
<td>Formally verified via <a href="#high-level-req-4">UpdateGlobalTimeAbortsIf</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a> and <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_update_global_time"></a>

### Function `update_global_time`


<pre><code>public fun update_global_time(account: &amp;signer, proposer: address, timestamp: u64)<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>include UpdateGlobalTimeAbortsIf;<br/>ensures (proposer !&#61; @vm_reserved) &#61;&#61;&gt; (spec_now_microseconds() &#61;&#61; timestamp);<br/></code></pre>




<a id="0x1_timestamp_UpdateGlobalTimeAbortsIf"></a>


<pre><code>schema UpdateGlobalTimeAbortsIf &#123;<br/>account: signer;<br/>proposer: address;<br/>timestamp: u64;<br/>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    aborts_if !system_addresses::is_vm(account);<br/>// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
    aborts_if (proposer &#61;&#61; @vm_reserved) &amp;&amp; (spec_now_microseconds() !&#61; timestamp);<br/>aborts_if (proposer !&#61; @vm_reserved) &amp;&amp; (spec_now_microseconds() &gt;&#61; timestamp);<br/>&#125;<br/></code></pre>




<a id="0x1_timestamp_spec_now_microseconds"></a>


<pre><code>fun spec_now_microseconds(): u64 &#123;<br/>   global&lt;CurrentTimeMicroseconds&gt;(@aptos_framework).microseconds<br/>&#125;<br/></code></pre>




<a id="0x1_timestamp_spec_now_seconds"></a>


<pre><code>fun spec_now_seconds(): u64 &#123;<br/>   spec_now_microseconds() / MICRO_CONVERSION_FACTOR<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
