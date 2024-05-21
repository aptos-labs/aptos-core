
<a id="0x1_validator_consensus_info"></a>

# Module `0x1::validator_consensus_info`

Common type: <code>ValidatorConsensusInfo</code>.


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


<pre><code>struct ValidatorConsensusInfo has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>pk_bytes: vector&lt;u8&gt;</code>
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

Create a default <code>ValidatorConsensusInfo</code> object. Value may be invalid. Only for place holding prupose.


<pre><code>public fun default(): validator_consensus_info::ValidatorConsensusInfo<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun default(): ValidatorConsensusInfo &#123;<br/>    ValidatorConsensusInfo &#123;<br/>        addr: @vm,<br/>        pk_bytes: vector[],<br/>        voting_power: 0,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_validator_consensus_info_new"></a>

## Function `new`

Create a <code>ValidatorConsensusInfo</code> object.


<pre><code>public fun new(addr: address, pk_bytes: vector&lt;u8&gt;, voting_power: u64): validator_consensus_info::ValidatorConsensusInfo<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new(addr: address, pk_bytes: vector&lt;u8&gt;, voting_power: u64): ValidatorConsensusInfo &#123;<br/>    ValidatorConsensusInfo &#123;<br/>        addr,<br/>        pk_bytes,<br/>        voting_power,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_validator_consensus_info_get_addr"></a>

## Function `get_addr`

Get <code>ValidatorConsensusInfo.addr</code>.


<pre><code>public fun get_addr(vci: &amp;validator_consensus_info::ValidatorConsensusInfo): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_addr(vci: &amp;ValidatorConsensusInfo): address &#123;<br/>    vci.addr<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_validator_consensus_info_get_pk_bytes"></a>

## Function `get_pk_bytes`

Get <code>ValidatorConsensusInfo.pk_bytes</code>.


<pre><code>public fun get_pk_bytes(vci: &amp;validator_consensus_info::ValidatorConsensusInfo): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_pk_bytes(vci: &amp;ValidatorConsensusInfo): vector&lt;u8&gt; &#123;<br/>    vci.pk_bytes<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_validator_consensus_info_get_voting_power"></a>

## Function `get_voting_power`

Get <code>ValidatorConsensusInfo.voting_power</code>.


<pre><code>public fun get_voting_power(vci: &amp;validator_consensus_info::ValidatorConsensusInfo): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_voting_power(vci: &amp;ValidatorConsensusInfo): u64 &#123;<br/>    vci.voting_power<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code>pragma verify &#61; true;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
