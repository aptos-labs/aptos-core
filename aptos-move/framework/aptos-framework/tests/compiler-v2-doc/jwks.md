
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
-  [Resource `FederatedJWKs`](#0x1_jwks_FederatedJWKs)
-  [Constants](#@Constants_0)
-  [Function `patch_federated_jwks`](#0x1_jwks_patch_federated_jwks)
-  [Function `update_federated_jwk_set`](#0x1_jwks_update_federated_jwk_set)
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


<pre><code><b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;
<b>use</b> <a href="../../../aptos-stdlib/tests/compiler-v2-doc/comparator.md#0x1_comparator">0x1::comparator</a>;
<b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_jwks_OIDCProvider"></a>

## Struct `OIDCProvider`

An OIDC provider.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The utf-8 encoded issuer string. E.g., b"https://www.facebook.com".
</dd>
<dt>
<code>config_url: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>providers: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_OIDCProvider">jwks::OIDCProvider</a>&gt;</code>
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
<code>id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>payload: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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
<code>kid: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>kty: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alg: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>e: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>n: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a></code>
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
<code>variant: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
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
<code>issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The utf-8 encoding of the issuer string (e.g., "https://www.facebook.com").
</dd>
<dt>
<code><a href="version.md#0x1_version">version</a>: u64</code>
</dt>
<dd>
 A version number is needed by JWK consensus to dedup the updates.
 e.g, when on chain version = 5, multiple nodes can propose an update with version = 6.
 Bumped every time the JWKs for the current issuer is updated.
 The Rust authenticator only uses the latest version.
</dd>
<dt>
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;</code>
</dt>
<dd>
 Vector of <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code>'s sorted by their unique ID (from <code>get_jwk_id</code>) in dictionary order.
</dd>
</dl>


</details>

<a id="0x1_jwks_AllProvidersJWKs"></a>

## Struct `AllProvidersJWKs`

Multiple <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> objects, indexed by issuer and key ID.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;</code>
</dt>
<dd>
 Vector of <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> sorted by <code>ProviderJWKs::issuer</code> in dictionary order.
</dd>
</dl>


</details>

<a id="0x1_jwks_ObservedJWKs"></a>

## Resource `ObservedJWKs`

The <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code> that validators observed and agreed on.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



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
<code><a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_Patch"></a>

## Struct `Patch`

A small edit or patch that is applied to a <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code> to obtain <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code>.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_Patch">Patch</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 - <code><a href="jwks.md#0x1_jwks_PatchRemoveAll">PatchRemoveAll</a></code>
 - <code><a href="jwks.md#0x1_jwks_PatchRemoveIssuer">PatchRemoveIssuer</a></code>
 - <code><a href="jwks.md#0x1_jwks_PatchRemoveJWK">PatchRemoveJWK</a></code>
 - <code><a href="jwks.md#0x1_jwks_PatchUpsertJWK">PatchUpsertJWK</a></code>
</dd>
</dl>


</details>

<a id="0x1_jwks_PatchRemoveAll"></a>

## Struct `PatchRemoveAll`

A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant to remove all JWKs.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchRemoveAll">PatchRemoveAll</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_jwks_PatchRemoveIssuer"></a>

## Struct `PatchRemoveIssuer`

A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant to remove an issuer and all its JWKs.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchRemoveIssuer">PatchRemoveIssuer</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_PatchRemoveJWK"></a>

## Struct `PatchRemoveJWK`

A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant to remove a specific JWK of an issuer.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchRemoveJWK">PatchRemoveJWK</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_PatchUpsertJWK"></a>

## Struct `PatchUpsertJWK`

A <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> variant to upsert a JWK for an issuer.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchUpsertJWK">PatchUpsertJWK</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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

A sequence of <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> objects that are applied *one by one* to the <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code>.

Maintained by governance proposals.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_Patches">Patches</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>patches: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_PatchedJWKs"></a>

## Resource `PatchedJWKs`

The result of applying the <code><a href="jwks.md#0x1_jwks_Patches">Patches</a></code> to the <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code>.
This is what applications should consume.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> <b>has</b> drop, key
</code></pre>



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

<a id="0x1_jwks_FederatedJWKs"></a>

## Resource `FederatedJWKs`

JWKs for federated keyless accounts are stored in this resource.


<pre><code><b>struct</b> <a href="jwks.md#0x1_jwks_FederatedJWKs">FederatedJWKs</a> <b>has</b> drop, key
</code></pre>



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


<a id="0x1_jwks_EFEDERATED_JWKS_TOO_LARGE"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EFEDERATED_JWKS_TOO_LARGE">EFEDERATED_JWKS_TOO_LARGE</a>: u64 = 8;
</code></pre>



<a id="0x1_jwks_EINSTALL_FEDERATED_JWKS_AT_APTOS_FRAMEWORK"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EINSTALL_FEDERATED_JWKS_AT_APTOS_FRAMEWORK">EINSTALL_FEDERATED_JWKS_AT_APTOS_FRAMEWORK</a>: u64 = 7;
</code></pre>



<a id="0x1_jwks_EINVALID_FEDERATED_JWK_SET"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EINVALID_FEDERATED_JWK_SET">EINVALID_FEDERATED_JWK_SET</a>: u64 = 9;
</code></pre>



<a id="0x1_jwks_EISSUER_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EISSUER_NOT_FOUND">EISSUER_NOT_FOUND</a>: u64 = 5;
</code></pre>



<a id="0x1_jwks_EJWK_ID_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EJWK_ID_NOT_FOUND">EJWK_ID_NOT_FOUND</a>: u64 = 6;
</code></pre>



<a id="0x1_jwks_ENATIVE_INCORRECT_VERSION"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_INCORRECT_VERSION">ENATIVE_INCORRECT_VERSION</a>: u64 = 259;
</code></pre>



<a id="0x1_jwks_ENATIVE_MISSING_RESOURCE_OBSERVED_JWKS"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_MISSING_RESOURCE_OBSERVED_JWKS">ENATIVE_MISSING_RESOURCE_OBSERVED_JWKS</a>: u64 = 258;
</code></pre>



<a id="0x1_jwks_ENATIVE_MISSING_RESOURCE_VALIDATOR_SET"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_MISSING_RESOURCE_VALIDATOR_SET">ENATIVE_MISSING_RESOURCE_VALIDATOR_SET</a>: u64 = 257;
</code></pre>



<a id="0x1_jwks_ENATIVE_MULTISIG_VERIFICATION_FAILED"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_MULTISIG_VERIFICATION_FAILED">ENATIVE_MULTISIG_VERIFICATION_FAILED</a>: u64 = 260;
</code></pre>



<a id="0x1_jwks_ENATIVE_NOT_ENOUGH_VOTING_POWER"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_ENATIVE_NOT_ENOUGH_VOTING_POWER">ENATIVE_NOT_ENOUGH_VOTING_POWER</a>: u64 = 261;
</code></pre>



<a id="0x1_jwks_EUNEXPECTED_EPOCH"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNEXPECTED_EPOCH">EUNEXPECTED_EPOCH</a>: u64 = 1;
</code></pre>



<a id="0x1_jwks_EUNEXPECTED_VERSION"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNEXPECTED_VERSION">EUNEXPECTED_VERSION</a>: u64 = 2;
</code></pre>



<a id="0x1_jwks_EUNKNOWN_JWK_VARIANT"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNKNOWN_JWK_VARIANT">EUNKNOWN_JWK_VARIANT</a>: u64 = 4;
</code></pre>



<a id="0x1_jwks_EUNKNOWN_PATCH_VARIANT"></a>



<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_EUNKNOWN_PATCH_VARIANT">EUNKNOWN_PATCH_VARIANT</a>: u64 = 3;
</code></pre>



<a id="0x1_jwks_MAX_FEDERATED_JWKS_SIZE_BYTES"></a>

We limit the size of a <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code> resource installed by a dapp owner for federated keyless accounts.
Note: If too large, validators waste work reading it for invalid TXN signatures.


<pre><code><b>const</b> <a href="jwks.md#0x1_jwks_MAX_FEDERATED_JWKS_SIZE_BYTES">MAX_FEDERATED_JWKS_SIZE_BYTES</a>: u64 = 2048;
</code></pre>



<a id="0x1_jwks_patch_federated_jwks"></a>

## Function `patch_federated_jwks`

Called by a federated keyless dapp owner to install the JWKs for the federated OIDC provider (e.g., Auth0, AWS
Cognito, etc). For type-safety, we explicitly use a <code><b>struct</b> <a href="jwks.md#0x1_jwks_FederatedJWKs">FederatedJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a>: AllProviderJWKs }</code> instead of
reusing <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a>: AllProviderJWKs }</code>, which is a JWK-consensus-specific struct.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_patch_federated_jwks">patch_federated_jwks</a>(jwk_owner: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, patches: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_patch_federated_jwks">patch_federated_jwks</a>(jwk_owner: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, patches: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_Patch">Patch</a>&gt;) <b>acquires</b> <a href="jwks.md#0x1_jwks_FederatedJWKs">FederatedJWKs</a> {
    // Prevents accidental calls in <a href="jwks.md#0x1_jwks">0x1::jwks</a> that install federated JWKs at the Aptos framework <b>address</b>.
    <b>assert</b>!(!<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(jwk_owner)),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EINSTALL_FEDERATED_JWKS_AT_APTOS_FRAMEWORK">EINSTALL_FEDERATED_JWKS_AT_APTOS_FRAMEWORK</a>)
    );

    <b>let</b> jwk_addr = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(jwk_owner);
    <b>if</b> (!<b>exists</b>&lt;<a href="jwks.md#0x1_jwks_FederatedJWKs">FederatedJWKs</a>&gt;(jwk_addr)) {
        <b>move_to</b>(jwk_owner, <a href="jwks.md#0x1_jwks_FederatedJWKs">FederatedJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a> { entries: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[] } });
    };

    <b>let</b> fed_jwks = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_FederatedJWKs">FederatedJWKs</a>&gt;(jwk_addr);
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&patches, |obj|{
        <b>let</b> patch: &<a href="jwks.md#0x1_jwks_Patch">Patch</a> = obj;
        <a href="jwks.md#0x1_jwks_apply_patch">apply_patch</a>(&<b>mut</b> fed_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, *patch);
    });

    // TODO: Can we check the size more efficiently instead of serializing it via BCS?
    <b>let</b> num_bytes = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(fed_jwks));
    <b>assert</b>!(num_bytes &lt; <a href="jwks.md#0x1_jwks_MAX_FEDERATED_JWKS_SIZE_BYTES">MAX_FEDERATED_JWKS_SIZE_BYTES</a>, <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EFEDERATED_JWKS_TOO_LARGE">EFEDERATED_JWKS_TOO_LARGE</a>));
}
</code></pre>



