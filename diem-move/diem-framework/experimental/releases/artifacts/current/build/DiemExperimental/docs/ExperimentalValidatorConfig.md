
<a name="0x1_ExperimentalValidatorConfig"></a>

# Module `0x1::ExperimentalValidatorConfig`



-  [Struct `ExperimentalValidatorConfig`](#0x1_ExperimentalValidatorConfig_ExperimentalValidatorConfig)
-  [Function `initialize`](#0x1_ExperimentalValidatorConfig_initialize)
-  [Function `publish`](#0x1_ExperimentalValidatorConfig_publish)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="ValidatorConfig.md#0x1_ValidatorConfig">0x1::ValidatorConfig</a>;
</code></pre>



<a name="0x1_ExperimentalValidatorConfig_ExperimentalValidatorConfig"></a>

## Struct `ExperimentalValidatorConfig`



<pre><code><b>struct</b> <a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig">ExperimentalValidatorConfig</a> <b>has</b> drop
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

<a name="0x1_ExperimentalValidatorConfig_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig_initialize">initialize</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig_initialize">initialize</a>(account: &signer) {
    <a href="ValidatorConfig.md#0x1_ValidatorConfig_initialize">ValidatorConfig::initialize</a>&lt;<a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig">ExperimentalValidatorConfig</a>&gt;(account);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_create">Capability::create</a>(account, &<a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig">ExperimentalValidatorConfig</a>{});
}
</code></pre>



</details>

<a name="0x1_ExperimentalValidatorConfig_publish"></a>

## Function `publish`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig_publish">publish</a>(root_account: &signer, validator_account: &signer, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig_publish">publish</a>(
    root_account: &signer,
    validator_account: &signer,
    human_name: vector&lt;u8&gt;,
) {
    <a href="ValidatorConfig.md#0x1_ValidatorConfig_publish">ValidatorConfig::publish</a>(
        validator_account,
        human_name,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(root_account, &<a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig">ExperimentalValidatorConfig</a>{})
    );
}
</code></pre>



</details>
