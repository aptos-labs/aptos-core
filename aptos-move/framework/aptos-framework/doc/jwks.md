
<a id="0x1_jwks"></a>

# Module `0x1::jwks`

JWK functions and structs.


-  [Struct `OIDCProvider`](#0x1_jwks_OIDCProvider)
-  [Resource `OIDCProviderSet`](#0x1_jwks_OIDCProviderSet)
-  [Resource `JWKConsensusConfig`](#0x1_jwks_JWKConsensusConfig)
-  [Struct `JWKConsensusConfigV0`](#0x1_jwks_JWKConsensusConfigV0)
-  [Struct `UnsupportedJWK`](#0x1_jwks_UnsupportedJWK)
-  [Struct `RSA_JWK`](#0x1_jwks_RSA_JWK)
-  [Struct `JWK`](#0x1_jwks_JWK)
-  [Struct `ProviderJWKSet`](#0x1_jwks_ProviderJWKSet)
-  [Struct `JWKMap`](#0x1_jwks_JWKMap)
-  [Resource `OnChainJWKMap`](#0x1_jwks_OnChainJWKMap)
-  [Struct `OnChainJWKMapUpdated`](#0x1_jwks_OnChainJWKMapUpdated)
-  [Struct `JWKMapEdit`](#0x1_jwks_JWKMapEdit)
-  [Struct `JWKMapEditCmdDelAll`](#0x1_jwks_JWKMapEditCmdDelAll)
-  [Struct `JWKMapEditCmdDelIssuer`](#0x1_jwks_JWKMapEditCmdDelIssuer)
-  [Struct `JWKMapEditCmdDelJwk`](#0x1_jwks_JWKMapEditCmdDelJwk)
-  [Struct `JWKMapEditCmdPutJwk`](#0x1_jwks_JWKMapEditCmdPutJwk)
-  [Resource `JWKMapPatch`](#0x1_jwks_JWKMapPatch)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_jwks_initialize)
-  [Function `update_oidc_provider`](#0x1_jwks_update_oidc_provider)
-  [Function `jwk_consensus_config_v0`](#0x1_jwks_jwk_consensus_config_v0)
-  [Function `update_jwk_consensus_config`](#0x1_jwks_update_jwk_consensus_config)
-  [Function `update_onchain_jwk_map`](#0x1_jwks_update_onchain_jwk_map)
-  [Function `update_jwk_map_patch`](#0x1_jwks_update_jwk_map_patch)
-  [Function `jwk_map_edit_del_all`](#0x1_jwks_jwk_map_edit_del_all)
-  [Function `jwk_map_edit_del_issuer`](#0x1_jwks_jwk_map_edit_del_issuer)
-  [Function `jwk_map_edit_del_jwk`](#0x1_jwks_jwk_map_edit_del_jwk)
-  [Function `jwk_map_edit_put_jwk`](#0x1_jwks_jwk_map_edit_put_jwk)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_jwks_OIDCProvider"></a>

## Struct `OIDCProvider`

An OIDC provider.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_OIDCProviderSet"></a>

## Resource `OIDCProviderSet`

The OIDC provider set. Maintained by governance proposals.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_OIDCProviderSet">OIDCProviderSet</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_OIDCProvider">jwks::OIDCProvider</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWKConsensusConfig"></a>

## Resource `JWKConsensusConfig`

Some extra configs that controls JWK consensus behavior. Maintained by governance proposals.

Currently supported <code>content</code> types:
- <code><a href="jwks.md#0x1_jwks_JWKConsensusConfigV0">JWKConsensusConfigV0</a></code>


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKConsensusConfig">JWKConsensusConfig</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>content: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWKConsensusConfigV0"></a>

## Struct `JWKConsensusConfigV0`



<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKConsensusConfigV0">JWKConsensusConfigV0</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>observation_interval_ms: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_UnsupportedJWK"></a>

## Struct `UnsupportedJWK`

An observed but not yet supported JWK.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_RSA_JWK"></a>

## Struct `RSA_JWK`

A JWK where <code>kty</code> is <code>RSA</code>.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>kid: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>kty: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alg: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>e: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>n: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWK"></a>

## Struct `JWK`

A JWK.

Currently supported <code>content</code> types:
- <code><a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a></code>
- <code><a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a></code>


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWK">JWK</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>content: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_ProviderJWKSet"></a>

## Struct `ProviderJWKSet`

A provider and its JWKs.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_ProviderJWKSet">ProviderJWKSet</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The utf-8 encoding of the issuer string (e.g., "https://accounts.google.com").
</dd>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;</code>
</dt>
<dd>
 The <code><a href="jwks.md#0x1_jwks">jwks</a></code> should each have a unique <code>id</code>, and should be sorted by <code>id</code> in alphabetical order.
</dd>
</dl>


</details>

<a id="0x1_jwks_JWKMap"></a>

## Struct `JWKMap`

All OIDC providers and their JWK sets.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKMap">JWKMap</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKSet">jwks::ProviderJWKSet</a>&gt;</code>
</dt>
<dd>
 Entries should each have a unique <code>issuer</code>, and should be sorted by <code>issuer</code> in an alphabetical order.
</dd>
</dl>


</details>

<a id="0x1_jwks_OnChainJWKMap"></a>

## Resource `OnChainJWKMap`

A <code><a href="jwks.md#0x1_jwks_JWKMap">JWKMap</a></code> maintained by JWK consensus.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_OnChainJWKMap">OnChainJWKMap</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="version.md#0x1_version">version</a>: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>jwk_map: <a href="jwks.md#0x1_jwks_JWKMap">jwks::JWKMap</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_OnChainJWKMapUpdated"></a>

## Struct `OnChainJWKMapUpdated`

When an on-chain JWK set update is done, this event is sent to reset the JWK consensus state in all validators.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="jwks.md#0x1_jwks_OnChainJWKMapUpdated">OnChainJWKMapUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="version.md#0x1_version">version</a>: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>jwk_map: <a href="jwks.md#0x1_jwks_JWKMap">jwks::JWKMap</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWKMapEdit"></a>

## Struct `JWKMapEdit`

A small edit that can be applied to a <code><a href="jwks.md#0x1_jwks_JWKMap">JWKMap</a></code>.

Currently supported <code>content</code> types:
- <code><a href="jwks.md#0x1_jwks_JWKMapEditCmdDelAll">JWKMapEditCmdDelAll</a></code>
- <code><a href="jwks.md#0x1_jwks_JWKMapEditCmdDelIssuer">JWKMapEditCmdDelIssuer</a></code>
- <code><a href="jwks.md#0x1_jwks_JWKMapEditCmdDelJwk">JWKMapEditCmdDelJwk</a></code>
- <code><a href="jwks.md#0x1_jwks_JWKMapEditCmdPutJwk">JWKMapEditCmdPutJwk</a></code>


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>content: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWKMapEditCmdDelAll"></a>

## Struct `JWKMapEditCmdDelAll`



<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKMapEditCmdDelAll">JWKMapEditCmdDelAll</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_jwks_JWKMapEditCmdDelIssuer"></a>

## Struct `JWKMapEditCmdDelIssuer`



<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKMapEditCmdDelIssuer">JWKMapEditCmdDelIssuer</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWKMapEditCmdDelJwk"></a>

## Struct `JWKMapEditCmdDelJwk`



<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKMapEditCmdDelJwk">JWKMapEditCmdDelJwk</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWKMapEditCmdPutJwk"></a>

## Struct `JWKMapEditCmdPutJwk`



<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKMapEditCmdPutJwk">JWKMapEditCmdPutJwk</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>jwk: <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWKMapPatch"></a>

## Resource `JWKMapPatch`

A sequence of <code><a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a></code> that needs to be applied *one by one* to the JWK consensus-maintained <code><a href="jwks.md#0x1_jwks_JWKMap">JWKMap</a></code> before being used.

Maintained by governance proposals.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKMapPatch">JWKMapPatch</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>edits: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWKMapEdit">jwks::JWKMapEdit</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_jwks_EINVALID_SIG"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EINVALID_SIG">EINVALID_SIG</a>: u64 = 3;
</code></pre>



<a id="0x1_jwks_EUNEXPECTED_EPOCH"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNEXPECTED_EPOCH">EUNEXPECTED_EPOCH</a>: u64 = 1;
</code></pre>



<a id="0x1_jwks_EUNEXPECTED_VERSION"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNEXPECTED_VERSION">EUNEXPECTED_VERSION</a>: u64 = 2;
</code></pre>



<a id="0x1_jwks_EUNKNOWN_JWK_MAP_EDIT"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNKNOWN_JWK_MAP_EDIT">EUNKNOWN_JWK_MAP_EDIT</a>: u64 = 4;
</code></pre>



<a id="0x1_jwks_initialize"></a>

## Function `initialize`

Initialize some JWK resources. Should only be invoked by genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_initialize">initialize</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_initialize">initialize</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="jwks.md#0x1_jwks_OIDCProviderSet">OIDCProviderSet</a> { providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] });
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="jwks.md#0x1_jwks_jwk_consensus_config_v0">jwk_consensus_config_v0</a>(10000));
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="jwks.md#0x1_jwks_OnChainJWKMap">OnChainJWKMap</a> { <a href="version.md#0x1_version">version</a>: 0, jwk_map: <a href="jwks.md#0x1_jwks_JWKMap">JWKMap</a> { entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] } });
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="jwks.md#0x1_jwks_JWKMapPatch">JWKMapPatch</a> { edits: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] });
}
</code></pre>



</details>

<a id="0x1_jwks_update_oidc_provider"></a>

## Function `update_oidc_provider`

(1) Remove the entry for a provider of a given name from the provider set, if it exists.
(2) Add a new entry for the provider with the new config endpoint, if provided.
(3) Return the removed config endpoint in (1), if it happened.

Designed to be used only in governance proposal-only.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_update_oidc_provider">update_oidc_provider</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, new_config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_update_oidc_provider">update_oidc_provider</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, new_config_url: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_OIDCProviderSet">OIDCProviderSet</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);

    <b>let</b> provider_set = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_OIDCProviderSet">OIDCProviderSet</a>&gt;(@aptos_framework);

    <b>let</b> (name_exists, idx) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&provider_set.providers, |obj| {
        <b>let</b> provider: &<a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> = obj;
        provider.name == name
    });

    <b>let</b> old_config_endpoint = <b>if</b> (name_exists) {
        <b>let</b> old_provider_info = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> provider_set.providers, idx);
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(old_provider_info.config_url)
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&new_config_url)) {
        <b>let</b> config_endpoint = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> new_config_url);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> provider_set.providers, <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> { name, config_url: config_endpoint });
    };

    old_config_endpoint
}
</code></pre>



</details>

<a id="0x1_jwks_jwk_consensus_config_v0"></a>

## Function `jwk_consensus_config_v0`

Create a <code><a href="jwks.md#0x1_jwks_JWKConsensusConfig">JWKConsensusConfig</a></code> with content type <code><a href="jwks.md#0x1_jwks_JWKConsensusConfigV0">JWKConsensusConfigV0</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_consensus_config_v0">jwk_consensus_config_v0</a>(observation_interval_ms: u64): <a href="jwks.md#0x1_jwks_JWKConsensusConfig">jwks::JWKConsensusConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_consensus_config_v0">jwk_consensus_config_v0</a>(observation_interval_ms: u64): <a href="jwks.md#0x1_jwks_JWKConsensusConfig">JWKConsensusConfig</a> {
    <b>let</b> v0 = <a href="jwks.md#0x1_jwks_JWKConsensusConfigV0">JWKConsensusConfigV0</a> { observation_interval_ms };
    <a href="jwks.md#0x1_jwks_JWKConsensusConfig">JWKConsensusConfig</a> {
        content: pack(v0),
    }
}
</code></pre>



</details>

<a id="0x1_jwks_update_jwk_consensus_config"></a>

## Function `update_jwk_consensus_config`

Update JWK consensus config. Should only be invoked by governance proposals.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_update_jwk_consensus_config">update_jwk_consensus_config</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwks.md#0x1_jwks_JWKConsensusConfig">jwks::JWKConsensusConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_update_jwk_consensus_config">update_jwk_consensus_config</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="jwks.md#0x1_jwks_JWKConsensusConfig">JWKConsensusConfig</a>) <b>acquires</b> <a href="jwks.md#0x1_jwks_JWKConsensusConfig">JWKConsensusConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);
    *<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_JWKConsensusConfig">JWKConsensusConfig</a>&gt;(@aptos_framework) = config;
}
</code></pre>



</details>

<a id="0x1_jwks_update_onchain_jwk_map"></a>

## Function `update_onchain_jwk_map`

Update the JWK set. Should only be invoked by validator transactions/governance proposals.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_update_onchain_jwk_map">update_onchain_jwk_map</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch: u64, <a href="version.md#0x1_version">version</a>: u64, jwk_map: <a href="jwks.md#0x1_jwks_JWKMap">jwks::JWKMap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_update_onchain_jwk_map">update_onchain_jwk_map</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch: u64, <a href="version.md#0x1_version">version</a>: u64, jwk_map: <a href="jwks.md#0x1_jwks_JWKMap">JWKMap</a>) <b>acquires</b> <a href="jwks.md#0x1_jwks_OnChainJWKMap">OnChainJWKMap</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(&<a href="account.md#0x1_account">account</a>);

    // Epoch check.
    <b>assert</b>!(<a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>() == epoch, invalid_argument(<a href="jwks.md#0x1_jwks_EUNEXPECTED_EPOCH">EUNEXPECTED_EPOCH</a>));

    <b>let</b> onchain_jwk_map = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_OnChainJWKMap">OnChainJWKMap</a>&gt;(@aptos_framework);

    // Version check.
    <b>assert</b>!(onchain_jwk_map.<a href="version.md#0x1_version">version</a> + 1 == <a href="version.md#0x1_version">version</a>, invalid_argument(<a href="jwks.md#0x1_jwks_EUNEXPECTED_VERSION">EUNEXPECTED_VERSION</a>));

    // Replace.
    *onchain_jwk_map = <a href="jwks.md#0x1_jwks_OnChainJWKMap">OnChainJWKMap</a> { <a href="version.md#0x1_version">version</a>, jwk_map };
    emit(<a href="jwks.md#0x1_jwks_OnChainJWKMapUpdated">OnChainJWKMapUpdated</a>{ epoch, <a href="version.md#0x1_version">version</a>, jwk_map });
}
</code></pre>



</details>

<a id="0x1_jwks_update_jwk_map_patch"></a>

## Function `update_jwk_map_patch`

Update the system <code><a href="jwks.md#0x1_jwks_JWKMapPatch">JWKMapPatch</a></code>. This is governance proposal-only.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_update_jwk_map_patch">update_jwk_map_patch</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, edits: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWKMapEdit">jwks::JWKMapEdit</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_update_jwk_map_patch">update_jwk_map_patch</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, edits: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a>&gt;) <b>acquires</b> <a href="jwks.md#0x1_jwks_JWKMapPatch">JWKMapPatch</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> patch = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_JWKMapPatch">JWKMapPatch</a>&gt;(@aptos_framework);
    patch.edits = edits;
}
</code></pre>



</details>

<a id="0x1_jwks_jwk_map_edit_del_all"></a>

## Function `jwk_map_edit_del_all`

Create a JWKMap edit command that deletes all entries.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_map_edit_del_all">jwk_map_edit_del_all</a>(): <a href="jwks.md#0x1_jwks_JWKMapEdit">jwks::JWKMapEdit</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_map_edit_del_all">jwk_map_edit_del_all</a>(): <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> {
    <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> {
        content: pack(<a href="jwks.md#0x1_jwks_JWKMapEditCmdDelAll">JWKMapEditCmdDelAll</a> {}),
    }
}
</code></pre>



</details>

<a id="0x1_jwks_jwk_map_edit_del_issuer"></a>

## Function `jwk_map_edit_del_issuer`

Create a JWKMap edit command that deletes the entry for a given issuer, if exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_map_edit_del_issuer">jwk_map_edit_del_issuer</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWKMapEdit">jwks::JWKMapEdit</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_map_edit_del_issuer">jwk_map_edit_del_issuer</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> {
    <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> {
        content: pack(<a href="jwks.md#0x1_jwks_JWKMapEditCmdDelIssuer">JWKMapEditCmdDelIssuer</a> { issuer }),
    }
}
</code></pre>



</details>

<a id="0x1_jwks_jwk_map_edit_del_jwk"></a>

## Function `jwk_map_edit_del_jwk`

Create a JWKMap edit command that deletes the entry for a given issuer, if exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_map_edit_del_jwk">jwk_map_edit_del_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWKMapEdit">jwks::JWKMapEdit</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_map_edit_del_jwk">jwk_map_edit_del_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> {
    <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> {
        content: pack(<a href="jwks.md#0x1_jwks_JWKMapEditCmdDelJwk">JWKMapEditCmdDelJwk</a> { issuer, jwk_id })
    }
}
</code></pre>



</details>

<a id="0x1_jwks_jwk_map_edit_put_jwk"></a>

## Function `jwk_map_edit_put_jwk`

Create a JWKMap edit command that upserts a JWK into an issuer's JWK set.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_map_edit_put_jwk">jwk_map_edit_put_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk: <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="jwks.md#0x1_jwks_JWKMapEdit">jwks::JWKMapEdit</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_jwk_map_edit_put_jwk">jwk_map_edit_put_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk: <a href="jwks.md#0x1_jwks_JWK">JWK</a>): <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> {
    <a href="jwks.md#0x1_jwks_JWKMapEdit">JWKMapEdit</a> {
        content: pack(<a href="jwks.md#0x1_jwks_JWKMapEditCmdPutJwk">JWKMapEditCmdPutJwk</a> { issuer, jwk })
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