</details>

<a id="0x1_jwks_update_federated_jwk_set"></a>

## Function `update_federated_jwk_set`

This can be called to install or update a set of JWKs for a federated OIDC provider.  This function should
be invoked to intially install a set of JWKs or to update a set of JWKs when a keypair is rotated.

The <code>iss</code> parameter is the value of the <code>iss</code> claim on the JWTs that are to be verified by the JWK set.
<code>kid_vec</code>, <code>alg_vec</code>, <code>e_vec</code>, <code>n_vec</code> are String vectors of the JWK attributes <code>kid</code>, <code>alg</code>, <code>e</code> and <code>n</code> respectively.
See https://datatracker.ietf.org/doc/html/rfc7517#section-4 for more details about the JWK attributes aforementioned.

For the example JWK set snapshot below containing 2 keys for Google found at https://www.googleapis.com/oauth2/v3/certs -
```json
{
"keys": [
{
"alg": "RS256",
"use": "sig",
"kty": "RSA",
"n": "wNHgGSG5B5xOEQNFPW2p_6ZxZbfPoAU5VceBUuNwQWLop0ohW0vpoZLU1tAsq_S9s5iwy27rJw4EZAOGBR9oTRq1Y6Li5pDVJfmzyRNtmWCWndR-bPqhs_dkJU7MbGwcvfLsN9FSHESFrS9sfGtUX-lZfLoGux23TKdYV9EE-H-NDASxrVFUk2GWc3rL6UEMWrMnOqV9-tghybDU3fcRdNTDuXUr9qDYmhmNegYjYu4REGjqeSyIG1tuQxYpOBH-tohtcfGY-oRTS09kgsSS9Q5BRM4qqCkGP28WhlSf4ui0-norS0gKMMI1P_ZAGEsLn9p2TlYMpewvIuhjJs1thw",
"kid": "d7b939771a7800c413f90051012d975981916d71",
"e": "AQAB"
},
{
"kty": "RSA",
"kid": "b2620d5e7f132b52afe8875cdf3776c064249d04",
"alg": "RS256",
"n": "pi22xDdK2fz5gclIbDIGghLDYiRO56eW2GUcboeVlhbAuhuT5mlEYIevkxdPOg5n6qICePZiQSxkwcYMIZyLkZhSJ2d2M6Szx2gDtnAmee6o_tWdroKu0DjqwG8pZU693oLaIjLku3IK20lTs6-2TeH-pUYMjEqiFMhn-hb7wnvH_FuPTjgz9i0rEdw_Hf3Wk6CMypaUHi31y6twrMWq1jEbdQNl50EwH-RQmQ9bs3Wm9V9t-2-_Jzg3AT0Ny4zEDU7WXgN2DevM8_FVje4IgztNy29XUkeUctHsr-431_Iu23JIy6U4Kxn36X3RlVUKEkOMpkDD3kd81JPW4Ger_w",
"e": "AQAB",
"use": "sig"
}
]
}
```

