
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


<pre><code><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_jwk_consensus_config_JWKConsensusConfig"></a>

## Resource `JWKConsensusConfig`

The configuration of the JWK consensus feature.


<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> <b>has</b> drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 A config variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 &#45; <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigOff">ConfigOff</a></code>
 &#45; <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a></code>
</dd>
</dl>


</details>

<a id="0x1_jwk_consensus_config_ConfigOff"></a>

## Struct `ConfigOff`

A JWK consensus config variant indicating JWK consensus should not run.


<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigOff">ConfigOff</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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


<pre><code><b>const</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_EDUPLICATE_PROVIDERS">EDUPLICATE_PROVIDERS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_jwk_consensus_config_initialize"></a>

## Function `initialize`

Initialize the configuration. Used in genesis or governance.


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_initialize">initialize</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_initialize">initialize</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;(@aptos_framework)) &#123;<br />        <b>move_to</b>(framework, config);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwk_consensus_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on&#45;chain governance to update JWK consensus configs for the next epoch.
Example usage:
```
use aptos_framework::jwk_consensus_config;
use aptos_framework::aptos_governance;
// ...
let config &#61; jwk_consensus_config::new_v1(vector[]);
jwk_consensus_config::set_for_next_epoch(&amp;framework_signer, config);
aptos_governance::reconfigure(&amp;framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(config);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwk_consensus_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;()) &#123;<br />        <b>let</b> new_config &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;();<br />        <b>if</b> (<b>exists</b>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;(@aptos_framework) &#61; new_config;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(framework, new_config);<br />        &#125;;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_off"></a>

## Function `new_off`

Construct a <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a></code> of variant <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigOff">ConfigOff</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_off">new_off</a>(): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_off">new_off</a>(): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> &#123;<br />    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigOff">ConfigOff</a> &#123;&#125; )<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_v1"></a>

## Function `new_v1`

Construct a <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a></code> of variant <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a></code>.

Abort if the given provider list contains duplicated provider names.


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_v1">new_v1</a>(oidc_providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">jwk_consensus_config::OIDCProvider</a>&gt;): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_v1">new_v1</a>(oidc_providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a>&gt;): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> &#123;<br />    <b>let</b> name_set &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>&lt;String, u64&gt;();<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;oidc_providers, &#124;provider&#124; &#123;<br />        <b>let</b> provider: &amp;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a> &#61; provider;<br />        <b>let</b> (_, old_value) &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(&amp;<b>mut</b> name_set, provider.name, 0);<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;old_value)) &#123;<br />            <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_EDUPLICATE_PROVIDERS">EDUPLICATE_PROVIDERS</a>))<br />        &#125;<br />    &#125;);<br />    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>( <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_ConfigV1">ConfigV1</a> &#123; oidc_providers &#125; )<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_oidc_provider"></a>

## Function `new_oidc_provider`

Construct an <code><a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a></code> object.


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_oidc_provider">new_oidc_provider</a>(name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">jwk_consensus_config::OIDCProvider</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_new_oidc_provider">new_oidc_provider</a>(name: String, config_url: String): <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a> &#123;<br />    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_OIDCProvider">OIDCProvider</a> &#123; name, config_url &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>requires</b> @aptos_framework &#61;&#61; std::signer::address_of(framework);<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
