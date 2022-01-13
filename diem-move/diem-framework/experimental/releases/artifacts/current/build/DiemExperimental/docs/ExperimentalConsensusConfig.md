
<a name="0x1_ExperimentalConsensusConfig"></a>

# Module `0x1::ExperimentalConsensusConfig`



-  [Struct `ExperimentalConsensusConfig`](#0x1_ExperimentalConsensusConfig_ExperimentalConsensusConfig)
-  [Function `initialize`](#0x1_ExperimentalConsensusConfig_initialize)
-  [Function `set`](#0x1_ExperimentalConsensusConfig_set)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemConsensusConfig.md#0x1_DiemConsensusConfig">0x1::DiemConsensusConfig</a>;
</code></pre>



<a name="0x1_ExperimentalConsensusConfig_ExperimentalConsensusConfig"></a>

## Struct `ExperimentalConsensusConfig`



<pre><code><b>struct</b> <a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig">ExperimentalConsensusConfig</a> <b>has</b> drop
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

<a name="0x1_ExperimentalConsensusConfig_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig_initialize">initialize</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig_initialize">initialize</a>(account: &signer) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemConsensusConfig.md#0x1_DiemConsensusConfig_initialize">DiemConsensusConfig::initialize</a>&lt;<a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig">ExperimentalConsensusConfig</a>&gt;(account);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_create">Capability::create</a>&lt;<a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig">ExperimentalConsensusConfig</a>&gt;(account, &<a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig">ExperimentalConsensusConfig</a> {});
}
</code></pre>



</details>

<a name="0x1_ExperimentalConsensusConfig_set"></a>

## Function `set`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig_set">set</a>(account: &signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig_set">set</a>(account: &signer, config: vector&lt;u8&gt;) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemConsensusConfig.md#0x1_DiemConsensusConfig_set">DiemConsensusConfig::set</a>(
        config, &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig">ExperimentalConsensusConfig</a> {})
    );
}
</code></pre>



</details>
