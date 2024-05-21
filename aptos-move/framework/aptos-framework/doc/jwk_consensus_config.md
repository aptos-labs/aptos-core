
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


<pre><code>use 0x1::config_buffer;<br/>use 0x1::copyable_any;<br/>use 0x1::error;<br/>use 0x1::option;<br/>use 0x1::simple_map;<br/>use 0x1::string;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_jwk_consensus_config_JWKConsensusConfig"></a>

## Resource `JWKConsensusConfig`

The configuration of the JWK consensus feature.


<pre><code>struct JWKConsensusConfig has drop, store, key<br/></code></pre>



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

<a id="0x1_jwk_consensus_config_ConfigOff"></a>

## Struct `ConfigOff`

A JWK consensus config variant indicating JWK consensus should not run.


<pre><code>struct ConfigOff has copy, drop, store<br/></code></pre>



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



<pre><code>struct OIDCProvider has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>config_url: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwk_consensus_config_ConfigV1"></a>

## Struct `ConfigV1`

A JWK consensus config variant indicating JWK consensus should run to watch a given list of OIDC providers.


<pre><code>struct ConfigV1 has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>oidc_providers: vector&lt;jwk_consensus_config::OIDCProvider&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_jwk_consensus_config_EDUPLICATE_PROVIDERS"></a>

<code>ConfigV1</code> creation failed with duplicated providers given.


<pre><code>const EDUPLICATE_PROVIDERS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_jwk_consensus_config_initialize"></a>

## Function `initialize`

Initialize the configuration. Used in genesis or governance.


<pre><code>public fun initialize(framework: &amp;signer, config: jwk_consensus_config::JWKConsensusConfig)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(framework: &amp;signer, config: JWKConsensusConfig) &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    if (!exists&lt;JWKConsensusConfig&gt;(@aptos_framework)) &#123;<br/>        move_to(framework, config);<br/>    &#125;<br/>&#125;<br/></code></pre>



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


<pre><code>public fun set_for_next_epoch(framework: &amp;signer, config: jwk_consensus_config::JWKConsensusConfig)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(framework: &amp;signer, config: JWKConsensusConfig) &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    config_buffer::upsert(config);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_jwk_consensus_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>JWKConsensusConfig</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires JWKConsensusConfig &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    if (config_buffer::does_exist&lt;JWKConsensusConfig&gt;()) &#123;<br/>        let new_config &#61; config_buffer::extract&lt;JWKConsensusConfig&gt;();<br/>        if (exists&lt;JWKConsensusConfig&gt;(@aptos_framework)) &#123;<br/>            &#42;borrow_global_mut&lt;JWKConsensusConfig&gt;(@aptos_framework) &#61; new_config;<br/>        &#125; else &#123;<br/>            move_to(framework, new_config);<br/>        &#125;;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_off"></a>

## Function `new_off`

Construct a <code>JWKConsensusConfig</code> of variant <code>ConfigOff</code>.


<pre><code>public fun new_off(): jwk_consensus_config::JWKConsensusConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_off(): JWKConsensusConfig &#123;<br/>    JWKConsensusConfig &#123;<br/>        variant: copyable_any::pack( ConfigOff &#123;&#125; )<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_v1"></a>

## Function `new_v1`

Construct a <code>JWKConsensusConfig</code> of variant <code>ConfigV1</code>.

Abort if the given provider list contains duplicated provider names.


<pre><code>public fun new_v1(oidc_providers: vector&lt;jwk_consensus_config::OIDCProvider&gt;): jwk_consensus_config::JWKConsensusConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_v1(oidc_providers: vector&lt;OIDCProvider&gt;): JWKConsensusConfig &#123;<br/>    let name_set &#61; simple_map::new&lt;String, u64&gt;();<br/>    vector::for_each_ref(&amp;oidc_providers, &#124;provider&#124; &#123;<br/>        let provider: &amp;OIDCProvider &#61; provider;<br/>        let (_, old_value) &#61; simple_map::upsert(&amp;mut name_set, provider.name, 0);<br/>        if (option::is_some(&amp;old_value)) &#123;<br/>            abort(error::invalid_argument(EDUPLICATE_PROVIDERS))<br/>        &#125;<br/>    &#125;);<br/>    JWKConsensusConfig &#123;<br/>        variant: copyable_any::pack( ConfigV1 &#123; oidc_providers &#125; )<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_jwk_consensus_config_new_oidc_provider"></a>

## Function `new_oidc_provider`

Construct an <code>OIDCProvider</code> object.


<pre><code>public fun new_oidc_provider(name: string::String, config_url: string::String): jwk_consensus_config::OIDCProvider<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_oidc_provider(name: String, config_url: String): OIDCProvider &#123;<br/>    OIDCProvider &#123; name, config_url &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)<br/></code></pre>




<pre><code>requires @aptos_framework &#61;&#61; std::signer::address_of(framework);<br/>include config_buffer::OnNewEpochRequirement&lt;JWKConsensusConfig&gt;;<br/>aborts_if false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
