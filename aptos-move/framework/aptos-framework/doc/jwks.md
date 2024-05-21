
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


<pre><code>use 0x1::chain_status;
use 0x1::comparator;
use 0x1::config_buffer;
use 0x1::copyable_any;
use 0x1::error;
use 0x1::event;
use 0x1::option;
use 0x1::reconfiguration;
use 0x1::string;
use 0x1::system_addresses;
use 0x1::vector;
</code></pre>



<a id="0x1_jwks_OIDCProvider"></a>

## Struct `OIDCProvider`

An OIDC provider.


<pre><code>struct OIDCProvider has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: vector&lt;u8&gt;</code>
</dt>
<dd>
 The utf-8 encoded issuer string. E.g., b"https://www.facebook.com".
</dd>
<dt>
<code>config_url: vector&lt;u8&gt;</code>
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


<pre><code>struct SupportedOIDCProviders has copy, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>providers: vector&lt;jwks::OIDCProvider&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_UnsupportedJWK"></a>

## Struct `UnsupportedJWK`

An JWK variant that represents the JWKs which were observed but not yet supported by Aptos.
Observing <code>UnsupportedJWK</code>s means the providers adopted a new key type/format, and the system should be updated.


<pre><code>struct UnsupportedJWK has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>payload: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_RSA_JWK"></a>

## Struct `RSA_JWK`

A JWK variant where <code>kty</code> is <code>RSA</code>.


<pre><code>struct RSA_JWK has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>kid: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>kty: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>alg: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>e: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>n: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_JWK"></a>

## Struct `JWK`

A JSON web key.


<pre><code>struct JWK has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: copyable_any::Any</code>
</dt>
<dd>
 A <code>JWK</code> variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 - <code>RSA_JWK</code>
 - <code>UnsupportedJWK</code>
</dd>
</dl>


</details>

<a id="0x1_jwks_ProviderJWKs"></a>

## Struct `ProviderJWKs`

A provider and its <code>JWK</code>s.


<pre><code>struct ProviderJWKs has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: vector&lt;u8&gt;</code>
</dt>
<dd>
 The utf-8 encoding of the issuer string (e.g., "https://www.facebook.com").
</dd>
<dt>
<code>version: u64</code>
</dt>
<dd>
 A version number is needed by JWK consensus to dedup the updates.
 e.g, when on chain version = 5, multiple nodes can propose an update with version = 6.
 Bumped every time the JWKs for the current issuer is updated.
 The Rust authenticator only uses the latest version.
</dd>
<dt>
<code>jwks: vector&lt;jwks::JWK&gt;</code>
</dt>
<dd>
 Vector of <code>JWK</code>'s sorted by their unique ID (from <code>get_jwk_id</code>) in dictionary order.
</dd>
</dl>


</details>

<a id="0x1_jwks_AllProvidersJWKs"></a>

## Struct `AllProvidersJWKs`

Multiple <code>ProviderJWKs</code> objects, indexed by issuer and key ID.


<pre><code>struct AllProvidersJWKs has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: vector&lt;jwks::ProviderJWKs&gt;</code>
</dt>
<dd>
 Vector of <code>ProviderJWKs</code> sorted by <code>ProviderJWKs::issuer</code> in dictionary order.
</dd>
</dl>


</details>

<a id="0x1_jwks_ObservedJWKs"></a>

## Resource `ObservedJWKs`

The <code>AllProvidersJWKs</code> that validators observed and agreed on.


<pre><code>struct ObservedJWKs has copy, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>jwks: jwks::AllProvidersJWKs</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_ObservedJWKsUpdated"></a>

## Struct `ObservedJWKsUpdated`

When <code>ObservedJWKs</code> is updated, this event is sent to resync the JWK consensus state in all validators.


<pre><code>&#35;[event]
struct ObservedJWKsUpdated has drop, store
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
<code>jwks: jwks::AllProvidersJWKs</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_Patch"></a>

## Struct `Patch`

A small edit or patch that is applied to a <code>AllProvidersJWKs</code> to obtain <code>PatchedJWKs</code>.


<pre><code>struct Patch has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: copyable_any::Any</code>
</dt>
<dd>
 A <code>Patch</code> variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 - <code>PatchRemoveAll</code>
 - <code>PatchRemoveIssuer</code>
 - <code>PatchRemoveJWK</code>
 - <code>PatchUpsertJWK</code>
