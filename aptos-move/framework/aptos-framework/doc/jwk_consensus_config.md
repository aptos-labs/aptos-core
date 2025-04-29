
<a id="0x1_jwk_consensus_config"></a>

# Module `0x1::jwk_consensus_config`

Structs and functions related to JWK consensus configurations.


-  [Resource `JWKConsensusConfig`](#0x1_jwk_consensus_config_JWKConsensusConfig)
-  [Struct `ConfigOff`](#0x1_jwk_consensus_config_ConfigOff)
-  [Struct `OIDCProvider`](#0x1_jwk_consensus_config_OIDCProvider)
-  [Struct `ConfigV1`](#0x1_jwk_consensus_config_ConfigV1)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_jwk_consensus_config_initialize)
-  [Function `set_for_next_epoch`](#0x1_jwk_consensus_config_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_jwk_consensus_config_on_new_epoch)
-  [Function `new_off`](#0x1_jwk_consensus_config_new_off)
-  [Function `new_v1`](#0x1_jwk_consensus_config_new_v1)
-  [Function `new_oidc_provider`](#0x1_jwk_consensus_config_new_oidc_provider)
-  [Specification](#@Specification_1)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)


<pre><code><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_jwk_consensus_config_JWKConsensusConfig"></a>

## Resource `JWKConsensusConfig`

The configuration of the JWK consensus feature.


<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> <b>has</b> drop, store, key
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
 - <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigOff">ConfigOff</a></code>
 - <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a></code>
</dd>
</dl>


</details>

<a id="0x1_jwk_consensus_config_ConfigOff"></a>

## Struct `ConfigOff`

A JWK consensus config variant indicating JWK consensus should not run.


<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigOff">ConfigOff</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_jwk_consensus_config_OIDCProvider"></a>

## Struct `OIDCProvider`



<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwk_consensus_config_ConfigV1"></a>

## Struct `ConfigV1`

A JWK consensus config variant indicating JWK consensus should run to watch a given list of OIDC providers.


<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>oidc_providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">jwk_consensus_config::OIDCProvider</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_jwk_consensus_config_EDUPLICATE_PROVIDERS"></a>

<code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a></code> creation failed with duplicated providers given.


<pre><code><b>const</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_EDUPLICATE_PROVIDERS">EDUPLICATE_PROVIDERS</a>: u64 = 1;
</code></pre>



<a id="0x1_jwk_consensus_config_initialize"></a>

## Function `initialize`

Initialize the configuration. Used in genesis or governance.


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(framework, config);
    }
}
</code></pre>



</details>

<a id="0x1_jwk_consensus_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on-chain governance to update JWK consensus configs for the next epoch.
Example usage:
```
use aptos_framework::jwk_consensus_config;
use aptos_framework::aptos_governance;
// ...
let config = jwk_consensus_config::new_v1(vector[]);
jwk_consensus_config::set_for_next_epoch(&framework_signer, config);
aptos_governance::reconfigure(&framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(config);
}
</code></pre>



</details>

<a id="0x1_jwk_consensus_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;()) {
        <b>let</b> new_config = <a href="config_buffer.md#0x1_config_buffer_extract_v2">config_buffer::extract_v2</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;();
        <b>if</b> (<b>exists</b>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;(@aptos_framework)) {
            *<b>borrow_global_mut</b>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;(@aptos_framework) = new_config;
        } <b>else</b> {
            <b>move_to</b>(framework, new_config);
        };
    }
}
</code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_off"></a>

## Function `new_off`

Construct a <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a></code> of variant <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigOff">ConfigOff</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_off">new_off</a>(): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_off">new_off</a>(): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> {
    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> {
        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigOff">ConfigOff</a> {} )
    }
}
</code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_v1"></a>

## Function `new_v1`

Construct a <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a></code> of variant <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a></code>.

Abort if the given provider list contains duplicated provider names.


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_v1">new_v1</a>(oidc_providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">jwk_consensus_config::OIDCProvider</a>&gt;): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_v1">new_v1</a>(oidc_providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a>&gt;): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> {
    <b>let</b> name_set = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>&lt;String, u64&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&oidc_providers, |provider| {
        <b>let</b> provider: &<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a> = provider;
        <b>let</b> (_, old_value) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(&<b>mut</b> name_set, provider.name, 0);
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&old_value)) {
            <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_EDUPLICATE_PROVIDERS">EDUPLICATE_PROVIDERS</a>))
        }
    });
    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> {
        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a> { oidc_providers } )
    }
}
</code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_oidc_provider"></a>

## Function `new_oidc_provider`

Construct an <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a></code> object.


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_oidc_provider">new_oidc_provider</a>(name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">jwk_consensus_config::OIDCProvider</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_oidc_provider">new_oidc_provider</a>(name: String, config_url: String): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a> {
    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a> { name, config_url }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>requires</b> @aptos_framework == std::signer::address_of(framework);
<b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;;
<b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
