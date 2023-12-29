
<a id="0x1_jwks"></a>

# Module `0x1::jwks`

JWK functions and structs.


-  [Struct `OIDCProvider`](#0x1_jwks_OIDCProvider)
-  [Resource `SupportedOIDCProviders`](#0x1_jwks_SupportedOIDCProviders)
-  [Struct `UnsupportedJWK`](#0x1_jwks_UnsupportedJWK)
-  [Struct `RSA_JWK`](#0x1_jwks_RSA_JWK)
-  [Struct `JWK`](#0x1_jwks_JWK)
-  [Struct `ProviderJWKs`](#0x1_jwks_ProviderJWKs)
-  [Struct `JWKs`](#0x1_jwks_JWKs)
-  [Resource `ObservedJWKs`](#0x1_jwks_ObservedJWKs)
-  [Struct `ObservedJWKsUpdated`](#0x1_jwks_ObservedJWKsUpdated)
-  [Struct `JWKPatch`](#0x1_jwks_JWKPatch)
-  [Struct `JWKPatchRemoveAll`](#0x1_jwks_JWKPatchRemoveAll)
-  [Struct `JWKPatchRemoveIssuer`](#0x1_jwks_JWKPatchRemoveIssuer)
-  [Struct `JWKPatchRemoveJWK`](#0x1_jwks_JWKPatchRemoveJWK)
-  [Struct `JWKPatchUpsertJWK`](#0x1_jwks_JWKPatchUpsertJWK)
-  [Resource `JWKPatches`](#0x1_jwks_JWKPatches)
-  [Resource `FinalJWKs`](#0x1_jwks_FinalJWKs)
-  [Constants](#@Constants_0)
-  [Function `exists_in_final_jwks`](#0x1_jwks_exists_in_final_jwks)
-  [Function `get_final_jwk`](#0x1_jwks_get_final_jwk)
-  [Function `try_get_final_jwk`](#0x1_jwks_try_get_final_jwk)
-  [Function `upsert_into_supported_oidc_providers`](#0x1_jwks_upsert_into_supported_oidc_providers)
-  [Function `remove_from_supported_oidc_providers`](#0x1_jwks_remove_from_supported_oidc_providers)
-  [Function `set_jwk_patches`](#0x1_jwks_set_jwk_patches)
-  [Function `new_jwk_patch_remove_all`](#0x1_jwks_new_jwk_patch_remove_all)
-  [Function `new_jwk_patch_remove_issuer`](#0x1_jwks_new_jwk_patch_remove_issuer)
-  [Function `new_jwk_patch_remove_jwk`](#0x1_jwks_new_jwk_patch_remove_jwk)
-  [Function `new_jwk_patch_upsert_jwk`](#0x1_jwks_new_jwk_patch_upsert_jwk)
-  [Function `new_rsa_jwk`](#0x1_jwks_new_rsa_jwk)
-  [Function `new_unsupported_jwk`](#0x1_jwks_new_unsupported_jwk)
-  [Function `initialize`](#0x1_jwks_initialize)
-  [Function `upsert_provider_jwks`](#0x1_jwks_upsert_provider_jwks)
-  [Function `regenerate_final_jwks`](#0x1_jwks_regenerate_final_jwks)
-  [Function `exists_in_jwks`](#0x1_jwks_exists_in_jwks)
-  [Function `exists_in_provider_jwks`](#0x1_jwks_exists_in_provider_jwks)
-  [Function `get_jwk_from_jwks`](#0x1_jwks_get_jwk_from_jwks)
-  [Function `get_jwk_from_provider_jwks`](#0x1_jwks_get_jwk_from_provider_jwks)
-  [Function `try_get_jwk_from_jwks`](#0x1_jwks_try_get_jwk_from_jwks)
-  [Function `try_get_jwk_from_provider_jwks`](#0x1_jwks_try_get_jwk_from_provider_jwks)
-  [Function `get_jwk_id`](#0x1_jwks_get_jwk_id)
-  [Function `upsert_into_jwks`](#0x1_jwks_upsert_into_jwks)
-  [Function `remove_from_jwks`](#0x1_jwks_remove_from_jwks)
-  [Function `upsert_into_provider_jwks`](#0x1_jwks_upsert_into_provider_jwks)
-  [Function `remove_from_provider_jwks`](#0x1_jwks_remove_from_provider_jwks)
-  [Function `apply_patch_to_jwks`](#0x1_jwks_apply_patch_to_jwks)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/comparator.md#0x1_comparator">0x1::comparator</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
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
 The utf-8 encoded issuer string. E.g., b"https://www.facebook.com".
</dd>
<dt>
<code>config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The ut8-8 encoded OpenID configuration URL of the provider.
 E.g., b"https://www.facebook.com/.well-known/openid-configuration/".
</dd>
</dl>


</details>

<a id="0x1_jwks_SupportedOIDCProviders"></a>

## Resource `SupportedOIDCProviders`

A list of OIDC providers whose JWKs should be watched by validators. Maintained by governance proposals.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> <b>has</b> key
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

<a id="0x1_jwks_UnsupportedJWK"></a>

## Struct `UnsupportedJWK`

An JWK variant that represents the JWKs which were observed but not yet supported by Aptos.
Observing <code><a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a></code>s means the providers adopted a new key type/format, and the system should be updated.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a> <b>has</b> <b>copy</b>, drop, store
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
<code>payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_RSA_JWK"></a>

## Struct `RSA_JWK`

A JWK variant where <code>kty</code> is <code>RSA</code>.


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

A JSON web key.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWK">JWK</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 A <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 - <code><a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a></code>
 - <code><a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a></code>
</dd>
</dl>


</details>

<a id="0x1_jwks_ProviderJWKs"></a>

## Struct `ProviderJWKs`

A provider and its <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code>s.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The utf-8 encoding of the issuer string (e.g., "https://www.facebook.com").
</dd>
<dt>
<code><a href="version.md#0x1_version">version</a>: u64</code>
</dt>
<dd>
 Bumped every time the JWKs for the current issuer is updated.
</dd>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;</code>
</dt>
<dd>
 The <code><a href="jwks.md#0x1_jwks">jwks</a></code> each has a unique <code>id</code> and are sorted by <code>id</code> in dictionary order.
</dd>
</dl>


</details>

<a id="0x1_jwks_JWKs"></a>

## Struct `JWKs`

Multiple <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>s, indexed by issuer and key ID.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKs">JWKs</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;</code>
</dt>
<dd>
 Entries each has a unique <code>issuer</code>, and are sorted by <code>issuer</code> in dictionary order.
</dd>
</dl>


</details>

<a id="0x1_jwks_ObservedJWKs"></a>

## Resource `ObservedJWKs`

The <code><a href="jwks.md#0x1_jwks_JWKs">JWKs</a></code> that validators observed and agreed on.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_ObservedJWKsUpdated"></a>

## Struct `ObservedJWKsUpdated`

When the <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code> is updated, this event is sent to resync the JWK consensus state in all validators.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="jwks.md#0x1_jwks_ObservedJWKsUpdated">ObservedJWKsUpdated</a> <b>has</b> drop, store
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
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWKPatch"></a>

## Struct `JWKPatch`

A small edit that can be applied to a <code><a href="jwks.md#0x1_jwks_JWKs">JWKs</a></code>.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 A <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 - <code><a href="jwks.md#0x1_jwks_JWKPatchRemoveAll">JWKPatchRemoveAll</a></code>
 - <code><a href="jwks.md#0x1_jwks_JWKPatchRemoveIssuer">JWKPatchRemoveIssuer</a></code>
 - <code><a href="jwks.md#0x1_jwks_JWKPatchRemoveJWK">JWKPatchRemoveJWK</a></code>
 - <code><a href="jwks.md#0x1_jwks_JWKPatchUpsertJWK">JWKPatchUpsertJWK</a></code>
</dd>
</dl>


</details>

<a id="0x1_jwks_JWKPatchRemoveAll"></a>

## Struct `JWKPatchRemoveAll`

A <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> variant to remove all JWKs.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKPatchRemoveAll">JWKPatchRemoveAll</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_jwks_JWKPatchRemoveIssuer"></a>

## Struct `JWKPatchRemoveIssuer`

A <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> variant to remove all JWKs from an issuer.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKPatchRemoveIssuer">JWKPatchRemoveIssuer</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_jwks_JWKPatchRemoveJWK"></a>

## Struct `JWKPatchRemoveJWK`

A <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> variant to remove a JWK.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKPatchRemoveJWK">JWKPatchRemoveJWK</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_jwks_JWKPatchUpsertJWK"></a>

## Struct `JWKPatchUpsertJWK`

A <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> variant to upsert a JWK.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKPatchUpsertJWK">JWKPatchUpsertJWK</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_jwks_JWKPatches"></a>

## Resource `JWKPatches`

A sequence of <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> that needs to be applied *one by one* to the <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code>.

Maintained by governance proposals.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>patches: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWKPatch">jwks::JWKPatch</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_FinalJWKs"></a>

## Resource `FinalJWKs`

The result of applying the <code><a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a></code> to the <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code>.
This is what applications should consume.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_jwks_EISSUER_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EISSUER_NOT_FOUND">EISSUER_NOT_FOUND</a>: u64 = 5;
</code></pre>



<a id="0x1_jwks_EJWK_ID_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EJWK_ID_NOT_FOUND">EJWK_ID_NOT_FOUND</a>: u64 = 6;
</code></pre>



<a id="0x1_jwks_EUNEXPECTED_EPOCH"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNEXPECTED_EPOCH">EUNEXPECTED_EPOCH</a>: u64 = 1;
</code></pre>



<a id="0x1_jwks_EUNEXPECTED_VERSION"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNEXPECTED_VERSION">EUNEXPECTED_VERSION</a>: u64 = 2;
</code></pre>



<a id="0x1_jwks_EUNKNOWN_JWKPATCH_VARIANT"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNKNOWN_JWKPATCH_VARIANT">EUNKNOWN_JWKPATCH_VARIANT</a>: u64 = 3;
</code></pre>



<a id="0x1_jwks_EUNKNOWN_JWK_VARIANT"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNKNOWN_JWK_VARIANT">EUNKNOWN_JWK_VARIANT</a>: u64 = 4;
</code></pre>



<a id="0x1_jwks_exists_in_final_jwks"></a>

## Function `exists_in_final_jwks`

Return whether a JWK can be found by issuer and key ID in the <code><a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_exists_in_final_jwks">exists_in_final_jwks</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_exists_in_final_jwks">exists_in_final_jwks</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool <b>acquires</b> <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a> {
    <b>let</b> <a href="jwks.md#0x1_jwks">jwks</a> = &<b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a>&gt;(@aptos_framework).<a href="jwks.md#0x1_jwks">jwks</a>;
    <a href="jwks.md#0x1_jwks_exists_in_jwks">exists_in_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, issuer, jwk_id)
}
</code></pre>



</details>

<a id="0x1_jwks_get_final_jwk"></a>

## Function `get_final_jwk`

Get a JWK by issuer and key ID from the <code><a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a></code>.
Abort if such a JWK does not exist.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_get_final_jwk">get_final_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_get_final_jwk">get_final_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">JWK</a> <b>acquires</b> <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a> {
    <b>let</b> <a href="jwks.md#0x1_jwks">jwks</a> = &<b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a>&gt;(@aptos_framework).<a href="jwks.md#0x1_jwks">jwks</a>;
    <a href="jwks.md#0x1_jwks_get_jwk_from_jwks">get_jwk_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, issuer, jwk_id)
}
</code></pre>



</details>

<a id="0x1_jwks_try_get_final_jwk"></a>

## Function `try_get_final_jwk`

Get a JWK by issuer and key ID from the <code><a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a></code>, if it exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_try_get_final_jwk">try_get_final_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_try_get_final_jwk">try_get_final_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a> {
    <b>let</b> <a href="jwks.md#0x1_jwks">jwks</a> = &<b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a>&gt;(@aptos_framework).<a href="jwks.md#0x1_jwks">jwks</a>;
    <a href="jwks.md#0x1_jwks_try_get_jwk_from_jwks">try_get_jwk_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, issuer, jwk_id)
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_into_supported_oidc_providers"></a>

## Function `upsert_into_supported_oidc_providers`

Upsert an OIDC provider metadata into the <code><a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a></code>.
Can only be called in a governance proposal.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_supported_oidc_providers">upsert_into_supported_oidc_providers</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_supported_oidc_providers">upsert_into_supported_oidc_providers</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);

    <b>let</b> provider_set = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework);

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

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> provider_set.providers, <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> { name, config_url });

    old_config_endpoint
}
</code></pre>



</details>

<a id="0x1_jwks_remove_from_supported_oidc_providers"></a>

## Function `remove_from_supported_oidc_providers`

Remove an OIDC provider from the <code><a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a></code>.
Can only be called in a governance proposal.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_from_supported_oidc_providers">remove_from_supported_oidc_providers</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_from_supported_oidc_providers">remove_from_supported_oidc_providers</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);

    <b>let</b> provider_set = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework);

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

    old_config_endpoint
}
</code></pre>



</details>

<a id="0x1_jwks_set_jwk_patches"></a>

## Function `set_jwk_patches`

Set the <code><a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a></code>. Only called in governance proposals.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_set_jwk_patches">set_jwk_patches</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, patches: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWKPatch">jwks::JWKPatch</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_set_jwk_patches">set_jwk_patches</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, patches: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a>&gt;) <b>acquires</b> <a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a>, <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a>, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a>&gt;(@aptos_framework).patches = patches;
    <a href="jwks.md#0x1_jwks_regenerate_final_jwks">regenerate_final_jwks</a>();
}
</code></pre>



</details>

<a id="0x1_jwks_new_jwk_patch_remove_all"></a>

## Function `new_jwk_patch_remove_all`

Create a <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> that removes all entries.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_jwk_patch_remove_all">new_jwk_patch_remove_all</a>(): <a href="jwks.md#0x1_jwks_JWKPatch">jwks::JWKPatch</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_jwk_patch_remove_all">new_jwk_patch_remove_all</a>(): <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> {
    <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> {
        variant: pack(<a href="jwks.md#0x1_jwks_JWKPatchRemoveAll">JWKPatchRemoveAll</a> {}),
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_jwk_patch_remove_issuer"></a>

## Function `new_jwk_patch_remove_issuer`

Create a <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> that removes the entry of a given issuer, if exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_jwk_patch_remove_issuer">new_jwk_patch_remove_issuer</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWKPatch">jwks::JWKPatch</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_jwk_patch_remove_issuer">new_jwk_patch_remove_issuer</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> {
    <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> {
        variant: pack(<a href="jwks.md#0x1_jwks_JWKPatchRemoveIssuer">JWKPatchRemoveIssuer</a> { issuer }),
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_jwk_patch_remove_jwk"></a>

## Function `new_jwk_patch_remove_jwk`

Create a <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> that removes the entry of a given issuer, if exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_jwk_patch_remove_jwk">new_jwk_patch_remove_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWKPatch">jwks::JWKPatch</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_jwk_patch_remove_jwk">new_jwk_patch_remove_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> {
    <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> {
        variant: pack(<a href="jwks.md#0x1_jwks_JWKPatchRemoveJWK">JWKPatchRemoveJWK</a> { issuer, jwk_id })
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_jwk_patch_upsert_jwk"></a>

## Function `new_jwk_patch_upsert_jwk`

Create a <code><a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a></code> that upserts a JWK into an issuer's JWK set.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_jwk_patch_upsert_jwk">new_jwk_patch_upsert_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk: <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="jwks.md#0x1_jwks_JWKPatch">jwks::JWKPatch</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_jwk_patch_upsert_jwk">new_jwk_patch_upsert_jwk</a>(issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk: <a href="jwks.md#0x1_jwks_JWK">JWK</a>): <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> {
    <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> {
        variant: pack(<a href="jwks.md#0x1_jwks_JWKPatchUpsertJWK">JWKPatchUpsertJWK</a> { issuer, jwk })
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_rsa_jwk"></a>

## Function `new_rsa_jwk`

Create a <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> of variant <code><a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_rsa_jwk">new_rsa_jwk</a>(kid: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, alg: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, e: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, n: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_rsa_jwk">new_rsa_jwk</a>(kid: String, alg: String, e: String, n: String): <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
    <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a> {
            kid,
            kty: utf8(b"RSA"),
            e,
            n,
            alg,
        }),
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_unsupported_jwk"></a>

## Function `new_unsupported_jwk`

Create a <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> of variant <code><a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_unsupported_jwk">new_unsupported_jwk</a>(id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_unsupported_jwk">new_unsupported_jwk</a>(id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
    <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a> { id, payload })
    }
}
</code></pre>



</details>

<a id="0x1_jwks_initialize"></a>

## Function `initialize`

Initialize some JWK resources. Should only be invoked by genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_initialize">initialize</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_initialize">initialize</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> { providers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] });
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_JWKs">JWKs</a> { entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] } });
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a> { patches: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[] });
    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_JWKs">JWKs</a> { entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a> [] } });
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_provider_jwks"></a>

## Function `upsert_provider_jwks`

Only used by validators to publish their observed JWK update.

NOTE: for validator-proposed updates, the quorum certificate acquisition and verification should be done before invoking this.
This function should only worry about on-chain state updates.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_JWKs">JWKs</a>) <b>acquires</b> <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>, <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a>, <a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);
    *<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>&gt;(@aptos_framework) = <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a> };
    <b>let</b> epoch = <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>();
    emit(<a href="jwks.md#0x1_jwks_ObservedJWKsUpdated">ObservedJWKsUpdated</a> { epoch, <a href="jwks.md#0x1_jwks">jwks</a> });
    <a href="jwks.md#0x1_jwks_regenerate_final_jwks">regenerate_final_jwks</a>();
}
</code></pre>



</details>

<a id="0x1_jwks_regenerate_final_jwks"></a>

## Function `regenerate_final_jwks`

Regenerate <code><a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a></code> from <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code> and <code><a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a></code> and save the result.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_regenerate_final_jwks">regenerate_final_jwks</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_regenerate_final_jwks">regenerate_final_jwks</a>() <b>acquires</b> <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a>, <a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a>, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> {
    <b>let</b> <a href="jwks.md#0x1_jwks">jwks</a> = <b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>&gt;(@aptos_framework).<a href="jwks.md#0x1_jwks">jwks</a>;
    <b>let</b> patches = <b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_JWKPatches">JWKPatches</a>&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&patches.patches, |obj|{
        <b>let</b> patch: &<a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a> = obj;
        <a href="jwks.md#0x1_jwks_apply_patch_to_jwks">apply_patch_to_jwks</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>, *patch);
    });
    *<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a>&gt;(@aptos_framework) = <a href="jwks.md#0x1_jwks_FinalJWKs">FinalJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a> };
}
</code></pre>



</details>

<a id="0x1_jwks_exists_in_jwks"></a>

## Function `exists_in_jwks`

Return whether a JWK can be found by issuer and key ID in a <code><a href="jwks.md#0x1_jwks_JWKs">JWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_exists_in_jwks">exists_in_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_exists_in_jwks">exists_in_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<a href="jwks.md#0x1_jwks_JWKs">JWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <b>let</b> (issuer_found, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, |obj| {
        <b>let</b> provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> = obj;
        !is_greater_than(&compare_u8_vector(issuer, provider_jwks.issuer))
    });

    issuer_found && <a href="jwks.md#0x1_jwks_exists_in_provider_jwks">exists_in_provider_jwks</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, index), jwk_id)
}
</code></pre>



</details>

<a id="0x1_jwks_exists_in_provider_jwks"></a>

## Function `exists_in_provider_jwks`

Return whether a JWK can be found by key ID in a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_exists_in_provider_jwks">exists_in_provider_jwks</a>(provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_exists_in_provider_jwks">exists_in_provider_jwks</a>(provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_any">vector::any</a>(&provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, |obj| {
        <b>let</b> jwk: &<a href="jwks.md#0x1_jwks_JWK">JWK</a> = obj;
        jwk_id == <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk)
    })
}
</code></pre>



</details>

<a id="0x1_jwks_get_jwk_from_jwks"></a>

## Function `get_jwk_from_jwks`

Get a JWK by issuer and key ID from a <code><a href="jwks.md#0x1_jwks_JWKs">JWKs</a></code>.
Abort if such a JWK does not exist.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_from_jwks">get_jwk_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_from_jwks">get_jwk_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<a href="jwks.md#0x1_jwks_JWKs">JWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(issuer));
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"get_jwk_from_jwks for <b>loop</b>"));
    <b>let</b> (issuer_found, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, |obj| {
        <b>let</b> provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> = obj;
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(provider_jwks.issuer));
        !is_greater_than(&compare_u8_vector(issuer, provider_jwks.issuer))
    });

    <b>assert</b>!(issuer_found, invalid_argument(<a href="jwks.md#0x1_jwks_EISSUER_NOT_FOUND">EISSUER_NOT_FOUND</a>));
    <a href="jwks.md#0x1_jwks_get_jwk_from_provider_jwks">get_jwk_from_provider_jwks</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, index), jwk_id)

}
</code></pre>



</details>

<a id="0x1_jwks_get_jwk_from_provider_jwks"></a>

## Function `get_jwk_from_provider_jwks`

Get a JWK by key ID from a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>.
Abort if such a JWK does not exist.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_from_provider_jwks">get_jwk_from_provider_jwks</a>(provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_from_provider_jwks">get_jwk_from_provider_jwks</a>(provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
    <b>let</b> (jwk_id_found, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, |obj|{
        <b>let</b> jwk: &<a href="jwks.md#0x1_jwks_JWK">JWK</a> = obj;
        !is_greater_than(&compare_u8_vector(jwk_id, <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk)))
    });

    <b>assert</b>!(jwk_id_found, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EJWK_ID_NOT_FOUND">EJWK_ID_NOT_FOUND</a>));
    *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, index)
}
</code></pre>



</details>

<a id="0x1_jwks_try_get_jwk_from_jwks"></a>

## Function `try_get_jwk_from_jwks`

Get a JWK by issuer and key ID from a <code><a href="jwks.md#0x1_jwks_JWKs">JWKs</a></code>, if it exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_from_jwks">try_get_jwk_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_from_jwks">try_get_jwk_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<a href="jwks.md#0x1_jwks_JWKs">JWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; {
    <b>let</b> (issuer_found, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, |obj| {
        <b>let</b> provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> = obj;
        !is_greater_than(&compare_u8_vector(issuer, provider_jwks.issuer))
    });

    <b>if</b> (issuer_found) {
        <a href="jwks.md#0x1_jwks_try_get_jwk_from_provider_jwks">try_get_jwk_from_provider_jwks</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, index), jwk_id)
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }

}
</code></pre>



</details>

<a id="0x1_jwks_try_get_jwk_from_provider_jwks"></a>

## Function `try_get_jwk_from_provider_jwks`

Get a JWK by key ID from a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>, if it exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_from_provider_jwks">try_get_jwk_from_provider_jwks</a>(provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_from_provider_jwks">try_get_jwk_from_provider_jwks</a>(provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; {
    <b>let</b> (jwk_id_found, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, |obj|{
        <b>let</b> jwk: &<a href="jwks.md#0x1_jwks_JWK">JWK</a> = obj;
        !is_greater_than(&compare_u8_vector(jwk_id, <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk)))
    });

    <b>if</b> (jwk_id_found) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, index))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_jwks_get_jwk_id"></a>

## Function `get_jwk_id`

Get the ID of a JWK.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk: &<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk: &<a href="jwks.md#0x1_jwks_JWK">JWK</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&jwk.variant));
    <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_RSA_JWK">0x1::jwks::RSA_JWK</a>") {
        <b>let</b> rsa = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a>&gt;(jwk.variant);
        *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&rsa.kid)
    } <b>else</b> <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_UnsupportedJWK">0x1::jwks::UnsupportedJWK</a>") {
        <b>let</b> unsupported = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a>&gt;(jwk.variant);
        unsupported.id
    } <b>else</b> {
        <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EUNKNOWN_JWK_VARIANT">EUNKNOWN_JWK_VARIANT</a>))
    }
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_into_jwks"></a>

## Function `upsert_into_jwks`

Upsert a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> into a <code><a href="jwks.md#0x1_jwks_JWKs">JWKs</a></code>. If this upsert replaced an existing entry, return it.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_jwks">upsert_into_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a>, provider_jwks: <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_jwks">upsert_into_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_JWKs">JWKs</a>, provider_jwks: <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>): Option&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt; {
    <b>let</b> found = <b>false</b>;
    <b>let</b> index = 0;
    <b>let</b> num_entries = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries);
    <b>while</b> (index &lt; num_entries) {
        <b>let</b> cur_entry = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, index);
        <b>let</b> comparison = compare_u8_vector(provider_jwks.issuer, cur_entry.issuer);
        <b>if</b> (is_greater_than(&comparison)) {
            index = index + 1;
        } <b>else</b> {
            found = is_equal(&comparison);
            <b>break</b>
        }
    };

    <b>let</b> ret = <b>if</b> (found) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index, provider_jwks);

    ret
}
</code></pre>



