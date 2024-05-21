
<a id="0x1_chain_status"></a>

# Module `0x1::chain_status`

This module code to assert that it is running in genesis (<code>Self::assert_genesis</code>) or after
genesis (<code>Self::assert_operating</code>). These are essentially distinct states of the system. Specifically,
if <code>Self::assert_operating</code> succeeds, assumptions about invariants over the global state can be made
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


<pre><code>use 0x1::error;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_chain_status_GenesisEndMarker"></a>

## Resource `GenesisEndMarker`

Marker to publish at the end of genesis.


<pre><code>struct GenesisEndMarker has key<br/></code></pre>



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


<pre><code>const ENOT_GENESIS: u64 &#61; 2;<br/></code></pre>



<a id="0x1_chain_status_ENOT_OPERATING"></a>

The blockchain is not in the operating status.


<pre><code>const ENOT_OPERATING: u64 &#61; 1;<br/></code></pre>



<a id="0x1_chain_status_set_genesis_end"></a>

## Function `set_genesis_end`

Marks that genesis has finished.


<pre><code>public(friend) fun set_genesis_end(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun set_genesis_end(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    move_to(aptos_framework, GenesisEndMarker &#123;&#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_chain_status_is_genesis"></a>

## Function `is_genesis`

Helper function to determine if Aptos is in genesis state.


<pre><code>&#35;[view]<br/>public fun is_genesis(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_genesis(): bool &#123;<br/>    !exists&lt;GenesisEndMarker&gt;(@aptos_framework)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_chain_status_is_operating"></a>

## Function `is_operating`

Helper function to determine if Aptos is operating. This is
the same as <code>!is_genesis()</code> and is provided for convenience.
Testing <code>is_operating()</code> is more frequent than <code>is_genesis()</code>.


<pre><code>&#35;[view]<br/>public fun is_operating(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_operating(): bool &#123;<br/>    exists&lt;GenesisEndMarker&gt;(@aptos_framework)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_chain_status_assert_operating"></a>

## Function `assert_operating`

Helper function to assert operating (not genesis) state.


<pre><code>public fun assert_operating()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_operating() &#123;<br/>    assert!(is_operating(), error::invalid_state(ENOT_OPERATING));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_chain_status_assert_genesis"></a>

## Function `assert_genesis`

Helper function to assert genesis state.


<pre><code>public fun assert_genesis()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_genesis() &#123;<br/>    assert!(is_genesis(), error::invalid_state(ENOT_OPERATING));<br/>&#125;<br/></code></pre>



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


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
invariant is_genesis() &#61;&#61; !is_operating();<br/></code></pre>



<a id="@Specification_1_set_genesis_end"></a>

### Function `set_genesis_end`


<pre><code>public(friend) fun set_genesis_end(aptos_framework: &amp;signer)<br/></code></pre>




<pre><code>pragma verify &#61; true;<br/>pragma delegate_invariants_to_caller;<br/>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if addr !&#61; @aptos_framework;<br/>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
aborts_if exists&lt;GenesisEndMarker&gt;(@aptos_framework);<br/>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures global&lt;GenesisEndMarker&gt;(@aptos_framework) &#61;&#61; GenesisEndMarker &#123;&#125;;<br/></code></pre>




<a id="0x1_chain_status_RequiresIsOperating"></a>


<pre><code>schema RequiresIsOperating &#123;<br/>requires is_operating();<br/>&#125;<br/></code></pre>



<a id="@Specification_1_assert_operating"></a>

### Function `assert_operating`


<pre><code>public fun assert_operating()<br/></code></pre>




<pre><code>aborts_if !is_operating();<br/></code></pre>



<a id="@Specification_1_assert_genesis"></a>

### Function `assert_genesis`


<pre><code>public fun assert_genesis()<br/></code></pre>




<pre><code>aborts_if !is_genesis();<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
