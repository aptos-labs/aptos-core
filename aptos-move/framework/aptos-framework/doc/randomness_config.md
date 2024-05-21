
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


<pre><code>use 0x1::config_buffer;
use 0x1::copyable_any;
use 0x1::fixed_point64;
use 0x1::string;
use 0x1::system_addresses;
</code></pre>



<a id="0x1_randomness_config_RandomnessConfig"></a>

## Resource `RandomnessConfig`

The configuration of the on-chain randomness feature.


<pre><code>struct RandomnessConfig has copy, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: copyable_any::Any</code>
</dt>
<dd>
 A config variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 - <code>ConfigOff</code>
 - <code>ConfigV1</code>
</dd>
</dl>


</details>

<a id="0x1_randomness_config_ConfigOff"></a>

## Struct `ConfigOff`

A randomness config variant indicating the feature is disabled.


<pre><code>struct ConfigOff has copy, drop, store
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


<pre><code>struct ConfigV1 has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>secrecy_threshold: fixed_point64::FixedPoint64</code>
</dt>
<dd>
 Any validator subset should not be able to reconstruct randomness if <code>subset_power / total_power &lt;&#61; secrecy_threshold</code>,
</dd>
<dt>
<code>reconstruction_threshold: fixed_point64::FixedPoint64</code>
</dt>
<dd>
 Any validator subset should be able to reconstruct randomness if <code>subset_power / total_power &gt; reconstruction_threshold</code>.
</dd>
</dl>


</details>

<a id="0x1_randomness_config_ConfigV2"></a>

## Struct `ConfigV2`

A randomness config variant indicating the feature is enabled with fast path.


<pre><code>struct ConfigV2 has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>secrecy_threshold: fixed_point64::FixedPoint64</code>
</dt>
<dd>
 Any validator subset should not be able to reconstruct randomness if <code>subset_power / total_power &lt;&#61; secrecy_threshold</code>,
</dd>
<dt>
<code>reconstruction_threshold: fixed_point64::FixedPoint64</code>
</dt>
<dd>
 Any validator subset should be able to reconstruct randomness if <code>subset_power / total_power &gt; reconstruction_threshold</code>.
</dd>
<dt>
<code>fast_path_secrecy_threshold: fixed_point64::FixedPoint64</code>
</dt>
<dd>
 Any validator subset should not be able to reconstruct randomness via the fast path if <code>subset_power / total_power &lt;&#61; fast_path_secrecy_threshold</code>,
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_randomness_config_EINVALID_CONFIG_VARIANT"></a>



<pre><code>const EINVALID_CONFIG_VARIANT: u64 &#61; 1;
</code></pre>



<a id="0x1_randomness_config_initialize"></a>

## Function `initialize`

Initialize the configuration. Used in genesis or governance.


<pre><code>public fun initialize(framework: &amp;signer, config: randomness_config::RandomnessConfig)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(framework: &amp;signer, config: RandomnessConfig) &#123;
    system_addresses::assert_aptos_framework(framework);
    if (!exists&lt;RandomnessConfig&gt;(@aptos_framework)) &#123;
        move_to(framework, config)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on-chain governance to update on-chain consensus configs for the next epoch.


<pre><code>public fun set_for_next_epoch(framework: &amp;signer, new_config: randomness_config::RandomnessConfig)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(framework: &amp;signer, new_config: RandomnessConfig) &#123;
    system_addresses::assert_aptos_framework(framework);
    config_buffer::upsert(new_config);
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>RandomnessConfig</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires RandomnessConfig &#123;
    system_addresses::assert_aptos_framework(framework);
    if (config_buffer::does_exist&lt;RandomnessConfig&gt;()) &#123;
        let new_config &#61; config_buffer::extract&lt;RandomnessConfig&gt;();
        if (exists&lt;RandomnessConfig&gt;(@aptos_framework)) &#123;
            &#42;borrow_global_mut&lt;RandomnessConfig&gt;(@aptos_framework) &#61; new_config;
        &#125; else &#123;
            move_to(framework, new_config);
        &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_enabled"></a>

## Function `enabled`

Check whether on-chain randomness main logic (e.g., <code>DKGManager</code>, <code>RandManager</code>, <code>BlockMetadataExt</code>) is enabled.

NOTE: this returning true does not mean randomness will run.
The feature works if and only if <code>consensus_config::validator_txn_enabled() &amp;&amp; randomness_config::enabled()</code>.


<pre><code>public fun enabled(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun enabled(): bool acquires RandomnessConfig &#123;
    if (exists&lt;RandomnessConfig&gt;(@aptos_framework)) &#123;
        let config &#61; borrow_global&lt;RandomnessConfig&gt;(@aptos_framework);
        let variant_type_name &#61; &#42;string::bytes(copyable_any::type_name(&amp;config.variant));
        variant_type_name !&#61; b&quot;0x1::randomness_config::ConfigOff&quot;
    &#125; else &#123;
        false
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_new_off"></a>

## Function `new_off`

Create a <code>ConfigOff</code> variant.


<pre><code>public fun new_off(): randomness_config::RandomnessConfig
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_off(): RandomnessConfig &#123;
    RandomnessConfig &#123;
        variant: copyable_any::pack( ConfigOff &#123;&#125; )
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_new_v1"></a>

## Function `new_v1`

Create a <code>ConfigV1</code> variant.


<pre><code>public fun new_v1(secrecy_threshold: fixed_point64::FixedPoint64, reconstruction_threshold: fixed_point64::FixedPoint64): randomness_config::RandomnessConfig
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_v1(secrecy_threshold: FixedPoint64, reconstruction_threshold: FixedPoint64): RandomnessConfig &#123;
    RandomnessConfig &#123;
        variant: copyable_any::pack( ConfigV1 &#123;
            secrecy_threshold,
            reconstruction_threshold
        &#125; )
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_new_v2"></a>

## Function `new_v2`

Create a <code>ConfigV2</code> variant.


<pre><code>public fun new_v2(secrecy_threshold: fixed_point64::FixedPoint64, reconstruction_threshold: fixed_point64::FixedPoint64, fast_path_secrecy_threshold: fixed_point64::FixedPoint64): randomness_config::RandomnessConfig
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_v2(
    secrecy_threshold: FixedPoint64,
    reconstruction_threshold: FixedPoint64,
    fast_path_secrecy_threshold: FixedPoint64,
): RandomnessConfig &#123;
    RandomnessConfig &#123;
        variant: copyable_any::pack( ConfigV2 &#123;
            secrecy_threshold,
            reconstruction_threshold,
            fast_path_secrecy_threshold,
        &#125; )
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_current"></a>

## Function `current`

Get the currently effective randomness configuration object.


<pre><code>public fun current(): randomness_config::RandomnessConfig
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun current(): RandomnessConfig acquires RandomnessConfig &#123;
    if (exists&lt;RandomnessConfig&gt;(@aptos_framework)) &#123;
        &#42;borrow_global&lt;RandomnessConfig&gt;(@aptos_framework)
    &#125; else &#123;
        new_off()
    &#125;
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>




<pre><code>requires @aptos_framework &#61;&#61; std::signer::address_of(framework);
include config_buffer::OnNewEpochRequirement&lt;RandomnessConfig&gt;;
aborts_if false;
</code></pre>



<a id="@Specification_1_current"></a>

### Function `current`


<pre><code>public fun current(): randomness_config::RandomnessConfig
</code></pre>




<pre><code>aborts_if false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