</details>

<a id="0x1_jwks_remove_from_jwks"></a>

## Function `remove_from_jwks`

Remove the entry of an issuer from a <code><a href="jwks.md#0x1_jwks_JWKs">JWKs</a></code> and return the entry, if exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_from_jwks">remove_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_from_jwks">remove_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_JWKs">JWKs</a>, issuer: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt; {
    <b>let</b> found = <b>false</b>;
    <b>let</b> index = 0;
    <b>let</b> num_entries = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries);
    <b>while</b> (index &lt; num_entries) {
        <b>let</b> cur_entry = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, index);
        <b>let</b> comparison = compare_u8_vector(issuer, cur_entry.issuer);
        <b>if</b> (is_greater_than(&comparison)) {
            index = index + 1;
        } <b>else</b> {
            found = is_equal(&comparison);
            <b>break</b>
        }
    };

    <b>let</b> ret = <b>if</b> (found) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    ret
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_into_provider_jwks"></a>

## Function `upsert_into_provider_jwks`

Upsert a <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> into a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>. If this upsert replaced an existing entry, return it.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_provider_jwks">upsert_into_provider_jwks</a>(set: &<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk: <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_provider_jwks">upsert_into_provider_jwks</a>(set: &<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk: <a href="jwks.md#0x1_jwks_JWK">JWK</a>): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; {
    <b>let</b> found = <b>false</b>;
    <b>let</b> index = 0;
    <b>let</b> num_entries = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&set.<a href="jwks.md#0x1_jwks">jwks</a>);
    <b>while</b> (index &lt; num_entries) {
        <b>let</b> cur_entry = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&set.<a href="jwks.md#0x1_jwks">jwks</a>, index);
        <b>let</b> comparison = compare_u8_vector(<a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(&jwk), <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(cur_entry));
        <b>if</b> (is_greater_than(&comparison)) {
            index = index + 1;
        } <b>else</b> {
            found = is_equal(&comparison);
            <b>break</b>
        }
    };

    // Now <b>if</b> `found == <b>true</b>`, `index` points <b>to</b> the <a href="jwks.md#0x1_jwks_JWK">JWK</a> we want <b>to</b> <b>update</b>/remove; otherwise, `index` points <b>to</b> <b>where</b> we want <b>to</b> insert.

    <b>let</b> ret = <b>if</b> (found) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> set.<a href="jwks.md#0x1_jwks">jwks</a>, index))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(&<b>mut</b> set.<a href="jwks.md#0x1_jwks">jwks</a>, index, jwk);

    ret
}
</code></pre>



