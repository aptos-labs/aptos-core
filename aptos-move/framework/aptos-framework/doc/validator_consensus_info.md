
<a id="0x1_validator_consensus_info"></a>

# Module `0x1::validator_consensus_info`

Common type: <code><a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a></code>.


-  [Struct `ValidatorConsensusInfo`](#0x1_validator_consensus_info_ValidatorConsensusInfo)
-  [Function `default`](#0x1_validator_consensus_info_default)
-  [Function `new`](#0x1_validator_consensus_info_new)
-  [Function `get_addr`](#0x1_validator_consensus_info_get_addr)
-  [Function `get_pk_bytes`](#0x1_validator_consensus_info_get_pk_bytes)
-  [Function `get_voting_power`](#0x1_validator_consensus_info_get_voting_power)
-  [Specification](#@Specification_0)


<pre><code></code></pre>



<a id="0x1_validator_consensus_info_ValidatorConsensusInfo"></a>

## Struct `ValidatorConsensusInfo`

Information about a validator that participates consensus.


<pre><code><b>struct</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_validator_consensus_info_default"></a>

## Function `default`

Create a default <code><a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a></code> object. Value may be invalid. Only for place holding prupose.


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_default">default</a>(): <a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_default">default</a>(): <a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a> &#123;<br />    <a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a> &#123;<br />        addr: @vm,<br />        pk_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],<br />        voting_power: 0,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_validator_consensus_info_new"></a>

## Function `new`

Create a <code><a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a></code> object.


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_new">new</a>(addr: <b>address</b>, pk_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, voting_power: u64): <a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_new">new</a>(addr: <b>address</b>, pk_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, voting_power: u64): <a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a> &#123;<br />    <a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a> &#123;<br />        addr,<br />        pk_bytes,<br />        voting_power,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_validator_consensus_info_get_addr"></a>

## Function `get_addr`

Get <code><a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a>.addr</code>.


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_get_addr">get_addr</a>(vci: &amp;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_get_addr">get_addr</a>(vci: &amp;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a>): <b>address</b> &#123;<br />    vci.addr<br />&#125;<br /></code></pre>



</details>

<a id="0x1_validator_consensus_info_get_pk_bytes"></a>

## Function `get_pk_bytes`

Get <code><a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a>.pk_bytes</code>.


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_get_pk_bytes">get_pk_bytes</a>(vci: &amp;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_get_pk_bytes">get_pk_bytes</a>(vci: &amp;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    vci.pk_bytes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_validator_consensus_info_get_voting_power"></a>

## Function `get_voting_power`

Get <code><a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a>.voting_power</code>.


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_get_voting_power">get_voting_power</a>(vci: &amp;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info_get_voting_power">get_voting_power</a>(vci: &amp;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">ValidatorConsensusInfo</a>): u64 &#123;<br />    vci.voting_power<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