</dd>
</dl>


</details>

<a id="0x1_jwks_PatchRemoveAll"></a>

## Struct `PatchRemoveAll`

A <code>Patch</code> variant to remove all JWKs.


<pre><code>struct PatchRemoveAll has copy, drop, store
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

A <code>Patch</code> variant to remove an issuer and all its JWKs.


<pre><code>struct PatchRemoveIssuer has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_PatchRemoveJWK"></a>

## Struct `PatchRemoveJWK`

A <code>Patch</code> variant to remove a specific JWK of an issuer.


<pre><code>struct PatchRemoveJWK has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>jwk_id: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_PatchUpsertJWK"></a>

## Struct `PatchUpsertJWK`

A <code>Patch</code> variant to upsert a JWK for an issuer.


<pre><code>struct PatchUpsertJWK has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issuer: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>jwk: jwks::JWK</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_Patches"></a>

## Resource `Patches`

A sequence of <code>Patch</code> objects that are applied *one by one* to the <code>ObservedJWKs</code>.

Maintained by governance proposals.


<pre><code>struct Patches has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>patches: vector&lt;jwks::Patch&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwks_PatchedJWKs"></a>

## Resource `PatchedJWKs`

The result of applying the <code>Patches</code> to the <code>ObservedJWKs</code>.
This is what applications should consume.


<pre><code>struct PatchedJWKs has drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>jwks: jwks::AllProvidersJWKs</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_jwks_EISSUER_NOT_FOUND"></a>



<pre><code>const EISSUER_NOT_FOUND: u64 &#61; 5;
</code></pre>



<a id="0x1_jwks_EJWK_ID_NOT_FOUND"></a>



<pre><code>const EJWK_ID_NOT_FOUND: u64 &#61; 6;
</code></pre>



<a id="0x1_jwks_ENATIVE_INCORRECT_VERSION"></a>



<pre><code>const ENATIVE_INCORRECT_VERSION: u64 &#61; 259;
</code></pre>



<a id="0x1_jwks_ENATIVE_MISSING_RESOURCE_OBSERVED_JWKS"></a>



<pre><code>const ENATIVE_MISSING_RESOURCE_OBSERVED_JWKS: u64 &#61; 258;
</code></pre>



<a id="0x1_jwks_ENATIVE_MISSING_RESOURCE_VALIDATOR_SET"></a>



<pre><code>const ENATIVE_MISSING_RESOURCE_VALIDATOR_SET: u64 &#61; 257;
</code></pre>



<a id="0x1_jwks_ENATIVE_MULTISIG_VERIFICATION_FAILED"></a>



<pre><code>const ENATIVE_MULTISIG_VERIFICATION_FAILED: u64 &#61; 260;
</code></pre>



<a id="0x1_jwks_ENATIVE_NOT_ENOUGH_VOTING_POWER"></a>



<pre><code>const ENATIVE_NOT_ENOUGH_VOTING_POWER: u64 &#61; 261;
</code></pre>



<a id="0x1_jwks_EUNEXPECTED_EPOCH"></a>



<pre><code>const EUNEXPECTED_EPOCH: u64 &#61; 1;
</code></pre>



<a id="0x1_jwks_EUNEXPECTED_VERSION"></a>



<pre><code>const EUNEXPECTED_VERSION: u64 &#61; 2;
</code></pre>



<a id="0x1_jwks_EUNKNOWN_JWK_VARIANT"></a>



<pre><code>const EUNKNOWN_JWK_VARIANT: u64 &#61; 4;
</code></pre>



<a id="0x1_jwks_EUNKNOWN_PATCH_VARIANT"></a>



<pre><code>const EUNKNOWN_PATCH_VARIANT: u64 &#61; 3;
</code></pre>



<a id="0x1_jwks_get_patched_jwk"></a>

## Function `get_patched_jwk`

Get a JWK by issuer and key ID from the <code>PatchedJWKs</code>.
Abort if such a JWK does not exist.
More convenient to call from Rust, since it does not wrap the JWK in an <code>Option</code>.


<pre><code>public fun get_patched_jwk(issuer: vector&lt;u8&gt;, jwk_id: vector&lt;u8&gt;): jwks::JWK
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_patched_jwk(issuer: vector&lt;u8&gt;, jwk_id: vector&lt;u8&gt;): JWK acquires PatchedJWKs &#123;
    option::extract(&amp;mut try_get_patched_jwk(issuer, jwk_id))
