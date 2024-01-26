
<a id="0x1_types"></a>

# Module `0x1::types`

Common types.


-  [Struct `ValidatorConsensusInfo`](#0x1_types_ValidatorConsensusInfo)
-  [Function `default_validator_consensus_info`](#0x1_types_default_validator_consensus_info)
-  [Function `new_validator_consensus_info`](#0x1_types_new_validator_consensus_info)
-  [Function `addr_from_validator_consensus_info`](#0x1_types_addr_from_validator_consensus_info)
-  [Function `pk_bytes_from_validator_consensus_info`](#0x1_types_pk_bytes_from_validator_consensus_info)
-  [Function `voting_power_from_validator_consensus_info`](#0x1_types_voting_power_from_validator_consensus_info)
-  [Specification](#@Specification_0)


<pre><code></code></pre>



<a id="0x1_types_ValidatorConsensusInfo"></a>

## Struct `ValidatorConsensusInfo`

Information about a validator that participates consensus.


<pre><code><b>struct</b> <a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>pk_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_power: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_types_default_validator_consensus_info"></a>

## Function `default_validator_consensus_info`

Create a <code><a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a></code> object.


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_default_validator_consensus_info">default_validator_consensus_info</a>(): <a href="types.md#0x1_types_ValidatorConsensusInfo">types::ValidatorConsensusInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_default_validator_consensus_info">default_validator_consensus_info</a>(): <a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a> {
    <a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a> {
        addr: @vm,
        pk_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        voting_power: 0,
    }
}
</code></pre>



</details>

<a id="0x1_types_new_validator_consensus_info"></a>

## Function `new_validator_consensus_info`

Create a <code><a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a></code> object.


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_new_validator_consensus_info">new_validator_consensus_info</a>(addr: <b>address</b>, pk_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, voting_power: u64): <a href="types.md#0x1_types_ValidatorConsensusInfo">types::ValidatorConsensusInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_new_validator_consensus_info">new_validator_consensus_info</a>(addr: <b>address</b>, pk_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, voting_power: u64): <a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a> {
    <a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a> {
        addr,
        pk_bytes,
        voting_power,
    }
}
</code></pre>



</details>

<a id="0x1_types_addr_from_validator_consensus_info"></a>

## Function `addr_from_validator_consensus_info`

Get <code><a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a>.addr</code>.


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_addr_from_validator_consensus_info">addr_from_validator_consensus_info</a>(vci: &<a href="types.md#0x1_types_ValidatorConsensusInfo">types::ValidatorConsensusInfo</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_addr_from_validator_consensus_info">addr_from_validator_consensus_info</a>(vci: &<a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a>): <b>address</b> {
    vci.addr
}
</code></pre>



</details>

<a id="0x1_types_pk_bytes_from_validator_consensus_info"></a>

## Function `pk_bytes_from_validator_consensus_info`

Get <code><a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a>.pk_bytes</code>.


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_pk_bytes_from_validator_consensus_info">pk_bytes_from_validator_consensus_info</a>(vci: &<a href="types.md#0x1_types_ValidatorConsensusInfo">types::ValidatorConsensusInfo</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_pk_bytes_from_validator_consensus_info">pk_bytes_from_validator_consensus_info</a>(vci: &<a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    vci.pk_bytes
}
</code></pre>



</details>

<a id="0x1_types_voting_power_from_validator_consensus_info"></a>

## Function `voting_power_from_validator_consensus_info`

Get <code><a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a>.voting_power</code>.


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_voting_power_from_validator_consensus_info">voting_power_from_validator_consensus_info</a>(vci: &<a href="types.md#0x1_types_ValidatorConsensusInfo">types::ValidatorConsensusInfo</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x1_types_voting_power_from_validator_consensus_info">voting_power_from_validator_consensus_info</a>(vci: &<a href="types.md#0x1_types_ValidatorConsensusInfo">ValidatorConsensusInfo</a>): u64 {
    vci.voting_power
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