We can call update_federated_jwk_set for Google's <code>iss</code> - "https://accounts.google.com" and for each vector
argument <code>kid_vec</code>, <code>alg_vec</code>, <code>e_vec</code>, <code>n_vec</code>, we set in index 0 the corresponding attribute in the first JWK and we set in index 1
the corresponding attribute in the second JWK as shown below.

```move
use std::string::utf8;
aptos_framework::jwks::update_federated_jwk_set(
jwk_owner,
b"https://accounts.google.com",
vector[utf8(b"d7b939771a7800c413f90051012d975981916d71"), utf8(b"b2620d5e7f132b52afe8875cdf3776c064249d04")],
vector[utf8(b"RS256"), utf8(b"RS256")],
vector[utf8(b"AQAB"), utf8(b"AQAB")],
vector[
utf8(b"wNHgGSG5B5xOEQNFPW2p_6ZxZbfPoAU5VceBUuNwQWLop0ohW0vpoZLU1tAsq_S9s5iwy27rJw4EZAOGBR9oTRq1Y6Li5pDVJfmzyRNtmWCWndR-bPqhs_dkJU7MbGwcvfLsN9FSHESFrS9sfGtUX-lZfLoGux23TKdYV9EE-H-NDASxrVFUk2GWc3rL6UEMWrMnOqV9-tghybDU3fcRdNTDuXUr9qDYmhmNegYjYu4REGjqeSyIG1tuQxYpOBH-tohtcfGY-oRTS09kgsSS9Q5BRM4qqCkGP28WhlSf4ui0-norS0gKMMI1P_ZAGEsLn9p2TlYMpewvIuhjJs1thw"),
utf8(b"pi22xDdK2fz5gclIbDIGghLDYiRO56eW2GUcboeVlhbAuhuT5mlEYIevkxdPOg5n6qICePZiQSxkwcYMIZyLkZhSJ2d2M6Szx2gDtnAmee6o_tWdroKu0DjqwG8pZU693oLaIjLku3IK20lTs6-2TeH-pUYMjEqiFMhn-hb7wnvH_FuPTjgz9i0rEdw_Hf3Wk6CMypaUHi31y6twrMWq1jEbdQNl50EwH-RQmQ9bs3Wm9V9t-2-_Jzg3AT0Ny4zEDU7WXgN2DevM8_FVje4IgztNy29XUkeUctHsr-431_Iu23JIy6U4Kxn36X3RlVUKEkOMpkDD3kd81JPW4Ger_w")
]
)
```

