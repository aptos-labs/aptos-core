
<a name="0x1_AptosValidatorConfig"></a>

# Module `0x1::AptosValidatorConfig`



-  [Function `initialize`](#0x1_AptosValidatorConfig_initialize)
-  [Function `publish`](#0x1_AptosValidatorConfig_publish)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorConfig.md#0x1_ValidatorConfig">0x1::ValidatorConfig</a>;
</code></pre>



<a name="0x1_AptosValidatorConfig_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_initialize">initialize</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_initialize">initialize</a>(account: &signer) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorConfig.md#0x1_ValidatorConfig_initialize">ValidatorConfig::initialize</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(account);
}
</code></pre>



</details>

<a name="0x1_AptosValidatorConfig_publish"></a>

## Function `publish`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_publish">publish</a>(root_account: &signer, validator_account: &signer, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_publish">publish</a>(
    root_account: &signer,
    validator_account: &signer,
    human_name: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorConfig.md#0x1_ValidatorConfig_publish">ValidatorConfig::publish</a>(
        validator_account,
        human_name,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(root_account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
    );
}
</code></pre>



</details>
