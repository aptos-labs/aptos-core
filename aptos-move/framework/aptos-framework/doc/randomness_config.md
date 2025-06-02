
<a id="0x1_randomness_config"></a>

# Module `0x1::randomness_config`

Structs and functions for on-chain randomness configurations.


-  [Resource `RandomnessConfig`](#0x1_randomness_config_RandomnessConfig)
-  [Struct `ConfigOff`](#0x1_randomness_config_ConfigOff)
-  [Struct `ConfigV1`](#0x1_randomness_config_ConfigV1)
-  [Struct `ConfigV2`](#0x1_randomness_config_ConfigV2)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_randomness_config_initialize)
-  [Function `set_for_next_epoch`](#0x1_randomness_config_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_randomness_config_on_new_epoch)
-  [Function `enabled`](#0x1_randomness_config_enabled)
-  [Function `new_off`](#0x1_randomness_config_new_off)
-  [Function `new_v1`](#0x1_randomness_config_new_v1)
-  [Function `new_v2`](#0x1_randomness_config_new_v2)
-  [Function `current`](#0x1_randomness_config_current)
-  [Specification](#@Specification_1)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)
    -  [Function `current`](#@Specification_1_current)


<pre><code><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_randomness_config_RandomnessConfig"></a>

## Resource `RandomnessConfig`

The configuration of the on-chain randomness feature.


<pre><code><b>struct</b> <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 A config variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 - <code><a href="randomness_config.md#0x1_randomness_config_ConfigOff">ConfigOff</a></code>
 - <code><a href="randomness_config.md#0x1_randomness_config_ConfigV1">ConfigV1</a></code>
</dd>
</dl>


</details>

<a id="0x1_randomness_config_ConfigOff"></a>

## Struct `ConfigOff`

A randomness config variant indicating the feature is disabled.


<pre><code><b>struct</b> <a href="randomness_config.md#0x1_randomness_config_ConfigOff">ConfigOff</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_randomness_config_ConfigV1"></a>

## Struct `ConfigV1`

A randomness config variant indicating the feature is enabled.


<pre><code><b>struct</b> <a href="randomness_config.md#0x1_randomness_config_ConfigV1">ConfigV1</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>
 Any validator subset should not be able to reconstruct randomness if <code>subset_power / total_power &lt;= secrecy_threshold</code>,
</dd>
<dt>
<code>reconstruction_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>
 Any validator subset should be able to reconstruct randomness if <code>subset_power / total_power &gt; reconstruction_threshold</code>.
</dd>
</dl>


</details>

<a id="0x1_randomness_config_ConfigV2"></a>

## Struct `ConfigV2`

A randomness config variant indicating the feature is enabled with fast path.


<pre><code><b>struct</b> <a href="randomness_config.md#0x1_randomness_config_ConfigV2">ConfigV2</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>
 Any validator subset should not be able to reconstruct randomness if <code>subset_power / total_power &lt;= secrecy_threshold</code>,
</dd>
<dt>
<code>reconstruction_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>
 Any validator subset should be able to reconstruct randomness if <code>subset_power / total_power &gt; reconstruction_threshold</code>.
</dd>
<dt>
<code>fast_path_secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>
 Any validator subset should not be able to reconstruct randomness via the fast path if <code>subset_power / total_power &lt;= fast_path_secrecy_threshold</code>,
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_randomness_config_EINVALID_CONFIG_VARIANT"></a>



<pre><code><b>const</b> <a href="randomness_config.md#0x1_randomness_config_EINVALID_CONFIG_VARIANT">EINVALID_CONFIG_VARIANT</a>: u64 = 1;
</code></pre>



<a id="0x1_randomness_config_initialize"></a>

## Function `initialize`

Initialize the configuration. Used in genesis or governance.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(framework, config)
    }
}
</code></pre>



</details>

<a id="0x1_randomness_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on-chain governance to update on-chain consensus configs for the next epoch.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_config: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_config: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(new_config);
}
</code></pre>



</details>

<a id="0x1_randomness_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;()) {
        <b>let</b> new_config = <a href="config_buffer.md#0x1_config_buffer_extract_v2">config_buffer::extract_v2</a>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;();
        <b>if</b> (<b>exists</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)) {
            *<b>borrow_global_mut</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework) = new_config;
        } <b>else</b> {
            <b>move_to</b>(framework, new_config);
        }
    }
}
</code></pre>



</details>

<a id="0x1_randomness_config_enabled"></a>

## Function `enabled`

Check whether on-chain randomness main logic (e.g., <code>DKGManager</code>, <code>RandManager</code>, <code>BlockMetadataExt</code>) is enabled.

NOTE: this returning true does not mean randomness will run.
The feature works if and only if <code><a href="consensus_config.md#0x1_consensus_config_validator_txn_enabled">consensus_config::validator_txn_enabled</a>() && <a href="randomness_config.md#0x1_randomness_config_enabled">randomness_config::enabled</a>()</code>.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_enabled">enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_enabled">enabled</a>(): bool <b>acquires</b> <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)) {
        <b>let</b> config = <b>borrow_global</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework);
        <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&config.variant));
        variant_type_name != b"<a href="randomness_config.md#0x1_randomness_config_ConfigOff">0x1::randomness_config::ConfigOff</a>"
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_randomness_config_new_off"></a>

## Function `new_off`

Create a <code><a href="randomness_config.md#0x1_randomness_config_ConfigOff">ConfigOff</a></code> variant.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_off">new_off</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_off">new_off</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
    <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="randomness_config.md#0x1_randomness_config_ConfigOff">ConfigOff</a> {} )
    }
}
</code></pre>



</details>

<a id="0x1_randomness_config_new_v1"></a>

## Function `new_v1`

Create a <code><a href="randomness_config.md#0x1_randomness_config_ConfigV1">ConfigV1</a></code> variant.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_v1">new_v1</a>(secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, reconstruction_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_v1">new_v1</a>(secrecy_threshold: FixedPoint64, reconstruction_threshold: FixedPoint64): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
    <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="randomness_config.md#0x1_randomness_config_ConfigV1">ConfigV1</a> {
            secrecy_threshold,
            reconstruction_threshold
        } )
    }
}
</code></pre>



</details>

<a id="0x1_randomness_config_new_v2"></a>

## Function `new_v2`

Create a <code><a href="randomness_config.md#0x1_randomness_config_ConfigV2">ConfigV2</a></code> variant.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_v2">new_v2</a>(secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, reconstruction_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, fast_path_secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_v2">new_v2</a>(
    secrecy_threshold: FixedPoint64,
    reconstruction_threshold: FixedPoint64,
    fast_path_secrecy_threshold: FixedPoint64,
): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
    <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="randomness_config.md#0x1_randomness_config_ConfigV2">ConfigV2</a> {
            secrecy_threshold,
            reconstruction_threshold,
            fast_path_secrecy_threshold,
        } )
    }
}
</code></pre>



</details>

<a id="0x1_randomness_config_current"></a>

## Function `current`

Get the currently effective randomness configuration object.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_current">current</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_current">current</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> <b>acquires</b> <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)) {
        *<b>borrow_global</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)
    } <b>else</b> {
        <a href="randomness_config.md#0x1_randomness_config_new_off">new_off</a>()
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>requires</b> @aptos_framework == std::signer::address_of(framework);
<b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;;
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_current"></a>

### Function `current`


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_current">current</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