</details>

<a id="0x1_jwks_remove_from_provider_jwks"></a>

## Function `remove_from_provider_jwks`

Remove the entry of a key ID from a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> and return the entry, if exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_from_provider_jwks">remove_from_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_from_provider_jwks">remove_from_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; {
    <b>let</b> found = <b>false</b>;
    <b>let</b> index = 0;
    <b>let</b> num_entries = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.<a href="jwks.md#0x1_jwks">jwks</a>);
    <b>while</b> (index &lt; num_entries) {
        <b>let</b> cur_entry = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.<a href="jwks.md#0x1_jwks">jwks</a>, index);
        <b>let</b> comparison = compare_u8_vector(jwk_id, <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(cur_entry));
        <b>if</b> (is_greater_than(&comparison)) {
            index = index + 1;
        } <b>else</b> {
            found = is_equal(&comparison);
            <b>break</b>
        }
    };

    // Now <b>if</b> `found == <b>true</b>`, `index` points <b>to</b> the <a href="jwks.md#0x1_jwks_JWK">JWK</a> we want <b>to</b> <b>update</b>/remove; otherwise, `index` points <b>to</b> <b>where</b> we want <b>to</b> insert.

    <b>let</b> ret = <b>if</b> (found) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.<a href="jwks.md#0x1_jwks">jwks</a>, index))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    ret
}
</code></pre>