&#125;
</code></pre>



</details>

<a id="0x1_jwks_try_get_patched_jwk"></a>

## Function `try_get_patched_jwk`

Get a JWK by issuer and key ID from the <code>PatchedJWKs</code>, if it exists.
More convenient to call from Move, since it does not abort.


<pre><code>public fun try_get_patched_jwk(issuer: vector&lt;u8&gt;, jwk_id: vector&lt;u8&gt;): option::Option&lt;jwks::JWK&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun try_get_patched_jwk(issuer: vector&lt;u8&gt;, jwk_id: vector&lt;u8&gt;): Option&lt;JWK&gt; acquires PatchedJWKs &#123;
    let jwks &#61; &amp;borrow_global&lt;PatchedJWKs&gt;(@aptos_framework).jwks;
    try_get_jwk_by_issuer(jwks, issuer, jwk_id)
&#125;
</code></pre>



</details>

<a id="0x1_jwks_upsert_oidc_provider"></a>

## Function `upsert_oidc_provider`

Deprecated by <code>upsert_oidc_provider_for_next_epoch()</code>.

TODO: update all the tests that reference this function, then disable this function.


<pre><code>public fun upsert_oidc_provider(fx: &amp;signer, name: vector&lt;u8&gt;, config_url: vector&lt;u8&gt;): option::Option&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert_oidc_provider(fx: &amp;signer, name: vector&lt;u8&gt;, config_url: vector&lt;u8&gt;): Option&lt;vector&lt;u8&gt;&gt; acquires SupportedOIDCProviders &#123;
    system_addresses::assert_aptos_framework(fx);
    chain_status::assert_genesis();

    let provider_set &#61; borrow_global_mut&lt;SupportedOIDCProviders&gt;(@aptos_framework);

    let old_config_url&#61; remove_oidc_provider_internal(provider_set, name);
    vector::push_back(&amp;mut provider_set.providers, OIDCProvider &#123; name, config_url &#125;);
    old_config_url
&#125;
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


<pre><code>public fun upsert_oidc_provider_for_next_epoch(fx: &amp;signer, name: vector&lt;u8&gt;, config_url: vector&lt;u8&gt;): option::Option&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert_oidc_provider_for_next_epoch(fx: &amp;signer, name: vector&lt;u8&gt;, config_url: vector&lt;u8&gt;): Option&lt;vector&lt;u8&gt;&gt; acquires SupportedOIDCProviders &#123;
    system_addresses::assert_aptos_framework(fx);

    let provider_set &#61; if (config_buffer::does_exist&lt;SupportedOIDCProviders&gt;()) &#123;
        config_buffer::extract&lt;SupportedOIDCProviders&gt;()
    &#125; else &#123;
        &#42;borrow_global_mut&lt;SupportedOIDCProviders&gt;(@aptos_framework)
    &#125;;

    let old_config_url &#61; remove_oidc_provider_internal(&amp;mut provider_set, name);
    vector::push_back(&amp;mut provider_set.providers, OIDCProvider &#123; name, config_url &#125;);
    config_buffer::upsert(provider_set);
    old_config_url
&#125;
</code></pre>



</details>

<a id="0x1_jwks_remove_oidc_provider"></a>

## Function `remove_oidc_provider`

Deprecated by <code>remove_oidc_provider_for_next_epoch()</code>.

TODO: update all the tests that reference this function, then disable this function.


<pre><code>public fun remove_oidc_provider(fx: &amp;signer, name: vector&lt;u8&gt;): option::Option&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_oidc_provider(fx: &amp;signer, name: vector&lt;u8&gt;): Option&lt;vector&lt;u8&gt;&gt; acquires SupportedOIDCProviders &#123;
    system_addresses::assert_aptos_framework(fx);
    chain_status::assert_genesis();

    let provider_set &#61; borrow_global_mut&lt;SupportedOIDCProviders&gt;(@aptos_framework);
    remove_oidc_provider_internal(provider_set, name)
&#125;
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


<pre><code>public fun remove_oidc_provider_for_next_epoch(fx: &amp;signer, name: vector&lt;u8&gt;): option::Option&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_oidc_provider_for_next_epoch(fx: &amp;signer, name: vector&lt;u8&gt;): Option&lt;vector&lt;u8&gt;&gt; acquires SupportedOIDCProviders &#123;
    system_addresses::assert_aptos_framework(fx);

    let provider_set &#61; if (config_buffer::does_exist&lt;SupportedOIDCProviders&gt;()) &#123;
        config_buffer::extract&lt;SupportedOIDCProviders&gt;()
    &#125; else &#123;
        &#42;borrow_global_mut&lt;SupportedOIDCProviders&gt;(@aptos_framework)
    &#125;;
    let ret &#61; remove_oidc_provider_internal(&amp;mut provider_set, name);
    config_buffer::upsert(provider_set);
    ret
&#125;
</code></pre>



</details>

<a id="0x1_jwks_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>SupportedOIDCProviders</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires SupportedOIDCProviders &#123;
    system_addresses::assert_aptos_framework(framework);
    if (config_buffer::does_exist&lt;SupportedOIDCProviders&gt;()) &#123;
        let new_config &#61; config_buffer::extract&lt;SupportedOIDCProviders&gt;();
        if (exists&lt;SupportedOIDCProviders&gt;(@aptos_framework)) &#123;
            &#42;borrow_global_mut&lt;SupportedOIDCProviders&gt;(@aptos_framework) &#61; new_config;
        &#125; else &#123;
            move_to(framework, new_config);
        &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_set_patches"></a>

## Function `set_patches`

Set the <code>Patches</code>. Only called in governance proposals.


<pre><code>public fun set_patches(fx: &amp;signer, patches: vector&lt;jwks::Patch&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_patches(fx: &amp;signer, patches: vector&lt;Patch&gt;) acquires Patches, PatchedJWKs, ObservedJWKs &#123;
    system_addresses::assert_aptos_framework(fx);
    borrow_global_mut&lt;Patches&gt;(@aptos_framework).patches &#61; patches;
    regenerate_patched_jwks();
&#125;
</code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_all"></a>

## Function `new_patch_remove_all`

Create a <code>Patch</code> that removes all entries.


<pre><code>public fun new_patch_remove_all(): jwks::Patch
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_patch_remove_all(): Patch &#123;
    Patch &#123;
        variant: copyable_any::pack(PatchRemoveAll &#123;&#125;),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_issuer"></a>

## Function `new_patch_remove_issuer`

Create a <code>Patch</code> that removes the entry of a given issuer, if exists.


<pre><code>public fun new_patch_remove_issuer(issuer: vector&lt;u8&gt;): jwks::Patch
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_patch_remove_issuer(issuer: vector&lt;u8&gt;): Patch &#123;
    Patch &#123;
        variant: copyable_any::pack(PatchRemoveIssuer &#123; issuer &#125;),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_new_patch_remove_jwk"></a>

## Function `new_patch_remove_jwk`

Create a <code>Patch</code> that removes the entry of a given issuer, if exists.


<pre><code>public fun new_patch_remove_jwk(issuer: vector&lt;u8&gt;, jwk_id: vector&lt;u8&gt;): jwks::Patch
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_patch_remove_jwk(issuer: vector&lt;u8&gt;, jwk_id: vector&lt;u8&gt;): Patch &#123;
    Patch &#123;
        variant: copyable_any::pack(PatchRemoveJWK &#123; issuer, jwk_id &#125;)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_new_patch_upsert_jwk"></a>

## Function `new_patch_upsert_jwk`

Create a <code>Patch</code> that upserts a JWK into an issuer's JWK set.


<pre><code>public fun new_patch_upsert_jwk(issuer: vector&lt;u8&gt;, jwk: jwks::JWK): jwks::Patch
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_patch_upsert_jwk(issuer: vector&lt;u8&gt;, jwk: JWK): Patch &#123;
    Patch &#123;
        variant: copyable_any::pack(PatchUpsertJWK &#123; issuer, jwk &#125;)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_new_rsa_jwk"></a>

## Function `new_rsa_jwk`

Create a <code>JWK</code> of variant <code>RSA_JWK</code>.


<pre><code>public fun new_rsa_jwk(kid: string::String, alg: string::String, e: string::String, n: string::String): jwks::JWK
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_rsa_jwk(kid: String, alg: String, e: String, n: String): JWK &#123;
    JWK &#123;
        variant: copyable_any::pack(RSA_JWK &#123;
            kid,
            kty: utf8(b&quot;RSA&quot;),
            e,
            n,
            alg,
        &#125;),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_new_unsupported_jwk"></a>

## Function `new_unsupported_jwk`

Create a <code>JWK</code> of variant <code>UnsupportedJWK</code>.


<pre><code>public fun new_unsupported_jwk(id: vector&lt;u8&gt;, payload: vector&lt;u8&gt;): jwks::JWK
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_unsupported_jwk(id: vector&lt;u8&gt;, payload: vector&lt;u8&gt;): JWK &#123;
    JWK &#123;
        variant: copyable_any::pack(UnsupportedJWK &#123; id, payload &#125;)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_initialize"></a>

## Function `initialize`

Initialize some JWK resources. Should only be invoked by genesis.


<pre><code>public fun initialize(fx: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(fx: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(fx);
    move_to(fx, SupportedOIDCProviders &#123; providers: vector[] &#125;);
    move_to(fx, ObservedJWKs &#123; jwks: AllProvidersJWKs &#123; entries: vector[] &#125; &#125;);
    move_to(fx, Patches &#123; patches: vector[] &#125;);
    move_to(fx, PatchedJWKs &#123; jwks: AllProvidersJWKs &#123; entries: vector[] &#125; &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_jwks_remove_oidc_provider_internal"></a>

## Function `remove_oidc_provider_internal`

Helper function that removes an OIDC provider from the <code>SupportedOIDCProviders</code>.
Returns the old config URL of the provider, if any, as an <code>Option</code>.


<pre><code>fun remove_oidc_provider_internal(provider_set: &amp;mut jwks::SupportedOIDCProviders, name: vector&lt;u8&gt;): option::Option&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun remove_oidc_provider_internal(provider_set: &amp;mut SupportedOIDCProviders, name: vector&lt;u8&gt;): Option&lt;vector&lt;u8&gt;&gt; &#123;
    let (name_exists, idx) &#61; vector::find(&amp;provider_set.providers, &#124;obj&#124; &#123;
        let provider: &amp;OIDCProvider &#61; obj;
        provider.name &#61;&#61; name
    &#125;);

    if (name_exists) &#123;
        let old_provider &#61; vector::swap_remove(&amp;mut provider_set.providers, idx);
        option::some(old_provider.config_url)
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_upsert_into_observed_jwks"></a>

## Function `upsert_into_observed_jwks`

Only used by validators to publish their observed JWK update.

NOTE: It is assumed verification has been done to ensure each update is quorum-certified,
and its <code>version</code> equals to the on-chain version + 1.


<pre><code>public fun upsert_into_observed_jwks(fx: &amp;signer, provider_jwks_vec: vector&lt;jwks::ProviderJWKs&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert_into_observed_jwks(fx: &amp;signer, provider_jwks_vec: vector&lt;ProviderJWKs&gt;) acquires ObservedJWKs, PatchedJWKs, Patches &#123;
    system_addresses::assert_aptos_framework(fx);
    let observed_jwks &#61; borrow_global_mut&lt;ObservedJWKs&gt;(@aptos_framework);
    vector::for_each(provider_jwks_vec, &#124;obj&#124; &#123;
        let provider_jwks: ProviderJWKs &#61; obj;
        upsert_provider_jwks(&amp;mut observed_jwks.jwks, provider_jwks);
    &#125;);

    let epoch &#61; reconfiguration::current_epoch();
    emit(ObservedJWKsUpdated &#123; epoch, jwks: observed_jwks.jwks &#125;);
    regenerate_patched_jwks();
&#125;
</code></pre>



</details>

<a id="0x1_jwks_remove_issuer_from_observed_jwks"></a>

## Function `remove_issuer_from_observed_jwks`

Only used by governance to delete an issuer from <code>ObservedJWKs</code>, if it exists.

Return the potentially existing <code>ProviderJWKs</code> of the given issuer.


<pre><code>public fun remove_issuer_from_observed_jwks(fx: &amp;signer, issuer: vector&lt;u8&gt;): option::Option&lt;jwks::ProviderJWKs&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_issuer_from_observed_jwks(fx: &amp;signer, issuer: vector&lt;u8&gt;): Option&lt;ProviderJWKs&gt; acquires ObservedJWKs, PatchedJWKs, Patches &#123;
    system_addresses::assert_aptos_framework(fx);
    let observed_jwks &#61; borrow_global_mut&lt;ObservedJWKs&gt;(@aptos_framework);
    let old_value &#61; remove_issuer(&amp;mut observed_jwks.jwks, issuer);

    let epoch &#61; reconfiguration::current_epoch();
    emit(ObservedJWKsUpdated &#123; epoch, jwks: observed_jwks.jwks &#125;);
    regenerate_patched_jwks();

    old_value
&#125;
</code></pre>



</details>

<a id="0x1_jwks_regenerate_patched_jwks"></a>

## Function `regenerate_patched_jwks`

Regenerate <code>PatchedJWKs</code> from <code>ObservedJWKs</code> and <code>Patches</code> and save the result.


<pre><code>fun regenerate_patched_jwks()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun regenerate_patched_jwks() acquires PatchedJWKs, Patches, ObservedJWKs &#123;
    let jwks &#61; borrow_global&lt;ObservedJWKs&gt;(@aptos_framework).jwks;
    let patches &#61; borrow_global&lt;Patches&gt;(@aptos_framework);
    vector::for_each_ref(&amp;patches.patches, &#124;obj&#124;&#123;
        let patch: &amp;Patch &#61; obj;
        apply_patch(&amp;mut jwks, &#42;patch);
    &#125;);
    &#42;borrow_global_mut&lt;PatchedJWKs&gt;(@aptos_framework) &#61; PatchedJWKs &#123; jwks &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_try_get_jwk_by_issuer"></a>

## Function `try_get_jwk_by_issuer`

Get a JWK by issuer and key ID from a <code>AllProvidersJWKs</code>, if it exists.


<pre><code>fun try_get_jwk_by_issuer(jwks: &amp;jwks::AllProvidersJWKs, issuer: vector&lt;u8&gt;, jwk_id: vector&lt;u8&gt;): option::Option&lt;jwks::JWK&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun try_get_jwk_by_issuer(jwks: &amp;AllProvidersJWKs, issuer: vector&lt;u8&gt;, jwk_id: vector&lt;u8&gt;): Option&lt;JWK&gt; &#123;
    let (issuer_found, index) &#61; vector::find(&amp;jwks.entries, &#124;obj&#124; &#123;
        let provider_jwks: &amp;ProviderJWKs &#61; obj;
        issuer &#61;&#61; provider_jwks.issuer
    &#125;);

    if (issuer_found) &#123;
        try_get_jwk_by_id(vector::borrow(&amp;jwks.entries, index), jwk_id)
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_try_get_jwk_by_id"></a>

## Function `try_get_jwk_by_id`

Get a JWK by key ID from a <code>ProviderJWKs</code>, if it exists.


<pre><code>fun try_get_jwk_by_id(provider_jwks: &amp;jwks::ProviderJWKs, jwk_id: vector&lt;u8&gt;): option::Option&lt;jwks::JWK&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun try_get_jwk_by_id(provider_jwks: &amp;ProviderJWKs, jwk_id: vector&lt;u8&gt;): Option&lt;JWK&gt; &#123;
    let (jwk_id_found, index) &#61; vector::find(&amp;provider_jwks.jwks, &#124;obj&#124;&#123;
        let jwk: &amp;JWK &#61; obj;
        jwk_id &#61;&#61; get_jwk_id(jwk)
    &#125;);

    if (jwk_id_found) &#123;
        option::some(&#42;vector::borrow(&amp;provider_jwks.jwks, index))
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_get_jwk_id"></a>

## Function `get_jwk_id`

Get the ID of a JWK.


<pre><code>fun get_jwk_id(jwk: &amp;jwks::JWK): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_jwk_id(jwk: &amp;JWK): vector&lt;u8&gt; &#123;
    let variant_type_name &#61; &#42;string::bytes(copyable_any::type_name(&amp;jwk.variant));
    if (variant_type_name &#61;&#61; b&quot;0x1::jwks::RSA_JWK&quot;) &#123;
        let rsa &#61; copyable_any::unpack&lt;RSA_JWK&gt;(jwk.variant);
        &#42;string::bytes(&amp;rsa.kid)
    &#125; else if (variant_type_name &#61;&#61; b&quot;0x1::jwks::UnsupportedJWK&quot;) &#123;
        let unsupported &#61; copyable_any::unpack&lt;UnsupportedJWK&gt;(jwk.variant);
        unsupported.id
    &#125; else &#123;
        abort(error::invalid_argument(EUNKNOWN_JWK_VARIANT))
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_jwks_upsert_provider_jwks"></a>

## Function `upsert_provider_jwks`

Upsert a <code>ProviderJWKs</code> into an <code>AllProvidersJWKs</code>. If this upsert replaced an existing entry, return it.
Maintains the sorted-by-issuer invariant in <code>AllProvidersJWKs</code>.


<pre><code>fun upsert_provider_jwks(jwks: &amp;mut jwks::AllProvidersJWKs, provider_jwks: jwks::ProviderJWKs): option::Option&lt;jwks::ProviderJWKs&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun upsert_provider_jwks(jwks: &amp;mut AllProvidersJWKs, provider_jwks: ProviderJWKs): Option&lt;ProviderJWKs&gt; &#123;
    // NOTE: Using a linear&#45;time search here because we do not expect too many providers.
    let found &#61; false;
    let index &#61; 0;
    let num_entries &#61; vector::length(&amp;jwks.entries);
    while (index &lt; num_entries) &#123;
        let cur_entry &#61; vector::borrow(&amp;jwks.entries, index);
        let comparison &#61; compare_u8_vector(provider_jwks.issuer, cur_entry.issuer);
        if (is_greater_than(&amp;comparison)) &#123;
            index &#61; index &#43; 1;
        &#125; else &#123;
            found &#61; is_equal(&amp;comparison);
            break
        &#125;
    &#125;;

    // Now if `found &#61;&#61; true`, `index` points to the JWK we want to update/remove; otherwise, `index` points to
    // where we want to insert.
    let ret &#61; if (found) &#123;
        let entry &#61; vector::borrow_mut(&amp;mut jwks.entries, index);
        let old_entry &#61; option::some(&#42;entry);
        &#42;entry &#61; provider_jwks;
        old_entry
    &#125; else &#123;
        vector::insert(&amp;mut jwks.entries, index, provider_jwks);
        option::none()
    &#125;;

    ret
&#125;
</code></pre>



</details>

<a id="0x1_jwks_remove_issuer"></a>

## Function `remove_issuer`

Remove the entry of an issuer from a <code>AllProvidersJWKs</code> and return the entry, if exists.
Maintains the sorted-by-issuer invariant in <code>AllProvidersJWKs</code>.


<pre><code>fun remove_issuer(jwks: &amp;mut jwks::AllProvidersJWKs, issuer: vector&lt;u8&gt;): option::Option&lt;jwks::ProviderJWKs&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun remove_issuer(jwks: &amp;mut AllProvidersJWKs, issuer: vector&lt;u8&gt;): Option&lt;ProviderJWKs&gt; &#123;
    let (found, index) &#61; vector::find(&amp;jwks.entries, &#124;obj&#124; &#123;
        let provider_jwk_set: &amp;ProviderJWKs &#61; obj;
        provider_jwk_set.issuer &#61;&#61; issuer
    &#125;);

    let ret &#61; if (found) &#123;
        option::some(vector::remove(&amp;mut jwks.entries, index))
    &#125; else &#123;
        option::none()
    &#125;;

    ret
&#125;
</code></pre>



</details>

<a id="0x1_jwks_upsert_jwk"></a>

## Function `upsert_jwk`

Upsert a <code>JWK</code> into a <code>ProviderJWKs</code>. If this upsert replaced an existing entry, return it.


<pre><code>fun upsert_jwk(set: &amp;mut jwks::ProviderJWKs, jwk: jwks::JWK): option::Option&lt;jwks::JWK&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun upsert_jwk(set: &amp;mut ProviderJWKs, jwk: JWK): Option&lt;JWK&gt; &#123;
    let found &#61; false;
    let index &#61; 0;
    let num_entries &#61; vector::length(&amp;set.jwks);
    while (index &lt; num_entries) &#123;
        let cur_entry &#61; vector::borrow(&amp;set.jwks, index);
        let comparison &#61; compare_u8_vector(get_jwk_id(&amp;jwk), get_jwk_id(cur_entry));
        if (is_greater_than(&amp;comparison)) &#123;
            index &#61; index &#43; 1;
        &#125; else &#123;
            found &#61; is_equal(&amp;comparison);
            break
        &#125;
    &#125;;

    // Now if `found &#61;&#61; true`, `index` points to the JWK we want to update/remove; otherwise, `index` points to
    // where we want to insert.
    let ret &#61; if (found) &#123;
        let entry &#61; vector::borrow_mut(&amp;mut set.jwks, index);
        let old_entry &#61; option::some(&#42;entry);
        &#42;entry &#61; jwk;
        old_entry
    &#125; else &#123;
        vector::insert(&amp;mut set.jwks, index, jwk);
        option::none()
    &#125;;

    ret
&#125;
</code></pre>



</details>

<a id="0x1_jwks_remove_jwk"></a>

## Function `remove_jwk`

Remove the entry of a key ID from a <code>ProviderJWKs</code> and return the entry, if exists.


<pre><code>fun remove_jwk(jwks: &amp;mut jwks::ProviderJWKs, jwk_id: vector&lt;u8&gt;): option::Option&lt;jwks::JWK&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun remove_jwk(jwks: &amp;mut ProviderJWKs, jwk_id: vector&lt;u8&gt;): Option&lt;JWK&gt; &#123;
    let (found, index) &#61; vector::find(&amp;jwks.jwks, &#124;obj&#124; &#123;
        let jwk: &amp;JWK &#61; obj;
        jwk_id &#61;&#61; get_jwk_id(jwk)
    &#125;);

    let ret &#61; if (found) &#123;
        option::some(vector::remove(&amp;mut jwks.jwks, index))
    &#125; else &#123;
        option::none()
    &#125;;

    ret
&#125;
</code></pre>



</details>

<a id="0x1_jwks_apply_patch"></a>

## Function `apply_patch`

Modify an <code>AllProvidersJWKs</code> object with a <code>Patch</code>.
Maintains the sorted-by-issuer invariant in <code>AllProvidersJWKs</code>.


<pre><code>fun apply_patch(jwks: &amp;mut jwks::AllProvidersJWKs, patch: jwks::Patch)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun apply_patch(jwks: &amp;mut AllProvidersJWKs, patch: Patch) &#123;
    let variant_type_name &#61; &#42;string::bytes(copyable_any::type_name(&amp;patch.variant));
    if (variant_type_name &#61;&#61; b&quot;0x1::jwks::PatchRemoveAll&quot;) &#123;
        jwks.entries &#61; vector[];
    &#125; else if (variant_type_name &#61;&#61; b&quot;0x1::jwks::PatchRemoveIssuer&quot;) &#123;
        let cmd &#61; copyable_any::unpack&lt;PatchRemoveIssuer&gt;(patch.variant);
        remove_issuer(jwks, cmd.issuer);
    &#125; else if (variant_type_name &#61;&#61; b&quot;0x1::jwks::PatchRemoveJWK&quot;) &#123;
        let cmd &#61; copyable_any::unpack&lt;PatchRemoveJWK&gt;(patch.variant);
        // TODO: This is inefficient: we remove the issuer, modify its JWKs &amp; and reinsert the updated issuer. Why
        // not just update it in place?
        let existing_jwk_set &#61; remove_issuer(jwks, cmd.issuer);
        if (option::is_some(&amp;existing_jwk_set)) &#123;
            let jwk_set &#61; option::extract(&amp;mut existing_jwk_set);
            remove_jwk(&amp;mut jwk_set, cmd.jwk_id);
            upsert_provider_jwks(jwks, jwk_set);
        &#125;;
    &#125; else if (variant_type_name &#61;&#61; b&quot;0x1::jwks::PatchUpsertJWK&quot;) &#123;
        let cmd &#61; copyable_any::unpack&lt;PatchUpsertJWK&gt;(patch.variant);
        // TODO: This is inefficient: we remove the issuer, modify its JWKs &amp; and reinsert the updated issuer. Why
        // not just update it in place?
        let existing_jwk_set &#61; remove_issuer(jwks, cmd.issuer);
        let jwk_set &#61; if (option::is_some(&amp;existing_jwk_set)) &#123;
            option::extract(&amp;mut existing_jwk_set)
        &#125; else &#123;
            ProviderJWKs &#123;
                version: 0,
                issuer: cmd.issuer,
                jwks: vector[],
            &#125;
        &#125;;
        upsert_jwk(&amp;mut jwk_set, cmd.jwk);
        upsert_provider_jwks(jwks, jwk_set);
    &#125; else &#123;
        abort(std::error::invalid_argument(EUNKNOWN_PATCH_VARIANT))
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
include config_buffer::OnNewEpochRequirement&lt;SupportedOIDCProviders&gt;;
aborts_if false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
