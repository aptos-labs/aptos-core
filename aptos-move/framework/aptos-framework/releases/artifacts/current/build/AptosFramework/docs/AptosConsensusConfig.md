
<a name="0x1_AptosConsensusConfig"></a>

# Module `0x1::AptosConsensusConfig`



-  [Function `initialize`](#0x1_AptosConsensusConfig_initialize)
-  [Function `set`](#0x1_AptosConsensusConfig_set)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/ConsensusConfig.md#0x1_ConsensusConfig">0x1::ConsensusConfig</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
</code></pre>



<a name="0x1_AptosConsensusConfig_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="AptosConsensusConfig.md#0x1_AptosConsensusConfig_initialize">initialize</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosConsensusConfig.md#0x1_AptosConsensusConfig_initialize">initialize</a>(account: &signer) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/ConsensusConfig.md#0x1_ConsensusConfig_initialize">ConsensusConfig::initialize</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(account);
}
</code></pre>



</details>

<a name="0x1_AptosConsensusConfig_set"></a>

## Function `set`



<pre><code><b>public</b> <b>fun</b> <a href="AptosConsensusConfig.md#0x1_AptosConsensusConfig_set">set</a>(account: &signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosConsensusConfig.md#0x1_AptosConsensusConfig_set">set</a>(account: &signer, config: vector&lt;u8&gt;) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/ConsensusConfig.md#0x1_ConsensusConfig_set">ConsensusConfig::set</a>(
        config, &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
    );
}
</code></pre>



</details>
