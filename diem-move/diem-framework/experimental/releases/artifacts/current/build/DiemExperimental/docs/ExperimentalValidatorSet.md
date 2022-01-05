
<a name="0x1_ExperimentalValidatorSet"></a>

# Module `0x1::ExperimentalValidatorSet`



-  [Struct `ExperimentalValidatorSet`](#0x1_ExperimentalValidatorSet_ExperimentalValidatorSet)
-  [Function `initialize_validator_set`](#0x1_ExperimentalValidatorSet_initialize_validator_set)
-  [Function `add_validator`](#0x1_ExperimentalValidatorSet_add_validator)
-  [Function `remove_validator`](#0x1_ExperimentalValidatorSet_remove_validator)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="DiemSystem.md#0x1_DiemSystem">0x1::DiemSystem</a>;
</code></pre>



<a name="0x1_ExperimentalValidatorSet_ExperimentalValidatorSet"></a>

## Struct `ExperimentalValidatorSet`



<pre><code><b>struct</b> <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet">ExperimentalValidatorSet</a> <b>has</b> drop
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

<a name="0x1_ExperimentalValidatorSet_initialize_validator_set"></a>

## Function `initialize_validator_set`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet_initialize_validator_set">initialize_validator_set</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet_initialize_validator_set">initialize_validator_set</a>(
    account: &signer,
) {
    <a href="DiemSystem.md#0x1_DiemSystem_initialize_validator_set">DiemSystem::initialize_validator_set</a>&lt;<a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet">ExperimentalValidatorSet</a>&gt;(account);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_create">Capability::create</a>(account, &<a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet">ExperimentalValidatorSet</a> {});
}
</code></pre>



</details>

<a name="0x1_ExperimentalValidatorSet_add_validator"></a>

## Function `add_validator`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet_add_validator">add_validator</a>(account: &signer, validator_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet_add_validator">add_validator</a>(
    account: &signer,
    validator_addr: <b>address</b>,
) {
    <a href="DiemSystem.md#0x1_DiemSystem_add_validator">DiemSystem::add_validator</a>(
        validator_addr,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet">ExperimentalValidatorSet</a> {})
    );
}
</code></pre>



</details>

<a name="0x1_ExperimentalValidatorSet_remove_validator"></a>

## Function `remove_validator`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet_remove_validator">remove_validator</a>(account: &signer, validator_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet_remove_validator">remove_validator</a>(
    account: &signer,
    validator_addr: <b>address</b>,
) {
    <a href="DiemSystem.md#0x1_DiemSystem_remove_validator">DiemSystem::remove_validator</a>(
        validator_addr,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet">ExperimentalValidatorSet</a> {})
    );
}
</code></pre>



</details>