See AIP-96 for more details about federated keyless - https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-96.md

NOTE: Currently only RSA keys are supported.


<pre><code><b>public</b> entry <b>fun</b> <a href="jwks.md#0x1_jwks_update_federated_jwk_set">update_federated_jwk_set</a>(jwk_owner: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, iss: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, kid_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>&gt;, alg_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>&gt;, e_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>&gt;, n_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="jwks.md#0x1_jwks_update_federated_jwk_set">update_federated_jwk_set</a>(jwk_owner: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, iss: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, kid_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;String&gt;, alg_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;String&gt;, e_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;String&gt;, n_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;String&gt;) <b>acquires</b> <a href="jwks.md#0x1_jwks_FederatedJWKs">FederatedJWKs</a> {
    <b>assert</b>!(!<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&kid_vec), <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EINVALID_FEDERATED_JWK_SET">EINVALID_FEDERATED_JWK_SET</a>));
    <b>let</b> num_jwk = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>&lt;String&gt;(&kid_vec);
    <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&alg_vec) == num_jwk , <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EINVALID_FEDERATED_JWK_SET">EINVALID_FEDERATED_JWK_SET</a>));
    <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&e_vec) == num_jwk, <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EINVALID_FEDERATED_JWK_SET">EINVALID_FEDERATED_JWK_SET</a>));
    <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&n_vec) == num_jwk, <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EINVALID_FEDERATED_JWK_SET">EINVALID_FEDERATED_JWK_SET</a>));

    <b>let</b> remove_all_patch = <a href="jwks.md#0x1_jwks_new_patch_remove_all">new_patch_remove_all</a>();
    <b>let</b> patches = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[remove_all_patch];
    <b>while</b> (!<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&kid_vec)) {
        <b>let</b> kid = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> kid_vec);
        <b>let</b> alg = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> alg_vec);
        <b>let</b> e = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> e_vec);
        <b>let</b> n = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> n_vec);
        <b>let</b> jwk = <a href="jwks.md#0x1_jwks_new_rsa_jwk">new_rsa_jwk</a>(kid, alg, e, n);
        <b>let</b> patch = <a href="jwks.md#0x1_jwks_new_patch_upsert_jwk">new_patch_upsert_jwk</a>(iss, jwk);
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> patches, patch)
    };
    <a href="jwks.md#0x1_jwks_patch_federated_jwks">patch_federated_jwks</a>(jwk_owner, patches);
}
</code></pre>



</details>

<a id="0x1_jwks_get_patched_jwk"></a>

## Function `get_patched_jwk`

Get a JWK by issuer and key ID from the <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code>.
Abort if such a JWK does not exist.
More convenient to call from Rust, since it does not wrap the JWK in an <code>Option</code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_get_patched_jwk">get_patched_jwk</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_get_patched_jwk">get_patched_jwk</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">JWK</a> <b>acquires</b> <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> {
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks_try_get_patched_jwk">try_get_patched_jwk</a>(issuer, jwk_id))
}
</code></pre>



</details>

<a id="0x1_jwks_try_get_patched_jwk"></a>

## Function `try_get_patched_jwk`

Get a JWK by issuer and key ID from the <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code>, if it exists.
More convenient to call from Move, since it does not abort.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_try_get_patched_jwk">try_get_patched_jwk</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_try_get_patched_jwk">try_get_patched_jwk</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> {
    <b>let</b> <a href="jwks.md#0x1_jwks">jwks</a> = &<b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>&gt;(@aptos_framework).<a href="jwks.md#0x1_jwks">jwks</a>;
    <a href="jwks.md#0x1_jwks_try_get_jwk_by_issuer">try_get_jwk_by_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>, issuer, jwk_id)
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_oidc_provider"></a>

## Function `upsert_oidc_provider`

Deprecated by <code><a href="jwks.md#0x1_jwks_upsert_oidc_provider_for_next_epoch">upsert_oidc_provider_for_next_epoch</a>()</code>.

TODO: update all the tests that reference this function, then disable this function.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_oidc_provider">upsert_oidc_provider</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_oidc_provider">upsert_oidc_provider</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();

    <b>let</b> provider_set = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework);

    <b>let</b> old_config_url= <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(provider_set, name);
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> provider_set.providers, <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> { name, config_url });
    old_config_url
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_oidc_provider_for_next_epoch"></a>

## Function `upsert_oidc_provider_for_next_epoch`

