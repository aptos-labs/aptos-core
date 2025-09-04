
<a id="0x1_permissioned_delegation"></a>

# Module `0x1::permissioned_delegation`



-  [Enum `AccountDelegation`](#0x1_permissioned_delegation_AccountDelegation)
-  [Enum `DelegationKey`](#0x1_permissioned_delegation_DelegationKey)
-  [Resource `RegisteredDelegations`](#0x1_permissioned_delegation_RegisteredDelegations)
-  [Constants](#@Constants_0)
-  [Function `gen_ed25519_key`](#0x1_permissioned_delegation_gen_ed25519_key)
-  [Function `check_txn_rate`](#0x1_permissioned_delegation_check_txn_rate)
-  [Function `add_permissioned_handle`](#0x1_permissioned_delegation_add_permissioned_handle)
-  [Function `remove_permissioned_handle`](#0x1_permissioned_delegation_remove_permissioned_handle)
-  [Function `permissioned_signer_by_key`](#0x1_permissioned_delegation_permissioned_signer_by_key)
-  [Function `handle_address_by_key`](#0x1_permissioned_delegation_handle_address_by_key)
-  [Function `authenticate`](#0x1_permissioned_delegation_authenticate)
-  [Function `get_storable_permissioned_handle`](#0x1_permissioned_delegation_get_storable_permissioned_handle)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="../../velor-stdlib/doc/bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../velor-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="rate_limiter.md#0x1_rate_limiter">0x1::rate_limiter</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_permissioned_delegation_AccountDelegation"></a>

## Enum `AccountDelegation`



<pre><code>enum <a href="permissioned_delegation.md#0x1_permissioned_delegation_AccountDelegation">AccountDelegation</a> <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="rate_limiter.md#0x1_rate_limiter">rate_limiter</a>: <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">rate_limiter::RateLimiter</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_delegation_DelegationKey"></a>

## Enum `DelegationKey`



<pre><code>enum <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">DelegationKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Ed25519PublicKey</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: <a href="../../velor-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_delegation_RegisteredDelegations"></a>

## Resource `RegisteredDelegations`



<pre><code><b>struct</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>delegations: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">permissioned_delegation::DelegationKey</a>, <a href="permissioned_delegation.md#0x1_permissioned_delegation_AccountDelegation">permissioned_delegation::AccountDelegation</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_permissioned_delegation_ENOT_MASTER_SIGNER"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>: u64 = 1;
</code></pre>



<a id="0x1_permissioned_delegation_EINVALID_PUBLIC_KEY"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>: u64 = 2;
</code></pre>



<a id="0x1_permissioned_delegation_EINVALID_SIGNATURE"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 4;
</code></pre>



<a id="0x1_permissioned_delegation_EDELEGATION_EXISTENCE"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_EDELEGATION_EXISTENCE">EDELEGATION_EXISTENCE</a>: u64 = 5;
</code></pre>



<a id="0x1_permissioned_delegation_EPUBLIC_KEY_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_EPUBLIC_KEY_NOT_FOUND">EPUBLIC_KEY_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x1_permissioned_delegation_ERATE_LIMITED"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_ERATE_LIMITED">ERATE_LIMITED</a>: u64 = 6;
</code></pre>



<a id="0x1_permissioned_delegation_gen_ed25519_key"></a>

## Function `gen_ed25519_key`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_gen_ed25519_key">gen_ed25519_key</a>(key: <a href="../../velor-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>): <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">permissioned_delegation::DelegationKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_gen_ed25519_key">gen_ed25519_key</a>(key: UnvalidatedPublicKey): <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">DelegationKey</a> {
    DelegationKey::Ed25519PublicKey(key)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_check_txn_rate"></a>

## Function `check_txn_rate`



<pre><code><b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_check_txn_rate">check_txn_rate</a>(bundle: &<b>mut</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_AccountDelegation">permissioned_delegation::AccountDelegation</a>, check_rate_limit: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_check_txn_rate">check_txn_rate</a>(bundle: &<b>mut</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_AccountDelegation">AccountDelegation</a>, check_rate_limit: bool) {
    <b>let</b> token_bucket = &<b>mut</b> bundle.<a href="rate_limiter.md#0x1_rate_limiter">rate_limiter</a>;
    <b>if</b> (check_rate_limit && token_bucket.is_some()) {
        <b>assert</b>!(<a href="rate_limiter.md#0x1_rate_limiter_request">rate_limiter::request</a>(token_bucket.borrow_mut(), 1), std::error::permission_denied(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ERATE_LIMITED">ERATE_LIMITED</a>));
    };
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_add_permissioned_handle"></a>

## Function `add_permissioned_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_add_permissioned_handle">add_permissioned_handle</a>(master: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">permissioned_delegation::DelegationKey</a>, <a href="rate_limiter.md#0x1_rate_limiter">rate_limiter</a>: <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">rate_limiter::RateLimiter</a>&gt;, expiration_time: u64): <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_add_permissioned_handle">add_permissioned_handle</a>(
    master: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">DelegationKey</a>,
    <a href="rate_limiter.md#0x1_rate_limiter">rate_limiter</a>: Option&lt;RateLimiter&gt;,
    expiration_time: u64,
): <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a> {
    <b>assert</b>!(!is_permissioned_signer(master), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a>&gt;(addr)) {
        <b>move_to</b>(master, <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a> {
            delegations: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_config">big_ordered_map::new_with_config</a>(50, 20, <b>false</b>)
        });
    };
    <b>let</b> handles = &<b>mut</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a>[addr].delegations;
    <b>assert</b>!(!handles.contains(&key), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EDELEGATION_EXISTENCE">EDELEGATION_EXISTENCE</a>));
    <b>let</b> handle = <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">permissioned_signer::create_storable_permissioned_handle</a>(master, expiration_time);
    <b>let</b> <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a> = <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(&handle);
    handles.add(key, AccountDelegation::V1 { handle, <a href="rate_limiter.md#0x1_rate_limiter">rate_limiter</a> });
    <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_remove_permissioned_handle"></a>

## Function `remove_permissioned_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_remove_permissioned_handle">remove_permissioned_handle</a>(master: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">permissioned_delegation::DelegationKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_remove_permissioned_handle">remove_permissioned_handle</a>(
    master: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">DelegationKey</a>,
) <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a> {
    <b>assert</b>!(!is_permissioned_signer(master), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
    <b>let</b> delegations = &<b>mut</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a>[addr].delegations;
    <b>assert</b>!(delegations.contains(&key), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EDELEGATION_EXISTENCE">EDELEGATION_EXISTENCE</a>));
    <b>let</b> delegation = delegations.remove(&key);
    match (delegation) {
        AccountDelegation::V1 { handle, <a href="rate_limiter.md#0x1_rate_limiter">rate_limiter</a>: _ } =&gt; {
            <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_storable_permissioned_handle">permissioned_signer::destroy_storable_permissioned_handle</a>(handle);
        }
    };
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_permissioned_signer_by_key"></a>

## Function `permissioned_signer_by_key`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_permissioned_signer_by_key">permissioned_signer_by_key</a>(master: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">permissioned_delegation::DelegationKey</a>): <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_permissioned_signer_by_key">permissioned_signer_by_key</a>(
    master: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">DelegationKey</a>,
): <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a> {
    <b>assert</b>!(!is_permissioned_signer(master), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
    <b>let</b> handle = <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(addr, key, <b>false</b>);
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(handle)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_handle_address_by_key"></a>

## Function `handle_address_by_key`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_handle_address_by_key">handle_address_by_key</a>(master: <b>address</b>, key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">permissioned_delegation::DelegationKey</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_handle_address_by_key">handle_address_by_key</a>(master: <b>address</b>, key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">DelegationKey</a>): <b>address</b> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a> {
    <b>let</b> handle = <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(master, key, <b>false</b>);
    <a href="permissioned_signer.md#0x1_permissioned_signer_permissions_storage_address">permissioned_signer::permissions_storage_address</a>(handle)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_authenticate"></a>

## Function `authenticate`

Authorization function for account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, abstraction_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_authenticate">authenticate</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    abstraction_auth_data: AbstractionAuthData
): <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a> {
    <b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <b>let</b> stream = <a href="../../velor-stdlib/doc/bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*<a href="auth_data.md#0x1_auth_data_authenticator">auth_data::authenticator</a>(&abstraction_auth_data));
    <b>let</b> public_key = new_unvalidated_public_key_from_bytes(
        <a href="../../velor-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x))
    );
    <b>let</b> signature = new_signature_from_bytes(
        <a href="../../velor-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x))
    );
    <b>assert</b>!(
        <a href="../../velor-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
            &signature,
            &public_key,
            *<a href="auth_data.md#0x1_auth_data_digest">auth_data::digest</a>(&abstraction_auth_data),
        ),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    );
    <b>let</b> handle = <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(addr, DelegationKey::Ed25519PublicKey(public_key), <b>true</b>);
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(handle)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_get_storable_permissioned_handle"></a>

## Function `get_storable_permissioned_handle`



<pre><code><b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(master: <b>address</b>, key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">permissioned_delegation::DelegationKey</a>, count_rate: bool): &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(
    master: <b>address</b>,
    key: <a href="permissioned_delegation.md#0x1_permissioned_delegation_DelegationKey">DelegationKey</a>,
    count_rate: bool
): &StorablePermissionedHandle {
    <b>if</b> (<b>exists</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a>&gt;(master)) {
        <b>let</b> delegations = &<b>mut</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_RegisteredDelegations">RegisteredDelegations</a>[master].delegations;
        <b>if</b> (delegations.contains(&key)) {
            <b>let</b> delegation = delegations.remove(&key);
            <a href="permissioned_delegation.md#0x1_permissioned_delegation_check_txn_rate">check_txn_rate</a>(&<b>mut</b> delegation, count_rate);
            delegations.add(key, delegation);
            &delegations.borrow(&key).handle
        } <b>else</b> {
            <b>abort</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
        }
    } <b>else</b> {
        <b>abort</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
