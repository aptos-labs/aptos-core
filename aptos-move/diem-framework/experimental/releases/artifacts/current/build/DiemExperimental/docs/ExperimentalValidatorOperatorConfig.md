
<a name="0x1_ExperimentalValidatorOperatorConfig"></a>

# Module `0x1::ExperimentalValidatorOperatorConfig`



-  [Struct `ExperimentalValidatorOperatorConfig`](#0x1_ExperimentalValidatorOperatorConfig_ExperimentalValidatorOperatorConfig)
-  [Function `initialize`](#0x1_ExperimentalValidatorOperatorConfig_initialize)
-  [Function `publish`](#0x1_ExperimentalValidatorOperatorConfig_publish)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig">0x1::ValidatorOperatorConfig</a>;
</code></pre>



<a name="0x1_ExperimentalValidatorOperatorConfig_ExperimentalValidatorOperatorConfig"></a>

## Struct `ExperimentalValidatorOperatorConfig`



<pre><code><b>struct</b> <a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig">ExperimentalValidatorOperatorConfig</a> <b>has</b> drop
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

<a name="0x1_ExperimentalValidatorOperatorConfig_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig_initialize">initialize</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig_initialize">initialize</a>(account: &signer) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig_initialize">ValidatorOperatorConfig::initialize</a>&lt;<a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig">ExperimentalValidatorOperatorConfig</a>&gt;(account);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_create">Capability::create</a>(account, &<a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig">ExperimentalValidatorOperatorConfig</a>{});
}
</code></pre>



</details>

<a name="0x1_ExperimentalValidatorOperatorConfig_publish"></a>

## Function `publish`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig_publish">publish</a>(root_account: &signer, validator_operator_account: &signer, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig_publish">publish</a>(
    root_account: &signer,
    validator_operator_account: &signer,
    human_name: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig_publish">ValidatorOperatorConfig::publish</a>(
        validator_operator_account,
        human_name,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(root_account, &<a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig">ExperimentalValidatorOperatorConfig</a>{})
    );
}
</code></pre>



</details>
