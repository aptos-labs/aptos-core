
<a name="0x1_AptosValidatorOperatorConfig"></a>

# Module `0x1::AptosValidatorOperatorConfig`



-  [Function `initialize`](#0x1_AptosValidatorOperatorConfig_initialize)
-  [Function `publish`](#0x1_AptosValidatorOperatorConfig_publish)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig">0x1::ValidatorOperatorConfig</a>;
</code></pre>



<a name="0x1_AptosValidatorOperatorConfig_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorOperatorConfig.md#0x1_AptosValidatorOperatorConfig_initialize">initialize</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorOperatorConfig.md#0x1_AptosValidatorOperatorConfig_initialize">initialize</a>(account: &signer) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig_initialize">ValidatorOperatorConfig::initialize</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(account);
}
</code></pre>



</details>

<a name="0x1_AptosValidatorOperatorConfig_publish"></a>

## Function `publish`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="AptosValidatorOperatorConfig.md#0x1_AptosValidatorOperatorConfig_publish">publish</a>(root_account: &signer, validator_operator_account: &signer, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="AptosValidatorOperatorConfig.md#0x1_AptosValidatorOperatorConfig_publish">publish</a>(
    root_account: &signer,
    validator_operator_account: &signer,
    human_name: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig_publish">ValidatorOperatorConfig::publish</a>(
        validator_operator_account,
        human_name,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(root_account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
    );
}
</code></pre>



</details>