Used in on-chain governances to update the supported OIDC providers, effective starting next epoch.
Example usage:
```
aptos_framework::jwks::upsert_oidc_provider_for_next_epoch(
&framework_signer,
b"https://accounts.google.com",
b"https://accounts.google.com/.well-known/openid-configuration"
);
aptos_framework::aptos_governance::reconfigure(&framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_oidc_provider_for_next_epoch">upsert_oidc_provider_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_oidc_provider_for_next_epoch">upsert_oidc_provider_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, config_url: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>let</b> provider_set = <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()) {
        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()
    } <b>else</b> {
        *<b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework)
    };

    <b>let</b> old_config_url = <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(&<b>mut</b> provider_set, name);
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> provider_set.providers, <a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> { name, config_url });
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(provider_set);
    old_config_url
}
</code></pre>



</details>

<a id="0x1_jwks_remove_oidc_provider"></a>

## Function `remove_oidc_provider`

Deprecated by <code><a href="jwks.md#0x1_jwks_remove_oidc_provider_for_next_epoch">remove_oidc_provider_for_next_epoch</a>()</code>.

TODO: update all the tests that reference this function, then disable this function.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider">remove_oidc_provider</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider">remove_oidc_provider</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();

    <b>let</b> provider_set = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework);
    <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(provider_set, name)
}
</code></pre>



</details>

<a id="0x1_jwks_remove_oidc_provider_for_next_epoch"></a>

## Function `remove_oidc_provider_for_next_epoch`

Used in on-chain governances to update the supported OIDC providers, effective starting next epoch.
Example usage:
```
aptos_framework::jwks::remove_oidc_provider_for_next_epoch(
&framework_signer,
b"https://accounts.google.com",
);
aptos_framework::aptos_governance::reconfigure(&framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider_for_next_epoch">remove_oidc_provider_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider_for_next_epoch">remove_oidc_provider_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>let</b> provider_set = <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()) {
        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()
    } <b>else</b> {
        *<b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework)
    };
    <b>let</b> ret = <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(&<b>mut</b> provider_set, name);
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(provider_set);
    ret
}
</code></pre>



</details>

<a id="0x1_jwks_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;()) {
        <b>let</b> new_config = <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;();
        <b>if</b> (<b>exists</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework)) {
            *<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;(@aptos_framework) = new_config;
        } <b>else</b> {
            <b>move_to</b>(framework, new_config);
        }
    }
}
</code></pre>



</details>

<a id="0x1_jwks_set_patches"></a>

## Function `set_patches`

Set the <code><a href="jwks.md#0x1_jwks_Patches">Patches</a></code>. Only called in governance proposals.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_set_patches">set_patches</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, patches: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_set_patches">set_patches</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, patches: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_Patch">Patch</a>&gt;) <b>acquires</b> <a href="jwks.md#0x1_jwks_Patches">Patches</a>, <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_Patches">Patches</a>&gt;(@aptos_framework).patches = patches;
    <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>();
}
</code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_all"></a>

## Function `new_patch_remove_all`

Create a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> that removes all entries.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_all">new_patch_remove_all</a>(): <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_all">new_patch_remove_all</a>(): <a href="jwks.md#0x1_jwks_Patch">Patch</a> {
    <a href="jwks.md#0x1_jwks_Patch">Patch</a> {
        variant: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_PatchRemoveAll">PatchRemoveAll</a> {}),
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_issuer"></a>

## Function `new_patch_remove_issuer`

Create a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> that removes the entry of a given issuer, if exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_issuer">new_patch_remove_issuer</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_issuer">new_patch_remove_issuer</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_Patch">Patch</a> {
    <a href="jwks.md#0x1_jwks_Patch">Patch</a> {
        variant: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_PatchRemoveIssuer">PatchRemoveIssuer</a> { issuer }),
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_jwk"></a>

## Function `new_patch_remove_jwk`

Create a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> that removes the entry of a given issuer, if exists.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_jwk">new_patch_remove_jwk</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_remove_jwk">new_patch_remove_jwk</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_Patch">Patch</a> {
    <a href="jwks.md#0x1_jwks_Patch">Patch</a> {
        variant: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_PatchRemoveJWK">PatchRemoveJWK</a> { issuer, jwk_id })
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_patch_upsert_jwk"></a>

## Function `new_patch_upsert_jwk`

Create a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code> that upserts a JWK into an issuer's JWK set.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_upsert_jwk">new_patch_upsert_jwk</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk: <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_patch_upsert_jwk">new_patch_upsert_jwk</a>(issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk: <a href="jwks.md#0x1_jwks_JWK">JWK</a>): <a href="jwks.md#0x1_jwks_Patch">Patch</a> {
    <a href="jwks.md#0x1_jwks_Patch">Patch</a> {
        variant: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_PatchUpsertJWK">PatchUpsertJWK</a> { issuer, jwk })
    }
}
</code></pre>



</details>

<a id="0x1_jwks_new_rsa_jwk"></a>

## Function `new_rsa_jwk`

Create a <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> of variant <code><a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_rsa_jwk">new_rsa_jwk</a>(kid: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>, alg: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>, e: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>, n: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_rsa_jwk">new_rsa_jwk</a>(kid: String, alg: String, e: String, n: String): <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
    <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
        variant: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a> {
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


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_unsupported_jwk">new_unsupported_jwk</a>(id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_new_unsupported_jwk">new_unsupported_jwk</a>(id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
    <a href="jwks.md#0x1_jwks_JWK">JWK</a> {
        variant: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a> { id, payload })
    }
}
</code></pre>



</details>

<a id="0x1_jwks_initialize"></a>

## Function `initialize`

Initialize some JWK resources. Should only be invoked by genesis.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_initialize">initialize</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_initialize">initialize</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <b>move_to</b>(fx, <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a> { providers: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[] });
    <b>move_to</b>(fx, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a> { entries: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[] } });
    <b>move_to</b>(fx, <a href="jwks.md#0x1_jwks_Patches">Patches</a> { patches: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[] });
    <b>move_to</b>(fx, <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a>: <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a> { entries: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[] } });
}
</code></pre>



</details>

<a id="0x1_jwks_remove_oidc_provider_internal"></a>

## Function `remove_oidc_provider_internal`

