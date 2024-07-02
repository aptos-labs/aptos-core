
<a id="0x1_chain_status"></a>

# Module `0x1::chain_status`

This module code to assert that it is running in genesis (<code><a href="chain_status.md#0x1_chain_status_assert_genesis">Self::assert_genesis</a></code>) or after
genesis (<code><a href="chain_status.md#0x1_chain_status_assert_operating">Self::assert_operating</a></code>). These are essentially distinct states of the system. Specifically,
if <code><a href="chain_status.md#0x1_chain_status_assert_operating">Self::assert_operating</a></code> succeeds, assumptions about invariants over the global state can be made
which reflect that the system has been successfully initialized.


-  [Resource `GenesisEndMarker`](#0x1_chain_status_GenesisEndMarker)
-  [Constants](#@Constants_0)
-  [Function `set_genesis_end`](#0x1_chain_status_set_genesis_end)
-  [Function `is_genesis`](#0x1_chain_status_is_genesis)
-  [Function `is_operating`](#0x1_chain_status_is_operating)
-  [Function `assert_operating`](#0x1_chain_status_assert_operating)
-  [Function `assert_genesis`](#0x1_chain_status_assert_genesis)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `set_genesis_end`](#@Specification_1_set_genesis_end)
    -  [Function `assert_operating`](#@Specification_1_assert_operating)
    -  [Function `assert_genesis`](#@Specification_1_assert_genesis)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_chain_status_GenesisEndMarker"></a>

## Resource `GenesisEndMarker`

Marker to publish at the end of genesis.


<pre><code><b>struct</b> <a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_chain_status_ENOT_GENESIS"></a>

The blockchain is not in the genesis status.


<pre><code><b>const</b> <a href="chain_status.md#0x1_chain_status_ENOT_GENESIS">ENOT_GENESIS</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_chain_status_ENOT_OPERATING"></a>

The blockchain is not in the operating status.


<pre><code><b>const</b> <a href="chain_status.md#0x1_chain_status_ENOT_OPERATING">ENOT_OPERATING</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_chain_status_set_genesis_end"></a>

## Function `set_genesis_end`

Marks that genesis has finished.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_status.md#0x1_chain_status_set_genesis_end">set_genesis_end</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_status.md#0x1_chain_status_set_genesis_end">set_genesis_end</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>move_to</b>(aptos_framework, <a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a> &#123;&#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_chain_status_is_genesis"></a>

## Function `is_genesis`

Helper function to determine if Aptos is in genesis state.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>(): bool &#123;<br />    !<b>exists</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a>&gt;(@aptos_framework)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_chain_status_is_operating"></a>

## Function `is_operating`

Helper function to determine if Aptos is operating. This is
the same as <code>!<a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>()</code> and is provided for convenience.
Testing <code><a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>()</code> is more frequent than <code><a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>()</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>(): bool &#123;<br />    <b>exists</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a>&gt;(@aptos_framework)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_chain_status_assert_operating"></a>

## Function `assert_operating`

Helper function to assert operating (not genesis) state.


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_operating">assert_operating</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_operating">assert_operating</a>() &#123;<br />    <b>assert</b>!(<a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="chain_status.md#0x1_chain_status_ENOT_OPERATING">ENOT_OPERATING</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_chain_status_assert_genesis"></a>

## Function `assert_genesis`

Helper function to assert genesis state.


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_genesis">assert_genesis</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_genesis">assert_genesis</a>() &#123;<br />    <b>assert</b>!(<a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="chain_status.md#0x1_chain_status_ENOT_OPERATING">ENOT_OPERATING</a>));<br />&#125;<br /></code></pre>



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
<td>The end of genesis mark should persist throughout the entire life of the chain.</td>
<td>Medium</td>
<td>The Aptos framework account should never drop the GenesisEndMarker resource.</td>
<td>Audited that GenesisEndMarker is published at the end of genesis and never removed. Formally verified via <a href="#high-level-req-1">set_genesis_end</a> that GenesisEndMarker is published.</td>
</tr>

<tr>
<td>2</td>
<td>The status of the chain should never be genesis and operating at the same time.</td>
<td>Low</td>
<td>The status of the chain is determined by the GenesisEndMarker resource.</td>
<td>Formally verified via <a href="#high-level-req-2">global invariant</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The status of the chain should only be changed once, from genesis to operating.</td>
<td>Low</td>
<td>Attempting to assign a resource type more than once will abort.</td>
<td>Formally verified via <a href="#high-level-req-3">set_genesis_end</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>invariant</b> <a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>() &#61;&#61; !<a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>();<br /></code></pre>



<a id="@Specification_1_set_genesis_end"></a>

### Function `set_genesis_end`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_status.md#0x1_chain_status_set_genesis_end">set_genesis_end</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> delegate_invariants_to_caller;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>ensures</b> <b>global</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a>&gt;(@aptos_framework) &#61;&#61; <a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a> &#123;&#125;;<br /></code></pre>




<a id="0x1_chain_status_RequiresIsOperating"></a>


<pre><code><b>schema</b> <a href="chain_status.md#0x1_chain_status_RequiresIsOperating">RequiresIsOperating</a> &#123;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>();<br />&#125;<br /></code></pre>



<a id="@Specification_1_assert_operating"></a>

### Function `assert_operating`


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_operating">assert_operating</a>()<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>();<br /></code></pre>



<a id="@Specification_1_assert_genesis"></a>

### Function `assert_genesis`


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_genesis">assert_genesis</a>()<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>();<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
