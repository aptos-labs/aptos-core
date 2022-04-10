
<a name="0x1_AptosValidatorSet"></a>

# Module `0x1::AptosValidatorSet`



-  [Function `initialize_validator_set`](#0x1_AptosValidatorSet_initialize_validator_set)
-  [Function `add_validator`](#0x1_AptosValidatorSet_add_validator)
-  [Function `remove_validator`](#0x1_AptosValidatorSet_remove_validator)
-  [Function `add_validator_internal`](#0x1_AptosValidatorSet_add_validator_internal)


<pre><code><b>use</b> <a href="../MoveStdlib/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
<b>use</b> <a href="../CoreFramework/ValidatorSystem.md#0x1_ValidatorSystem">0x1::ValidatorSystem</a>;
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
    <a href="../CoreFramework/ValidatorSystem.md#0x1_ValidatorSystem_initialize_validator_set">ValidatorSystem::initialize_validator_set</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(account);
}
</code></pre>



</details>

<a name="0x1_AptosValidatorSet_add_validator"></a>

## Function `add_validator`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_add_validator">add_validator</a>(account: signer, validator_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_add_validator">add_validator</a>(
    account: signer,
    validator_addr: <b>address</b>,
) {
    <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_add_validator_internal">add_validator_internal</a>(&account, validator_addr);
}
</code></pre>



</details>

<a name="0x1_AptosValidatorSet_remove_validator"></a>

## Function `remove_validator`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_remove_validator">remove_validator</a>(account: signer, validator_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_remove_validator">remove_validator</a>(
    account: signer,
    validator_addr: <b>address</b>,
) {
    <a href="../CoreFramework/ValidatorSystem.md#0x1_ValidatorSystem_remove_validator">ValidatorSystem::remove_validator</a>(
        validator_addr,
        <a href="../MoveStdlib/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(&account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
    );
}
</code></pre>



</details>

<a name="0x1_AptosValidatorSet_add_validator_internal"></a>

## Function `add_validator_internal`



<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_add_validator_internal">add_validator_internal</a>(account: &signer, validator_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorSet.md#0x1_AptosValidatorSet_add_validator_internal">add_validator_internal</a>(
    account: &signer,
    validator_addr: <b>address</b>,
) {
    <a href="../CoreFramework/ValidatorSystem.md#0x1_ValidatorSystem_add_validator">ValidatorSystem::add_validator</a>(
        validator_addr,
        <a href="../MoveStdlib/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
    );
}
</code></pre>



</details>
