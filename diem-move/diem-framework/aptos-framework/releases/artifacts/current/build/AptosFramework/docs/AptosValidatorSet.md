
<a name="0x1_AptosValidatorSet"></a>

# Module `0x1::AptosValidatorSet`



-  [Function `initialize_validator_set`](#0x1_AptosValidatorSet_initialize_validator_set)
-  [Function `add_validator`](#0x1_AptosValidatorSet_add_validator)
-  [Function `remove_validator`](#0x1_AptosValidatorSet_remove_validator)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/DiemSystem.md#0x1_DiemSystem">0x1::DiemSystem</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
</code></pre>



<a name="0x1_AptosValidatorSet_initialize_validator_set"></a>

## Function `initialize_validator_set`



<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_initialize_validator_set">initialize_validator_set</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_initialize_validator_set">initialize_validator_set</a>(
    account: &signer,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/DiemSystem.md#0x1_DiemSystem_initialize_validator_set">DiemSystem::initialize_validator_set</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(account);
}
</code></pre>



</details>

<a name="0x1_AptosValidatorSet_add_validator"></a>

## Function `add_validator`



<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_add_validator">add_validator</a>(account: &signer, validator_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_add_validator">add_validator</a>(
    account: &signer,
    validator_addr: <b>address</b>,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/DiemSystem.md#0x1_DiemSystem_add_validator">DiemSystem::add_validator</a>(
        validator_addr,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
    );
}
</code></pre>



</details>

<a name="0x1_AptosValidatorSet_remove_validator"></a>

## Function `remove_validator`



<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_remove_validator">remove_validator</a>(account: &signer, validator_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_remove_validator">remove_validator</a>(
    account: &signer,
    validator_addr: <b>address</b>,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/DiemSystem.md#0x1_DiemSystem_remove_validator">DiemSystem::remove_validator</a>(
        validator_addr,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
    );
}
</code></pre>



</details>
