
<a id="0x1_permissioned_delegation"></a>

# Module `0x1::permissioned_delegation`



-  [Enum `HandleBundle`](#0x1_permissioned_delegation_HandleBundle)
-  [Resource `Delegation`](#0x1_permissioned_delegation_Delegation)
-  [Constants](#@Constants_0)
-  [Function `fetch_handle`](#0x1_permissioned_delegation_fetch_handle)
-  [Function `add_permissioned_handle`](#0x1_permissioned_delegation_add_permissioned_handle)
-  [Function `remove_permissioned_handle`](#0x1_permissioned_delegation_remove_permissioned_handle)
-  [Function `permissioned_signer_by_key`](#0x1_permissioned_delegation_permissioned_signer_by_key)
-  [Function `handle_address_by_key`](#0x1_permissioned_delegation_handle_address_by_key)
-  [Function `authenticate`](#0x1_permissioned_delegation_authenticate)
-  [Function `get_storable_permissioned_handle`](#0x1_permissioned_delegation_get_storable_permissioned_handle)


<pre><code><b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="token_bucket.md#0x1_token_bucket">0x1::token_bucket</a>;
</code></pre>



<a id="0x1_permissioned_delegation_HandleBundle"></a>

## Enum `HandleBundle`



<pre><code>enum <a href="permissioned_delegation.md#0x1_permissioned_delegation_HandleBundle">HandleBundle</a> <b>has</b> store
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
<code>bucket: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="token_bucket.md#0x1_token_bucket_Bucket">token_bucket::Bucket</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_delegation_Delegation"></a>

## Resource `Delegation`



<pre><code><b>struct</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle_bundles: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>, <a href="permissioned_delegation.md#0x1_permissioned_delegation_HandleBundle">permissioned_delegation::HandleBundle</a>&gt;</code>
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



<a id="0x1_permissioned_delegation_EHANDLE_EXISTENCE"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_EHANDLE_EXISTENCE">EHANDLE_EXISTENCE</a>: u64 = 5;
</code></pre>



<a id="0x1_permissioned_delegation_EINVALID_SIGNATURE"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 4;
</code></pre>



<a id="0x1_permissioned_delegation_EPUBLIC_KEY_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_EPUBLIC_KEY_NOT_FOUND">EPUBLIC_KEY_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x1_permissioned_delegation_ERATE_LIMITED"></a>



<pre><code><b>const</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_ERATE_LIMITED">ERATE_LIMITED</a>: u64 = 6;
</code></pre>



<a id="0x1_permissioned_delegation_fetch_handle"></a>

## Function `fetch_handle`



<pre><code><b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_fetch_handle">fetch_handle</a>(bundle: &<b>mut</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_HandleBundle">permissioned_delegation::HandleBundle</a>, check_rate_limit: bool): &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_fetch_handle">fetch_handle</a>(bundle: &<b>mut</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_HandleBundle">HandleBundle</a>, check_rate_limit: bool): &StorablePermissionedHandle {
    <b>let</b> <a href="token_bucket.md#0x1_token_bucket">token_bucket</a> = &<b>mut</b> bundle.bucket;
    <b>if</b> (check_rate_limit && <a href="token_bucket.md#0x1_token_bucket">token_bucket</a>.is_some()) {
        <b>assert</b>!(<a href="token_bucket.md#0x1_token_bucket_request">token_bucket::request</a>(<a href="token_bucket.md#0x1_token_bucket">token_bucket</a>.borrow_mut(), 1), std::error::permission_denied(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ERATE_LIMITED">ERATE_LIMITED</a>));
    };
    &bundle.handle
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_add_permissioned_handle"></a>

## Function `add_permissioned_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_add_permissioned_handle">add_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, max_txn_per_minute: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, expiration_time: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_add_permissioned_handle">add_permissioned_handle</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    max_txn_per_minute: Option&lt;u64&gt;,
    expiration_time: u64,
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>assert</b>!(!is_permissioned_signer(master), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
    <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(key);
    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(addr)) {
        <b>move_to</b>(master, <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
            handle_bundles: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>()
        });
    };
    <b>let</b> handles = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(addr).handle_bundles;
    <b>assert</b>!(!handles.contains(pubkey), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EHANDLE_EXISTENCE">EHANDLE_EXISTENCE</a>));
    <b>let</b> handle = <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">permissioned_signer::create_storable_permissioned_handle</a>(master, expiration_time);
    <b>let</b> bucket = max_txn_per_minute.map(|capacity|<a href="token_bucket.md#0x1_token_bucket_initialize_bucket">token_bucket::initialize_bucket</a>(capacity));
    <b>let</b> <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a> = <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(&handle);
    handles.add(pubkey, HandleBundle::V1 { bucket, handle });
    <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_remove_permissioned_handle"></a>

## Function `remove_permissioned_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_remove_permissioned_handle">remove_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_remove_permissioned_handle">remove_permissioned_handle</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>assert</b>!(!is_permissioned_signer(master), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
    <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(key);
    <b>let</b> handle_bundles = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(addr).handle_bundles;
    <b>assert</b>!(handle_bundles.contains(pubkey), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EHANDLE_EXISTENCE">EHANDLE_EXISTENCE</a>));
    <b>let</b> bundle = handle_bundles.remove(pubkey);
    match (bundle) {
        HandleBundle::V1 { handle, bucket: _ } =&gt; {
            <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_storable_permissioned_handle">permissioned_signer::destroy_storable_permissioned_handle</a>(handle);
        }
    };
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_permissioned_signer_by_key"></a>

## Function `permissioned_signer_by_key`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_permissioned_signer_by_key">permissioned_signer_by_key</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_permissioned_signer_by_key">permissioned_signer_by_key</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>assert</b>!(!is_permissioned_signer(master), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
    <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(key);
    <b>let</b> handle = <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(addr, pubkey, <b>false</b>);
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(handle)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_handle_address_by_key"></a>

## Function `handle_address_by_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_handle_address_by_key">handle_address_by_key</a>(master: <b>address</b>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_handle_address_by_key">handle_address_by_key</a>(master: <b>address</b>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(key);
    <b>let</b> handle = <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(master, pubkey, <b>false</b>);
    <a href="permissioned_signer.md#0x1_permissioned_signer_permissions_storage_address">permissioned_signer::permissions_storage_address</a>(handle)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_authenticate"></a>

## Function `authenticate`

Authorization function for account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, abstraction_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, abstraction_auth_data: AbstractionAuthData): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*<a href="auth_data.md#0x1_auth_data_authenticator">auth_data::authenticator</a>(&abstraction_auth_data));
    <b>let</b> public_key = new_unvalidated_public_key_from_bytes(
        <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x))
    );
    <b>let</b> signature = new_signature_from_bytes(
        <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x))
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
            &signature,
            &public_key,
            *<a href="auth_data.md#0x1_auth_data_digest">auth_data::digest</a>(&abstraction_auth_data),
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    );
    <b>let</b> handle = <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(addr, public_key, <b>true</b>);
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">permissioned_signer::signer_from_storable_permissioned_handle</a>(handle)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_get_storable_permissioned_handle"></a>

## Function `get_storable_permissioned_handle`



<pre><code><b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(master: <b>address</b>, pubkey: <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>, count_rate: bool): &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_storable_permissioned_handle">get_storable_permissioned_handle</a>(
    master: <b>address</b>,
    pubkey: UnvalidatedPublicKey,
    count_rate: bool
): &StorablePermissionedHandle {
    <b>if</b> (<b>exists</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(master)) {
        <b>let</b> bundles = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(master).handle_bundles;
        <b>if</b> (bundles.contains(pubkey)) {
            <a href="permissioned_delegation.md#0x1_permissioned_delegation_fetch_handle">fetch_handle</a>(bundles.borrow_mut(pubkey), count_rate)
        } <b>else</b> {
            <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
        }
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
