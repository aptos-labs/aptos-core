
<a name="0x1_ExperimentalVMConfig"></a>

# Module `0x1::ExperimentalVMConfig`



-  [Struct `ExperimentalVMConfig`](#0x1_ExperimentalVMConfig_ExperimentalVMConfig)
-  [Function `initialize`](#0x1_ExperimentalVMConfig_initialize)
-  [Function `set_gas_constants`](#0x1_ExperimentalVMConfig_set_gas_constants)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemVMConfig.md#0x1_DiemVMConfig">0x1::DiemVMConfig</a>;
</code></pre>



<a name="0x1_ExperimentalVMConfig_ExperimentalVMConfig"></a>

## Struct `ExperimentalVMConfig`



<pre><code><b>struct</b> <a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig">ExperimentalVMConfig</a> <b>has</b> drop
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

<a name="0x1_ExperimentalVMConfig_initialize"></a>

## Function `initialize`

Publishes the VM config.


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig_initialize">initialize</a>(account: &signer, instruction_schedule: vector&lt;u8&gt;, native_schedule: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig_initialize">initialize</a>(
    account: &signer,
    instruction_schedule: vector&lt;u8&gt;,
    native_schedule: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemVMConfig.md#0x1_DiemVMConfig_initialize">DiemVMConfig::initialize</a>&lt;<a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig">ExperimentalVMConfig</a>&gt;(account, instruction_schedule, native_schedule);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_create">Capability::create</a>&lt;<a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig">ExperimentalVMConfig</a>&gt;(account, &<a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig">ExperimentalVMConfig</a> {});
}
</code></pre>



</details>

<a name="0x1_ExperimentalVMConfig_set_gas_constants"></a>

## Function `set_gas_constants`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig_set_gas_constants">set_gas_constants</a>(account: &signer, global_memory_per_byte_cost: u64, global_memory_per_byte_write_cost: u64, min_transaction_gas_units: u64, large_transaction_cutoff: u64, intrinsic_gas_per_byte: u64, maximum_number_of_gas_units: u64, min_price_per_gas_unit: u64, max_price_per_gas_unit: u64, max_transaction_size_in_bytes: u64, gas_unit_scaling_factor: u64, default_account_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig_set_gas_constants">set_gas_constants</a>(
    account: &signer,
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
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemVMConfig.md#0x1_DiemVMConfig_set_gas_constants">DiemVMConfig::set_gas_constants</a>&lt;<a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig">ExperimentalVMConfig</a>&gt;(
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
        &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig">ExperimentalVMConfig</a> {}),
    );
}
</code></pre>



</details>
