
<a name="0x1_DiemConsensusConfig"></a>

# Module `0x1::DiemConsensusConfig`

Maintains the consensus config for the Diem blockchain. The config is stored in a
DiemConfig, and may be updated by Diem root.


-  [Struct `DiemConsensusConfig`](#0x1_DiemConsensusConfig_DiemConsensusConfig)
-  [Function `initialize`](#0x1_DiemConsensusConfig_initialize)
-  [Function `set`](#0x1_DiemConsensusConfig_set)


<pre><code><b>use</b> <a href="DiemConfig.md#0x1_DiemConfig">0x1::DiemConfig</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_DiemConsensusConfig_DiemConsensusConfig"></a>

## Struct `DiemConsensusConfig`



<pre><code><b>struct</b> <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig">DiemConsensusConfig</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>config: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_DiemConsensusConfig_initialize"></a>

## Function `initialize`

Publishes the DiemConsensusConfig config.


<pre><code><b>public</b> <b>fun</b> <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig_initialize">initialize</a>(dr_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig_initialize">initialize</a>(dr_account: &signer) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(dr_account);
    <a href="DiemConfig.md#0x1_DiemConfig_publish_new_config">DiemConfig::publish_new_config</a>(dr_account, <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig">DiemConsensusConfig</a> { config: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>() });
}
</code></pre>



</details>

<a name="0x1_DiemConsensusConfig_set"></a>

## Function `set`

Allows Diem root to update the config.


<pre><code><b>public</b> <b>fun</b> <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig_set">set</a>(dr_account: &signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig_set">set</a>(dr_account: &signer, config: vector&lt;u8&gt;) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(dr_account);

    <a href="DiemConfig.md#0x1_DiemConfig_set">DiemConfig::set</a>(
        dr_account,
        <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig">DiemConsensusConfig</a> { config }
    );
}
</code></pre>



</details>