Helper function that removes an OIDC provider from the <code><a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a></code>.
Returns the old config URL of the provider, if any, as an <code>Option</code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(provider_set: &<b>mut</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">jwks::SupportedOIDCProviders</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_oidc_provider_internal">remove_oidc_provider_internal</a>(provider_set: &<b>mut</b> <a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>, name: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <b>let</b> (name_exists, idx) = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_find">vector::find</a>(&provider_set.providers, |obj| {
        <b>let</b> provider: &<a href="jwks.md#0x1_jwks_OIDCProvider">OIDCProvider</a> = obj;
        provider.name == name
    });

    <b>if</b> (name_exists) {
        <b>let</b> old_provider = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> provider_set.providers, idx);
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(old_provider.config_url)
    } <b>else</b> {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_into_observed_jwks"></a>

## Function `upsert_into_observed_jwks`

Only used by validators to publish their observed JWK update.

NOTE: It is assumed verification has been done to ensure each update is quorum-certified,
and its <code><a href="version.md#0x1_version">version</a></code> equals to the on-chain version + 1.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_observed_jwks">upsert_into_observed_jwks</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, provider_jwks_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_upsert_into_observed_jwks">upsert_into_observed_jwks</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, provider_jwks_vec: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt;) <b>acquires</b> <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>, <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>, <a href="jwks.md#0x1_jwks_Patches">Patches</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <b>let</b> observed_jwks = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>&gt;(@aptos_framework);
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_for_each">vector::for_each</a>(provider_jwks_vec, |obj| {
        <b>let</b> provider_jwks: <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> = obj;
        <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(&<b>mut</b> observed_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, provider_jwks);
    });

    <b>let</b> epoch = <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>();
    emit(<a href="jwks.md#0x1_jwks_ObservedJWKsUpdated">ObservedJWKsUpdated</a> { epoch, <a href="jwks.md#0x1_jwks">jwks</a>: observed_jwks.<a href="jwks.md#0x1_jwks">jwks</a> });
    <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>();
}
</code></pre>



</details>

<a id="0x1_jwks_remove_issuer_from_observed_jwks"></a>

## Function `remove_issuer_from_observed_jwks`

Only used by governance to delete an issuer from <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code>, if it exists.

Return the potentially existing <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> of the given issuer.


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_issuer_from_observed_jwks">remove_issuer_from_observed_jwks</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwks.md#0x1_jwks_remove_issuer_from_observed_jwks">remove_issuer_from_observed_jwks</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt; <b>acquires</b> <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>, <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>, <a href="jwks.md#0x1_jwks_Patches">Patches</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <b>let</b> observed_jwks = <b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>&gt;(@aptos_framework);
    <b>let</b> old_value = <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(&<b>mut</b> observed_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, issuer);

    <b>let</b> epoch = <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>();
    emit(<a href="jwks.md#0x1_jwks_ObservedJWKsUpdated">ObservedJWKsUpdated</a> { epoch, <a href="jwks.md#0x1_jwks">jwks</a>: observed_jwks.<a href="jwks.md#0x1_jwks">jwks</a> });
    <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>();

    old_value
}
</code></pre>



</details>

<a id="0x1_jwks_regenerate_patched_jwks"></a>

## Function `regenerate_patched_jwks`

Regenerate <code><a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a></code> from <code><a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a></code> and <code><a href="jwks.md#0x1_jwks_Patches">Patches</a></code> and save the result.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_regenerate_patched_jwks">regenerate_patched_jwks</a>() <b>acquires</b> <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>, <a href="jwks.md#0x1_jwks_Patches">Patches</a>, <a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a> {
    <b>let</b> <a href="jwks.md#0x1_jwks">jwks</a> = <b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_ObservedJWKs">ObservedJWKs</a>&gt;(@aptos_framework).<a href="jwks.md#0x1_jwks">jwks</a>;
    <b>let</b> patches = <b>borrow_global</b>&lt;<a href="jwks.md#0x1_jwks_Patches">Patches</a>&gt;(@aptos_framework);
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&patches.patches, |obj|{
        <b>let</b> patch: &<a href="jwks.md#0x1_jwks_Patch">Patch</a> = obj;
        <a href="jwks.md#0x1_jwks_apply_patch">apply_patch</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>, *patch);
    });
    *<b>borrow_global_mut</b>&lt;<a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a>&gt;(@aptos_framework) = <a href="jwks.md#0x1_jwks_PatchedJWKs">PatchedJWKs</a> { <a href="jwks.md#0x1_jwks">jwks</a> };
}
</code></pre>



</details>

<a id="0x1_jwks_try_get_jwk_by_issuer"></a>

## Function `try_get_jwk_by_issuer`

