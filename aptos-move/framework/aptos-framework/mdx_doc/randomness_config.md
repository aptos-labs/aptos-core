
<a id="0x1_randomness_config"></a>

# Module `0x1::randomness_config`

Structs and functions for on&#45;chain randomness configurations.


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


<pre><code><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_randomness_config_RandomnessConfig"></a>

## Resource `RandomnessConfig`

The configuration of the on&#45;chain randomness feature.


<pre><code><b>struct</b> <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 A config variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 &#45; <code><a href="randomness_config.md#0x1_randomness_config_ConfigOff">ConfigOff</a></code>
 &#45; <code><a href="randomness_config.md#0x1_randomness_config_ConfigV1">ConfigV1</a></code>
</dd>
</dl>


</details>

<a id="0x1_randomness_config_ConfigOff"></a>

## Struct `ConfigOff`

A randomness config variant indicating the feature is disabled.


<pre><code><b>struct</b> <a href="randomness_config.md#0x1_randomness_config_ConfigOff">ConfigOff</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="randomness_config.md#0x1_randomness_config_ConfigV1">ConfigV1</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>
 Any validator subset should not be able to reconstruct randomness if <code>subset_power / total_power &lt;&#61; secrecy_threshold</code>,
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


<pre><code><b>struct</b> <a href="randomness_config.md#0x1_randomness_config_ConfigV2">ConfigV2</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>
 Any validator subset should not be able to reconstruct randomness if <code>subset_power / total_power &lt;&#61; secrecy_threshold</code>,
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
 Any validator subset should not be able to reconstruct randomness via the fast path if <code>subset_power / total_power &lt;&#61; fast_path_secrecy_threshold</code>,
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_randomness_config_EINVALID_CONFIG_VARIANT"></a>



<pre><code><b>const</b> <a href="randomness_config.md#0x1_randomness_config_EINVALID_CONFIG_VARIANT">EINVALID_CONFIG_VARIANT</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_randomness_config_initialize"></a>

## Function `initialize`

Initialize the configuration. Used in genesis or governance.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_initialize">initialize</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_initialize">initialize</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)) &#123;<br />        <b>move_to</b>(framework, config)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on&#45;chain governance to update on&#45;chain consensus configs for the next epoch.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_config: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_config: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(new_config);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;()) &#123;<br />        <b>let</b> new_config &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;();<br />        <b>if</b> (<b>exists</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework) &#61; new_config;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(framework, new_config);<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_config_enabled"></a>

## Function `enabled`

Check whether on&#45;chain randomness main logic (e.g., <code>DKGManager</code>, <code>RandManager</code>, <code>BlockMetadataExt</code>) is enabled.

NOTE: this returning true does not mean randomness will run.
The feature works if and only if <code><a href="consensus_config.md#0x1_consensus_config_validator_txn_enabled">consensus_config::validator_txn_enabled</a>() &amp;&amp; <a href="randomness_config.md#0x1_randomness_config_enabled">randomness_config::enabled</a>()</code>.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_enabled">enabled</a>(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_enabled">enabled</a>(): bool <b>acquires</b> <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)) &#123;<br />        <b>let</b> config &#61; <b>borrow_global</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework);<br />        <b>let</b> variant_type_name &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&amp;config.variant));<br />        variant_type_name !&#61; b&quot;<a href="randomness_config.md#0x1_randomness_config_ConfigOff">0x1::randomness_config::ConfigOff</a>&quot;<br />    &#125; <b>else</b> &#123;<br />        <b>false</b><br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_config_new_off"></a>

## Function `new_off`

Create a <code><a href="randomness_config.md#0x1_randomness_config_ConfigOff">ConfigOff</a></code> variant.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_off">new_off</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_off">new_off</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />    <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="randomness_config.md#0x1_randomness_config_ConfigOff">ConfigOff</a> &#123;&#125; )<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_config_new_v1"></a>

## Function `new_v1`

Create a <code><a href="randomness_config.md#0x1_randomness_config_ConfigV1">ConfigV1</a></code> variant.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_v1">new_v1</a>(secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, reconstruction_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_v1">new_v1</a>(secrecy_threshold: FixedPoint64, reconstruction_threshold: FixedPoint64): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />    <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="randomness_config.md#0x1_randomness_config_ConfigV1">ConfigV1</a> &#123;<br />            secrecy_threshold,<br />            reconstruction_threshold<br />        &#125; )<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_config_new_v2"></a>

## Function `new_v2`

Create a <code><a href="randomness_config.md#0x1_randomness_config_ConfigV2">ConfigV2</a></code> variant.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_v2">new_v2</a>(secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, reconstruction_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, fast_path_secrecy_threshold: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_new_v2">new_v2</a>(<br />    secrecy_threshold: FixedPoint64,<br />    reconstruction_threshold: FixedPoint64,<br />    fast_path_secrecy_threshold: FixedPoint64,<br />): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />    <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="randomness_config.md#0x1_randomness_config_ConfigV2">ConfigV2</a> &#123;<br />            secrecy_threshold,<br />            reconstruction_threshold,<br />            fast_path_secrecy_threshold,<br />        &#125; )<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_config_current"></a>

## Function `current`

Get the currently effective randomness configuration object.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_current">current</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_current">current</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> <b>acquires</b> <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)) &#123;<br />        &#42;<b>borrow_global</b>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;(@aptos_framework)<br />    &#125; <b>else</b> &#123;<br />        <a href="randomness_config.md#0x1_randomness_config_new_off">new_off</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>requires</b> @aptos_framework &#61;&#61; std::signer::address_of(framework);<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">RandomnessConfig</a>&gt;;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_current"></a>

### Function `current`


<pre><code><b>public</b> <b>fun</b> <a href="randomness_config.md#0x1_randomness_config_current">current</a>(): <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a><br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
