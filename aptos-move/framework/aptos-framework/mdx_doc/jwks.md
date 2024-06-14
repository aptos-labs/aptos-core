
<a id="0x1_jwks"></a>

# Module `0x1::jwks`

JWK functions and structs.

Note: An important design constraint for this module is that the JWK consensus Rust code is unable to
spawn a VM and make a Move function call. Instead, the JWK consensus Rust code will have to directly
write some of the resources in this file. As a result, the structs in this file are declared so as to
have a simple layout which is easily accessible in Rust.


-  [Struct `OIDCProvider`](#0x1_jwks_OIDCProvider)
-  [Resource `SupportedOIDCProviders`](#0x1_jwks_SupportedOIDCProviders)
-  [Struct `UnsupportedJWK`](#0x1_jwks_UnsupportedJWK)
-  [Struct `RSA_JWK`](#0x1_jwks_RSA_JWK)
-  [Struct `JWK`](#0x1_jwks_JWK)
-  [Struct `ProviderJWKs`](#0x1_jwks_ProviderJWKs)
-  [Struct `AllProvidersJWKs`](#0x1_jwks_AllProvidersJWKs)
-  [Resource `ObservedJWKs`](#0x1_jwks_ObservedJWKs)
-  [Struct `ObservedJWKsUpdated`](#0x1_jwks_ObservedJWKsUpdated)
-  [Struct `Patch`](#0x1_jwks_Patch)
-  [Struct `PatchRemoveAll`](#0x1_jwks_PatchRemoveAll)
-  [Struct `PatchRemoveIssuer`](#0x1_jwks_PatchRemoveIssuer)
-  [Struct `PatchRemoveJWK`](#0x1_jwks_PatchRemoveJWK)
-  [Struct `PatchUpsertJWK`](#0x1_jwks_PatchUpsertJWK)
-  [Resource `Patches`](#0x1_jwks_Patches)
-  [Resource `PatchedJWKs`](#0x1_jwks_PatchedJWKs)
-  [Constants](#@Constants_0)
-  [Function `get_patched_jwk`](#0x1_jwks_get_patched_jwk)
-  [Function `try_get_patched_jwk`](#0x1_jwks_try_get_patched_jwk)
-  [Function `upsert_oidc_provider`](#0x1_jwks_upsert_oidc_provider)
-  [Function `upsert_oidc_provider_for_next_epoch`](#0x1_jwks_upsert_oidc_provider_for_next_epoch)
-  [Function `remove_oidc_provider`](#0x1_jwks_remove_oidc_provider)
-  [Function `remove_oidc_provider_for_next_epoch`](#0x1_jwks_remove_oidc_provider_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_jwks_on_new_epoch)
-  [Function `set_patches`](#0x1_jwks_set_patches)
-  [Function `new_patch_remove_all`](#0x1_jwks_new_patch_remove_all)
-  [Function `new_patch_remove_issuer`](#0x1_jwks_new_patch_remove_issuer)
-  [Function `new_patch_remove_jwk`](#0x1_jwks_new_patch_remove_jwk)
-  [Function `new_patch_upsert_jwk`](#0x1_jwks_new_patch_upsert_jwk)
-  [Function `new_rsa_jwk`](#0x1_jwks_new_rsa_jwk)
-  [Function `new_unsupported_jwk`](#0x1_jwks_new_unsupported_jwk)
-  [Function `initialize`](#0x1_jwks_initialize)
-  [Function `remove_oidc_provider_internal`](#0x1_jwks_remove_oidc_provider_internal)
-  [Function `upsert_into_observed_jwks`](#0x1_jwks_upsert_into_observed_jwks)
-  [Function `remove_issuer_from_observed_jwks`](#0x1_jwks_remove_issuer_from_observed_jwks)
-  [Function `regenerate_patched_jwks`](#0x1_jwks_regenerate_patched_jwks)
-  [Function `try_get_jwk_by_issuer`](#0x1_jwks_try_get_jwk_by_issuer)
-  [Function `try_get_jwk_by_id`](#0x1_jwks_try_get_jwk_by_id)
-  [Function `get_jwk_id`](#0x1_jwks_get_jwk_id)
-  [Function `upsert_provider_jwks`](#0x1_jwks_upsert_provider_jwks)
-  [Function `remove_issuer`](#0x1_jwks_remove_issuer)
-  [Function `upsert_jwk`](#0x1_jwks_upsert_jwk)
-  [Function `remove_jwk`](#0x1_jwks_remove_jwk)
-  [Function `apply_patch`](#0x1_jwks_apply_patch)
-  [Specification](#@Specification_1)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)


<pre><code><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/comparator.md#0x1_comparator">0x1::comparator</a>;<br /><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_jwks_OIDCProvider"></a>

## Struct `OIDCProvider`

An OIDC provider.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The utf&#45;8 encoded issuer string. E.g., b&quot;https://www.facebook.com&quot;.
</dd>
<dt>
<code>config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The ut8&#45;8 encoded OpenID configuration URL of the provider.
 E.g., b&quot;https://www.facebook.com/.well&#45;known/openid&#45;configuration/&quot;.
</dd>
</dl>


</details>

<a id="0x1_jwks_SupportedOIDCProviders"></a>

## Resource `SupportedOIDCProviders`

A list of OIDC providers whose JWKs should be watched by validators. Maintained by governance proposals.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



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

<a id="0x1_jwks_UnsupportedJWK"></a>

## Struct `UnsupportedJWK`

An JWK variant that represents the JWKs which were observed but not yet supported by Aptos.
Observing <code><a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a></code>s means the providers adopted a new key type/format, and the system should be updated.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_RSA_JWK"></a>

## Struct `RSA_JWK`

A JWK variant where <code>kty</code> is <code>RSA</code>.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

A JSON web key.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWK">JWK</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 A <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 &#45; <code><a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a></code>
 &#45; <code><a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a></code>
</dd>
</dl>


</details>

<a id="0x1_jwks_ProviderJWKs"></a>

## Struct `ProviderJWKs`

A provider and its <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code>s.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The utf&#45;8 encoding of the issuer string (e.g., &quot;https://www.facebook.com&quot;).
</dd>
<dt>
<code><a href="version.md#0x1_version">version</a>: u64</code>
</dt>
<dd>
 A version number is needed by JWK consensus to dedup the updates.
 e.g, when on chain version &#61; 5, multiple nodes can propose an update with version &#61; 6.
 Bumped every time the JWKs for the current issuer is updated.
 The Rust authenticator only uses the latest version.
</dd>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;</code>
</dt>
<dd>
 Vector of <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code>&apos;s sorted by their unique ID (from <code>get_jwk_id</code>) in dictionary order.
</dd>
</dl>


</details>

<a id="0x1_jwks_AllProvidersJWKs"></a>

## Struct `AllProvidersJWKs`

Multiple <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> objects, indexed by issuer and key ID.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;</code>
</dt>
<dd>
 Vector of <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> sorted by <code>ProviderJWKs::issuer</code> in dictionary order.
</dd>
</dl>


</details>

<a id="0x1_jwks_ObservedJWKs"></a>

## Resource `ObservedJWKs`

The <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code> that validators observed and agreed on.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_ObservedJWKsUpdated"></a>

## Struct `ObservedJWKsUpdated`

When <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code> is updated, this event is sent to resync the JWK consensus state in all validators.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="jwks.md#0x1_jwks_ObservedJWKsUpdated">ObservedJWKsUpdated</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_Patch"></a>

## Struct `Patch`

A small edit or patch that is applied to a <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code> to obtain <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code>.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_Patch">Patch</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 &#45; <code><a href="jwks.md#0x1_jwks_PatchRemoveAll">PatchRemoveAll</a></code>
 &#45; <code><a href="jwks.md#0x1_jwks_PatchRemoveIssuer">PatchRemoveIssuer</a></code>
 &#45; <code><a href="jwks.md#0x1_jwks_PatchRemoveJWK">PatchRemoveJWK</a></code>
 &#45; <code><a href="jwks.md#0x1_jwks_PatchUpsertJWK">PatchUpsertJWK</a></code>
</dd>
</dl>


</details>

<a id="0x1_jwks_PatchRemoveAll"></a>

## Struct `PatchRemoveAll`

A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant to remove all JWKs.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchRemoveAll">PatchRemoveAll</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_jwks_PatchRemoveIssuer"></a>

## Struct `PatchRemoveIssuer`

A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant to remove an issuer and all its JWKs.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchRemoveIssuer">PatchRemoveIssuer</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_jwks_PatchRemoveJWK"></a>

## Struct `PatchRemoveJWK`

A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant to remove a specific JWK of an issuer.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchRemoveJWK">PatchRemoveJWK</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_jwks_PatchUpsertJWK"></a>

## Struct `PatchUpsertJWK`

A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant to upsert a JWK for an issuer.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchUpsertJWK">PatchUpsertJWK</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_jwks_Patches"></a>

## Resource `Patches`

A sequence of <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> objects that are applied &#42;one by one&#42; to the <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code>.

Maintained by governance proposals.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_Patches">Patches</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>patches: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_PatchedJWKs"></a>

## Resource `PatchedJWKs`

The result of applying the <code><a href="jwks.md#0x1_jwks_Patches">Patches</a></code> to the <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code>.
This is what applications should consume.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> <b>has</b> drop, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_jwks_EISSUER_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EISSUER_NOT_FOUND">EISSUER_NOT_FOUND</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_jwks_EJWK_ID_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EJWK_ID_NOT_FOUND">EJWK_ID_NOT_FOUND</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_jwks_ENATIVE_INCORRECT_VERSION"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_INCORRECT_VERSION">ENATIVE_INCORRECT_VERSION</a>: u64 &#61; 259;<br /></code></pre>



<a id="0x1_jwks_ENATIVE_MISSING_RESOURCE_OBSERVED_JWKS"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_MISSING_RESOURCE_OBSERVED_JWKS">ENATIVE_MISSING_RESOURCE_OBSERVED_JWKS</a>: u64 &#61; 258;<br /></code></pre>



<a id="0x1_jwks_ENATIVE_MISSING_RESOURCE_VALIDATOR_SET"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_MISSING_RESOURCE_VALIDATOR_SET">ENATIVE_MISSING_RESOURCE_VALIDATOR_SET</a>: u64 &#61; 257;<br /></code></pre>



<a id="0x1_jwks_ENATIVE_MULTISIG_VERIFICATION_FAILED"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_MULTISIG_VERIFICATION_FAILED">ENATIVE_MULTISIG_VERIFICATION_FAILED</a>: u64 &#61; 260;<br /></code></pre>



<a id="0x1_jwks_ENATIVE_NOT_ENOUGH_VOTING_POWER"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_NOT_ENOUGH_VOTING_POWER">ENATIVE_NOT_ENOUGH_VOTING_POWER</a>: u64 &#61; 261;<br /></code></pre>



<a id="0x1_jwks_EUNEXPECTED_EPOCH"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNEXPECTED_EPOCH">EUNEXPECTED_EPOCH</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_jwks_EUNEXPECTED_VERSION"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNEXPECTED_VERSION">EUNEXPECTED_VERSION</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_jwks_EUNKNOWN_JWK_VARIANT"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNKNOWN_JWK_VARIANT">EUNKNOWN_JWK_VARIANT</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_jwks_EUNKNOWN_PATCH_VARIANT"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNKNOWN_PATCH_VARIANT">EUNKNOWN_PATCH_VARIANT</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_jwks_get_patched_jwk"></a>

## Function `get_patched_jwk`

Get a JWK by issuer and key ID from the <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code>.
Abort if such a JWK does not exist.
More convenient to call from Rust, since it does not wrap the JWK in an <code>Option</code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_get_patched_jwk">get_patched_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_get_patched_jwk">get_patched_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">JWK</a> <b>acquires</b> <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> &#123;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> <a href="jwks.md#0x1_jwks_try_get_patched_jwk">try_get_patched_jwk</a>(issuer, jwk_id))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_try_get_patched_jwk"></a>

## Function `try_get_patched_jwk`

Get a JWK by issuer and key ID from the <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code>, if it exists.
More convenient to call from Move, since it does not abort.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_try_get_patched_jwk">try_get_patched_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_try_get_patched_jwk">try_get_patched_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> &#123;<br />    <b>let</b> <a href="jwks.md#0x1_jwks">jwks</a> &#61; &amp;<b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>&gt;(@aptos_framework).<a href="jwks.md#0x1_jwks">jwks</a>;<br />    <a href="jwks.md#0x1_jwks_try_get_jwk_by_issuer">try_get_jwk_by_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>, issuer, jwk_id)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_upsert_oidc_provider"></a>

## Function `upsert_oidc_provider`

Deprecated by <code><a href="jwks.md#0x1_jwks_upsert_oidc_provider_for_next_epoch">upsert_oidc_provider_for_next_epoch</a>()</code>.

TODO: update all the tests that reference this function, then disable this function.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_oidc_provider">upsert_oidc_provider</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_oidc_provider">upsert_oidc_provider</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>let</b> provider_set &#61; <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework);<br /><br />    <b>let</b> old_config_url&#61; <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(provider_set, name);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> provider_set.providers, <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> &#123; name, config_url &#125;);<br />    old_config_url<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_upsert_oidc_provider_for_next_epoch"></a>

## Function `upsert_oidc_provider_for_next_epoch`

Used in on&#45;chain governances to update the supported OIDC providers, effective starting next epoch.
Example usage:
```
aptos_framework::jwks::upsert_oidc_provider_for_next_epoch(
&amp;framework_signer,
b&quot;https://accounts.google.com&quot;,
b&quot;https://accounts.google.com/.well&#45;known/openid&#45;configuration&quot;
);
aptos_framework::aptos_governance::reconfigure(&amp;framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_oidc_provider_for_next_epoch">upsert_oidc_provider_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_oidc_provider_for_next_epoch">upsert_oidc_provider_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br /><br />    <b>let</b> provider_set &#61; <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()) &#123;<br />        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()<br />    &#125; <b>else</b> &#123;<br />        &#42;<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework)<br />    &#125;;<br /><br />    <b>let</b> old_config_url &#61; <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(&amp;<b>mut</b> provider_set, name);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> provider_set.providers, <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> &#123; name, config_url &#125;);<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(provider_set);<br />    old_config_url<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_remove_oidc_provider"></a>

## Function `remove_oidc_provider`

Deprecated by <code><a href="jwks.md#0x1_jwks_remove_oidc_provider_for_next_epoch">remove_oidc_provider_for_next_epoch</a>()</code>.

TODO: update all the tests that reference this function, then disable this function.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider">remove_oidc_provider</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider">remove_oidc_provider</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>let</b> provider_set &#61; <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework);<br />    <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(provider_set, name)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_remove_oidc_provider_for_next_epoch"></a>

## Function `remove_oidc_provider_for_next_epoch`

Used in on&#45;chain governances to update the supported OIDC providers, effective starting next epoch.
Example usage:
```
aptos_framework::jwks::remove_oidc_provider_for_next_epoch(
&amp;framework_signer,
b&quot;https://accounts.google.com&quot;,
);
aptos_framework::aptos_governance::reconfigure(&amp;framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider_for_next_epoch">remove_oidc_provider_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider_for_next_epoch">remove_oidc_provider_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br /><br />    <b>let</b> provider_set &#61; <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()) &#123;<br />        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()<br />    &#125; <b>else</b> &#123;<br />        &#42;<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework)<br />    &#125;;<br />    <b>let</b> ret &#61; <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(&amp;<b>mut</b> provider_set, name);<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(provider_set);<br />    ret<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()) &#123;<br />        <b>let</b> new_config &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;();<br />        <b>if</b> (<b>exists</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework) &#61; new_config;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(framework, new_config);<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_set_patches"></a>

## Function `set_patches`

Set the <code><a href="jwks.md#0x1_jwks_Patches">Patches</a></code>. Only called in governance proposals.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_set_patches">set_patches</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, patches: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_set_patches">set_patches</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, patches: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_Patch">Patch</a>&gt;) <b>acquires</b> <a href="jwks.md#0x1_jwks_Patches">Patches</a>, <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_Patches">Patches</a>&gt;(@aptos_framework).patches &#61; patches;<br />    <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_all"></a>

## Function `new_patch_remove_all`

Create a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> that removes all entries.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_all">new_patch_remove_all</a>(): <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_all">new_patch_remove_all</a>(): <a href="jwks.md#0x1_jwks_Patch">Patch</a> &#123;<br />    <a href="jwks.md#0x1_jwks_Patch">Patch</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_PatchRemoveAll">PatchRemoveAll</a> &#123;&#125;),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_issuer"></a>

## Function `new_patch_remove_issuer`

Create a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> that removes the entry of a given issuer, if exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_issuer">new_patch_remove_issuer</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_issuer">new_patch_remove_issuer</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_Patch">Patch</a> &#123;<br />    <a href="jwks.md#0x1_jwks_Patch">Patch</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_PatchRemoveIssuer">PatchRemoveIssuer</a> &#123; issuer &#125;),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_jwk"></a>

## Function `new_patch_remove_jwk`

Create a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> that removes the entry of a given issuer, if exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_jwk">new_patch_remove_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_jwk">new_patch_remove_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_Patch">Patch</a> &#123;<br />    <a href="jwks.md#0x1_jwks_Patch">Patch</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_PatchRemoveJWK">PatchRemoveJWK</a> &#123; issuer, jwk_id &#125;)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_new_patch_upsert_jwk"></a>

## Function `new_patch_upsert_jwk`

Create a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> that upserts a JWK into an issuer&apos;s JWK set.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_upsert_jwk">new_patch_upsert_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk: <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_upsert_jwk">new_patch_upsert_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk: <a href="jwks.md#0x1_jwks_JWK">JWK</a>): <a href="jwks.md#0x1_jwks_Patch">Patch</a> &#123;<br />    <a href="jwks.md#0x1_jwks_Patch">Patch</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_PatchUpsertJWK">PatchUpsertJWK</a> &#123; issuer, jwk &#125;)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_new_rsa_jwk"></a>

## Function `new_rsa_jwk`

Create a <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> of variant <code><a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_rsa_jwk">new_rsa_jwk</a>(kid: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, alg: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, e: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, n: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_rsa_jwk">new_rsa_jwk</a>(kid: String, alg: String, e: String, n: String): <a href="jwks.md#0x1_jwks_JWK">JWK</a> &#123;<br />    <a href="jwks.md#0x1_jwks_JWK">JWK</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a> &#123;<br />            kid,<br />            kty: utf8(b&quot;RSA&quot;),<br />            e,<br />            n,<br />            alg,<br />        &#125;),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_new_unsupported_jwk"></a>

## Function `new_unsupported_jwk`

Create a <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> of variant <code><a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_unsupported_jwk">new_unsupported_jwk</a>(id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_unsupported_jwk">new_unsupported_jwk</a>(id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">JWK</a> &#123;<br />    <a href="jwks.md#0x1_jwks_JWK">JWK</a> &#123;<br />        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a> &#123; id, payload &#125;)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_initialize"></a>

## Function `initialize`

Initialize some JWK resources. Should only be invoked by genesis.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_initialize">initialize</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_initialize">initialize</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <b>move_to</b>(fx, <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> &#123; providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] &#125;);<br />    <b>move_to</b>(fx, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> &#123; <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a> &#123; entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] &#125; &#125;);<br />    <b>move_to</b>(fx, <a href="jwks.md#0x1_jwks_Patches">Patches</a> &#123; patches: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] &#125;);<br />    <b>move_to</b>(fx, <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> &#123; <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a> &#123; entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] &#125; &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_remove_oidc_provider_internal"></a>

## Function `remove_oidc_provider_internal`

Helper function that removes an OIDC provider from the <code><a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a></code>.
Returns the old config URL of the provider, if any, as an <code>Option</code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(provider_set: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">jwks::SupportedOIDCProviders</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(provider_set: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; &#123;<br />    <b>let</b> (name_exists, idx) &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&amp;provider_set.providers, &#124;obj&#124; &#123;<br />        <b>let</b> provider: &amp;<a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> &#61; obj;<br />        provider.name &#61;&#61; name<br />    &#125;);<br /><br />    <b>if</b> (name_exists) &#123;<br />        <b>let</b> old_provider &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&amp;<b>mut</b> provider_set.providers, idx);<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(old_provider.config_url)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_upsert_into_observed_jwks"></a>

## Function `upsert_into_observed_jwks`

Only used by validators to publish their observed JWK update.

NOTE: It is assumed verification has been done to ensure each update is quorum&#45;certified,
and its <code><a href="version.md#0x1_version">version</a></code> equals to the on&#45;chain version &#43; 1.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_observed_jwks">upsert_into_observed_jwks</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, provider_jwks_vec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_observed_jwks">upsert_into_observed_jwks</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, provider_jwks_vec: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt;) <b>acquires</b> <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>, <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>, <a href="jwks.md#0x1_jwks_Patches">Patches</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <b>let</b> observed_jwks &#61; <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>&gt;(@aptos_framework);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(provider_jwks_vec, &#124;obj&#124; &#123;<br />        <b>let</b> provider_jwks: <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> &#61; obj;<br />        <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(&amp;<b>mut</b> observed_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, provider_jwks);<br />    &#125;);<br /><br />    <b>let</b> epoch &#61; <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>();<br />    emit(<a href="jwks.md#0x1_jwks_ObservedJWKsUpdated">ObservedJWKsUpdated</a> &#123; epoch, <a href="jwks.md#0x1_jwks">jwks</a>: observed_jwks.<a href="jwks.md#0x1_jwks">jwks</a> &#125;);<br />    <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_remove_issuer_from_observed_jwks"></a>

## Function `remove_issuer_from_observed_jwks`

Only used by governance to delete an issuer from <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code>, if it exists.

Return the potentially existing <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> of the given issuer.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_issuer_from_observed_jwks">remove_issuer_from_observed_jwks</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_issuer_from_observed_jwks">remove_issuer_from_observed_jwks</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>, <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>, <a href="jwks.md#0x1_jwks_Patches">Patches</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <b>let</b> observed_jwks &#61; <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>&gt;(@aptos_framework);<br />    <b>let</b> old_value &#61; <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(&amp;<b>mut</b> observed_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, issuer);<br /><br />    <b>let</b> epoch &#61; <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>();<br />    emit(<a href="jwks.md#0x1_jwks_ObservedJWKsUpdated">ObservedJWKsUpdated</a> &#123; epoch, <a href="jwks.md#0x1_jwks">jwks</a>: observed_jwks.<a href="jwks.md#0x1_jwks">jwks</a> &#125;);<br />    <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>();<br /><br />    old_value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_regenerate_patched_jwks"></a>

## Function `regenerate_patched_jwks`

Regenerate <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code> from <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code> and <code><a href="jwks.md#0x1_jwks_Patches">Patches</a></code> and save the result.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>() <b>acquires</b> <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>, <a href="jwks.md#0x1_jwks_Patches">Patches</a>, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> &#123;<br />    <b>let</b> <a href="jwks.md#0x1_jwks">jwks</a> &#61; <b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>&gt;(@aptos_framework).<a href="jwks.md#0x1_jwks">jwks</a>;<br />    <b>let</b> patches &#61; <b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_Patches">Patches</a>&gt;(@aptos_framework);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;patches.patches, &#124;obj&#124;&#123;<br />        <b>let</b> patch: &amp;<a href="jwks.md#0x1_jwks_Patch">Patch</a> &#61; obj;<br />        <a href="jwks.md#0x1_jwks_apply_patch">apply_patch</a>(&amp;<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>, &#42;patch);<br />    &#125;);<br />    &#42;<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>&gt;(@aptos_framework) &#61; <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> &#123; <a href="jwks.md#0x1_jwks">jwks</a> &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_try_get_jwk_by_issuer"></a>

## Function `try_get_jwk_by_issuer`

Get a JWK by issuer and key ID from a <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>, if it exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_by_issuer">try_get_jwk_by_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_by_issuer">try_get_jwk_by_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; &#123;<br />    <b>let</b> (issuer_found, index) &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&amp;<a href="jwks.md#0x1_jwks">jwks</a>.entries, &#124;obj&#124; &#123;<br />        <b>let</b> provider_jwks: &amp;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> &#61; obj;<br />        issuer &#61;&#61; provider_jwks.issuer<br />    &#125;);<br /><br />    <b>if</b> (issuer_found) &#123;<br />        <a href="jwks.md#0x1_jwks_try_get_jwk_by_id">try_get_jwk_by_id</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;<a href="jwks.md#0x1_jwks">jwks</a>.entries, index), jwk_id)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_try_get_jwk_by_id"></a>

## Function `try_get_jwk_by_id`

Get a JWK by key ID from a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>, if it exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_by_id">try_get_jwk_by_id</a>(provider_jwks: &amp;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_by_id">try_get_jwk_by_id</a>(provider_jwks: &amp;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; &#123;<br />    <b>let</b> (jwk_id_found, index) &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&amp;provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, &#124;obj&#124;&#123;<br />        <b>let</b> jwk: &amp;<a href="jwks.md#0x1_jwks_JWK">JWK</a> &#61; obj;<br />        jwk_id &#61;&#61; <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk)<br />    &#125;);<br /><br />    <b>if</b> (jwk_id_found) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(&#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, index))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_get_jwk_id"></a>

## Function `get_jwk_id`

Get the ID of a JWK.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk: &amp;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk: &amp;<a href="jwks.md#0x1_jwks_JWK">JWK</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <b>let</b> variant_type_name &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&amp;jwk.variant));<br />    <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="jwks.md#0x1_jwks_RSA_JWK">0x1::jwks::RSA_JWK</a>&quot;) &#123;<br />        <b>let</b> rsa &#61; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a>&gt;(jwk.variant);<br />        &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&amp;rsa.kid)<br />    &#125; <b>else</b> <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="jwks.md#0x1_jwks_UnsupportedJWK">0x1::jwks::UnsupportedJWK</a>&quot;) &#123;<br />        <b>let</b> unsupported &#61; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a>&gt;(jwk.variant);<br />        unsupported.id<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EUNKNOWN_JWK_VARIANT">EUNKNOWN_JWK_VARIANT</a>))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_upsert_provider_jwks"></a>

## Function `upsert_provider_jwks`

Upsert a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> into an <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>. If this upsert replaced an existing entry, return it.
Maintains the sorted&#45;by&#45;issuer invariant in <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a>, provider_jwks: <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a>, provider_jwks: <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>): Option&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt; &#123;<br />    // NOTE: Using a linear&#45;time search here because we do not expect too many providers.<br />    <b>let</b> found &#61; <b>false</b>;<br />    <b>let</b> index &#61; 0;<br />    <b>let</b> num_entries &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;<a href="jwks.md#0x1_jwks">jwks</a>.entries);<br />    <b>while</b> (index &lt; num_entries) &#123;<br />        <b>let</b> cur_entry &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;<a href="jwks.md#0x1_jwks">jwks</a>.entries, index);<br />        <b>let</b> comparison &#61; compare_u8_vector(provider_jwks.issuer, cur_entry.issuer);<br />        <b>if</b> (is_greater_than(&amp;comparison)) &#123;<br />            index &#61; index &#43; 1;<br />        &#125; <b>else</b> &#123;<br />            found &#61; is_equal(&amp;comparison);<br />            <b>break</b><br />        &#125;<br />    &#125;;<br /><br />    // Now <b>if</b> `found &#61;&#61; <b>true</b>`, `index` points <b>to</b> the <a href="jwks.md#0x1_jwks_JWK">JWK</a> we want <b>to</b> <b>update</b>/remove; otherwise, `index` points <b>to</b><br />    // <b>where</b> we want <b>to</b> insert.<br />    <b>let</b> ret &#61; <b>if</b> (found) &#123;<br />        <b>let</b> entry &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index);<br />        <b>let</b> old_entry &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(&#42;entry);<br />        &#42;entry &#61; provider_jwks;<br />        old_entry<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(&amp;<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index, provider_jwks);<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;;<br /><br />    ret<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_remove_issuer"></a>

## Function `remove_issuer`

Remove the entry of an issuer from a <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code> and return the entry, if exists.
Maintains the sorted&#45;by&#45;issuer invariant in <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt; &#123;<br />    <b>let</b> (found, index) &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&amp;<a href="jwks.md#0x1_jwks">jwks</a>.entries, &#124;obj&#124; &#123;<br />        <b>let</b> provider_jwk_set: &amp;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> &#61; obj;<br />        provider_jwk_set.issuer &#61;&#61; issuer<br />    &#125;);<br /><br />    <b>let</b> ret &#61; <b>if</b> (found) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&amp;<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;;<br /><br />    ret<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_upsert_jwk"></a>

## Function `upsert_jwk`

Upsert a <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> into a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>. If this upsert replaced an existing entry, return it.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_jwk">upsert_jwk</a>(set: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk: <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_jwk">upsert_jwk</a>(set: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk: <a href="jwks.md#0x1_jwks_JWK">JWK</a>): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; &#123;<br />    <b>let</b> found &#61; <b>false</b>;<br />    <b>let</b> index &#61; 0;<br />    <b>let</b> num_entries &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;set.<a href="jwks.md#0x1_jwks">jwks</a>);<br />    <b>while</b> (index &lt; num_entries) &#123;<br />        <b>let</b> cur_entry &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;set.<a href="jwks.md#0x1_jwks">jwks</a>, index);<br />        <b>let</b> comparison &#61; compare_u8_vector(<a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(&amp;jwk), <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(cur_entry));<br />        <b>if</b> (is_greater_than(&amp;comparison)) &#123;<br />            index &#61; index &#43; 1;<br />        &#125; <b>else</b> &#123;<br />            found &#61; is_equal(&amp;comparison);<br />            <b>break</b><br />        &#125;<br />    &#125;;<br /><br />    // Now <b>if</b> `found &#61;&#61; <b>true</b>`, `index` points <b>to</b> the <a href="jwks.md#0x1_jwks_JWK">JWK</a> we want <b>to</b> <b>update</b>/remove; otherwise, `index` points <b>to</b><br />    // <b>where</b> we want <b>to</b> insert.<br />    <b>let</b> ret &#61; <b>if</b> (found) &#123;<br />        <b>let</b> entry &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> set.<a href="jwks.md#0x1_jwks">jwks</a>, index);<br />        <b>let</b> old_entry &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(&#42;entry);<br />        &#42;entry &#61; jwk;<br />        old_entry<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(&amp;<b>mut</b> set.<a href="jwks.md#0x1_jwks">jwks</a>, index, jwk);<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;;<br /><br />    ret<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_remove_jwk"></a>

## Function `remove_jwk`

Remove the entry of a key ID from a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> and return the entry, if exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_jwk">remove_jwk</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_jwk">remove_jwk</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; &#123;<br />    <b>let</b> (found, index) &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&amp;<a href="jwks.md#0x1_jwks">jwks</a>.<a href="jwks.md#0x1_jwks">jwks</a>, &#124;obj&#124; &#123;<br />        <b>let</b> jwk: &amp;<a href="jwks.md#0x1_jwks_JWK">JWK</a> &#61; obj;<br />        jwk_id &#61;&#61; <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk)<br />    &#125;);<br /><br />    <b>let</b> ret &#61; <b>if</b> (found) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&amp;<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.<a href="jwks.md#0x1_jwks">jwks</a>, index))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;;<br /><br />    ret<br />&#125;<br /></code></pre>



</details>

<a id="0x1_jwks_apply_patch"></a>

## Function `apply_patch`

Modify an <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code> object with a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code>.
Maintains the sorted&#45;by&#45;issuer invariant in <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_apply_patch">apply_patch</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a>, patch: <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_apply_patch">apply_patch</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &amp;<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a>, patch: <a href="jwks.md#0x1_jwks_Patch">Patch</a>) &#123;<br />    <b>let</b> variant_type_name &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&amp;patch.variant));<br />    <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="jwks.md#0x1_jwks_PatchRemoveAll">0x1::jwks::PatchRemoveAll</a>&quot;) &#123;<br />        <a href="jwks.md#0x1_jwks">jwks</a>.entries &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    &#125; <b>else</b> <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="jwks.md#0x1_jwks_PatchRemoveIssuer">0x1::jwks::PatchRemoveIssuer</a>&quot;) &#123;<br />        <b>let</b> cmd &#61; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_PatchRemoveIssuer">PatchRemoveIssuer</a>&gt;(patch.variant);<br />        <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>, cmd.issuer);<br />    &#125; <b>else</b> <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="jwks.md#0x1_jwks_PatchRemoveJWK">0x1::jwks::PatchRemoveJWK</a>&quot;) &#123;<br />        <b>let</b> cmd &#61; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_PatchRemoveJWK">PatchRemoveJWK</a>&gt;(patch.variant);<br />        // TODO: This is inefficient: we remove the issuer, modify its JWKs &amp; and reinsert the updated issuer. Why<br />        // not just <b>update</b> it in place?<br />        <b>let</b> existing_jwk_set &#61; <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>, cmd.issuer);<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;existing_jwk_set)) &#123;<br />            <b>let</b> jwk_set &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> existing_jwk_set);<br />            <a href="jwks.md#0x1_jwks_remove_jwk">remove_jwk</a>(&amp;<b>mut</b> jwk_set, cmd.jwk_id);<br />            <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, jwk_set);<br />        &#125;;<br />    &#125; <b>else</b> <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="jwks.md#0x1_jwks_PatchUpsertJWK">0x1::jwks::PatchUpsertJWK</a>&quot;) &#123;<br />        <b>let</b> cmd &#61; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_PatchUpsertJWK">PatchUpsertJWK</a>&gt;(patch.variant);<br />        // TODO: This is inefficient: we remove the issuer, modify its JWKs &amp; and reinsert the updated issuer. Why<br />        // not just <b>update</b> it in place?<br />        <b>let</b> existing_jwk_set &#61; <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>, cmd.issuer);<br />        <b>let</b> jwk_set &#61; <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;existing_jwk_set)) &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> existing_jwk_set)<br />        &#125; <b>else</b> &#123;<br />            <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> &#123;<br />                <a href="version.md#0x1_version">version</a>: 0,<br />                issuer: cmd.issuer,<br />                <a href="jwks.md#0x1_jwks">jwks</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],<br />            &#125;<br />        &#125;;<br />        <a href="jwks.md#0x1_jwks_upsert_jwk">upsert_jwk</a>(&amp;<b>mut</b> jwk_set, cmd.jwk);<br />        <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, jwk_set);<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b>(std::error::invalid_argument(<a href="jwks.md#0x1_jwks_EUNKNOWN_PATCH_VARIANT">EUNKNOWN_PATCH_VARIANT</a>))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>requires</b> @aptos_framework &#61;&#61; std::signer::address_of(framework);<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
