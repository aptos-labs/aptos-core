
<a id="0x1_permissioned_delegation"></a>

# Module `0x1::permissioned_delegation`



-  [Resource `Delegation`](#0x1_permissioned_delegation_Delegation)
-  [Constants](#@Constants_0)
-  [Function `add_permissioned_handle`](#0x1_permissioned_delegation_add_permissioned_handle)
-  [Function `remove_permissioned_handle`](#0x1_permissioned_delegation_remove_permissioned_handle)
-  [Function `permissioned_signer_by_key`](#0x1_permissioned_delegation_permissioned_signer_by_key)
-  [Function `remove_permissioned_handle_by_delegate`](#0x1_permissioned_delegation_remove_permissioned_handle_by_delegate)
-  [Function `handle_address_by_key`](#0x1_permissioned_delegation_handle_address_by_key)
-  [Function `authenticate`](#0x1_permissioned_delegation_authenticate)
-  [Function `get_permissioned_signer`](#0x1_permissioned_delegation_get_permissioned_signer)


<pre><code><b>use</b> <a href="bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
</code></pre>



<a id="0x1_permissioned_delegation_Delegation"></a>

## Resource `Delegation`



<pre><code><b>struct</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handles: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>&gt;</code>
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



<a id="0x1_permissioned_delegation_add_permissioned_handle"></a>

## Function `add_permissioned_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_add_permissioned_handle">add_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, expiration_time: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_add_permissioned_handle">add_permissioned_handle</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    expiration_time: u64,
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>assert</b>!(!is_permissioned_signer(master), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
    <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(key);
    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(addr)) {
        <b>move_to</b>(master, <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
            handles: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>()
        });
    };
    <b>let</b> handles = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(addr).handles;
    <b>assert</b>!(!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(handles, pubkey), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EHANDLE_EXISTENCE">EHANDLE_EXISTENCE</a>));
    <b>let</b> handle = <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">permissioned_signer::create_storable_permissioned_handle</a>(master, expiration_time);
    <b>let</b> <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a> = <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned">permissioned_signer::signer_from_storable_permissioned</a>(&handle);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(handles, pubkey, handle);
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
    <b>let</b> handles = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(addr).handles;
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(handles, pubkey), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EHANDLE_EXISTENCE">EHANDLE_EXISTENCE</a>));
    <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_storable_permissioned_handle">permissioned_signer::destroy_storable_permissioned_handle</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(handles, pubkey));
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
    <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_permissioned_signer">get_permissioned_signer</a>(addr, pubkey)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_remove_permissioned_handle_by_delegate"></a>

## Function `remove_permissioned_handle_by_delegate`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_remove_permissioned_handle_by_delegate">remove_permissioned_handle_by_delegate</a>(master: <b>address</b>, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_remove_permissioned_handle_by_delegate">remove_permissioned_handle_by_delegate</a>(
    master: <b>address</b>,
    signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): StorablePermissionedHandle <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(signature);
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
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 2, 3],
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    );
    <b>let</b> handles = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(master).handles;
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(handles, public_key), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EHANDLE_EXISTENCE">EHANDLE_EXISTENCE</a>));
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(handles, public_key)
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
    <b>let</b> handles = &<b>borrow_global</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(master).handles;
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(handles, pubkey), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EHANDLE_EXISTENCE">EHANDLE_EXISTENCE</a>));
    <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permissioned_signer::permission_address</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(handles, pubkey))
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_authenticate"></a>

## Function `authenticate`

Authorization function for account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, transaction_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_authenticate">authenticate</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    transaction_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(signature);
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
            transaction_hash,
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    );
    <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_permissioned_signer">get_permissioned_signer</a>(addr, public_key)
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_get_permissioned_signer"></a>

## Function `get_permissioned_signer`



<pre><code><b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_permissioned_signer">get_permissioned_signer</a>(master: <b>address</b>, pubkey: <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_get_permissioned_signer">get_permissioned_signer</a>(master: <b>address</b>, pubkey: UnvalidatedPublicKey): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(master)) {
        <b>let</b> handles = &<b>borrow_global</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(master).handles;
        <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(handles, pubkey)) {
            <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned">permissioned_signer::signer_from_storable_permissioned</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(handles, pubkey));
            <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
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