Get a JWK by issuer and key ID from an <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>, if it exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_by_issuer">try_get_jwk_by_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a>, issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_by_issuer">try_get_jwk_by_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a>, issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; {
    <b>let</b> (issuer_found, index) = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_find">vector::find</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, |obj| {
        <b>let</b> provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> = obj;
        issuer == provider_jwks.issuer
    });

    <b>if</b> (issuer_found) {
        <a href="jwks.md#0x1_jwks_try_get_jwk_by_id">try_get_jwk_by_id</a>(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, index), jwk_id)
    } <b>else</b> {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_jwks_try_get_jwk_by_id"></a>

## Function `try_get_jwk_by_id`

Get a JWK by key ID from a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>, if it exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_by_id">try_get_jwk_by_id</a>(provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_try_get_jwk_by_id">try_get_jwk_by_id</a>(provider_jwks: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; {
    <b>let</b> (jwk_id_found, index) = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_find">vector::find</a>(&provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, |obj|{
        <b>let</b> jwk: &<a href="jwks.md#0x1_jwks_JWK">JWK</a> = obj;
        jwk_id == <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk)
    });

    <b>if</b> (jwk_id_found) {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(*<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&provider_jwks.<a href="jwks.md#0x1_jwks">jwks</a>, index))
    } <b>else</b> {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_jwks_get_jwk_id"></a>

## Function `get_jwk_id`

Get the ID of a JWK.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk: &<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk: &<a href="jwks.md#0x1_jwks_JWK">JWK</a>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> variant_type_name = *<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&jwk.variant));
    <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_RSA_JWK">0x1::jwks::RSA_JWK</a>") {
        <b>let</b> rsa = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_RSA_JWK">RSA_JWK</a>&gt;(jwk.variant);
        *<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_bytes">string::bytes</a>(&rsa.kid)
    } <b>else</b> <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_UnsupportedJWK">0x1::jwks::UnsupportedJWK</a>") {
        <b>let</b> unsupported = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_UnsupportedJWK">UnsupportedJWK</a>&gt;(jwk.variant);
        unsupported.id
    } <b>else</b> {
        <b>abort</b>(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="jwks.md#0x1_jwks_EUNKNOWN_JWK_VARIANT">EUNKNOWN_JWK_VARIANT</a>))
    }
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_provider_jwks"></a>

## Function `upsert_provider_jwks`

Upsert a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> into an <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>. If this upsert replaced an existing entry, return it.
Maintains the sorted-by-issuer invariant in <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a>, provider_jwks: <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a>, provider_jwks: <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>): Option&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt; {
    // NOTE: Using a linear-time search here because we do not expect too many providers.
    <b>let</b> found = <b>false</b>;
    <b>let</b> index = 0;
    <b>let</b> num_entries = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries);
    <b>while</b> (index &lt; num_entries) {
        <b>let</b> cur_entry = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, index);
        <b>let</b> comparison = compare_u8_vector(provider_jwks.issuer, cur_entry.issuer);
        <b>if</b> (is_greater_than(&comparison)) {
            index = index + 1;
        } <b>else</b> {
            found = is_equal(&comparison);
            <b>break</b>
        }
    };

    // Now <b>if</b> `found == <b>true</b>`, `index` points <b>to</b> the <a href="jwks.md#0x1_jwks_JWK">JWK</a> we want <b>to</b> <b>update</b>/remove; otherwise, `index` points <b>to</b>
    // <b>where</b> we want <b>to</b> insert.
    <b>let</b> ret = <b>if</b> (found) {
        <b>let</b> entry = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index);
        <b>let</b> old_entry = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(*entry);
        *entry = provider_jwks;
        old_entry
    } <b>else</b> {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_insert">vector::insert</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index, provider_jwks);
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>()
    };

    ret
}
</code></pre>



</details>

<a id="0x1_jwks_remove_issuer"></a>

## Function `remove_issuer`

Remove the entry of an issuer from a <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code> and return the entry, if exists.
Maintains the sorted-by-issuer invariant in <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a>, issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a>, issuer: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>&gt; {
    <b>let</b> (found, index) = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_find">vector::find</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.entries, |obj| {
        <b>let</b> provider_jwk_set: &<a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> = obj;
        provider_jwk_set.issuer == issuer
    });

    <b>let</b> ret = <b>if</b> (found) {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.entries, index))
    } <b>else</b> {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>()
    };

    ret
}
</code></pre>



</details>

<a id="0x1_jwks_upsert_jwk"></a>

## Function `upsert_jwk`

Upsert a <code><a href="jwks.md#0x1_jwks_JWK">JWK</a></code> into a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code>. If this upsert replaced an existing entry, return it.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_jwk">upsert_jwk</a>(set: &<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk: <a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_upsert_jwk">upsert_jwk</a>(set: &<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk: <a href="jwks.md#0x1_jwks_JWK">JWK</a>): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; {
    <b>let</b> found = <b>false</b>;
    <b>let</b> index = 0;
    <b>let</b> num_entries = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&set.<a href="jwks.md#0x1_jwks">jwks</a>);
    <b>while</b> (index &lt; num_entries) {
        <b>let</b> cur_entry = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&set.<a href="jwks.md#0x1_jwks">jwks</a>, index);
        <b>let</b> comparison = compare_u8_vector(<a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(&jwk), <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(cur_entry));
        <b>if</b> (is_greater_than(&comparison)) {
            index = index + 1;
        } <b>else</b> {
            found = is_equal(&comparison);
            <b>break</b>
        }
    };

    // Now <b>if</b> `found == <b>true</b>`, `index` points <b>to</b> the <a href="jwks.md#0x1_jwks_JWK">JWK</a> we want <b>to</b> <b>update</b>/remove; otherwise, `index` points <b>to</b>
    // <b>where</b> we want <b>to</b> insert.
    <b>let</b> ret = <b>if</b> (found) {
        <b>let</b> entry = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> set.<a href="jwks.md#0x1_jwks">jwks</a>, index);
        <b>let</b> old_entry = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(*entry);
        *entry = jwk;
        old_entry
    } <b>else</b> {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_insert">vector::insert</a>(&<b>mut</b> set.<a href="jwks.md#0x1_jwks">jwks</a>, index, jwk);
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>()
    };

    ret
}
</code></pre>



