
<a id="0x1_chain_id"></a>

# Module `0x1::chain_id`

The chain id distinguishes between different chains (e.g., testnet and the main network).
One important role is to prevent transactions intended for one chain from being executed on another.
This code provides a container for storing a chain id and functions to initialize and get it.


-  [Resource `ChainId`](#0x1_chain_id_ChainId)
-  [Function `initialize`](#0x1_chain_id_initialize)
-  [Function `get`](#0x1_chain_id_get)
-  [Specification](#@Specification_0)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_0_initialize)
    -  [Function `get`](#@Specification_0_get)


<pre><code><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_chain_id_ChainId"></a>

## Resource `ChainId`



<pre><code><b>struct</b> <a href="chain_id.md#0x1_chain_id_ChainId">ChainId</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_chain_id_initialize"></a>

## Function `initialize`

Only called during genesis.
Publish the chain ID <code>id</code> of this instance under the SystemAddresses address


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_id.md#0x1_chain_id_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: u8)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_id.md#0x1_chain_id_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: u8) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>move_to</b>(aptos_framework, <a href="chain_id.md#0x1_chain_id_ChainId">ChainId</a> &#123; id &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_chain_id_get"></a>

## Function `get`

Return the chain ID of this instance.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="chain_id.md#0x1_chain_id_get">get</a>(): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_id.md#0x1_chain_id_get">get</a>(): u8 <b>acquires</b> <a href="chain_id.md#0x1_chain_id_ChainId">ChainId</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">ChainId</a>&gt;(@aptos_framework).id<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>During genesis, the ChainId resource should be created and moved under the Aptos framework account with the specified chain id.</td>
<td>Medium</td>
<td>The chain_id::initialize function is responsible for generating the ChainId resource and then storing it under the aptos_framework account.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The chain id can only be fetched if the chain id resource exists under the Aptos framework account.</td>
<td>Low</td>
<td>The chain_id::get function fetches the chain id by borrowing the ChainId resource from the aptos_framework account.</td>
<td>Formally verified via <a href="#high-level-req-2">get</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_0_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_id.md#0x1_chain_id_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: u8)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">ChainId</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">ChainId</a>&gt;(addr);<br /><b>ensures</b> <b>global</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">ChainId</a>&gt;(addr).id &#61;&#61; id;<br /></code></pre>



<a id="@Specification_0_get"></a>

### Function `get`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="chain_id.md#0x1_chain_id_get">get</a>(): u8<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">ChainId</a>&gt;(@aptos_framework);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
