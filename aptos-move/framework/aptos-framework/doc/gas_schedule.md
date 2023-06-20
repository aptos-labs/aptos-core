
<a name="0x1_gas_schedule"></a>

# Module `0x1::gas_schedule`

This module defines structs and methods to initialize the gas schedule, which dictates how much
it costs to execute Move on the network.


-  [Struct `GasEntry`](#0x1_gas_schedule_GasEntry)
-  [Resource `GasSchedule`](#0x1_gas_schedule_GasSchedule)
-  [Resource `GasScheduleV2`](#0x1_gas_schedule_GasScheduleV2)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_gas_schedule_initialize)
-  [Function `set_gas_schedule`](#0x1_gas_schedule_set_gas_schedule)
-  [Function `set_storage_gas_config`](#0x1_gas_schedule_set_storage_gas_config)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `set_gas_schedule`](#@Specification_1_set_gas_schedule)
    -  [Function `set_storage_gas_config`](#@Specification_1_set_storage_gas_config)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="storage_gas.md#0x1_storage_gas">0x1::storage_gas</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="util.md#0x1_util">0x1::util</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_gas_schedule_GasEntry"></a>

## Struct `GasEntry`



<pre><code><b>struct</b> <a href="gas_schedule.md#0x1_gas_schedule_GasEntry">GasEntry</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>val: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_gas_schedule_GasSchedule"></a>

## Resource `GasSchedule`



<pre><code><b>struct</b> <a href="gas_schedule.md#0x1_gas_schedule_GasSchedule">GasSchedule</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasEntry">gas_schedule::GasEntry</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_gas_schedule_GasScheduleV2"></a>

## Resource `GasScheduleV2`



<pre><code><b>struct</b> <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>feature_version: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasEntry">gas_schedule::GasEntry</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_gas_schedule_EINVALID_GAS_FEATURE_VERSION"></a>



<pre><code><b>const</b> <a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_FEATURE_VERSION">EINVALID_GAS_FEATURE_VERSION</a>: u64 = 2;
</code></pre>



<a name="0x1_gas_schedule_EINVALID_GAS_SCHEDULE"></a>

The provided gas schedule bytes are empty or invalid


<pre><code><b>const</b> <a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_SCHEDULE">EINVALID_GAS_SCHEDULE</a>: u64 = 1;
</code></pre>



<a name="0x1_gas_schedule_initialize"></a>

## Function `initialize`

Only called during genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&gas_schedule_blob), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_SCHEDULE">EINVALID_GAS_SCHEDULE</a>));

    // TODO(Gas): check <b>if</b> gas schedule is consistent
    <b>let</b> <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> = from_bytes(gas_schedule_blob);
    <b>move_to</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(aptos_framework, <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>);
}
</code></pre>



</details>

<a name="0x1_gas_schedule_set_gas_schedule"></a>

## Function `set_gas_schedule`

This can be called by on-chain governance to update the gas schedule.


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_gas_schedule">set_gas_schedule</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_gas_schedule">set_gas_schedule</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="gas_schedule.md#0x1_gas_schedule_GasSchedule">GasSchedule</a>, <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&gas_schedule_blob), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_SCHEDULE">EINVALID_GAS_SCHEDULE</a>));

    <b>if</b> (<b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework)) {
        <b>let</b> <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a> = <b>borrow_global_mut</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework);
        <b>let</b> new_gas_schedule: <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> = from_bytes(gas_schedule_blob);
        <b>assert</b>!(new_gas_schedule.feature_version &gt;= <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>.feature_version,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_FEATURE_VERSION">EINVALID_GAS_FEATURE_VERSION</a>));
        // TODO(Gas): check <b>if</b> gas schedule is consistent
        *<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a> = new_gas_schedule;
    }
    <b>else</b> {
        <b>if</b> (<b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasSchedule">GasSchedule</a>&gt;(@aptos_framework)) {
            _ = <b>move_from</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasSchedule">GasSchedule</a>&gt;(@aptos_framework);
        };
        <b>let</b> new_gas_schedule: <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> = from_bytes(gas_schedule_blob);
        // TODO(Gas): check <b>if</b> gas schedule is consistent
        <b>move_to</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(aptos_framework, new_gas_schedule);
    };

    // Need <b>to</b> trigger <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> so validator nodes can sync on the updated gas schedule.
    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();
}
</code></pre>



</details>

<a name="0x1_gas_schedule_set_storage_gas_config"></a>

## Function `set_storage_gas_config`



<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config">set_storage_gas_config</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config">set_storage_gas_config</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: StorageGasConfig) {
    <a href="storage_gas.md#0x1_storage_gas_set_config">storage_gas::set_config</a>(aptos_framework, config);
    // Need <b>to</b> trigger <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> so the VM is guaranteed <b>to</b> load the new gas fee starting from the next
    // transaction.
    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();
}
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a name="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>{ <a href="account.md#0x1_account">account</a>: aptos_framework };
<b>aborts_if</b> len(gas_schedule_blob) == 0;
<b>aborts_if</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
<b>ensures</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
</code></pre>



<a name="@Specification_1_set_gas_schedule"></a>

### Function `set_gas_schedule`


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_gas_schedule">set_gas_schedule</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);
<b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
<b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;
<b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>{ <a href="account.md#0x1_account">account</a>: aptos_framework };
<b>aborts_if</b> len(gas_schedule_blob) == 0;
<b>let</b> new_gas_schedule = <a href="util.md#0x1_util_spec_from_bytes">util::spec_from_bytes</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(gas_schedule_blob);
<b>let</b> <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a> = <b>global</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework);
<b>aborts_if</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework) && new_gas_schedule.feature_version &lt; <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>.feature_version;
<b>ensures</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
</code></pre>



<a name="@Specification_1_set_storage_gas_config"></a>

### Function `set_storage_gas_config`


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config">set_storage_gas_config</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 200;
<b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);
<b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
<b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>{ <a href="account.md#0x1_account">account</a>: aptos_framework };
<b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;
<b>aborts_if</b> !<b>exists</b>&lt;StorageGasConfig&gt;(@aptos_framework);
</code></pre>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