</details>

<a id="0x1_jwks_apply_patch_to_jwks"></a>

## Function `apply_patch_to_jwks`

Modify a <code><a href="jwks.md#0x1_jwks_JWKs">JWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_apply_patch_to_jwks">apply_patch_to_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_JWKs">jwks::JWKs</a>, patch: <a href="jwks.md#0x1_jwks_JWKPatch">jwks::JWKPatch</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_apply_patch_to_jwks">apply_patch_to_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_JWKs">JWKs</a>, patch: <a href="jwks.md#0x1_jwks_JWKPatch">JWKPatch</a>) {
    <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&patch.variant));
    <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_JWKPatchRemoveAll">0x1::jwks::JWKPatchRemoveAll</a>") {
        <a href="jwks.md#0x1_jwks">jwks</a>.entries = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    } <b>else</b> <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_JWKPatchRemoveIssuer">0x1::jwks::JWKPatchRemoveIssuer</a>") {
        <b>let</b> cmd = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_JWKPatchRemoveIssuer">JWKPatchRemoveIssuer</a>&gt;(patch.variant);
        <b>let</b> (found, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, |obj| {
            <b>let</b> provider_jwk_set: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> = obj;
            provider_jwk_set.issuer == cmd.issuer
        });
        <b>if</b> (found) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index);
        };
    } <b>else</b> <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_JWKPatchRemoveJWK">0x1::jwks::JWKPatchRemoveJWK</a>") {
        <b>let</b> cmd = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_JWKPatchRemoveJWK">JWKPatchRemoveJWK</a>&gt;(patch.variant);
        <b>let</b> existing_jwk_set = <a href="jwks.md#0x1_jwks_remove_from_jwks">remove_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, cmd.issuer);
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&existing_jwk_set)) {
            <b>let</b> jwk_set = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> existing_jwk_set);
            <a href="jwks.md#0x1_jwks_remove_from_provider_jwks">remove_from_provider_jwks</a>(&<b>mut</b> jwk_set, cmd.jwk_id);
            <a href="jwks.md#0x1_jwks_upsert_into_jwks">upsert_into_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, jwk_set);
        };
    } <b>else</b> <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_JWKPatchUpsertJWK">0x1::jwks::JWKPatchUpsertJWK</a>") {
        <b>let</b> cmd = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_JWKPatchUpsertJWK">JWKPatchUpsertJWK</a>&gt;(patch.variant);
        <b>let</b> existing_jwk_set = <a href="jwks.md#0x1_jwks_remove_from_jwks">remove_from_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, cmd.issuer);
        <b>let</b> jwk_set = <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&existing_jwk_set)) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> existing_jwk_set)
        } <b>else</b> {
            <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> {
                <a href="version.md#0x1_version">version</a>: 0,
                issuer: cmd.issuer,
                <a href="jwks.md#0x1_jwks">jwks</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
            }
        };
        <a href="jwks.md#0x1_jwks_upsert_into_provider_jwks">upsert_into_provider_jwks</a>(&<b>mut</b> jwk_set, cmd.jwk);
        <a href="jwks.md#0x1_jwks_upsert_into_jwks">upsert_into_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, jwk_set);
    } <b>else</b> {
        <b>abort</b>(std::error::invalid_argument(<a href="jwks.md#0x1_jwks_EUNKNOWN_JWKPATCH_VARIANT">EUNKNOWN_JWKPATCH_VARIANT</a>))
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