</details>

<a id="0x1_jwks_remove_jwk"></a>

## Function `remove_jwk`

Remove the entry of a key ID from a <code><a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a></code> and return the entry, if exists.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_jwk">remove_jwk</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">jwks::ProviderJWKs</a>, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="jwks.md#0x1_jwks_JWK">jwks::JWK</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_remove_jwk">remove_jwk</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a>, jwk_id: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="jwks.md#0x1_jwks_JWK">JWK</a>&gt; {
    <b>let</b> (found, index) = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_find">vector::find</a>(&<a href="jwks.md#0x1_jwks">jwks</a>.<a href="jwks.md#0x1_jwks">jwks</a>, |obj| {
        <b>let</b> jwk: &<a href="jwks.md#0x1_jwks_JWK">JWK</a> = obj;
        jwk_id == <a href="jwks.md#0x1_jwks_get_jwk_id">get_jwk_id</a>(jwk)
    });

    <b>let</b> ret = <b>if</b> (found) {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> <a href="jwks.md#0x1_jwks">jwks</a>.<a href="jwks.md#0x1_jwks">jwks</a>, index))
    } <b>else</b> {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>()
    };

    ret
}
</code></pre>



</details>

<a id="0x1_jwks_apply_patch"></a>

## Function `apply_patch`

Modify an <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code> object with a <code><a href="jwks.md#0x1_jwks_Patch">Patch</a></code>.
Maintains the sorted-by-issuer invariant in <code><a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a></code>.


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_apply_patch">apply_patch</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">jwks::AllProvidersJWKs</a>, patch: <a href="jwks.md#0x1_jwks_Patch">jwks::Patch</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="jwks.md#0x1_jwks_apply_patch">apply_patch</a>(<a href="jwks.md#0x1_jwks">jwks</a>: &<b>mut</b> <a href="jwks.md#0x1_jwks_AllProvidersJWKs">AllProvidersJWKs</a>, patch: <a href="jwks.md#0x1_jwks_Patch">Patch</a>) {
    <b>let</b> variant_type_name = *<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&patch.variant));
    <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_PatchRemoveAll">0x1::jwks::PatchRemoveAll</a>") {
        <a href="jwks.md#0x1_jwks">jwks</a>.entries = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[];
    } <b>else</b> <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_PatchRemoveIssuer">0x1::jwks::PatchRemoveIssuer</a>") {
        <b>let</b> cmd = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_PatchRemoveIssuer">PatchRemoveIssuer</a>&gt;(patch.variant);
        <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>, cmd.issuer);
    } <b>else</b> <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_PatchRemoveJWK">0x1::jwks::PatchRemoveJWK</a>") {
        <b>let</b> cmd = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_PatchRemoveJWK">PatchRemoveJWK</a>&gt;(patch.variant);
        // TODO: This is inefficient: we remove the issuer, modify its JWKs & and reinsert the updated issuer. Why
        // not just <b>update</b> it in place?
        <b>let</b> existing_jwk_set = <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>, cmd.issuer);
        <b>if</b> (<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&existing_jwk_set)) {
            <b>let</b> jwk_set = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> existing_jwk_set);
            <a href="jwks.md#0x1_jwks_remove_jwk">remove_jwk</a>(&<b>mut</b> jwk_set, cmd.jwk_id);
            <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, jwk_set);
        };
    } <b>else</b> <b>if</b> (variant_type_name == b"<a href="jwks.md#0x1_jwks_PatchUpsertJWK">0x1::jwks::PatchUpsertJWK</a>") {
        <b>let</b> cmd = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="jwks.md#0x1_jwks_PatchUpsertJWK">PatchUpsertJWK</a>&gt;(patch.variant);
        // TODO: This is inefficient: we remove the issuer, modify its JWKs & and reinsert the updated issuer. Why
        // not just <b>update</b> it in place?
        <b>let</b> existing_jwk_set = <a href="jwks.md#0x1_jwks_remove_issuer">remove_issuer</a>(<a href="jwks.md#0x1_jwks">jwks</a>, cmd.issuer);
        <b>let</b> jwk_set = <b>if</b> (<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&existing_jwk_set)) {
            <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> existing_jwk_set)
        } <b>else</b> {
            <a href="jwks.md#0x1_jwks_ProviderJWKs">ProviderJWKs</a> {
                <a href="version.md#0x1_version">version</a>: 0,
                issuer: cmd.issuer,
                <a href="jwks.md#0x1_jwks">jwks</a>: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[],
            }
        };
        <a href="jwks.md#0x1_jwks_upsert_jwk">upsert_jwk</a>(&<b>mut</b> jwk_set, cmd.jwk);
        <a href="jwks.md#0x1_jwks_upsert_provider_jwks">upsert_provider_jwks</a>(<a href="jwks.md#0x1_jwks">jwks</a>, jwk_set);
    } <b>else</b> {
        <b>abort</b>(std::error::invalid_argument(<a href="jwks.md#0x1_jwks_EUNKNOWN_PATCH_VARIANT">EUNKNOWN_PATCH_VARIANT</a>))
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwks.md#0x1_jwks_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>requires</b> @aptos_framework == std::signer::address_of(framework);
<b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">SupportedOIDCProviders</a>&gt;;
<b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
