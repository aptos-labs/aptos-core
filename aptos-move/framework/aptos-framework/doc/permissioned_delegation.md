
<a id="0x1_permissioned_delegation"></a>

# Module `0x1::permissioned_delegation`



-  [Resource `Delegation`](#0x1_permissioned_delegation_Delegation)
-  [Constants](#@Constants_0)
-  [Function `add_permissioned_handle`](#0x1_permissioned_delegation_add_permissioned_handle)
-  [Function `remove_permissioned_handle`](#0x1_permissioned_delegation_remove_permissioned_handle)
-  [Function `authenticate`](#0x1_permissioned_delegation_authenticate)


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
<code>handles: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>&gt;</code>
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



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_add_permissioned_handle">add_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, handle: <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_add_permissioned_handle">add_permissioned_handle</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    handle: PermissionedHandle
) <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
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
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(handles, pubkey, handle);
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
    destroy_permissioned_handle(<a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(handles, pubkey));
}
</code></pre>



</details>

<a id="0x1_permissioned_delegation_authenticate"></a>

## Function `authenticate`

Authorization function for account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(signature);
    <b>let</b> public_key = new_unvalidated_public_key_from_bytes(
        <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x))
    );
    <b>let</b> signature = new_signature_from_bytes(
        <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x))
    );
    // <b>assert</b>!(
    //     <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
    //         &signature,
    //         &public_key,
    //         <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 2, 3],
    //     ),
    //     <a href="permissioned_delegation.md#0x1_permissioned_delegation_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>
    // );
    <b>if</b> (<b>exists</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(addr)) {
        <b>let</b> handles = &<b>borrow_global</b>&lt;<a href="permissioned_delegation.md#0x1_permissioned_delegation_Delegation">Delegation</a>&gt;(addr).handles;
        <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(handles, public_key)) {
            signer_from_permissioned(<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(handles, public_key))
        } <b>else</b> {
            <a href="account.md#0x1_account">account</a>
        }
    } <b>else</b> {
        <a href="account.md#0x1_account">account</a>
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
