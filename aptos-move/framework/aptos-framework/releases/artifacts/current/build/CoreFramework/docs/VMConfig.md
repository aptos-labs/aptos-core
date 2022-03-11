
<a name="0x1_VMConfig"></a>

# Module `0x1::VMConfig`

This module defines structs and methods to initialize VM configurations,
including different costs of running the VM.


-  [Resource `VMConfigChainMarker`](#0x1_VMConfig_VMConfigChainMarker)
-  [Resource `VMConfig`](#0x1_VMConfig_VMConfig)
-  [Struct `GasSchedule`](#0x1_VMConfig_GasSchedule)
-  [Struct `GasConstants`](#0x1_VMConfig_GasConstants)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_VMConfig_initialize)
-  [Function `set_gas_constants`](#0x1_VMConfig_set_gas_constants)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="Reconfiguration.md#0x1_Reconfiguration">0x1::Reconfiguration</a>;
<b>use</b> <a href="SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
<b>use</b> <a href="Timestamp.md#0x1_Timestamp">0x1::Timestamp</a>;
</code></pre>



<a name="0x1_VMConfig_VMConfigChainMarker"></a>

## Resource `VMConfigChainMarker`

Marker to be stored under @CoreResources during genesis


<pre><code><b>struct</b> <a href="VMConfig.md#0x1_VMConfig_VMConfigChainMarker">VMConfigChainMarker</a>&lt;T&gt; <b>has</b> key
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

<a name="0x1_VMConfig_VMConfig"></a>

## Resource `VMConfig`

The struct to hold config data needed to operate the VM.


<pre><code><b>struct</b> <a href="VMConfig.md#0x1_VMConfig">VMConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>gas_schedule: <a href="VMConfig.md#0x1_VMConfig_GasSchedule">VMConfig::GasSchedule</a></code>
</dt>
<dd>
 Cost of running the VM.
</dd>
</dl>


</details>

<a name="0x1_VMConfig_GasSchedule"></a>

## Struct `GasSchedule`

The gas schedule keeps two separate schedules for the gas:
* The instruction_schedule: This holds the gas for each bytecode instruction.
* The native_schedule: This holds the gas for used (per-byte operated over) for each native
function.
A couple notes:
1. In the case that an instruction is deleted from the bytecode, that part of the cost schedule
still needs to remain the same; once a slot in the table is taken by an instruction, that is its
slot for the rest of time (since that instruction could already exist in a module on-chain).
2. The initialization of the module will publish the instruction table to the root account
address, and will preload the vector with the gas schedule for instructions. The VM will then
load this into memory at the startup of each block.


<pre><code><b>struct</b> <a href="VMConfig.md#0x1_VMConfig_GasSchedule">GasSchedule</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>instruction_schedule: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>native_schedule: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_constants: <a href="VMConfig.md#0x1_VMConfig_GasConstants">VMConfig::GasConstants</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_VMConfig_GasConstants"></a>

## Struct `GasConstants`



<pre><code><b>struct</b> <a href="VMConfig.md#0x1_VMConfig_GasConstants">GasConstants</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>global_memory_per_byte_cost: u64</code>
</dt>
<dd>
 The cost per-byte read from global storage.
</dd>
<dt>
<code>global_memory_per_byte_write_cost: u64</code>
</dt>
<dd>
 The cost per-byte written to storage.
</dd>
<dt>
<code>min_transaction_gas_units: u64</code>
</dt>
<dd>
 The flat minimum amount of gas required for any transaction.
 Charged at the start of execution.
</dd>
<dt>
<code>large_transaction_cutoff: u64</code>
</dt>
<dd>
 Any transaction over this size will be charged an additional amount per byte.
</dd>
<dt>
<code>intrinsic_gas_per_byte: u64</code>
</dt>
<dd>
 The units of gas to be charged per byte over the <code>large_transaction_cutoff</code> in addition to
 <code>min_transaction_gas_units</code> for transactions whose size exceeds <code>large_transaction_cutoff</code>.
</dd>
<dt>
<code>maximum_number_of_gas_units: u64</code>
</dt>
<dd>
 ~5 microseconds should equal one unit of computational gas. We bound the maximum
 computational time of any given transaction at roughly 20 seconds. We want this number and
 <code>MAX_PRICE_PER_GAS_UNIT</code> to always satisfy the inequality that
 MAXIMUM_NUMBER_OF_GAS_UNITS * MAX_PRICE_PER_GAS_UNIT < min(u64::MAX, GasUnits<GasCarrier>::MAX)
 NB: The bound is set quite high since custom scripts aren't allowed except from predefined
 and vetted senders.
</dd>
<dt>
<code>min_price_per_gas_unit: u64</code>
</dt>
<dd>
 The minimum gas price that a transaction can be submitted with.
</dd>
<dt>
<code>max_price_per_gas_unit: u64</code>
</dt>
<dd>
 The maximum gas unit price that a transaction can be submitted with.
</dd>
<dt>
<code>max_transaction_size_in_bytes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_unit_scaling_factor: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>default_account_size: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_VMConfig_ECONFIG"></a>

Error with config


<pre><code><b>const</b> <a href="VMConfig.md#0x1_VMConfig_ECONFIG">ECONFIG</a>: u64 = 1;
</code></pre>



<a name="0x1_VMConfig_ECHAIN_MARKER"></a>

Error with chain marker


<pre><code><b>const</b> <a href="VMConfig.md#0x1_VMConfig_ECHAIN_MARKER">ECHAIN_MARKER</a>: u64 = 0;
</code></pre>



<a name="0x1_VMConfig_EGAS_CONSTANT_INCONSISTENCY"></a>

The provided gas constants were inconsistent.


<pre><code><b>const</b> <a href="VMConfig.md#0x1_VMConfig_EGAS_CONSTANT_INCONSISTENCY">EGAS_CONSTANT_INCONSISTENCY</a>: u64 = 2;
</code></pre>



<a name="0x1_VMConfig_initialize"></a>

## Function `initialize`

Initialize the table under the root account


<pre><code><b>public</b> <b>fun</b> <a href="VMConfig.md#0x1_VMConfig_initialize">initialize</a>&lt;T&gt;(account: &signer, instruction_schedule: vector&lt;u8&gt;, native_schedule: vector&lt;u8&gt;, min_price_per_gas_unit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="VMConfig.md#0x1_VMConfig_initialize">initialize</a>&lt;T&gt;(
    account: &signer,
    instruction_schedule: vector&lt;u8&gt;,
    native_schedule: vector&lt;u8&gt;,
    min_price_per_gas_unit: u64,
) {
    <a href="Timestamp.md#0x1_Timestamp_assert_genesis">Timestamp::assert_genesis</a>();

    <a href="SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(account);

    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="VMConfig.md#0x1_VMConfig_VMConfigChainMarker">VMConfigChainMarker</a>&lt;T&gt;&gt;(@CoreResources),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="VMConfig.md#0x1_VMConfig_ECHAIN_MARKER">ECHAIN_MARKER</a>)
    );

    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="VMConfig.md#0x1_VMConfig">VMConfig</a>&gt;(@CoreResources),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="VMConfig.md#0x1_VMConfig_ECONFIG">ECONFIG</a>)
    );

    <b>move_to</b>(account, <a href="VMConfig.md#0x1_VMConfig_VMConfigChainMarker">VMConfigChainMarker</a>&lt;T&gt;{});

    <b>let</b> gas_constants = <a href="VMConfig.md#0x1_VMConfig_GasConstants">GasConstants</a> {
        global_memory_per_byte_cost: 4,
        global_memory_per_byte_write_cost: 9,
        min_transaction_gas_units: 600,
        large_transaction_cutoff: 600,
        intrinsic_gas_per_byte: 8,
        maximum_number_of_gas_units: 4000000,
        min_price_per_gas_unit,
        max_price_per_gas_unit: 10000,
        max_transaction_size_in_bytes: 4096,
        gas_unit_scaling_factor: 1000,
        default_account_size: 800,
    };

    <b>move_to</b>(
        account,
        <a href="VMConfig.md#0x1_VMConfig">VMConfig</a> {
            gas_schedule: <a href="VMConfig.md#0x1_VMConfig_GasSchedule">GasSchedule</a> {
                instruction_schedule,
                native_schedule,
                gas_constants,
            }
        },
    );
}
</code></pre>



</details>

<a name="0x1_VMConfig_set_gas_constants"></a>

## Function `set_gas_constants`



<pre><code><b>public</b> <b>fun</b> <a href="VMConfig.md#0x1_VMConfig_set_gas_constants">set_gas_constants</a>&lt;T&gt;(global_memory_per_byte_cost: u64, global_memory_per_byte_write_cost: u64, min_transaction_gas_units: u64, large_transaction_cutoff: u64, intrinsic_gas_per_byte: u64, maximum_number_of_gas_units: u64, min_price_per_gas_unit: u64, max_price_per_gas_unit: u64, max_transaction_size_in_bytes: u64, gas_unit_scaling_factor: u64, default_account_size: u64, _cap: &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_Cap">Capability::Cap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="VMConfig.md#0x1_VMConfig_set_gas_constants">set_gas_constants</a>&lt;T&gt;(
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
    _cap: &Cap&lt;T&gt;
) <b>acquires</b> <a href="VMConfig.md#0x1_VMConfig">VMConfig</a> {
    <a href="Timestamp.md#0x1_Timestamp_assert_operating">Timestamp::assert_operating</a>();

    <b>assert</b>!(<b>exists</b>&lt;<a href="VMConfig.md#0x1_VMConfig_VMConfigChainMarker">VMConfigChainMarker</a>&lt;T&gt;&gt;(@CoreResources), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_not_published">Errors::not_published</a>(<a href="VMConfig.md#0x1_VMConfig_ECHAIN_MARKER">ECHAIN_MARKER</a>));

    <b>assert</b>!(
        min_price_per_gas_unit &lt;= max_price_per_gas_unit,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="VMConfig.md#0x1_VMConfig_EGAS_CONSTANT_INCONSISTENCY">EGAS_CONSTANT_INCONSISTENCY</a>)
    );
    <b>assert</b>!(
        min_transaction_gas_units &lt;= maximum_number_of_gas_units,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="VMConfig.md#0x1_VMConfig_EGAS_CONSTANT_INCONSISTENCY">EGAS_CONSTANT_INCONSISTENCY</a>)
    );

    <b>assert</b>!(<b>exists</b>&lt;<a href="VMConfig.md#0x1_VMConfig">VMConfig</a>&gt;(@CoreResources), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_not_published">Errors::not_published</a>(<a href="VMConfig.md#0x1_VMConfig_ECONFIG">ECONFIG</a>));

    <b>let</b> gas_constants = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="VMConfig.md#0x1_VMConfig">VMConfig</a>&gt;(@CoreResources).gas_schedule.gas_constants;

    gas_constants.global_memory_per_byte_cost       = global_memory_per_byte_cost;
    gas_constants.global_memory_per_byte_write_cost = global_memory_per_byte_write_cost;
    gas_constants.min_transaction_gas_units         = min_transaction_gas_units;
    gas_constants.large_transaction_cutoff          = large_transaction_cutoff;
    gas_constants.intrinsic_gas_per_byte            = intrinsic_gas_per_byte;
    gas_constants.maximum_number_of_gas_units       = maximum_number_of_gas_units;
    gas_constants.min_price_per_gas_unit            = min_price_per_gas_unit;
    gas_constants.max_price_per_gas_unit            = max_price_per_gas_unit;
    gas_constants.max_transaction_size_in_bytes     = max_transaction_size_in_bytes;
    gas_constants.gas_unit_scaling_factor           = gas_unit_scaling_factor;
    gas_constants.default_account_size              = default_account_size;

    <a href="Reconfiguration.md#0x1_Reconfiguration_reconfigure">Reconfiguration::reconfigure</a>();
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")
[ACCESS_CONTROL]: https://github.com/diem/dip/blob/main/dips/dip-2.md
[ROLE]: https://github.com/diem/dip/blob/main/dips/dip-2.md#roles
[PERMISSION]: https://github.com/diem/dip/blob/main/dips/dip-2.md#permissions
