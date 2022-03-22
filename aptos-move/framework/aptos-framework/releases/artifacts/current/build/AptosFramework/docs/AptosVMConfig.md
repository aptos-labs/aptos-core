
<a name="0x1_AptosVMConfig"></a>

# Module `0x1::AptosVMConfig`



-  [Function `initialize`](#0x1_AptosVMConfig_initialize)
-  [Function `set_gas_constants`](#0x1_AptosVMConfig_set_gas_constants)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/VMConfig.md#0x1_VMConfig">0x1::VMConfig</a>;
</code></pre>



<a name="0x1_AptosVMConfig_initialize"></a>

## Function `initialize`

Publishes the VM config.


<pre><code><b>public</b> <b>fun</b> <a href="AptosVMConfig.md#0x1_AptosVMConfig_initialize">initialize</a>(account: &signer, instruction_schedule: vector&lt;u8&gt;, native_schedule: vector&lt;u8&gt;, min_price_per_gas_unit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosVMConfig.md#0x1_AptosVMConfig_initialize">initialize</a>(
    account: &signer,
    instruction_schedule: vector&lt;u8&gt;,
    native_schedule: vector&lt;u8&gt;,
    min_price_per_gas_unit: u64,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/VMConfig.md#0x1_VMConfig_initialize">VMConfig::initialize</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(account, instruction_schedule, native_schedule, min_price_per_gas_unit);
}
</code></pre>



</details>

<a name="0x1_AptosVMConfig_set_gas_constants"></a>

## Function `set_gas_constants`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosVMConfig.md#0x1_AptosVMConfig_set_gas_constants">set_gas_constants</a>(account: signer, global_memory_per_byte_cost: u64, global_memory_per_byte_write_cost: u64, min_transaction_gas_units: u64, large_transaction_cutoff: u64, intrinsic_gas_per_byte: u64, maximum_number_of_gas_units: u64, min_price_per_gas_unit: u64, max_price_per_gas_unit: u64, max_transaction_size_in_bytes: u64, gas_unit_scaling_factor: u64, default_account_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosVMConfig.md#0x1_AptosVMConfig_set_gas_constants">set_gas_constants</a>(
    account: signer,
    global_memory_per_byte_cost: u64,
    global_memory_per_byte_write_cost: u64,
    min_transaction_gas_units: u64,
    large_transaction_cutoff: u64,
    intrinsic_gas_per_byte: u64,
    maximum_number_of_gas_units: u64,
    min_price_per_gas_unit: u64,
    max_price_per_gas_unit: u64,
    max_transaction_size_in_bytes: u64,
    gas_unit_scaling_factor: u64,
    default_account_size: u64,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/VMConfig.md#0x1_VMConfig_set_gas_constants">VMConfig::set_gas_constants</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(
        global_memory_per_byte_cost,
        global_memory_per_byte_write_cost,
        min_transaction_gas_units,
        large_transaction_cutoff,
        intrinsic_gas_per_byte,
        maximum_number_of_gas_units,
        min_price_per_gas_unit,
        max_price_per_gas_unit,
        max_transaction_size_in_bytes,
        gas_unit_scaling_factor,
        default_account_size,
        &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(&account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>()),
    );
}
</code></pre>



</details>
