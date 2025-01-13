
<a id="0x1_permissioned_signer"></a>

# Module `0x1::permissioned_signer`

A _permissioned signer_ consists of a pair of the original signer and a generated
address which is used store information about associated permissions.

A permissioned signer is a restricted version of a signer. Functions <code><b>move_to</b></code> and
<code>address_of</code> behave the same, and can be passed wherever signer is needed. However,
code can internally query for the permissions to assert additional restrictions on
the use of the signer.

A client which is interested in restricting access granted via a signer can create a permissioned signer
and pass on to other existing code without changes to existing APIs. Core functions in the framework, for
example account functions, can then assert availability of permissions, effectively restricting
existing code in a compatible way.

After introducing the core functionality, examples are provided for withdraw limit on accounts, and
for blind signing.


-  [Resource `GrantedPermissionHandles`](#0x1_permissioned_signer_GrantedPermissionHandles)
-  [Enum `PermissionedHandle`](#0x1_permissioned_signer_PermissionedHandle)
-  [Enum `StorablePermissionedHandle`](#0x1_permissioned_signer_StorablePermissionedHandle)
-  [Enum Resource `PermissionStorage`](#0x1_permissioned_signer_PermissionStorage)
-  [Enum `StoredPermission`](#0x1_permissioned_signer_StoredPermission)
-  [Enum `Permission`](#0x1_permissioned_signer_Permission)
-  [Constants](#@Constants_0)
-  [Function `create_permissioned_handle`](#0x1_permissioned_signer_create_permissioned_handle)
-  [Function `create_storable_permissioned_handle`](#0x1_permissioned_signer_create_storable_permissioned_handle)
-  [Function `destroy_permissioned_handle`](#0x1_permissioned_signer_destroy_permissioned_handle)
-  [Function `destroy_storable_permissioned_handle`](#0x1_permissioned_signer_destroy_storable_permissioned_handle)
-  [Function `destroy_permissions_storage_address`](#0x1_permissioned_signer_destroy_permissions_storage_address)
-  [Function `signer_from_permissioned_handle`](#0x1_permissioned_signer_signer_from_permissioned_handle)
-  [Function `signer_from_storable_permissioned_handle`](#0x1_permissioned_signer_signer_from_storable_permissioned_handle)
-  [Function `revoke_permission_storage_address`](#0x1_permissioned_signer_revoke_permission_storage_address)
-  [Function `revoke_all_handles`](#0x1_permissioned_signer_revoke_all_handles)
-  [Function `permissions_storage_address`](#0x1_permissioned_signer_permissions_storage_address)
-  [Function `assert_master_signer`](#0x1_permissioned_signer_assert_master_signer)
-  [Function `is_above`](#0x1_permissioned_signer_is_above)
-  [Function `consume_capacity`](#0x1_permissioned_signer_consume_capacity)
-  [Function `increase_capacity`](#0x1_permissioned_signer_increase_capacity)
-  [Function `merge`](#0x1_permissioned_signer_merge)
-  [Function `map_or`](#0x1_permissioned_signer_map_or)
-  [Function `insert_or`](#0x1_permissioned_signer_insert_or)
-  [Function `authorize`](#0x1_permissioned_signer_authorize)
-  [Function `authorize_unlimited`](#0x1_permissioned_signer_authorize_unlimited)
-  [Function `check_permission_exists`](#0x1_permissioned_signer_check_permission_exists)
-  [Function `check_permission_capacity_above`](#0x1_permissioned_signer_check_permission_capacity_above)
-  [Function `check_permission_consume`](#0x1_permissioned_signer_check_permission_consume)
-  [Function `capacity`](#0x1_permissioned_signer_capacity)
-  [Function `revoke_permission`](#0x1_permissioned_signer_revoke_permission)
-  [Function `extract_permission`](#0x1_permissioned_signer_extract_permission)
-  [Function `extract_all_permission`](#0x1_permissioned_signer_extract_all_permission)
-  [Function `address_of`](#0x1_permissioned_signer_address_of)
-  [Function `consume_permission`](#0x1_permissioned_signer_consume_permission)
-  [Function `store_permission`](#0x1_permissioned_signer_store_permission)
-  [Function `is_permissioned_signer`](#0x1_permissioned_signer_is_permissioned_signer)
-  [Function `permission_address`](#0x1_permissioned_signer_permission_address)
-  [Function `signer_from_permissioned_handle_impl`](#0x1_permissioned_signer_signer_from_permissioned_handle_impl)
-  [Specification](#@Specification_1)
    -  [Function `create_permissioned_handle`](#@Specification_1_create_permissioned_handle)
    -  [Function `create_storable_permissioned_handle`](#@Specification_1_create_storable_permissioned_handle)
    -  [Function `destroy_permissioned_handle`](#@Specification_1_destroy_permissioned_handle)
    -  [Function `destroy_storable_permissioned_handle`](#@Specification_1_destroy_storable_permissioned_handle)
    -  [Function `revoke_permission_storage_address`](#@Specification_1_revoke_permission_storage_address)
    -  [Function `authorize`](#@Specification_1_authorize)
    -  [Function `check_permission_exists`](#@Specification_1_check_permission_exists)
    -  [Function `check_permission_capacity_above`](#@Specification_1_check_permission_capacity_above)
    -  [Function `check_permission_consume`](#@Specification_1_check_permission_consume)
    -  [Function `capacity`](#@Specification_1_capacity)
    -  [Function `consume_permission`](#@Specification_1_consume_permission)
    -  [Function `is_permissioned_signer`](#@Specification_1_is_permissioned_signer)
    -  [Function `permission_address`](#@Specification_1_permission_address)
    -  [Function `signer_from_permissioned_handle_impl`](#@Specification_1_signer_from_permissioned_handle_impl)


<pre><code><b>use</b> <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_permissioned_signer_GrantedPermissionHandles"></a>

## Resource `GrantedPermissionHandles`



<pre><code><b>struct</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active_handles: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_permissioned_signer_PermissionedHandle"></a>

## Enum `PermissionedHandle`



<pre><code>enum <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a>
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>master_account_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>permissions_storage_addr: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_signer_StorablePermissionedHandle"></a>

## Enum `StorablePermissionedHandle`



<pre><code>enum <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a> <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>master_account_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>permissions_storage_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_time: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_signer_PermissionStorage"></a>

## Enum Resource `PermissionStorage`



<pre><code>enum <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> <b>has</b> key
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>perms: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_signer_StoredPermission"></a>

## Enum `StoredPermission`



<pre><code>enum <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Unlimited</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>Capacity</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: u256</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_signer_Permission"></a>

## Enum `Permission`



<pre><code>enum <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;K&gt;
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>key: K</code>
</dt>
<dd>

</dd>
<dt>
<code>perm: <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_permissioned_signer_ECANNOT_AUTHORIZE"></a>

Cannot authorize a permission.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_AUTHORIZE">ECANNOT_AUTHORIZE</a>: u64 = 2;
</code></pre>



<a id="0x1_permissioned_signer_ECANNOT_EXTRACT_PERMISSION"></a>

signer doesn't have enough capacity to extract permission.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_EXTRACT_PERMISSION">ECANNOT_EXTRACT_PERMISSION</a>: u64 = 4;
</code></pre>



<a id="0x1_permissioned_signer_ENOT_MASTER_SIGNER"></a>

Trying to grant permission using master signer.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>: u64 = 1;
</code></pre>



<a id="0x1_permissioned_signer_ENOT_PERMISSIONED_SIGNER"></a>

Access permission information from a master signer.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_PERMISSIONED_SIGNER">ENOT_PERMISSIONED_SIGNER</a>: u64 = 3;
</code></pre>



<a id="0x1_permissioned_signer_E_NOT_ACTIVE"></a>

destroying permission handle that has already been revoked or not owned by the
given master signer.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_E_NOT_ACTIVE">E_NOT_ACTIVE</a>: u64 = 8;
</code></pre>



<a id="0x1_permissioned_signer_E_PERMISSION_EXPIRED"></a>

permission handle has expired.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_EXPIRED">E_PERMISSION_EXPIRED</a>: u64 = 5;
</code></pre>



<a id="0x1_permissioned_signer_E_PERMISSION_MISMATCH"></a>

storing extracted permission into a different signer.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_MISMATCH">E_PERMISSION_MISMATCH</a>: u64 = 6;
</code></pre>



<a id="0x1_permissioned_signer_E_PERMISSION_REVOKED"></a>

permission handle has been revoked by the original signer.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_REVOKED">E_PERMISSION_REVOKED</a>: u64 = 7;
</code></pre>



<a id="0x1_permissioned_signer_U256_MAX"></a>



<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_U256_MAX">U256_MAX</a>: u256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;
</code></pre>



<a id="0x1_permissioned_signer_create_permissioned_handle"></a>

## Function `create_permissioned_handle`

Create an ephermeral permission handle based on the master signer.

This handle can be used to derive a signer that can be used in the context of
the current transaction.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_permissioned_handle">create_permissioned_handle</a>(master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_permissioned_handle">create_permissioned_handle</a>(master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a> {
    <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(master);
    <b>let</b> permissions_storage_addr = generate_auid_address();
    <b>let</b> master_account_addr = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);

    <b>move_to</b>(
        &<a href="create_signer.md#0x1_create_signer">create_signer</a>(permissions_storage_addr),
        PermissionStorage::V1 { perms: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>() }
    );

    PermissionedHandle::V1 { master_account_addr, permissions_storage_addr }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_create_storable_permissioned_handle"></a>

## Function `create_storable_permissioned_handle`

Create an storable permission handle based on the master signer.

This handle can be used to derive a signer that can be stored by a smart contract.
This is as dangerous as key delegation, thus it remains public(package) for now.

The caller should check if <code>expiration_time</code> is not too far in the future.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">create_storable_permissioned_handle</a>(master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, expiration_time: u64): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">create_storable_permissioned_handle</a>(
    master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, expiration_time: u64
): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a> <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> {
    <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(master);
    <b>let</b> permissions_storage_addr = generate_auid_address();
    <b>let</b> master_account_addr = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);

    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; expiration_time,
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_EXPIRED">E_PERMISSION_EXPIRED</a>)
    );

    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr)) {
        <b>move_to</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(
            master, <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> { active_handles: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_empty">vector::empty</a>() }
        );
    };

    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr).active_handles,
        permissions_storage_addr
    );

    <b>move_to</b>(
        &<a href="create_signer.md#0x1_create_signer">create_signer</a>(permissions_storage_addr),
        PermissionStorage::V1 { perms: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>() }
    );

    StorablePermissionedHandle::V1 {
        master_account_addr,
        permissions_storage_addr,
        expiration_time
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_destroy_permissioned_handle"></a>

## Function `destroy_permissioned_handle`

Destroys an ephermeral permission handle. Clean up the permission stored in that handle


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissioned_handle">destroy_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissioned_handle">destroy_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a>) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>let</b> PermissionedHandle::V1 { master_account_addr: _, permissions_storage_addr } =
        p;
    <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(permissions_storage_addr);
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_destroy_storable_permissioned_handle"></a>

## Function `destroy_storable_permissioned_handle`

Destroys a storable permission handle. Clean up the permission stored in that handle


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_storable_permissioned_handle">destroy_storable_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_storable_permissioned_handle">destroy_storable_permissioned_handle</a>(
    p: <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a>
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> {
    <b>let</b> StorablePermissionedHandle::V1 {
        master_account_addr,
        permissions_storage_addr,
        expiration_time: _
    } = p;

    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_REVOKED">E_PERMISSION_REVOKED</a>),
    );
    <b>let</b> granted_permissions =
        <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr);
    <b>let</b> (found, idx) = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_index_of">vector::index_of</a>(
        &granted_permissions.active_handles, &permissions_storage_addr
    );

    // Removing the <b>address</b> from the active handle list <b>if</b> it's still active.
    <b>if</b>(found) {
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> granted_permissions.active_handles, idx);
    };

    <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(permissions_storage_addr);
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_destroy_permissions_storage_address"></a>

## Function `destroy_permissions_storage_address`



<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(permissions_storage_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(
    permissions_storage_addr: <b>address</b>
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissions_storage_addr)) {
        <b>let</b> PermissionStorage::V1 { perms } =
            <b>move_from</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissions_storage_addr);
        <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_destroy">simple_map::destroy</a>(
            perms,
            |_dk| {},
            |_dv| {}
        );
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_permissioned_handle"></a>

## Function `signer_from_permissioned_handle`

Generate the permissioned signer based on the ephermeral permission handle.

This signer can be used as a regular signer for other smart contracts. However when such
signer interacts with various framework functions, it would subject to permission checks
and would abort if check fails.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle">signer_from_permissioned_handle</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle">signer_from_permissioned_handle</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a> {
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(
        p.master_account_addr, p.permissions_storage_addr
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_storable_permissioned_handle"></a>

## Function `signer_from_storable_permissioned_handle`

Generate the permissioned signer based on the storable permission handle.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">signer_from_storable_permissioned_handle</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">signer_from_storable_permissioned_handle</a>(
    p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a>
): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a> {
    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; p.expiration_time,
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_EXPIRED">E_PERMISSION_EXPIRED</a>)
    );
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(p.permissions_storage_addr),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_REVOKED">E_PERMISSION_REVOKED</a>)
    );
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(
        p.master_account_addr, p.permissions_storage_addr
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_revoke_permission_storage_address"></a>

## Function `revoke_permission_storage_address`

Revoke a specific storable permission handle immediately. This would disallow owner of
the storable permission handle to derive signer from it anymore.


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission_storage_address">revoke_permission_storage_address</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, permissions_storage_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission_storage_address">revoke_permission_storage_address</a>(
    s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, permissions_storage_addr: <b>address</b>
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s), <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>)
    );
    <b>let</b> master_account_addr = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s);

    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_REVOKED">E_PERMISSION_REVOKED</a>),
    );
    <b>let</b> granted_permissions =
        <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr);
    <b>let</b> (found, idx) = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_index_of">vector::index_of</a>(
        &granted_permissions.active_handles, &permissions_storage_addr
    );

    // The <b>address</b> <b>has</b> <b>to</b> be in the activated list in the master <a href="account.md#0x1_account">account</a> <b>address</b>.
    <b>assert</b>!(found, <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_NOT_ACTIVE">E_NOT_ACTIVE</a>));
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> granted_permissions.active_handles, idx);
    <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(permissions_storage_addr);
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_revoke_all_handles"></a>

## Function `revoke_all_handles`

Revoke all storable permission handle of the signer immediately.


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_all_handles">revoke_all_handles</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_all_handles">revoke_all_handles</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s), <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>)
    );
    <b>let</b> master_account_addr = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s);
    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr)) { <b>return</b> };

    <b>let</b> granted_permissions =
        <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr);
    <b>let</b> delete_list = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_trim_reverse">vector::trim_reverse</a>(
        &<b>mut</b> granted_permissions.active_handles, 0
    );
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_destroy">vector::destroy</a>(
        delete_list,
        |<b>address</b>| {
            <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(<b>address</b>);
        }
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_permissions_storage_address"></a>

## Function `permissions_storage_address`

Return the permission handle address so that it could be used for revocation purpose.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permissions_storage_address">permissions_storage_address</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permissions_storage_address">permissions_storage_address</a>(
    p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a>
): <b>address</b> {
    p.permissions_storage_addr
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_assert_master_signer"></a>

## Function `assert_master_signer`

Helper function that would abort if the signer passed in is a permissioned signer.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s), <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>)
    );
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_is_above"></a>

## Function `is_above`

=====================================================================================================
StoredPermission operations

check if StoredPermission has at least <code>threshold</code> capacity.


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_above">is_above</a>(perm: &<a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>, threshold: u256): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_above">is_above</a>(perm: &<a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>, threshold: u256): bool {
    match (perm) {
        StoredPermission::Capacity(capacity) =&gt; *capacity &gt; threshold,
        StoredPermission::Unlimited =&gt; <b>true</b>,
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_consume_capacity"></a>

## Function `consume_capacity`

consume <code>threshold</code> capacity from StoredPermission


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_consume_capacity">consume_capacity</a>(perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>, threshold: u256): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_consume_capacity">consume_capacity</a>(perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>, threshold: u256): bool {
    match (perm) {
        StoredPermission::Capacity(current_capacity) =&gt; {
            <b>if</b> (*current_capacity &gt;= threshold) {
                *current_capacity = *current_capacity - threshold;
                <b>true</b>
            } <b>else</b> { <b>false</b> }
        }
        StoredPermission::Unlimited =&gt; <b>true</b>
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_increase_capacity"></a>

## Function `increase_capacity`

increase <code>threshold</code> capacity from StoredPermission


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_increase_capacity">increase_capacity</a>(perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>, threshold: u256)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_increase_capacity">increase_capacity</a>(perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>, threshold: u256) {
    match (perm) {
        StoredPermission::Capacity(current_capacity) =&gt; {
            *current_capacity = *current_capacity + threshold;
        }
        StoredPermission::Unlimited =&gt; (),
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_merge"></a>

## Function `merge`

merge the two stored permission


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_merge">merge</a>(lhs: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>, rhs: <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_merge">merge</a>(lhs: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>, rhs: <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>) {
    match (rhs) {
        StoredPermission::Capacity(new_capacity) =&gt; {
            match (lhs) {
                StoredPermission::Capacity(current_capacity) =&gt; {
                    *current_capacity = *current_capacity + new_capacity;
                }
                StoredPermission::Unlimited =&gt; (),
            }
        }
        StoredPermission::Unlimited =&gt; *lhs = StoredPermission::Unlimited,
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_map_or"></a>

## Function `map_or`

=====================================================================================================
Permission Management

Authorizes <code>permissioned</code> with the given permission. This requires to have access to the <code>master</code>
signer.


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_map_or">map_or</a>&lt;PermKey: <b>copy</b>, drop, store, T&gt;(permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey, mutate: |&<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>|T, default: T): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_map_or">map_or</a>&lt;PermKey: <b>copy</b> + drop + store, T&gt;(
    permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>,
    perm: PermKey,
    mutate: |&<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>| T,
    default: T,
): T {
    <b>let</b> permission_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permission_signer_addr),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_NOT_ACTIVE">E_NOT_ACTIVE</a>)
    );
    <b>let</b> perms =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permission_signer_addr).perms;
    <b>let</b> key = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>if</b> (<a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(perms, &key)) {
        mutate(<a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(perms, &key))
    } <b>else</b> {
        default
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_insert_or"></a>

## Function `insert_or`



<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_insert_or">insert_or</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey, mutate: |&<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>|, default: <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_insert_or">insert_or</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>,
    perm: PermKey,
    mutate: |&<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>|,
    default: <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>,
) {
    <b>let</b> permission_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permission_signer_addr),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_NOT_ACTIVE">E_NOT_ACTIVE</a>)
    );
    <b>let</b> perms =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permission_signer_addr).perms;
    <b>let</b> key = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>if</b> (<a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(perms, &key)) {
        mutate(<a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(perms, &key));
    } <b>else</b> {
        <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(perms, key, default);
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_authorize"></a>

## Function `authorize`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize">authorize</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, capacity: u256, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize">authorize</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>,
    capacity: u256,
    perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)
            && !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(master)
            && <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master) == <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_AUTHORIZE">ECANNOT_AUTHORIZE</a>)
    );
    <a href="permissioned_signer.md#0x1_permissioned_signer_insert_or">insert_or</a>(
        permissioned,
        perm,
        |stored_permission| {
            <a href="permissioned_signer.md#0x1_permissioned_signer_increase_capacity">increase_capacity</a>(stored_permission, capacity);
        },
        StoredPermission::Capacity(capacity),
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_authorize_unlimited"></a>

## Function `authorize_unlimited`

Authorizes <code>permissioned</code> with the given unlimited permission.
Unlimited permission can be consumed however many times.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">authorize_unlimited</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">authorize_unlimited</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>,
    perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)
            && !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(master)
            && <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master) == <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_AUTHORIZE">ECANNOT_AUTHORIZE</a>)
    );
    <a href="permissioned_signer.md#0x1_permissioned_signer_insert_or">insert_or</a>(
        permissioned,
        perm,
        |stored_permission| {
            *stored_permission = StoredPermission::Unlimited;
        },
        StoredPermission::Unlimited,
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_check_permission_exists"></a>

## Function `check_permission_exists`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>(s, 0, perm)
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_check_permission_capacity_above"></a>

## Function `check_permission_capacity_above`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        // master <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a> <b>has</b> all permissions
        <b>return</b> <b>true</b>
    };
    <a href="permissioned_signer.md#0x1_permissioned_signer_map_or">map_or</a>(
        s,
        perm,
        |stored_permission| {
            <a href="permissioned_signer.md#0x1_permissioned_signer_is_above">is_above</a>(stored_permission, threshold)
        },
        <b>false</b>,
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_check_permission_consume"></a>

## Function `check_permission_consume`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        // master <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a> <b>has</b> all permissions
        <b>return</b> <b>true</b>
    };
    <a href="permissioned_signer.md#0x1_permissioned_signer_map_or">map_or</a>(
        s,
        perm,
        |stored_permission| {
             <a href="permissioned_signer.md#0x1_permissioned_signer_consume_capacity">consume_capacity</a>(stored_permission, threshold)
        },
        <b>false</b>,
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_capacity"></a>

## Function `capacity`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_capacity">capacity</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_capacity">capacity</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey
): Option&lt;u256&gt; <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        <b>return</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_U256_MAX">U256_MAX</a>)
    };
    <a href="permissioned_signer.md#0x1_permissioned_signer_map_or">map_or</a>(
        s,
        perm,
        |stored_permission: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>| {
            <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(match (stored_permission) {
                StoredPermission::Capacity(capacity) =&gt; *capacity,
                StoredPermission::Unlimited =&gt; <a href="permissioned_signer.md#0x1_permissioned_signer_U256_MAX">U256_MAX</a>,
            })
        },
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>(),
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_revoke_permission"></a>

## Function `revoke_permission`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission">revoke_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission">revoke_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)) {
        // Master <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a> <b>has</b> no permissions associated <b>with</b> it.
        <b>return</b>
    };
    <b>let</b> addr = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned);
    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(addr)) { <b>return</b> };
    <b>let</b> perm_storage = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(addr).perms;
    <b>let</b> key = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>if</b> (<a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(perm_storage, &key)) {
        <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(
            &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(addr).perms,
            &<a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm)
        );
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_extract_permission"></a>

## Function `extract_permission`

=====================================================================================================
Another flavor of api to extract and store permissions


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_extract_permission">extract_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, weight: u256, perm: PermKey): <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_extract_permission">extract_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, weight: u256, perm: PermKey
): <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt; <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>(s, weight, perm),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_EXTRACT_PERMISSION">ECANNOT_EXTRACT_PERMISSION</a>)
    );
    Permission::V1 {
        owner_address: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s),
        key: perm,
        perm: StoredPermission::Capacity(weight),
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_extract_all_permission"></a>

## Function `extract_all_permission`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_extract_all_permission">extract_all_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm_key: PermKey): <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_extract_all_permission">extract_all_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm_key: PermKey
): <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt; <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_EXTRACT_PERMISSION">ECANNOT_EXTRACT_PERMISSION</a>)
    );
    <b>let</b> addr = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(s);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(addr),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_EXTRACT_PERMISSION">ECANNOT_EXTRACT_PERMISSION</a>)
    );
    <b>let</b> key = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm_key);
    <b>let</b> storage = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(addr).perms;
    <b>let</b> (_, value) = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(storage, &key);

    Permission::V1 {
        owner_address: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s),
        key: perm_key,
        perm: value,
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_address_of"></a>

## Function `address_of`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">address_of</a>&lt;PermKey&gt;(perm: &<a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">address_of</a>&lt;PermKey&gt;(perm: &<a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt;): <b>address</b> {
    perm.owner_address
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_consume_permission"></a>

## Function `consume_permission`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_consume_permission">consume_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;, weight: u256, perm_key: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_consume_permission">consume_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt;, weight: u256, perm_key: PermKey
): bool {
    <b>if</b> (perm.key != perm_key) {
        <b>return</b> <b>false</b>
    };
    <a href="permissioned_signer.md#0x1_permissioned_signer_consume_capacity">consume_capacity</a>(&<b>mut</b> perm.perm, weight)
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_store_permission"></a>

## Function `store_permission`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_store_permission">store_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_store_permission">store_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt;
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s),
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_PERMISSIONED_SIGNER">ENOT_PERMISSIONED_SIGNER</a>)
    );
    <b>let</b> Permission::V1 { key, perm, owner_address } = perm;

    <b>assert</b>!(
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s) == owner_address,
        <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_MISMATCH">E_PERMISSION_MISMATCH</a>)
    );

    <a href="permissioned_signer.md#0x1_permissioned_signer_insert_or">insert_or</a>(
        s,
        key,
        |stored_permission| {
            <a href="permissioned_signer.md#0x1_permissioned_signer_merge">merge</a>(stored_permission, perm);
        },
        perm,
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_is_permissioned_signer"></a>

## Function `is_permissioned_signer`


Check whether this is a permissioned signer.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): bool;
</code></pre>



</details>

<a id="0x1_permissioned_signer_permission_address"></a>

## Function `permission_address`

Return the address used for storing permissions. Aborts if not a permissioned signer.


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): <b>address</b>;
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_permissioned_handle_impl"></a>

## Function `signer_from_permissioned_handle_impl`

Creates a permissioned signer from an existing universal signer. The function aborts if the
given signer is already a permissioned signer.

The implementation of this function requires to extend the value representation for signers in the VM.
invariants:
signer::address_of(master) == signer::address_of(signer_from_permissioned_handle(create_permissioned_handle(master))),


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(master_account_addr: <b>address</b>, permissions_storage_addr: <b>address</b>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(
    master_account_addr: <b>address</b>, permissions_storage_addr: <b>address</b>
): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
<b>axiom</b> <b>forall</b> a: <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>:
    (
        <b>forall</b> i in 0..len(a.active_handles):
            <b>forall</b> j in 0..len(a.active_handles):
                i != j ==&gt;
                    a.active_handles[i] != a.active_handles[j]
    );
</code></pre>




<a id="0x1_permissioned_signer_spec_is_permissioned_signer"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): bool;
</code></pre>



<a id="@Specification_1_create_permissioned_handle"></a>

### Function `create_permissioned_handle`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_permissioned_handle">create_permissioned_handle</a>(master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(master);
<b>let</b> permissions_storage_addr = <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">transaction_context::spec_generate_unique_address</a>();
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissions_storage_addr);
<b>let</b> master_account_addr = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
<b>ensures</b> result.master_account_addr == master_account_addr;
<b>ensures</b> result.permissions_storage_addr == permissions_storage_addr;
</code></pre>



<a id="@Specification_1_create_storable_permissioned_handle"></a>

### Function `create_storable_permissioned_handle`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">create_storable_permissioned_handle</a>(master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, expiration_time: u64): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(master);
<b>let</b> permissions_storage_addr = <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">transaction_context::spec_generate_unique_address</a>();
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissions_storage_addr);
<b>let</b> master_account_addr = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr);
<b>ensures</b> result.master_account_addr == master_account_addr;
<b>ensures</b> result.permissions_storage_addr == permissions_storage_addr;
<b>ensures</b> result.expiration_time == expiration_time;
<b>ensures</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(
    <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr).active_handles,
    permissions_storage_addr
);
<b>ensures</b> <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr);
</code></pre>



<a id="@Specification_1_destroy_permissioned_handle"></a>

### Function `destroy_permissioned_handle`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissioned_handle">destroy_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>)
</code></pre>




<pre><code><b>ensures</b> !<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(p.permissions_storage_addr);
</code></pre>



<a id="@Specification_1_destroy_storable_permissioned_handle"></a>

### Function `destroy_storable_permissioned_handle`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_storable_permissioned_handle">destroy_storable_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>)
</code></pre>




<pre><code><b>ensures</b> !<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(p.permissions_storage_addr);
<b>let</b> <b>post</b> granted_permissions = <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(
    p.master_account_addr
);
</code></pre>



<a id="@Specification_1_revoke_permission_storage_address"></a>

### Function `revoke_permission_storage_address`


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission_storage_address">revoke_permission_storage_address</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, permissions_storage_addr: <b>address</b>)
</code></pre>




<a id="@Specification_1_authorize"></a>

### Function `authorize`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize">authorize</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(master: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, capacity: u256, perm: PermKey)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>aborts_if</b> !<a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(permissioned);
<b>aborts_if</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(master);
<b>aborts_if</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned) != <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
<b>ensures</b> <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(
    <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(permissioned)
);
</code></pre>



<a id="@Specification_1_check_permission_exists"></a>

### Function `check_permission_exists`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey): bool
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_permissioned_signer_spec_check_permission_exists"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_check_permission_exists">spec_check_permission_exists</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(s: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey): bool {
   <b>use</b> aptos_std::type_info;
   <b>use</b> std::bcs;
   <b>let</b> addr = <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s);
   <b>let</b> key = Any {
       type_name: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;PermKey&gt;(),
       data: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs_serialize">bcs::serialize</a>(perm)
   };
   <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s)) { <b>true</b> }
   <b>else</b> <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(addr)) { <b>false</b> }
   <b>else</b> {
       <a href="../../../aptos-stdlib/tests/compiler-v2-doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(addr).perms, key)
   }
}
</code></pre>



<a id="@Specification_1_check_permission_capacity_above"></a>

### Function `check_permission_capacity_above`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>




<pre><code><b>let</b> permissioned_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s);
<b>ensures</b> !<a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s) ==&gt; result == <b>true</b>;
<b>ensures</b> (
    <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s)
        && !<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissioned_signer_addr)
) ==&gt; result == <b>false</b>;
<b>let</b> key = Any {
    type_name: <a href="../../../aptos-stdlib/tests/compiler-v2-doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;SimpleMap&lt;Any, u256&gt;&gt;(),
    data: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs_serialize">bcs::serialize</a>(perm)
};
</code></pre>



<a id="@Specification_1_check_permission_consume"></a>

### Function `check_permission_consume`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>




<pre><code><b>let</b> permissioned_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s);
<b>ensures</b> !<a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s) ==&gt; result == <b>true</b>;
<b>ensures</b> (
    <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s)
        && !<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissioned_signer_addr)
) ==&gt; result == <b>false</b>;
</code></pre>



<a id="@Specification_1_capacity"></a>

### Function `capacity`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_capacity">capacity</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, perm: PermKey): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>




<a id="@Specification_1_consume_permission"></a>

### Function `consume_permission`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_consume_permission">consume_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;, weight: u256, perm_key: PermKey): bool
</code></pre>




<a id="@Specification_1_is_permissioned_signer"></a>

### Function `is_permissioned_signer`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s);
</code></pre>




<a id="0x1_permissioned_signer_spec_permission_address"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): <b>address</b>;
</code></pre>



<a id="@Specification_1_permission_address"></a>

### Function `permission_address`


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract]!<a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(permissioned);
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(permissioned);
</code></pre>




<a id="0x1_permissioned_signer_spec_signer_from_permissioned_handle_impl"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_signer_from_permissioned_handle_impl">spec_signer_from_permissioned_handle_impl</a>(
   master_account_addr: <b>address</b>, permissions_storage_addr: <b>address</b>
): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>;
</code></pre>



<a id="@Specification_1_signer_from_permissioned_handle_impl"></a>

### Function `signer_from_permissioned_handle_impl`


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(master_account_addr: <b>address</b>, permissions_storage_addr: <b>address</b>): <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result
    == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_signer_from_permissioned_handle_impl">spec_signer_from_permissioned_handle_impl</a>(
        master_account_addr, permissions_storage_addr
    );
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
