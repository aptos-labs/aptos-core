
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


<pre><code><b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_chain_status_GenesisEndMarker"></a>

## Resource `GenesisEndMarker`

Marker to publish at the end of genesis.


<pre><code><b>struct</b> <a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a> <b>has</b> key
</code></pre>



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


<pre><code><b>const</b> <a href="chain_status.md#0x1_chain_status_ENOT_GENESIS">ENOT_GENESIS</a>: u64 = 2;
</code></pre>



<a id="0x1_chain_status_ENOT_OPERATING"></a>

The blockchain is not in the operating status.


<pre><code><b>const</b> <a href="chain_status.md#0x1_chain_status_ENOT_OPERATING">ENOT_OPERATING</a>: u64 = 1;
</code></pre>



<a id="0x1_chain_status_set_genesis_end"></a>

## Function `set_genesis_end`

Marks that genesis has finished.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_status.md#0x1_chain_status_set_genesis_end">set_genesis_end</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_status.md#0x1_chain_status_set_genesis_end">set_genesis_end</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <b>move_to</b>(velor_framework, <a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a> {});
}
</code></pre>



</details>

<a id="0x1_chain_status_is_genesis"></a>

## Function `is_genesis`

Helper function to determine if Velor is in genesis state.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>(): bool {
    !<b>exists</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a>&gt;(@velor_framework)
}
</code></pre>



</details>

<a id="0x1_chain_status_is_operating"></a>

## Function `is_operating`

Helper function to determine if Velor is operating. This is
the same as <code>!<a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>()</code> and is provided for convenience.
Testing <code><a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>()</code> is more frequent than <code><a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>()</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>(): bool {
    <b>exists</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a>&gt;(@velor_framework)
}
</code></pre>



</details>

<a id="0x1_chain_status_assert_operating"></a>

## Function `assert_operating`

Helper function to assert operating (not genesis) state.


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_operating">assert_operating</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_operating">assert_operating</a>() {
    <b>assert</b>!(<a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="chain_status.md#0x1_chain_status_ENOT_OPERATING">ENOT_OPERATING</a>));
}
</code></pre>



</details>

<a id="0x1_chain_status_assert_genesis"></a>

## Function `assert_genesis`

Helper function to assert genesis state.


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_genesis">assert_genesis</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_genesis">assert_genesis</a>() {
    <b>assert</b>!(<a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="chain_status.md#0x1_chain_status_ENOT_OPERATING">ENOT_OPERATING</a>));
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
<td>The end of genesis mark should persist throughout the entire life of the chain.</td>
<td>Medium</td>
<td>The Velor framework account should never drop the GenesisEndMarker resource.</td>
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


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>invariant</b> <a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>() == !<a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>();
</code></pre>



<a id="@Specification_1_set_genesis_end"></a>

### Function `set_genesis_end`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chain_status.md#0x1_chain_status_set_genesis_end">set_genesis_end</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> delegate_invariants_to_caller;
<b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
<b>aborts_if</b> addr != @velor_framework;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a>&gt;(@velor_framework);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> <b>global</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a>&gt;(@velor_framework) == <a href="chain_status.md#0x1_chain_status_GenesisEndMarker">GenesisEndMarker</a> {};
</code></pre>




<a id="0x1_chain_status_RequiresIsOperating"></a>


<pre><code><b>schema</b> <a href="chain_status.md#0x1_chain_status_RequiresIsOperating">RequiresIsOperating</a> {
    <b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>();
}
</code></pre>



<a id="@Specification_1_assert_operating"></a>

### Function `assert_operating`


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_operating">assert_operating</a>()
</code></pre>




<pre><code><b>aborts_if</b> !<a href="chain_status.md#0x1_chain_status_is_operating">is_operating</a>();
</code></pre>



<a id="@Specification_1_assert_genesis"></a>

### Function `assert_genesis`


<pre><code><b>public</b> <b>fun</b> <a href="chain_status.md#0x1_chain_status_assert_genesis">assert_genesis</a>()
</code></pre>




<pre><code><b>aborts_if</b> !<a href="chain_status.md#0x1_chain_status_is_genesis">is_genesis</a>();
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
