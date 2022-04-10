
<a name="0x1_AptosValidatorConfig"></a>

# Module `0x1::AptosValidatorConfig`



-  [Function `initialize`](#0x1_AptosValidatorConfig_initialize)
-  [Function `publish`](#0x1_AptosValidatorConfig_publish)
-  [Function `register_validator_config`](#0x1_AptosValidatorConfig_register_validator_config)
-  [Function `set_validator_operator`](#0x1_AptosValidatorConfig_set_validator_operator)
-  [Function `set_validator_config_and_reconfigure`](#0x1_AptosValidatorConfig_set_validator_config_and_reconfigure)


<pre><code><b>use</b> <a href="../MoveStdlib/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
<b>use</b> <a href="../CoreFramework/ValidatorConfig.md#0x1_ValidatorConfig">0x1::ValidatorConfig</a>;
<b>use</b> <a href="../CoreFramework/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig">0x1::ValidatorOperatorConfig</a>;
<b>use</b> <a href="../CoreFramework/ValidatorSystem.md#0x1_ValidatorSystem">0x1::ValidatorSystem</a>;
</code></pre>



<a name="0x1_AptosValidatorConfig_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_initialize">initialize</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_initialize">initialize</a>(account: &signer) {
    <a href="../CoreFramework/ValidatorConfig.md#0x1_ValidatorConfig_initialize">ValidatorConfig::initialize</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(account);
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
    <a href="../CoreFramework/ValidatorConfig.md#0x1_ValidatorConfig_publish">ValidatorConfig::publish</a>(
        validator_account,
        human_name,
        <a href="../MoveStdlib/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(root_account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>())
    );
}
</code></pre>



</details>

<a name="0x1_AptosValidatorConfig_register_validator_config"></a>

## Function `register_validator_config`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_register_validator_config">register_validator_config</a>(validator_operator_account: signer, validator_address: <b>address</b>, consensus_pubkey: vector&lt;u8&gt;, validator_network_addresses: vector&lt;u8&gt;, fullnode_network_addresses: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_register_validator_config">register_validator_config</a>(
    validator_operator_account: signer,
    validator_address: <b>address</b>,
    consensus_pubkey: vector&lt;u8&gt;,
    validator_network_addresses: vector&lt;u8&gt;,
    fullnode_network_addresses: vector&lt;u8&gt;,
) {
    <a href="../CoreFramework/ValidatorConfig.md#0x1_ValidatorConfig_set_config">ValidatorConfig::set_config</a>(
        &validator_operator_account,
        validator_address,
        consensus_pubkey,
        validator_network_addresses,
        fullnode_network_addresses
    );
}
</code></pre>



</details>

<a name="0x1_AptosValidatorConfig_set_validator_operator"></a>

## Function `set_validator_operator`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_set_validator_operator">set_validator_operator</a>(account: signer, operator_name: vector&lt;u8&gt;, operator_account: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_set_validator_operator">set_validator_operator</a>(
    account: signer,
    operator_name: vector&lt;u8&gt;,
    operator_account: <b>address</b>
) {
    <b>assert</b>!(<a href="../CoreFramework/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig_get_human_name">ValidatorOperatorConfig::get_human_name</a>(operator_account) == operator_name, 0);
    <a href="../CoreFramework/ValidatorConfig.md#0x1_ValidatorConfig_set_operator">ValidatorConfig::set_operator</a>(&account, operator_account);
}
</code></pre>



</details>

<a name="0x1_AptosValidatorConfig_set_validator_config_and_reconfigure"></a>

## Function `set_validator_config_and_reconfigure`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_set_validator_config_and_reconfigure">set_validator_config_and_reconfigure</a>(validator_operator_account: signer, validator_account: <b>address</b>, consensus_pubkey: vector&lt;u8&gt;, validator_network_addresses: vector&lt;u8&gt;, fullnode_network_addresses: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosValidatorConfig.md#0x1_AptosValidatorConfig_set_validator_config_and_reconfigure">set_validator_config_and_reconfigure</a>(
    validator_operator_account: signer,
    validator_account: <b>address</b>,
    consensus_pubkey: vector&lt;u8&gt;,
    validator_network_addresses: vector&lt;u8&gt;,
    fullnode_network_addresses: vector&lt;u8&gt;,
) {
    <a href="../CoreFramework/ValidatorConfig.md#0x1_ValidatorConfig_set_config">ValidatorConfig::set_config</a>(
        &validator_operator_account,
        validator_account,
        consensus_pubkey,
        validator_network_addresses,
        fullnode_network_addresses
    );
    <a href="../CoreFramework/ValidatorSystem.md#0x1_ValidatorSystem_update_config_and_reconfigure">ValidatorSystem::update_config_and_reconfigure</a>(&validator_operator_account, validator_account);
}
</code></pre>



</details>
